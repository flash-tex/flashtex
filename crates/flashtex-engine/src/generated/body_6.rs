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
    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §948
    pub fn push_alignment(&mut self) {
        let mut p: halfword = 0; // §948
        p = self.get_node(align_stack_node_size);
        { let __v968 = self.align_ptr; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v968); }
        { let __v969 = self.cur_align; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v969); }
        { let __v970 = self.mem[crate::ix::U((align_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v970); }
        { let __v971 = self.cur_span; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v971); }
        { let __v972 = self.cur_loop; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v972); }
        { let __v973 = self.align_state; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v973); }
        { let __v974 = self.cur_head; self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_lh(__v974); }
        { let __v975 = self.cur_tail; self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_rh(__v975); }
        { let __v976 = self.cur_pre_head; self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_lh(__v976); }
        { let __v977 = self.cur_pre_tail; self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_rh(__v977); }
        self.align_ptr = p;
        self.cur_head = self.get_avail();
        self.cur_pre_head = self.get_avail();
    }

    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §948
    pub fn pop_alignment(&mut self) {
        let mut p: halfword = 0; // §948
        {
            { let __ix978 = self.cur_head; let __v979 = self.avail; self.mem[crate::ix::U((__ix978) as usize)].set_hh_rh(__v979); }
            self.avail = self.cur_head;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        {
            { let __ix980 = self.cur_pre_head; let __v981 = self.avail; self.mem[crate::ix::U((__ix980) as usize)].set_hh_rh(__v981); }
            self.avail = self.cur_pre_head;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        p = self.align_ptr;
        self.cur_tail = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh();
        self.cur_head = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh();
        self.cur_pre_tail = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_pre_head = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh();
        self.align_state = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
        self.cur_loop = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
        self.cur_span = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
        { let __v982 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U((align_head) as usize)].set_hh_rh(__v982); }
        self.cur_align = self.mem[crate::ix::U((p) as usize)].hh().lh();
        self.align_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
        self.free_node(p, align_stack_node_size);
    }

    /// The preamble is copied directly, except that \.{\\tabskip} causes a change
    /// to the tabskip glue, thereby possibly expanding macros that immediately
    /// follow it. An appearance of \.{\\span} also causes such an expansion.
    /// Note that if the preamble contains `\.{\\global\\tabskip}', the `\.{\\global}'
    /// token survives in the preamble and the `\.{\\tabskip}' defines new
    /// tabskip glue (locally).
    /// @<Declare the procedure called `get_preamble_token`
    // §958
    pub fn get_preamble_token(&mut self) {
        'l_restart_b: loop {
            self.get_token();
            while ((self.cur_chr == span_code) && (self.cur_cmd == tab_mark)) {
                {
                    self.get_token();
                    if (self.cur_cmd > max_command) {
                        {
                            self.expand();
                            self.get_token();
                        }
                    }
                }
            }
            if (self.cur_cmd == endv) {
                self.fatal_error(679i32);
            }
            if ((self.cur_cmd == assign_glue) && (self.cur_chr == 26639i32)) {
                {
                    self.scan_optional_equals();
                    self.scan_glue(glue_val);
                    if (self.eqtb[crate::ix::U(((29320i32) - 1) as usize)].int() > 0i32) {
                        self.geq_define(26639i32, glue_ref, self.cur_val);
                    } else {
                        self.eq_define(26639i32, glue_ref, self.cur_val);
                    }
                    continue 'l_restart_b;
                }
            }
            break 'l_restart_b;
        }
    }

    /// When \.{\\halign} or \.{\\valign} has been scanned in an appropriate
    /// mode, \TeX\ calls `init_align`, whose task is to get everything off to a
    /// good start. This mostly involves scanning the preamble and putting its
    /// information into the preamble list.
    // §950
    pub fn init_align(&mut self) {
        let mut save_cs_ptr: halfword = 0; // §950
        let mut p: halfword = 0; // §950
        'l_done_f: {
            save_cs_ptr = self.cur_cs;
            self.push_alignment();
            self.align_state = (1000000i32).wrapping_neg();
            // §952
            if ((self.cur_list.mode_field == mmode) && ((self.cur_list.tail_field != self.cur_list.head_field) || (self.cur_list.aux_field.int() != null))) {
                {
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(780i32);
                    }
                    self.print_esc(597i32);
                    self.print(1304i32);
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 1305i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1306i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1307i32;
                    }
                    self.error();
                    self.flush_math();
                }
            }
            // §950
            self.push_nest();
            // §951
            if (self.cur_list.mode_field == mmode) {
                {
                    self.cur_list.mode_field = (1i32).wrapping_neg();
                    { let __v983 = self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(2i32)) as usize)].aux_field.int(); self.cur_list.aux_field.set_int(__v983); }
                }
            } else {
                if (self.cur_list.mode_field > 0i32) {
                    self.cur_list.mode_field = (self.cur_list.mode_field).wrapping_neg();
                }
            }
            // §950
            self.scan_spec(align_group, false);
            // §953
            self.mem[crate::ix::U((align_head) as usize)].set_hh_rh(null);
            self.cur_align = align_head;
            self.cur_loop = null;
            self.scanner_status = aligning;
            self.warning_index = save_cs_ptr;
            self.align_state = (1000000i32).wrapping_neg();
            while true {
                {
                    'l_done2_f: {
                        'l_done1_f: {
                            // §954
                            { let __ix984 = self.cur_align; let __v985 = self.new_param_glue(tab_skip_code); self.mem[crate::ix::U((__ix984) as usize)].set_hh_rh(__v985); }
                            self.cur_align = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
                            // §953
                            if (self.cur_cmd == car_ret) {
                                break 'l_done_f;
                            }
                            // §959
                            p = hold_head;
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                            while true {
                                {
                                    self.get_preamble_token();
                                    if (self.cur_cmd == mac_param) {
                                        break 'l_done1_f;
                                    }
                                    if (((self.cur_cmd <= car_ret) && (self.cur_cmd >= tab_mark)) && (self.align_state == (1000000i32).wrapping_neg())) {
                                        if (((p == hold_head) && (self.cur_loop == null)) && (self.cur_cmd == tab_mark)) {
                                            self.cur_loop = self.cur_align;
                                        } else {
                                            {
                                                {
                                                    self.dg_mark();
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(264i32);
                                                    }
                                                    self.print(1313i32);
                                                }
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 1314i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 1315i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 1316i32;
                                                }
                                                self.back_error();
                                                break 'l_done1_f;
                                            }
                                        }
                                    } else {
                                        if ((self.cur_cmd != spacer) || (p != hold_head)) {
                                            {
                                                { let __v986 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v986); }
                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                { let __v987 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v987); }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §955
                        { let __ix988 = self.cur_align; let __v989 = self.new_null_box(); self.mem[crate::ix::U((__ix988) as usize)].set_hh_rh(__v989); }
                        self.cur_align = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
                        { let __ix990 = self.cur_align; self.mem[crate::ix::U((__ix990) as usize)].set_hh_lh(end_span); }
                        { let __ix991 = (self.cur_align).wrapping_add(1i32); self.mem[crate::ix::U((__ix991) as usize)].set_int((1073741824i32).wrapping_neg()); }
                        { let __ix992 = (self.cur_align).wrapping_add(3i32); let __v993 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix992) as usize)].set_int(__v993); }
                        // §960
                        p = hold_head;
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                        while true {
                            {
                                'l_continue_b: loop {
                                    self.get_preamble_token();
                                    if (((self.cur_cmd <= car_ret) && (self.cur_cmd >= tab_mark)) && (self.align_state == (1000000i32).wrapping_neg())) {
                                        break 'l_done2_f;
                                    }
                                    if (self.cur_cmd == mac_param) {
                                        {
                                            {
                                                self.dg_mark();
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(1317i32);
                                            }
                                            {
                                                self.help_ptr = 3i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 1314i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 1315i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 1318i32;
                                            }
                                            self.error();
                                            continue 'l_continue_b;
                                        }
                                    }
                                    { let __v994 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v994); }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    { let __v995 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v995); }
                                    break 'l_continue_b;
                                }
                            }
                        }
                    }
                    { let __v996 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v996); }
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(end_template_token);
                    // §955
                    { let __ix997 = (self.cur_align).wrapping_add(2i32); let __v998 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix997) as usize)].set_int(__v998); }
                }
            }
        }
        // §953
        self.scanner_status = normal;
        // §950
        self.new_save_level(align_group);
        if (self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh() != null) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh(), every_cr_text);
        }
        self.align_peek();
    }

    /// The parameter to `init_span` is a pointer to the alignrecord where the
    /// next column or group of columns will begin. A new semantic level is
    /// entered, so that the columns will generate a list for subsequent packaging.
    /// @<Declare the procedure called `init_span`
    // §963
    pub fn init_span(&mut self, mut p: halfword) {
        self.push_nest();
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            self.cur_list.aux_field.set_hh_lh(1000i32);
        } else {
            {
                { let __v999 = self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v999); }
                self.normal_paragraph();
            }
        }
        self.cur_span = p;
    }

    /// To start a row (i.e., a `row' that rhymes with `dough' but not with `bough'),
    /// we enter a new semantic level, copy the first tabskip glue, and change
    /// from internal vertical mode to restricted horizontal mode or vice versa.
    /// The `space_factor` and `prev_depth` are not used on this semantic level,
    /// but we clear them to zero just to be tidy.
    // §962
    pub fn init_row(&mut self) {
        self.push_nest();
        self.cur_list.mode_field = ((106i32).wrapping_neg()).wrapping_sub(self.cur_list.mode_field);
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            self.cur_list.aux_field.set_hh_lh(0i32);
        } else {
            self.cur_list.aux_field.set_int(0i32);
        }
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1000 = self.cur_list.tail_field; let __v1001 = self.new_glue(self.mem[crate::ix::U(((self.mem[crate::ix::U((align_head) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix1000) as usize)].set_hh_rh(__v1001); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix1002 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1002) as usize)].set_hh_b1(12i32); }
        self.cur_align = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
        self.cur_tail = self.cur_head;
        self.cur_pre_tail = self.cur_pre_head;
        self.init_span(self.cur_align);
    }

    /// When a column begins, we assume that `cur_cmd` is either `omit` or else
    /// the current token should be put back into the input until the \<u_j>
    /// template has been scanned.  (Note that `cur_cmd` might be `tab_mark` or
    /// `car_ret`.)  We also assume that `align_state` is approximately 1000000 at
    /// this time.  We remain in the same mode, and start the template if it is
    /// called for.
    // §964
    pub fn init_col(&mut self) {
        { let __ix1003 = (self.cur_align).wrapping_add(5i32); let __v1004 = self.cur_cmd; self.mem[crate::ix::U((__ix1003) as usize)].set_hh_lh(__v1004); }
        if (self.cur_cmd == omit) {
            self.align_state = 0i32;
        } else {
            {
                self.back_input();
                self.begin_token_list(self.mem[crate::ix::U(((self.cur_align).wrapping_add(3i32)) as usize)].int(), u_template);
            }
        }
    }

    /// When the `endv` command at the end of a \<v_j> template comes through the
    /// scanner, things really start to happen; and it is the `fin_col` routine
    /// that makes them happen. This routine returns `true` if a row as well as a
    /// column has been finished.
    // §967
    pub fn fin_col(&mut self) -> bool {
        let mut fin_col: bool = false;
        let mut p: halfword = 0; // §967
        let mut q: halfword = 0; // §967
        let mut r: halfword = 0; // §967
        let mut s: halfword = 0; // §967
        let mut u: halfword = 0; // §967
        let mut w: scaled = 0; // §967
        let mut o: glue_ord = 0; // §967
        let mut n: halfword = 0; // §967
        'l_exit_f: {
            if (self.cur_align == null) {
                self.confusion(1319i32);
            }
            q = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
            if (q == null) {
                self.confusion(1319i32);
            }
            if (self.align_state < 500000i32) {
                self.fatal_error(679i32);
            }
            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
            // §968
            if ((p == null) && (self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh() < cr_code)) {
                if (self.cur_loop != null) {
                    // §969
                    {
                        { let __v1005 = self.new_null_box(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1005); }
                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(end_span);
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int((1073741824i32).wrapping_neg());
                        self.cur_loop = self.mem[crate::ix::U((self.cur_loop) as usize)].hh().rh();
                        // §970
                        q = hold_head;
                        r = self.mem[crate::ix::U(((self.cur_loop).wrapping_add(3i32)) as usize)].int();
                        while (r != null) {
                            {
                                { let __v1006 = self.get_avail(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1006); }
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                { let __v1007 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v1007); }
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                        { let __v1008 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v1008); }
                        q = hold_head;
                        r = self.mem[crate::ix::U(((self.cur_loop).wrapping_add(2i32)) as usize)].int();
                        while (r != null) {
                            {
                                { let __v1009 = self.get_avail(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1009); }
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                { let __v1010 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v1010); }
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                        { let __v1011 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v1011); }
                        // §969
                        self.cur_loop = self.mem[crate::ix::U((self.cur_loop) as usize)].hh().rh();
                        { let __v1012 = self.new_glue(self.mem[crate::ix::U(((self.cur_loop).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v1012); }
                        { let __ix1013 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1013) as usize)].set_hh_b1(12i32); }
                    }
                } else {
                    // §968
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1320i32);
                        }
                        self.print_esc(1309i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1321i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1322i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1323i32;
                        }
                        { let __ix1014 = (self.cur_align).wrapping_add(5i32); self.mem[crate::ix::U((__ix1014) as usize)].set_hh_lh(cr_code); }
                        self.error();
                    }
                }
            }
            // §967
            if (self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh() != span_code) {
                {
                    self.unsave();
                    self.new_save_level(align_group);
                    // §972
                    {
                        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
                            {
                                self.adjust_tail = self.cur_tail;
                                self.pre_adjust_tail = self.cur_pre_tail;
                                u = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional);
                                w = self.mem[crate::ix::U(((u).wrapping_add(1i32)) as usize)].int();
                                self.cur_tail = self.adjust_tail;
                                self.adjust_tail = null;
                                self.cur_pre_tail = self.pre_adjust_tail;
                                self.pre_adjust_tail = null;
                            }
                        } else {
                            {
                                u = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional, 0i32);
                                w = self.mem[crate::ix::U(((u).wrapping_add(3i32)) as usize)].int();
                            }
                        }
                        n = min_quarterword;
                        if (self.cur_span != self.cur_align) {
                            // §974
                            {
                                q = self.cur_span;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                                    if (q == self.cur_align) { break; }
                                }
                                if (n > max_quarterword) {
                                    self.confusion(1324i32);
                                }
                                q = self.cur_span;
                                while (self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().lh()) as usize)].hh().rh() < n) {
                                    q = self.mem[crate::ix::U((q) as usize)].hh().lh();
                                }
                                if (self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().lh()) as usize)].hh().rh() > n) {
                                    {
                                        s = self.get_node(span_node_size);
                                        { let __v1015 = self.mem[crate::ix::U((q) as usize)].hh().lh(); self.mem[crate::ix::U((s) as usize)].set_hh_lh(__v1015); }
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(n);
                                        self.mem[crate::ix::U((q) as usize)].set_hh_lh(s);
                                        self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].set_int(w);
                                    }
                                } else {
                                    if (self.mem[crate::ix::U(((self.mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int() < w) {
                                        { let __ix1016 = (self.mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1016) as usize)].set_int(w); }
                                    }
                                }
                            }
                        } else {
                            // §972
                            if (w > self.mem[crate::ix::U(((self.cur_align).wrapping_add(1i32)) as usize)].int()) {
                                { let __ix1017 = (self.cur_align).wrapping_add(1i32); self.mem[crate::ix::U((__ix1017) as usize)].set_int(w); }
                            }
                        }
                        self.mem[crate::ix::U((u) as usize)].set_hh_b0(unset_node);
                        self.mem[crate::ix::U((u) as usize)].set_hh_b1(n);
                        // §835
                        if (self.total_stretch[crate::ix::U((filll) as usize)] != 0i32) {
                            o = filll;
                        } else {
                            if (self.total_stretch[crate::ix::U((fill) as usize)] != 0i32) {
                                o = fill;
                            } else {
                                if (self.total_stretch[crate::ix::U((fil) as usize)] != 0i32) {
                                    o = fil;
                                } else {
                                    o = normal;
                                }
                            }
                        }
                        // §972
                        self.mem[crate::ix::U(((u).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                        { let __v1018 = self.total_stretch[crate::ix::U((o) as usize)]; self.mem[crate::ix::U(((u).wrapping_add(6i32)) as usize)].set_int(__v1018); }
                        // §841
                        if (self.total_shrink[crate::ix::U((filll) as usize)] != 0i32) {
                            o = filll;
                        } else {
                            if (self.total_shrink[crate::ix::U((fill) as usize)] != 0i32) {
                                o = fill;
                            } else {
                                if (self.total_shrink[crate::ix::U((fil) as usize)] != 0i32) {
                                    o = fil;
                                } else {
                                    o = normal;
                                }
                            }
                        }
                        // §972
                        self.mem[crate::ix::U(((u).wrapping_add(5i32)) as usize)].set_hh_b0(o);
                        { let __v1019 = self.total_shrink[crate::ix::U((o) as usize)]; self.mem[crate::ix::U(((u).wrapping_add(4i32)) as usize)].set_int(__v1019); }
                        self.pop_nest();
                        { let __ix1020 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1020) as usize)].set_hh_rh(u); }
                        self.cur_list.tail_field = u;
                    }
                    // §971
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1021 = self.cur_list.tail_field; let __v1022 = self.new_glue(self.mem[crate::ix::U(((self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix1021) as usize)].set_hh_rh(__v1022); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix1023 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1023) as usize)].set_hh_b1(12i32); }
                    // §967
                    if (self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh() >= cr_code) {
                        {
                            fin_col = true;
                            break 'l_exit_f;
                        }
                    }
                    self.init_span(p);
                }
            }
            self.align_state = 1000000i32;
            loop {
                self.get_x_or_protected();
                if (self.cur_cmd != spacer) { break; }
            }
            self.cur_align = p;
            self.init_col();
            fin_col = false;
        }
        fin_col
    }

    /// At the end of a row, we append an unset box to the current vlist (for
    /// \.{\\halign}) or the current hlist (for \.{\\valign}). This unset box
    /// contains the unset boxes for the columns, separated by the tabskip glue.
    /// Everything will be set later.
    // §975
    pub fn fin_row(&mut self) {
        let mut p: halfword = 0; // §975
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            {
                p = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional);
                self.pop_nest();
                if (self.cur_pre_head != self.cur_pre_tail) {
                    {
                        { let __ix1024 = self.cur_list.tail_field; let __v1025 = self.mem[crate::ix::U((self.cur_pre_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1024) as usize)].set_hh_rh(__v1025); }
                        self.cur_list.tail_field = self.cur_pre_tail;
                    }
                }
                self.append_to_vlist(p);
                if (self.cur_head != self.cur_tail) {
                    {
                        { let __ix1026 = self.cur_list.tail_field; let __v1027 = self.mem[crate::ix::U((self.cur_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1026) as usize)].set_hh_rh(__v1027); }
                        self.cur_list.tail_field = self.cur_tail;
                    }
                }
            }
        } else {
            {
                p = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional, max_dimen);
                self.pop_nest();
                { let __ix1028 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1028) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(unset_node);
        self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(0i32);
        if (self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh() != null) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh(), every_cr_text);
        }
        self.align_peek();
    }

    /// Finally, we will reach the end of the alignment, and we can breathe a
    /// sigh of relief that memory hasn't overflowed. All the unset boxes will now be
    /// set so that the columns line up, taking due account of spanned columns.
    // §976
    pub fn fin_align(&mut self) {
        let mut p: halfword = 0; // §976
        let mut q: halfword = 0; // §976
        let mut r: halfword = 0; // §976
        let mut s: halfword = 0; // §976
        let mut u: halfword = 0; // §976
        let mut v: halfword = 0; // §976
        let mut t: scaled = 0; // §976
        let mut w: scaled = 0; // §976
        let mut o: scaled = 0; // §976
        let mut n: halfword = 0; // §976
        let mut rule_save: scaled = 0; // §976
        let mut aux_save: memory_word = memory_word::default(); // §976
        if (self.cur_group != align_group) {
            self.confusion(1325i32);
        }
        self.unsave();
        if (self.cur_group != align_group) {
            self.confusion(1326i32);
        }
        self.unsave();
        if (self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(1i32)) as usize)].mode_field == mmode) {
            o = self.eqtb[crate::ix::U(((29918i32) - 1) as usize)].int();
        } else {
            o = 0i32;
        }
        // §977
        q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
        loop {
            self.flush_list(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int());
            self.flush_list(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int());
            p = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                // §978
                {
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                    r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                    s = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh();
                    if (s != zero_glue) {
                        {
                            { let __v1029 = (self.mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v1029); }
                            self.delete_glue_ref(s);
                            self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(zero_glue);
                        }
                    }
                }
            }
            // §977
            if (self.mem[crate::ix::U((q) as usize)].hh().lh() != end_span) {
                // §979
                {
                    t = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.mem[crate::ix::U(((self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int());
                    r = self.mem[crate::ix::U((q) as usize)].hh().lh();
                    s = end_span;
                    self.mem[crate::ix::U((s) as usize)].set_hh_lh(p);
                    n = 1i32;
                    loop {
                        { let __v1030 = (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v1030); }
                        u = self.mem[crate::ix::U((r) as usize)].hh().lh();
                        while (self.mem[crate::ix::U((r) as usize)].hh().rh() > n) {
                            {
                                s = self.mem[crate::ix::U((s) as usize)].hh().lh();
                                n = (self.mem[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32);
                            }
                        }
                        if (self.mem[crate::ix::U((r) as usize)].hh().rh() < n) {
                            {
                                { let __v1031 = self.mem[crate::ix::U((s) as usize)].hh().lh(); self.mem[crate::ix::U((r) as usize)].set_hh_lh(__v1031); }
                                self.mem[crate::ix::U((s) as usize)].set_hh_lh(r);
                                { let __v1032 = (self.mem[crate::ix::U((r) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1032); }
                                s = r;
                            }
                        } else {
                            {
                                if (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((self.mem[crate::ix::U((s) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int()) {
                                    { let __ix1033 = (self.mem[crate::ix::U((s) as usize)].hh().lh()).wrapping_add(1i32); let __v1034 = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix1033) as usize)].set_int(__v1034); }
                                }
                                self.free_node(r, span_node_size);
                            }
                        }
                        r = u;
                        if (r == end_span) { break; }
                    }
                }
            }
            // §977
            self.mem[crate::ix::U((q) as usize)].set_hh_b0(unset_node);
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(min_quarterword);
            self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
            self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
            self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(0i32);
            q = p;
            if (q == null) { break; }
        }
        // §980
        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
        self.pack_begin_line = (self.cur_list.ml_field).wrapping_neg();
        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
            {
                rule_save = self.eqtb[crate::ix::U(((29919i32) - 1) as usize)].int();
                self.eqtb[crate::ix::U(((29919i32) - 1) as usize)].set_int(0i32);
                p = self.hpack(self.mem[crate::ix::U((align_head) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int());
                self.eqtb[crate::ix::U(((29919i32) - 1) as usize)].set_int(rule_save);
            }
        } else {
            {
                q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
                loop {
                    { let __v1035 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1035); }
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                    if (q == null) { break; }
                }
                p = self.vpackage(self.mem[crate::ix::U((align_head) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(), max_dimen);
                q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
                loop {
                    { let __v1036 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1036); }
                    self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                    if (q == null) { break; }
                }
            }
        }
        self.pack_begin_line = 0i32;
        // §981
        q = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
        s = self.cur_list.head_field;
        while (q != null) {
            {
                if (!(q >= self.hi_mem_min)) {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == unset_node) {
                        // §983
                        {
                            if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(hlist_node);
                                    { let __v1037 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1037); }
                                    if (self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(1i32)) as usize)].mode_field == mmode) {
                                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(2i32);
                                    }
                                }
                            } else {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(vlist_node);
                                    { let __v1038 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1038); }
                                }
                            }
                            { let __v1039 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b1(__v1039); }
                            { let __v1040 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b0(__v1040); }
                            { let __v1041 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr(); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_gr(__v1041); }
                            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(o);
                            r = self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh();
                            s = self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh();
                            loop {
                                // §984
                                n = self.mem[crate::ix::U((r) as usize)].hh().b1();
                                t = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int();
                                w = t;
                                u = hold_head;
                                self.mem[crate::ix::U((r) as usize)].set_hh_b1(0i32);
                                while (n > min_quarterword) {
                                    {
                                        n = (n).wrapping_sub(1i32);
                                        // §985
                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                        v = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().lh();
                                        { let __v1042 = self.new_glue(v); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v1042); }
                                        u = self.mem[crate::ix::U((u) as usize)].hh().rh();
                                        self.mem[crate::ix::U((u) as usize)].set_hh_b1(12i32);
                                        t = (t).wrapping_add(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                                        if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() == stretching) {
                                            {
                                                if (self.mem[crate::ix::U((v) as usize)].hh().b0() == self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) {
                                                    t = (t).wrapping_add(crate::system::pas_round((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr() * ((self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()) as f64))));
                                                }
                                            }
                                        } else {
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() == shrinking) {
                                                {
                                                    if (self.mem[crate::ix::U((v) as usize)].hh().b1() == self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) {
                                                        t = (t).wrapping_sub(crate::system::pas_round((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr() * ((self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()) as f64))));
                                                    }
                                                }
                                            }
                                        }
                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                        { let __v1043 = self.new_null_box(); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v1043); }
                                        u = self.mem[crate::ix::U((u) as usize)].hh().rh();
                                        t = (t).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int());
                                        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                            { let __v1044 = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((u).wrapping_add(1i32)) as usize)].set_int(__v1044); }
                                        } else {
                                            {
                                                self.mem[crate::ix::U((u) as usize)].set_hh_b0(vlist_node);
                                                { let __v1045 = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((u).wrapping_add(3i32)) as usize)].set_int(__v1045); }
                                            }
                                        }
                                    }
                                }
                                // §984
                                if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                    // §986
                                    {
                                        { let __v1046 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1046); }
                                        { let __v1047 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v1047); }
                                        if (t == self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()) {
                                            {
                                                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                                                self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                            }
                                        } else {
                                            if (t > self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()) {
                                                {
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        { let __v1048 = ((((t).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int())) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v1048); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v1049 = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(__v1049); }
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b1() == normal) && ((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t) > self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int())) {
                                                            self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                                        } else {
                                                            { let __v1050 = ((((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t)) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v1050); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(hlist_node);
                                    }
                                } else {
                                    // §987
                                    {
                                        { let __v1051 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v1051); }
                                        if (t == self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()) {
                                            {
                                                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                                                self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                            }
                                        } else {
                                            if (t > self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()) {
                                                {
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        { let __v1052 = ((((t).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int())) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v1052); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v1053 = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(__v1053); }
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b1() == normal) && ((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(t) > self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int())) {
                                                            self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                                        } else {
                                                            { let __v1054 = ((((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(t)) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v1054); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(w);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(vlist_node);
                                    }
                                }
                                // §984
                                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                                if (u != hold_head) {
                                    {
                                        { let __v1055 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v1055); }
                                        { let __v1056 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1056); }
                                        r = u;
                                    }
                                }
                                // §983
                                r = self.mem[crate::ix::U((self.mem[crate::ix::U((r) as usize)].hh().rh()) as usize)].hh().rh();
                                s = self.mem[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().rh()) as usize)].hh().rh();
                                if (r == null) { break; }
                            }
                        }
                    } else {
                        // §981
                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == rule_node) {
                            // §982
                            {
                                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v1057 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1057); }
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v1058 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1058); }
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v1059 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1059); }
                                }
                                if (o != 0i32) {
                                    {
                                        r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                                        q = self.hpack(q, 0i32, additional);
                                        self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(o);
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                                    }
                                }
                            }
                        }
                    }
                }
                // §981
                s = q;
                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
            }
        }
        // §976
        self.flush_node_list(p);
        self.pop_alignment();
        // §988
        aux_save = self.cur_list.aux_field;
        p = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
        q = self.cur_list.tail_field;
        self.pop_nest();
        if (self.cur_list.mode_field == mmode) {
            // §1384
            {
                self.do_assignments();
                if (self.cur_cmd != math_shift) {
                    // §1385
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1587i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1305i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1306i32;
                        }
                        self.back_error();
                    }
                } else {
                    // §1375
                    {
                        self.get_x_token();
                        if (self.cur_cmd != math_shift) {
                            {
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1583i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1584i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1585i32;
                                }
                                self.back_error();
                            }
                        }
                    }
                }
                // §1384
                self.flush_node_list(self.cur_list.eTeX_aux_field);
                self.pop_nest();
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1060 = self.cur_list.tail_field; let __v1061 = self.new_penalty(self.eqtb[crate::ix::U(((29288i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix1060) as usize)].set_hh_rh(__v1061); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1062 = self.cur_list.tail_field; let __v1063 = self.new_param_glue(above_display_skip_code); self.mem[crate::ix::U((__ix1062) as usize)].set_hh_rh(__v1063); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                { let __ix1064 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1064) as usize)].set_hh_rh(p); }
                if (p != null) {
                    self.cur_list.tail_field = q;
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1065 = self.cur_list.tail_field; let __v1066 = self.new_penalty(self.eqtb[crate::ix::U(((29289i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix1065) as usize)].set_hh_rh(__v1066); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1067 = self.cur_list.tail_field; let __v1068 = self.new_param_glue(below_display_skip_code); self.mem[crate::ix::U((__ix1067) as usize)].set_hh_rh(__v1068); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                self.cur_list.aux_field.set_int(aux_save.int());
                self.resume_after_display();
            }
        } else {
            // §988
            {
                self.cur_list.aux_field = aux_save;
                { let __ix1069 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1069) as usize)].set_hh_rh(p); }
                if (p != null) {
                    self.cur_list.tail_field = q;
                }
                if (self.cur_list.mode_field == vmode) {
                    self.build_page();
                }
            }
        }
    }

    /// The tricky part about alignments is getting the templates into the
    /// scanner at the right time, and recovering control when a row or column
    /// is finished.
    /// We usually begin a row after each \.{\\cr} has been sensed, unless that
    /// \.{\\cr} is followed by \.{\\noalign} or by the right brace that terminates
    /// the alignment. The `align_peek` routine is used to look ahead and do
    /// the right thing; it either gets a new row started, or gets a \.{\\noalign}
    /// started, or finishes off the alignment.
    /// @<Declare the procedure called `align_peek`
    // §961
    pub fn align_peek(&mut self) {
        'l_restart_b: loop {
            self.align_state = 1000000i32;
            loop {
                self.get_x_or_protected();
                if (self.cur_cmd != spacer) { break; }
            }
            if (self.cur_cmd == no_align) {
                {
                    self.scan_left_brace();
                    self.new_save_level(no_align_group);
                    if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                        self.normal_paragraph();
                    }
                }
            } else {
                if (self.cur_cmd == right_brace) {
                    self.fin_align();
                } else {
                    if ((self.cur_cmd == car_ret) && (self.cur_chr == cr_cr_code)) {
                        continue 'l_restart_b;
                    } else {
                        {
                            self.init_row();
                            self.init_col();
                        }
                    }
                }
            }
            break 'l_restart_b;
        }
    }

    /// @<Declare subprocedures for `line_break`
    // §1002
    pub fn finite_shrink(&mut self, mut p: halfword) -> halfword {
        let mut finite_shrink: halfword = 0;
        let mut q: halfword = 0; // §1002
        if self.no_shrink_error_yet {
            {
                self.no_shrink_error_yet = false;
                if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                    self.end_diagnostic(true);
                }
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1327i32);
                }
                {
                    self.help_ptr = 5i32;
                    self.help_line[crate::ix::U((4i32) as usize)] = 1328i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 1329i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1330i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1331i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1332i32;
                }
                self.error();
                if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                    self.begin_diagnostic();
                }
            }
        }
        q = self.new_spec(p);
        self.mem[crate::ix::U((q) as usize)].set_hh_b1(normal);
        self.delete_glue_ref(p);
        finite_shrink = q;
        finite_shrink
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn push_node(&mut self, mut p: halfword) {
        if (self.hlist_stack_level > max_hlist_stack) {
            self.pdf_error(1333i32, 1334i32);
        }
        self.hlist_stack[crate::ix::U((self.hlist_stack_level) as usize)] = p;
        self.hlist_stack_level = (self.hlist_stack_level).wrapping_add(1i32);
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn pop_node(&mut self) -> halfword {
        let mut pop_node: halfword = 0;
        self.hlist_stack_level = (self.hlist_stack_level).wrapping_sub(1i32);
        if (self.hlist_stack_level < 0i32) {
            self.pdf_error(1335i32, 1336i32);
        }
        pop_node = self.hlist_stack[crate::ix::U((self.hlist_stack_level) as usize)];
        pop_node
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn find_protchar_left(&mut self, mut l: halfword, mut d: bool) -> halfword {
        let mut find_protchar_left: halfword = 0;
        let mut t: halfword = 0; // §1005
        let mut run: bool = false; // §1005
        if ((((((self.mem[crate::ix::U((l) as usize)].hh().rh() != null) && (self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() == null)) {
            l = self.mem[crate::ix::U((l) as usize)].hh().rh();
        } else {
            if d {
                while ((self.mem[crate::ix::U((l) as usize)].hh().rh() != null) && (!((l >= self.hi_mem_min) || (self.mem[crate::ix::U((l) as usize)].hh().b0() < math_node)))) {
                    l = self.mem[crate::ix::U((l) as usize)].hh().rh();
                }
            }
        }
        self.hlist_stack_level = 0i32;
        run = true;
        loop {
            t = l;
            while ((run && (self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() != null)) {
                {
                    self.push_node(l);
                    l = self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh();
                }
            }
            while (run && ((!(l >= self.hi_mem_min)) && ((((((((((self.mem[crate::ix::U((l) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == penalty_node)) || (((self.mem[crate::ix::U((l) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((l) as usize)].hh().b1() != pdf_refximage_node)) && (self.mem[crate::ix::U((l) as usize)].hh().b1() != pdf_refxform_node))) || ((((self.mem[crate::ix::U((l) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (self.mem[crate::ix::U((l) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == kern_node) && (((self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((l) as usize)].hh().b1() == normal)) || (self.mem[crate::ix::U((l) as usize)].hh().b1() == auto_kern)))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() == null))))) {
                {
                    while ((self.mem[crate::ix::U((l) as usize)].hh().rh() == null) && (self.hlist_stack_level > 0i32)) {
                        {
                            l = self.pop_node();
                        }
                    }
                    if (self.mem[crate::ix::U((l) as usize)].hh().rh() != null) {
                        l = self.mem[crate::ix::U((l) as usize)].hh().rh();
                    } else {
                        if (self.hlist_stack_level == 0i32) {
                            run = false;
                        }
                    }
                }
            }
            if (t == l) { break; }
        }
        find_protchar_left = l;
        find_protchar_left
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn find_protchar_right(&mut self, mut l: halfword, mut r: halfword) -> halfword {
        let mut find_protchar_right: halfword = 0;
        let mut t: halfword = 0; // §1005
        let mut run: bool = false; // §1005
        find_protchar_right = null;
        if (r == null) {
            return find_protchar_right;
        }
        self.hlist_stack_level = 0i32;
        run = true;
        loop {
            t = r;
            while ((run && (self.mem[crate::ix::U((r) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null)) {
                {
                    self.push_node(l);
                    self.push_node(r);
                    l = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh();
                    r = l;
                    while (self.mem[crate::ix::U((r) as usize)].hh().rh() != null) {
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
            }
            while (run && ((!(r >= self.hi_mem_min)) && ((((((((((self.mem[crate::ix::U((r) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == penalty_node)) || (((self.mem[crate::ix::U((r) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((r) as usize)].hh().b1() != pdf_refximage_node)) && (self.mem[crate::ix::U((r) as usize)].hh().b1() != pdf_refxform_node))) || ((((self.mem[crate::ix::U((r) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (self.mem[crate::ix::U((r) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == kern_node) && (((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((r) as usize)].hh().b1() == normal)) || (self.mem[crate::ix::U((r) as usize)].hh().b1() == auto_kern)))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((r) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() == null))))) {
                {
                    while ((r == l) && (self.hlist_stack_level > 0i32)) {
                        {
                            r = self.pop_node();
                            l = self.pop_node();
                        }
                    }
                    if ((r != l) && (r != null)) {
                        r = self.prev_rightmost(l, r);
                    } else {
                        if ((r == l) && (self.hlist_stack_level == 0i32)) {
                            run = false;
                        }
                    }
                }
            }
            if (t == r) { break; }
        }
        find_protchar_right = r;
        find_protchar_right
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn total_pw(&mut self, mut q: halfword, mut p: halfword) -> scaled {
        let mut total_pw: scaled = 0;
        let mut l: halfword = 0; // §1005
        let mut r: halfword = 0; // §1005
        let mut n: i32 = 0; // §1005
        'l_done_f: {
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == null) {
                l = self.first_p;
            } else {
                l = self.mem[crate::ix::U(((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().rh();
            }
            r = self.prev_rightmost(self.prev_p, p);
            if (((p != null) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node)) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null)) {
                {
                    r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                    while (self.mem[crate::ix::U((r) as usize)].hh().rh() != null) {
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
            } else {
                r = self.find_protchar_right(l, r);
            }
            if ((l != null) && (self.mem[crate::ix::U((l) as usize)].hh().b0() == disc_node)) {
                {
                    if (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh() != null) {
                        {
                            l = self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh();
                            break 'l_done_f;
                        }
                    } else {
                        {
                            n = self.mem[crate::ix::U((l) as usize)].hh().b1();
                            l = self.mem[crate::ix::U((l) as usize)].hh().rh();
                            while (n > 0i32) {
                                {
                                    if (self.mem[crate::ix::U((l) as usize)].hh().rh() != null) {
                                        l = self.mem[crate::ix::U((l) as usize)].hh().rh();
                                    }
                                    n = (n).wrapping_sub(1i32);
                                }
                            }
                        }
                    }
                }
            }
            l = self.find_protchar_left(l, true);
        }
        total_pw = (self.char_pw(l, left_side)).wrapping_add(self.char_pw(r, right_side));
        total_pw
    }

    /// The heart of the line-breaking procedure is ``try_break`', a subroutine
    /// that tests if the current breakpoint `cur_p` is feasible, by running
    /// through the active list to see what lines of text can be made from active
    /// nodes to~`cur_p`.  If feasible breaks are possible, new break nodes are
    /// created.  If `cur_p` is too far from an active node, that node is
    /// deactivated.
    /// The parameter `pi` to `try_break` is the penalty associated
    /// with a break at `cur_p`; we have `pi=eject_penalty` if the break is forced,
    /// and `pi=inf_penalty` if the break is illegal.
    /// The other parameter, `break_type`, is set to `hyphenated` or `unhyphenated`,
    /// depending on whether or not the current break is at a `disc_node`. The
    /// end of a paragraph is also regarded as ``hyphenated`'; this case is
    /// distinguishable by the condition `cur_p=null`.
    // §1005
    pub fn try_break(&mut self, mut pi: i32, mut break_type: small_number) {
        let mut r: halfword = 0; // §1005
        let mut margin_kern_stretch: scaled = 0; // §1005
        let mut margin_kern_shrink: scaled = 0; // §1005
        let mut lp: halfword = 0; // §1005
        let mut rp: halfword = 0; // §1005
        let mut cp: halfword = 0; // §1005
        let mut prev_r: halfword = 0; // §1005
        let mut old_l: halfword = 0; // §1005
        let mut no_break_yet: bool = false; // §1005
        let mut prev_prev_r: halfword = 0; // §1006
        let mut s: halfword = 0; // §1006
        let mut q: halfword = 0; // §1006
        let mut v: halfword = 0; // §1006
        let mut t: i32 = 0; // §1006
        let mut f: internal_font_number = 0; // §1006
        let mut l: halfword = 0; // §1006
        let mut node_r_stays_active: bool = false; // §1006
        let mut line_width: scaled = 0; // §1006
        let mut fit_class: i32 = 0; // §1006
        let mut b: halfword = 0; // §1006
        let mut d: i32 = 0; // §1006
        let mut artificial_demerits: bool = false; // §1006
        let mut save_link: halfword = 0; // §1006
        let mut shortfall: scaled = 0; // §1006
        let mut g: scaled = 0; // §1844
        'l_exit_f: {
            // §1007
            if ((pi).wrapping_abs() >= inf_penalty) {
                if (pi > 0i32) {
                    break 'l_exit_f;
                } else {
                    pi = (10000i32).wrapping_neg();
                }
            }
            // §1005
            no_break_yet = true;
            prev_r = active;
            old_l = 0i32;
            { let __v1070 = self.active_width[crate::ix::U(((1i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1070; }
            { let __v1071 = self.active_width[crate::ix::U(((2i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1071; }
            { let __v1072 = self.active_width[crate::ix::U(((3i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1072; }
            { let __v1073 = self.active_width[crate::ix::U(((4i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1073; }
            { let __v1074 = self.active_width[crate::ix::U(((5i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1074; }
            { let __v1075 = self.active_width[crate::ix::U(((6i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1075; }
            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                {
                    { let __v1076 = self.active_width[crate::ix::U(((7i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1076; }
                    { let __v1077 = self.active_width[crate::ix::U(((8i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1077; }
                }
            }
            while true {
                {
                    'l_continue_b: loop {
                        r = self.mem[crate::ix::U((prev_r) as usize)].hh().rh();
                        // §1008
                        if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                            {
                                { let __v1078 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1078; }
                                { let __v1079 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1079; }
                                { let __v1080 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1080; }
                                { let __v1081 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1081; }
                                { let __v1082 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1082; }
                                { let __v1083 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1083; }
                                if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                    {
                                        { let __v1084 = (self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1084; }
                                        { let __v1085 = (self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(8i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1085; }
                                    }
                                }
                                prev_prev_r = prev_r;
                                prev_r = r;
                                continue 'l_continue_b;
                            }
                        }
                        // §1011
                        {
                            l = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh();
                            if (l > old_l) {
                                {
                                    if ((self.minimum_demerits < awful_bad) && ((old_l != self.easy_line) || (r == last_active))) {
                                        // §1012
                                        {
                                            if no_break_yet {
                                                // §1013
                                                {
                                                    'l_done_f: {
                                                        no_break_yet = false;
                                                        { let __v1086 = self.background[crate::ix::U(((1i32) - 1) as usize)]; self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1086; }
                                                        { let __v1087 = self.background[crate::ix::U(((2i32) - 1) as usize)]; self.break_width[crate::ix::U(((2i32) - 1) as usize)] = __v1087; }
                                                        { let __v1088 = self.background[crate::ix::U(((3i32) - 1) as usize)]; self.break_width[crate::ix::U(((3i32) - 1) as usize)] = __v1088; }
                                                        { let __v1089 = self.background[crate::ix::U(((4i32) - 1) as usize)]; self.break_width[crate::ix::U(((4i32) - 1) as usize)] = __v1089; }
                                                        { let __v1090 = self.background[crate::ix::U(((5i32) - 1) as usize)]; self.break_width[crate::ix::U(((5i32) - 1) as usize)] = __v1090; }
                                                        { let __v1091 = self.background[crate::ix::U(((6i32) - 1) as usize)]; self.break_width[crate::ix::U(((6i32) - 1) as usize)] = __v1091; }
                                                        if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                            {
                                                                { let __v1092 = self.background[crate::ix::U(((7i32) - 1) as usize)]; self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1092; }
                                                                { let __v1093 = self.background[crate::ix::U(((8i32) - 1) as usize)]; self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1093; }
                                                            }
                                                        }
                                                        s = self.cur_p;
                                                        if (break_type > unhyphenated) {
                                                            if (self.cur_p != null) {
                                                                // §1016
                                                                {
                                                                    t = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1();
                                                                    v = self.cur_p;
                                                                    s = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                                                                    while (t > 0i32) {
                                                                        {
                                                                            t = (t).wrapping_sub(1i32);
                                                                            v = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                                            // §1017
                                                                            if (v >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[crate::ix::U((v) as usize)].hh().b0();
                                                                                    { let __v1094 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((v) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1094; }
                                                                                    if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                                        {
                                                                                            self.prev_char_p = v;
                                                                                            { let __v1095 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.char_stretch(f, self.mem[crate::ix::U((v) as usize)].hh().b1())); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1095; }
                                                                                            { let __v1096 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.char_shrink(f, self.mem[crate::ix::U((v) as usize)].hh().b1())); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1096; }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                match self.mem[crate::ix::U((v) as usize)].hh().b0() {
                                                                                    ligature_node => {
                                                                                        {
                                                                                            f = self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                            { let __v1097 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1097; }
                                                                                            if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                                                {
                                                                                                    self.prev_char_p = v;
                                                                                                    { let __v1098 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.char_stretch(f, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b1())); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1098; }
                                                                                                    { let __v1099 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.char_shrink(f, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b1())); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1099; }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    hlist_node | vlist_node | rule_node | kern_node => {
                                                                                        {
                                                                                            { let __v1100 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1100; }
                                                                                            if (((self.mem[crate::ix::U((v) as usize)].hh().b0() == kern_node) && (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32)) && (self.mem[crate::ix::U((v) as usize)].hh().b1() == normal)) {
                                                                                                {
                                                                                                    { let __v1101 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.kern_stretch(v)); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1101; }
                                                                                                    { let __v1102 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.kern_shrink(v)); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1102; }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(1337i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §1016
                                                                    while (s != null) {
                                                                        {
                                                                            // §1018
                                                                            if (s >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                                    { let __v1103 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((s) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1103; }
                                                                                    if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                                        {
                                                                                            self.prev_char_p = s;
                                                                                            { let __v1104 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1104; }
                                                                                            { let __v1105 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1105; }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                                    ligature_node => {
                                                                                        {
                                                                                            f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                            { let __v1106 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1106; }
                                                                                            if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                                                {
                                                                                                    self.prev_char_p = s;
                                                                                                    { let __v1107 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1107; }
                                                                                                    { let __v1108 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1108; }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    hlist_node | vlist_node | rule_node | kern_node => {
                                                                                        {
                                                                                            { let __v1109 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1109; }
                                                                                            if (((self.mem[crate::ix::U((s) as usize)].hh().b0() == kern_node) && (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32)) && (self.mem[crate::ix::U((s) as usize)].hh().b1() == normal)) {
                                                                                                {
                                                                                                    { let __v1110 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.kern_stretch(s)); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1110; }
                                                                                                    { let __v1111 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.kern_shrink(s)); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1111; }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(1338i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                            // §1016
                                                                            s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                        }
                                                                    }
                                                                    { let __v1112 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((1i32) - 1) as usize)]); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1112; }
                                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                                        {
                                                                            { let __v1113 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((7i32) - 1) as usize)]); self.break_width[crate::ix::U(((7i32) - 1) as usize)] = __v1113; }
                                                                            { let __v1114 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((8i32) - 1) as usize)]); self.break_width[crate::ix::U(((8i32) - 1) as usize)] = __v1114; }
                                                                        }
                                                                    }
                                                                    if (self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh() == null) {
                                                                        s = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1013
                                                        while (s != null) {
                                                            {
                                                                if (s >= self.hi_mem_min) {
                                                                    break 'l_done_f;
                                                                }
                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                    glue_node => {
                                                                        // §1014
                                                                        {
                                                                            v = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().lh();
                                                                            { let __v1115 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1115; }
                                                                            { let __ix1116 = (2i32).wrapping_add(self.mem[crate::ix::U((v) as usize)].hh().b0()); let __v1117 = (self.break_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((v) as usize)].hh().b0())) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()); self.break_width[crate::ix::U(((__ix1116) - 1) as usize)] = __v1117; }
                                                                            { let __v1118 = (self.break_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()); self.break_width[crate::ix::U(((6i32) - 1) as usize)] = __v1118; }
                                                                        }
                                                                    }
                                                                    penalty_node => {
                                                                        // §1013
                                                                    }
                                                                    math_node => {
                                                                        { let __v1119 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1119; }
                                                                    }
                                                                    kern_node => {
                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b1() != explicit) {
                                                                            break 'l_done_f;
                                                                        } else {
                                                                            { let __v1120 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v1120; }
                                                                        }
                                                                    }
                                                                    _ => {
                                                                        break 'l_done_f;
                                                                    }
                                                                }
                                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // §1019
                                            if (self.mem[crate::ix::U((prev_r) as usize)].hh().b0() == delta_node) {
                                                {
                                                    { let __v1121 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].set_int(__v1121); }
                                                    { let __v1122 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].set_int(__v1122); }
                                                    { let __v1123 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].set_int(__v1123); }
                                                    { let __v1124 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].set_int(__v1124); }
                                                    { let __v1125 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].set_int(__v1125); }
                                                    { let __v1126 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].set_int(__v1126); }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1127 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(7i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((7i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(7i32)) as usize)].set_int(__v1127); }
                                                            { let __v1128 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(8i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((8i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(8i32)) as usize)].set_int(__v1128); }
                                                        }
                                                    }
                                                }
                                            } else {
                                                if (prev_r == active) {
                                                    {
                                                        { let __v1129 = self.break_width[crate::ix::U(((1i32) - 1) as usize)]; self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1129; }
                                                        { let __v1130 = self.break_width[crate::ix::U(((2i32) - 1) as usize)]; self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1130; }
                                                        { let __v1131 = self.break_width[crate::ix::U(((3i32) - 1) as usize)]; self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1131; }
                                                        { let __v1132 = self.break_width[crate::ix::U(((4i32) - 1) as usize)]; self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1132; }
                                                        { let __v1133 = self.break_width[crate::ix::U(((5i32) - 1) as usize)]; self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1133; }
                                                        { let __v1134 = self.break_width[crate::ix::U(((6i32) - 1) as usize)]; self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1134; }
                                                        if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                            {
                                                                { let __v1135 = self.break_width[crate::ix::U(((7i32) - 1) as usize)]; self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1135; }
                                                                { let __v1136 = self.break_width[crate::ix::U(((8i32) - 1) as usize)]; self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1136; }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        q = self.get_node(delta_node_size);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_b0(delta_node);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                                        { let __v1137 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1137); }
                                                        { let __v1138 = (self.break_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1138); }
                                                        { let __v1139 = (self.break_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1139); }
                                                        { let __v1140 = (self.break_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v1140); }
                                                        { let __v1141 = (self.break_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_int(__v1141); }
                                                        { let __v1142 = (self.break_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(__v1142); }
                                                        if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                            {
                                                                { let __v1143 = (self.break_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(7i32)) as usize)].set_int(__v1143); }
                                                                { let __v1144 = (self.break_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(8i32)) as usize)].set_int(__v1144); }
                                                            }
                                                        }
                                                        self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                        prev_prev_r = prev_r;
                                                        prev_r = q;
                                                    }
                                                }
                                            }
                                            // §1012
                                            if ((self.eqtb[crate::ix::U(((29293i32) - 1) as usize)].int()).wrapping_abs() >= (awful_bad).wrapping_sub(self.minimum_demerits)) {
                                                self.minimum_demerits = 1073741822i32;
                                            } else {
                                                self.minimum_demerits = (self.minimum_demerits).wrapping_add((self.eqtb[crate::ix::U(((29293i32) - 1) as usize)].int()).wrapping_abs());
                                            }
                                            {
                                                let __for_end_11 = tight_fit;
                                                fit_class = very_loose_fit;
                                                while fit_class <= __for_end_11 {
                                                    {
                                                        if (self.minimal_demerits[crate::ix::U((fit_class) as usize)] <= self.minimum_demerits) {
                                                            // §1021
                                                            {
                                                                q = self.get_node(passive_node_size);
                                                                { let __v1145 = self.passive; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1145); }
                                                                self.passive = q;
                                                                { let __v1146 = self.cur_p; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v1146); }
                                                                self.pass_number = (self.pass_number).wrapping_add(1i32);
                                                                { let __v1147 = self.pass_number; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v1147); }
                                                                { let __v1148 = self.best_place[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1148); }
                                                                q = self.get_node(self.active_node_size);
                                                                { let __v1149 = self.passive; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v1149); }
                                                                { let __v1150 = (self.best_pl_line[crate::ix::U((fit_class) as usize)]).wrapping_add(1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1150); }
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_b1(fit_class);
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_b0(break_type);
                                                                { let __v1151 = self.minimal_demerits[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1151); }
                                                                if self.do_last_line_fit {
                                                                    // §1851
                                                                    {
                                                                        { let __v1152 = self.best_pl_short[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1152); }
                                                                        { let __v1153 = self.best_pl_glue[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v1153); }
                                                                    }
                                                                }
                                                                // §1021
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                                self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                                prev_r = q;
                                                                if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                                                                    // §1022
                                                                    {
                                                                        self.print_nl(1339i32);
                                                                        self.print_int(((self.mem[crate::ix::U((self.passive) as usize)].hh().lh()) as i64));
                                                                        self.print(1340i32);
                                                                        self.print_int((((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(1i32)) as i64));
                                                                        self.print_char(46i32);
                                                                        self.print_int(((fit_class) as i64));
                                                                        if (break_type == hyphenated) {
                                                                            self.print_char(45i32);
                                                                        }
                                                                        self.print(1341i32);
                                                                        self.print_int(((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()) as i64));
                                                                        if self.do_last_line_fit {
                                                                            // §1852
                                                                            {
                                                                                self.print(2063i32);
                                                                                self.print_scaled(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int());
                                                                                if (self.cur_p == null) {
                                                                                    self.print(2064i32);
                                                                                } else {
                                                                                    self.print(1411i32);
                                                                                }
                                                                                self.print_scaled(self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].int());
                                                                            }
                                                                        }
                                                                        // §1022
                                                                        self.print(1342i32);
                                                                        if (self.mem[crate::ix::U(((self.passive).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                                                                            self.print_char(48i32);
                                                                        } else {
                                                                            self.print_int(((self.mem[crate::ix::U((self.mem[crate::ix::U(((self.passive).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().lh()) as i64));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1012
                                                        self.minimal_demerits[crate::ix::U((fit_class) as usize)] = awful_bad;
                                                    }
                                                    fit_class = fit_class.wrapping_add(1);
                                                }
                                            }
                                            self.minimum_demerits = awful_bad;
                                            // §1020
                                            if (r != last_active) {
                                                {
                                                    q = self.get_node(delta_node_size);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(delta_node);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                                    { let __v1154 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1154); }
                                                    { let __v1155 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1155); }
                                                    { let __v1156 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1156); }
                                                    { let __v1157 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v1157); }
                                                    { let __v1158 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_int(__v1158); }
                                                    { let __v1159 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(__v1159); }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1160 = (self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((7i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(7i32)) as usize)].set_int(__v1160); }
                                                            { let __v1161 = (self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((8i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(8i32)) as usize)].set_int(__v1161); }
                                                        }
                                                    }
                                                    self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                    prev_prev_r = prev_r;
                                                    prev_r = q;
                                                }
                                            }
                                        }
                                    }
                                    // §1011
                                    if (r == last_active) {
                                        break 'l_exit_f;
                                    }
                                    // §1026
                                    if (l > self.easy_line) {
                                        {
                                            line_width = self.second_width;
                                            old_l = 268435454i32;
                                        }
                                    } else {
                                        {
                                            old_l = l;
                                            if (l > self.last_special_line) {
                                                line_width = self.second_width;
                                            } else {
                                                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) {
                                                    line_width = self.first_width;
                                                } else {
                                                    line_width = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(l))) as usize)].int();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §1027
                        {
                            'l_L60_f: {
                                'l_found_f: {
                                    artificial_demerits = false;
                                    shortfall = (line_width).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]);
                                    if (self.eqtb[crate::ix::U(((29361i32) - 1) as usize)].int() > 1i32) {
                                        shortfall = (shortfall).wrapping_add(self.total_pw(r, self.cur_p));
                                    }
                                    if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && (shortfall != 0i32)) {
                                        {
                                            margin_kern_stretch = 0i32;
                                            margin_kern_shrink = 0i32;
                                            if (self.eqtb[crate::ix::U(((29361i32) - 1) as usize)].int() > 1i32) {
                                                // §822
                                                {
                                                    lp = self.last_leftmost_char;
                                                    rp = self.last_rightmost_char;
                                                    {
                                                        cp = self.avail;
                                                        if (cp == null) {
                                                            cp = self.get_avail();
                                                        } else {
                                                            {
                                                                self.avail = self.mem[crate::ix::U((cp) as usize)].hh().rh();
                                                                self.mem[crate::ix::U((cp) as usize)].set_hh_rh(null);
                                                                self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                                                self.dl_new_node(cp);
                                                            }
                                                        }
                                                    }
                                                    if (lp != null) {
                                                        {
                                                            { let __v1162 = self.mem[crate::ix::U((lp) as usize)].hh().b1(); self.mem[crate::ix::U((cp) as usize)].set_hh_b1(__v1162); }
                                                            { let __v1163 = self.mem[crate::ix::U((lp) as usize)].hh().b0(); self.mem[crate::ix::U((cp) as usize)].set_hh_b0(__v1163); }
                                                            self.do_subst_font(cp, 1000i32);
                                                            if (self.mem[crate::ix::U((cp) as usize)].hh().b0() != self.mem[crate::ix::U((lp) as usize)].hh().b0()) {
                                                                margin_kern_stretch = ((margin_kern_stretch).wrapping_add(self.char_pw(lp, left_side))).wrapping_sub(self.char_pw(cp, left_side));
                                                            }
                                                            { let __v1164 = self.mem[crate::ix::U((lp) as usize)].hh().b0(); self.mem[crate::ix::U((cp) as usize)].set_hh_b0(__v1164); }
                                                            self.do_subst_font(cp, (1000i32).wrapping_neg());
                                                            if (self.mem[crate::ix::U((cp) as usize)].hh().b0() != self.mem[crate::ix::U((lp) as usize)].hh().b0()) {
                                                                margin_kern_shrink = ((margin_kern_shrink).wrapping_add(self.char_pw(cp, left_side))).wrapping_sub(self.char_pw(lp, left_side));
                                                            }
                                                        }
                                                    }
                                                    if (rp != null) {
                                                        {
                                                            { let __v1165 = self.mem[crate::ix::U((rp) as usize)].hh().b1(); self.mem[crate::ix::U((cp) as usize)].set_hh_b1(__v1165); }
                                                            { let __v1166 = self.mem[crate::ix::U((rp) as usize)].hh().b0(); self.mem[crate::ix::U((cp) as usize)].set_hh_b0(__v1166); }
                                                            self.do_subst_font(cp, 1000i32);
                                                            if (self.mem[crate::ix::U((cp) as usize)].hh().b0() != self.mem[crate::ix::U((rp) as usize)].hh().b0()) {
                                                                margin_kern_stretch = ((margin_kern_stretch).wrapping_add(self.char_pw(rp, left_side))).wrapping_sub(self.char_pw(cp, left_side));
                                                            }
                                                            { let __v1167 = self.mem[crate::ix::U((rp) as usize)].hh().b0(); self.mem[crate::ix::U((cp) as usize)].set_hh_b0(__v1167); }
                                                            self.do_subst_font(cp, (1000i32).wrapping_neg());
                                                            if (self.mem[crate::ix::U((cp) as usize)].hh().b0() != self.mem[crate::ix::U((rp) as usize)].hh().b0()) {
                                                                margin_kern_shrink = ((margin_kern_shrink).wrapping_add(self.char_pw(cp, left_side))).wrapping_sub(self.char_pw(rp, left_side));
                                                            }
                                                        }
                                                    }
                                                    {
                                                        { let __v1168 = self.avail; self.mem[crate::ix::U((cp) as usize)].set_hh_rh(__v1168); }
                                                        self.avail = cp;
                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                    }
                                                }
                                            }
                                            // §1027
                                            if ((shortfall > 0i32) && ((self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(margin_kern_stretch) > 0i32)) {
                                                {
                                                    if ((self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(margin_kern_stretch) > shortfall) {
                                                        shortfall = (((self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(margin_kern_stretch) / (self.max_stretch_ratio / self.cur_font_step)) / 2i32);
                                                    } else {
                                                        shortfall = (shortfall).wrapping_sub((self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(margin_kern_stretch));
                                                    }
                                                }
                                            } else {
                                                if ((shortfall < 0i32) && ((self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(margin_kern_shrink) > 0i32)) {
                                                    {
                                                        if ((self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(margin_kern_shrink) > (shortfall).wrapping_neg()) {
                                                            shortfall = ((((self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(margin_kern_shrink) / (self.max_shrink_ratio / self.cur_font_step)) / 2i32)).wrapping_neg();
                                                        } else {
                                                            shortfall = (shortfall).wrapping_add((self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(margin_kern_shrink));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if (shortfall > 0i32) {
                                        // §1028
                                        if (((self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] != 0i32) || (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] != 0i32)) || (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] != 0i32)) {
                                            {
                                                if self.do_last_line_fit {
                                                    {
                                                        if (self.cur_p == null) {
                                                            // §1846
                                                            {
                                                                'l_not_found_f: {
                                                                    if ((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int() <= 0i32)) {
                                                                        break 'l_not_found_f;
                                                                    }
                                                                    if (((self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] != self.fill_width[crate::ix::U((0i32) as usize)]) || (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] != self.fill_width[crate::ix::U((1i32) as usize)])) || (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] != self.fill_width[crate::ix::U((2i32) as usize)])) {
                                                                        break 'l_not_found_f;
                                                                    }
                                                                    if (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() > 0i32) {
                                                                        g = self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)];
                                                                    } else {
                                                                        g = self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)];
                                                                    }
                                                                    if (g <= 0i32) {
                                                                        break 'l_not_found_f;
                                                                    }
                                                                    self.arith_error = false;
                                                                    g = self.fract(g, self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int(), max_dimen);
                                                                    if (self.eqtb[crate::ix::U(((29385i32) - 1) as usize)].int() < 1000i32) {
                                                                        g = self.fract(g, self.eqtb[crate::ix::U(((29385i32) - 1) as usize)].int(), 1000i32, max_dimen);
                                                                    }
                                                                    if self.arith_error {
                                                                        if (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() > 0i32) {
                                                                            g = max_dimen;
                                                                        } else {
                                                                            g = (1073741823i32).wrapping_neg();
                                                                        }
                                                                    }
                                                                    if (g > 0i32) {
                                                                        // §1847
                                                                        {
                                                                            if (g > shortfall) {
                                                                                g = shortfall;
                                                                            }
                                                                            if (g > 7230584i32) {
                                                                                if (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] < 1663497i32) {
                                                                                    {
                                                                                        b = inf_bad;
                                                                                        fit_class = very_loose_fit;
                                                                                        break 'l_found_f;
                                                                                    }
                                                                                }
                                                                            }
                                                                            b = self.badness(g, self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]);
                                                                            if (b > 12i32) {
                                                                                if (b > 99i32) {
                                                                                    fit_class = very_loose_fit;
                                                                                } else {
                                                                                    fit_class = loose_fit;
                                                                                }
                                                                            } else {
                                                                                fit_class = decent_fit;
                                                                            }
                                                                            break 'l_found_f;
                                                                        }
                                                                    } else {
                                                                        // §1846
                                                                        if (g < 0i32) {
                                                                            // §1848
                                                                            {
                                                                                if ((g).wrapping_neg() > self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]) {
                                                                                    g = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_neg();
                                                                                }
                                                                                b = self.badness((g).wrapping_neg(), self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]);
                                                                                if (b > 12i32) {
                                                                                    fit_class = tight_fit;
                                                                                } else {
                                                                                    fit_class = decent_fit;
                                                                                }
                                                                                break 'l_found_f;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §1846
                                                            }
                                                        }
                                                        // §1028
                                                        shortfall = 0i32;
                                                    }
                                                }
                                                b = 0i32;
                                                fit_class = decent_fit;
                                            }
                                        } else {
                                            {
                                                'l_done1_f: {
                                                    if (shortfall > 7230584i32) {
                                                        if (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] < 1663497i32) {
                                                            {
                                                                b = inf_bad;
                                                                fit_class = very_loose_fit;
                                                                break 'l_done1_f;
                                                            }
                                                        }
                                                    }
                                                    b = self.badness(shortfall, self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]);
                                                    if (b > 12i32) {
                                                        if (b > 99i32) {
                                                            fit_class = very_loose_fit;
                                                        } else {
                                                            fit_class = loose_fit;
                                                        }
                                                    } else {
                                                        fit_class = decent_fit;
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // §1029
                                        {
                                            if ((shortfall).wrapping_neg() > self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]) {
                                                b = 10001i32;
                                            } else {
                                                b = self.badness((shortfall).wrapping_neg(), self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]);
                                            }
                                            if (b > 12i32) {
                                                fit_class = tight_fit;
                                            } else {
                                                fit_class = decent_fit;
                                            }
                                        }
                                    }
                                    // §1027
                                    if self.do_last_line_fit {
                                        // §1849
                                        {
                                            if (self.cur_p == null) {
                                                shortfall = 0i32;
                                            }
                                            if (shortfall > 0i32) {
                                                g = self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)];
                                            } else {
                                                if (shortfall < 0i32) {
                                                    g = self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)];
                                                } else {
                                                    g = 0i32;
                                                }
                                            }
                                        }
                                    }
                                }
                                // §1027
                                if ((b > inf_bad) || (pi == (10000i32).wrapping_neg())) {
                                    // §1030
                                    {
                                        if (((self.final_pass && (self.minimum_demerits == awful_bad)) && (self.mem[crate::ix::U((r) as usize)].hh().rh() == last_active)) && (prev_r == active)) {
                                            artificial_demerits = true;
                                        } else {
                                            if (b > self.threshold) {
                                                break 'l_L60_f;
                                            }
                                        }
                                        node_r_stays_active = false;
                                    }
                                } else {
                                    // §1027
                                    {
                                        prev_r = r;
                                        if (b > self.threshold) {
                                            continue 'l_continue_b;
                                        }
                                        node_r_stays_active = true;
                                    }
                                }
                                // §1031
                                if artificial_demerits {
                                    d = 0i32;
                                } else {
                                    // §1035
                                    {
                                        d = (self.eqtb[crate::ix::U(((29279i32) - 1) as usize)].int()).wrapping_add(b);
                                        if ((d).wrapping_abs() >= 10000i32) {
                                            d = 100000000i32;
                                        } else {
                                            d = (d).wrapping_mul(d);
                                        }
                                        if (pi != 0i32) {
                                            if (pi > 0i32) {
                                                d = (d).wrapping_add((pi).wrapping_mul(pi));
                                            } else {
                                                if (pi > (10000i32).wrapping_neg()) {
                                                    d = (d).wrapping_sub((pi).wrapping_mul(pi));
                                                }
                                            }
                                        }
                                        if ((break_type == hyphenated) && (self.mem[crate::ix::U((r) as usize)].hh().b0() == hyphenated)) {
                                            if (self.cur_p != null) {
                                                d = (d).wrapping_add(self.eqtb[crate::ix::U(((29291i32) - 1) as usize)].int());
                                            } else {
                                                d = (d).wrapping_add(self.eqtb[crate::ix::U(((29292i32) - 1) as usize)].int());
                                            }
                                        }
                                        if (((fit_class).wrapping_sub(self.mem[crate::ix::U((r) as usize)].hh().b1())).wrapping_abs() > 1i32) {
                                            d = (d).wrapping_add(self.eqtb[crate::ix::U(((29293i32) - 1) as usize)].int());
                                        }
                                    }
                                }
                                // §1031
                                if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                                    // §1032
                                    {
                                        if (self.printed_node != self.cur_p) {
                                            // §1033
                                            {
                                                self.print_nl(347i32);
                                                if (self.cur_p == null) {
                                                    self.short_display(self.mem[crate::ix::U((self.printed_node) as usize)].hh().rh());
                                                } else {
                                                    {
                                                        save_link = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                                        { let __ix1169 = self.cur_p; self.mem[crate::ix::U((__ix1169) as usize)].set_hh_rh(null); }
                                                        self.print_nl(347i32);
                                                        self.short_display(self.mem[crate::ix::U((self.printed_node) as usize)].hh().rh());
                                                        { let __ix1170 = self.cur_p; self.mem[crate::ix::U((__ix1170) as usize)].set_hh_rh(save_link); }
                                                    }
                                                }
                                                self.printed_node = self.cur_p;
                                            }
                                        }
                                        // §1032
                                        self.print_nl(64i32);
                                        if (self.cur_p == null) {
                                            self.print_esc(681i32);
                                        } else {
                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() != glue_node) {
                                                {
                                                    if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == penalty_node) {
                                                        self.print_esc(609i32);
                                                    } else {
                                                        if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == disc_node) {
                                                            self.print_esc(360i32);
                                                        } else {
                                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == kern_node) {
                                                                self.print_esc(324i32);
                                                            } else {
                                                                self.print_esc(354i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.print(1343i32);
                                        if (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() == null) {
                                            self.print_char(48i32);
                                        } else {
                                            self.print_int(((self.mem[crate::ix::U((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()) as i64));
                                        }
                                        self.print(1344i32);
                                        if (b > inf_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(((b) as i64));
                                        }
                                        self.print(1345i32);
                                        self.print_int(((pi) as i64));
                                        self.print(1346i32);
                                        if artificial_demerits {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(((d) as i64));
                                        }
                                    }
                                }
                                // §1031
                                d = (d).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int());
                                if (d <= self.minimal_demerits[crate::ix::U((fit_class) as usize)]) {
                                    {
                                        self.minimal_demerits[crate::ix::U((fit_class) as usize)] = d;
                                        { let __v1171 = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh(); self.best_place[crate::ix::U((fit_class) as usize)] = __v1171; }
                                        self.best_pl_line[crate::ix::U((fit_class) as usize)] = l;
                                        if self.do_last_line_fit {
                                            // §1850
                                            {
                                                self.best_pl_short[crate::ix::U((fit_class) as usize)] = shortfall;
                                                self.best_pl_glue[crate::ix::U((fit_class) as usize)] = g;
                                            }
                                        }
                                        // §1031
                                        if (d < self.minimum_demerits) {
                                            self.minimum_demerits = d;
                                        }
                                    }
                                }
                                // §1027
                                if node_r_stays_active {
                                    continue 'l_continue_b;
                                }
                            }
                            { let __v1172 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(__v1172); }
                            // §1036
                            self.free_node(r, self.active_node_size);
                            if (prev_r == active) {
                                // §1037
                                {
                                    r = self.mem[crate::ix::U((active) as usize)].hh().rh();
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                                        {
                                            { let __v1173 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1173; }
                                            { let __v1174 = (self.active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1174; }
                                            { let __v1175 = (self.active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1175; }
                                            { let __v1176 = (self.active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1176; }
                                            { let __v1177 = (self.active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1177; }
                                            { let __v1178 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1178; }
                                            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                {
                                                    { let __v1179 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].int()); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1179; }
                                                    { let __v1180 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(8i32)) as usize)].int()); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1180; }
                                                }
                                            }
                                            { let __v1181 = self.active_width[crate::ix::U(((1i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1181; }
                                            { let __v1182 = self.active_width[crate::ix::U(((2i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1182; }
                                            { let __v1183 = self.active_width[crate::ix::U(((3i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1183; }
                                            { let __v1184 = self.active_width[crate::ix::U(((4i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1184; }
                                            { let __v1185 = self.active_width[crate::ix::U(((5i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1185; }
                                            { let __v1186 = self.active_width[crate::ix::U(((6i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1186; }
                                            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                {
                                                    { let __v1187 = self.active_width[crate::ix::U(((7i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1187; }
                                                    { let __v1188 = self.active_width[crate::ix::U(((8i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1188; }
                                                }
                                            }
                                            { let __v1189 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((active) as usize)].set_hh_rh(__v1189); }
                                            self.free_node(r, delta_node_size);
                                        }
                                    }
                                }
                            } else {
                                // §1036
                                if (self.mem[crate::ix::U((prev_r) as usize)].hh().b0() == delta_node) {
                                    {
                                        r = self.mem[crate::ix::U((prev_r) as usize)].hh().rh();
                                        if (r == last_active) {
                                            {
                                                { let __v1190 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1190; }
                                                { let __v1191 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1191; }
                                                { let __v1192 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1192; }
                                                { let __v1193 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1193; }
                                                { let __v1194 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1194; }
                                                { let __v1195 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1195; }
                                                if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                    {
                                                        { let __v1196 = (self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(7i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1196; }
                                                        { let __v1197 = (self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(8i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1197; }
                                                    }
                                                }
                                                self.mem[crate::ix::U((prev_prev_r) as usize)].set_hh_rh(last_active);
                                                self.free_node(prev_r, delta_node_size);
                                                prev_r = prev_prev_r;
                                            }
                                        } else {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                                                {
                                                    { let __v1198 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1198; }
                                                    { let __v1199 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1199; }
                                                    { let __v1200 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1200; }
                                                    { let __v1201 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1201; }
                                                    { let __v1202 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1202; }
                                                    { let __v1203 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1203; }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1204 = (self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1204; }
                                                            { let __v1205 = (self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(8i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1205; }
                                                        }
                                                    }
                                                    { let __v1206 = (self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].set_int(__v1206); }
                                                    { let __v1207 = (self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].set_int(__v1207); }
                                                    { let __v1208 = (self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].set_int(__v1208); }
                                                    { let __v1209 = (self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].set_int(__v1209); }
                                                    { let __v1210 = (self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].set_int(__v1210); }
                                                    { let __v1211 = (self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].set_int(__v1211); }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1212 = (self.mem[crate::ix::U(((prev_r).wrapping_add(7i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(7i32)) as usize)].set_int(__v1212); }
                                                            { let __v1213 = (self.mem[crate::ix::U(((prev_r).wrapping_add(8i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(8i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(8i32)) as usize)].set_int(__v1213); }
                                                        }
                                                    }
                                                    { let __v1214 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(__v1214); }
                                                    self.free_node(r, delta_node_size);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        break 'l_continue_b;
                    }
                }
            }
        }
        // §1005
        if (self.cur_p == self.printed_node) {
            // §1034
            if (self.cur_p != null) {
                if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == disc_node) {
                    {
                        t = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1();
                        while (t > 0i32) {
                            {
                                t = (t).wrapping_sub(1i32);
                                self.printed_node = self.mem[crate::ix::U((self.printed_node) as usize)].hh().rh();
                            }
                        }
                    }
                }
            }
        }
    }

    /// The total number of lines that will be set by `post_line_break`
    /// is `best_line-prev_graf-1`. The last breakpoint is specified by
    /// `break_node(best_bet)`, and this passive node points to the other breakpoints
    /// via the `prev_break` links. The finishing-up phase starts by linking the
    /// relevant passive nodes in forward order, changing `prev_break` to
    /// `next_break`. (The `next_break` fields actually reside in the same memory
    /// space as the `prev_break` fields did, but we give them a new name because
    /// of their new significance.) Then the lines are justified, one by one.
    // §1053
    pub fn post_line_break(&mut self, mut d: bool) {
        let mut q: halfword = 0; // §1053
        let mut r: halfword = 0; // §1053
        let mut s: halfword = 0; // §1053
        let mut p: halfword = 0; // §1053
        let mut k: halfword = 0; // §1053
        let mut w: scaled = 0; // §1053
        let mut glue_break: bool = false; // §1053
        let mut ptmp: halfword = 0; // §1053
        let mut disc_break: bool = false; // §1053
        let mut post_disc_break: bool = false; // §1053
        let mut cur_width: scaled = 0; // §1053
        let mut cur_indent: scaled = 0; // §1053
        let mut t: quarterword = 0; // §1053
        let mut pen: i32 = 0; // §1053
        let mut cur_line: halfword = 0; // §1053
        let mut LR_ptr: halfword = 0; // §1053
        LR_ptr = self.cur_list.eTeX_aux_field;
        // §1054
        q = self.mem[crate::ix::U(((self.best_bet).wrapping_add(1i32)) as usize)].hh().rh();
        self.cur_p = null;
        loop {
            r = q;
            q = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
            { let __v1215 = self.cur_p; self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v1215); }
            self.cur_p = r;
            if (q == null) { break; }
        }
        // §1053
        cur_line = (self.cur_list.pg_field).wrapping_add(1i32);
        loop {
            'l_done_f: {
                // §1056
                if (self.eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                    // §1707
                    {
                        q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                        if (LR_ptr != null) {
                            {
                                self.temp_ptr = LR_ptr;
                                r = q;
                                loop {
                                    s = self.new_math(0i32, (self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().lh()).wrapping_sub(1i32));
                                    self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                    r = s;
                                    self.temp_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                    if (self.temp_ptr == null) { break; }
                                }
                                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(r);
                            }
                        }
                        while (q != self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh()) {
                            {
                                if (!(q >= self.hi_mem_min)) {
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        // §1708
                                        if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                            {
                                                if (LR_ptr != null) {
                                                    if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                        {
                                                            self.temp_ptr = LR_ptr;
                                                            LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                            {
                                                                { let __ix1216 = self.temp_ptr; let __v1217 = self.avail; self.mem[crate::ix::U((__ix1216) as usize)].set_hh_rh(__v1217); }
                                                                self.avail = self.temp_ptr;
                                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            {
                                                self.temp_ptr = self.get_avail();
                                                { let __ix1218 = self.temp_ptr; let __v1219 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1218) as usize)].set_hh_lh(__v1219); }
                                                { let __ix1220 = self.temp_ptr; self.mem[crate::ix::U((__ix1220) as usize)].set_hh_rh(LR_ptr); }
                                                LR_ptr = self.temp_ptr;
                                            }
                                        }
                                    }
                                }
                                // §1707
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            }
                        }
                    }
                }
                // §1057
                q = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                disc_break = false;
                post_disc_break = false;
                glue_break = false;
                if (q != null) {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == glue_node) {
                        {
                            self.delete_glue_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh());
                            { let __v1221 = self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1221); }
                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(9i32);
                            { let __ix1222 = self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh(); let __v1223 = (self.mem[crate::ix::U((self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1222) as usize)].set_hh_rh(__v1223); }
                            glue_break = true;
                            break 'l_done_f;
                        }
                    } else {
                        {
                            if (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node) {
                                // §1058
                                {
                                    t = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                    // §1059
                                    if (t == 0i32) {
                                        r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                    } else {
                                        {
                                            r = q;
                                            while (t > 1i32) {
                                                {
                                                    r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                                    t = (t).wrapping_sub(1i32);
                                                }
                                            }
                                            s = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                            r = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                            self.mem[crate::ix::U((s) as usize)].set_hh_rh(null);
                                            self.flush_node_list(self.mem[crate::ix::U((q) as usize)].hh().rh());
                                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                        }
                                    }
                                    // §1058
                                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() != null) {
                                        // §1060
                                        {
                                            s = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                                            while (self.mem[crate::ix::U((s) as usize)].hh().rh() != null) {
                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                            }
                                            self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                            r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                                            post_disc_break = true;
                                        }
                                    }
                                    // §1058
                                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                        // §1061
                                        {
                                            s = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(s);
                                            while (self.mem[crate::ix::U((s) as usize)].hh().rh() != null) {
                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                            }
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(null);
                                            q = s;
                                        }
                                    }
                                    // §1058
                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                    disc_break = true;
                                }
                            } else {
                                // §1057
                                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == kern_node) {
                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                                } else {
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        {
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                                            if (self.eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                                                // §1708
                                                if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                                    {
                                                        if (LR_ptr != null) {
                                                            if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = LR_ptr;
                                                                    LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix1224 = self.temp_ptr; let __v1225 = self.avail; self.mem[crate::ix::U((__ix1224) as usize)].set_hh_rh(__v1225); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        self.temp_ptr = self.get_avail();
                                                        { let __ix1226 = self.temp_ptr; let __v1227 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1226) as usize)].set_hh_lh(__v1227); }
                                                        { let __ix1228 = self.temp_ptr; self.mem[crate::ix::U((__ix1228) as usize)].set_hh_rh(LR_ptr); }
                                                        LR_ptr = self.temp_ptr;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // §1057
                    {
                        q = temp_head;
                        while (self.mem[crate::ix::U((q) as usize)].hh().rh() != null) {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                    }
                }
            }
            if (self.eqtb[crate::ix::U(((29361i32) - 1) as usize)].int() > 0i32) {
                {
                    if (disc_break && ((q >= self.hi_mem_min) || (self.mem[crate::ix::U((q) as usize)].hh().b0() != disc_node))) {
                        {
                            p = q;
                            ptmp = p;
                        }
                    } else {
                        {
                            p = self.prev_rightmost(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), q);
                            ptmp = p;
                            p = self.find_protchar_right(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), p);
                        }
                    }
                    w = self.char_pw(p, right_side);
                    if (w != 0i32) {
                        {
                            k = self.new_margin_kern((w).wrapping_neg(), self.last_rightmost_char, right_side);
                            { let __v1229 = self.mem[crate::ix::U((ptmp) as usize)].hh().rh(); self.mem[crate::ix::U((k) as usize)].set_hh_rh(__v1229); }
                            self.mem[crate::ix::U((ptmp) as usize)].set_hh_rh(k);
                            if (ptmp == q) {
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            }
                        }
                    }
                }
            }
            if (!glue_break) {
                {
                    // §1062
                    r = self.new_param_glue(right_skip_code);
                    { let __v1230 = self.mem[crate::ix::U((q) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1230); }
                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                    q = r;
                }
            }
            // §1056
            if (self.eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                // §1709
                if (LR_ptr != null) {
                    {
                        s = temp_head;
                        r = self.mem[crate::ix::U((s) as usize)].hh().rh();
                        while (r != q) {
                            {
                                s = r;
                                r = self.mem[crate::ix::U((s) as usize)].hh().rh();
                            }
                        }
                        r = LR_ptr;
                        while (r != null) {
                            {
                                self.temp_ptr = self.new_math(0i32, self.mem[crate::ix::U((r) as usize)].hh().lh());
                                { let __v1231 = self.temp_ptr; self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1231); }
                                s = self.temp_ptr;
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                    }
                }
            }
            // §1063
            r = self.mem[crate::ix::U((q) as usize)].hh().rh();
            self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
            q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
            self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(r);
            if (self.eqtb[crate::ix::U(((29361i32) - 1) as usize)].int() > 0i32) {
                {
                    p = q;
                    p = self.find_protchar_left(p, false);
                    w = self.char_pw(p, left_side);
                    if (w != 0i32) {
                        {
                            k = self.new_margin_kern((w).wrapping_neg(), self.last_leftmost_char, left_side);
                            self.mem[crate::ix::U((k) as usize)].set_hh_rh(q);
                            q = k;
                        }
                    }
                }
            }
            if (self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh() != zero_glue) {
                {
                    r = self.new_param_glue(left_skip_code);
                    self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                    q = r;
                }
            }
            // §1066
            if (cur_line > self.last_special_line) {
                {
                    cur_width = self.second_width;
                    cur_indent = self.second_indent;
                }
            } else {
                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) {
                    {
                        cur_width = self.first_width;
                        cur_indent = self.first_indent;
                    }
                } else {
                    {
                        cur_width = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(cur_line))) as usize)].int();
                        cur_indent = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(cur_line))).wrapping_sub(1i32)) as usize)].int();
                    }
                }
            }
            self.adjust_tail = adjust_head;
            self.pre_adjust_tail = pre_adjust_head;
            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 0i32) {
                self.just_box = self.hpack(q, cur_width, cal_expand_ratio);
            } else {
                self.just_box = self.hpack(q, cur_width, exactly);
            }
            { let __ix1232 = (self.just_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1232) as usize)].set_int(cur_indent); }
            // §1065
            if (self.eqtb[crate::ix::U(((29933i32) - 1) as usize)].int() != self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int()) {
                { let __ix1233 = (self.just_box).wrapping_add(3i32); let __v1234 = self.eqtb[crate::ix::U(((29933i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1233) as usize)].set_int(__v1234); }
            }
            if (self.eqtb[crate::ix::U(((29934i32) - 1) as usize)].int() != self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int()) {
                { let __ix1235 = (self.just_box).wrapping_add(2i32); let __v1236 = self.eqtb[crate::ix::U(((29934i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1235) as usize)].set_int(__v1236); }
            }
            if ((self.eqtb[crate::ix::U(((29931i32) - 1) as usize)].int() != self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int()) && (cur_line == (self.cur_list.pg_field).wrapping_add(1i32))) {
                { let __ix1237 = (self.just_box).wrapping_add(3i32); let __v1238 = self.eqtb[crate::ix::U(((29931i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1237) as usize)].set_int(__v1238); }
            }
            if ((self.eqtb[crate::ix::U(((29932i32) - 1) as usize)].int() != self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int()) && ((cur_line).wrapping_add(1i32) == self.best_line)) {
                { let __ix1239 = (self.just_box).wrapping_add(2i32); let __v1240 = self.eqtb[crate::ix::U(((29932i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1239) as usize)].set_int(__v1240); }
            }
            if (pre_adjust_head != self.pre_adjust_tail) {
                {
                    { let __ix1241 = self.cur_list.tail_field; let __v1242 = self.mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1241) as usize)].set_hh_rh(__v1242); }
                    self.cur_list.tail_field = self.pre_adjust_tail;
                }
            }
            self.pre_adjust_tail = null;
            self.append_to_vlist(self.just_box);
            if (adjust_head != self.adjust_tail) {
                {
                    { let __ix1243 = self.cur_list.tail_field; let __v1244 = self.mem[crate::ix::U((adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1243) as usize)].set_hh_rh(__v1244); }
                    self.cur_list.tail_field = self.adjust_tail;
                }
            }
            self.adjust_tail = null;
            // §1067
            if ((cur_line).wrapping_add(1i32) != self.best_line) {
                {
                    q = self.eqtb[crate::ix::U(((inter_line_penalties_loc) - 1) as usize)].hh().rh();
                    if (q != null) {
                        {
                            r = cur_line;
                            if (r > self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) {
                                r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            }
                            pen = self.mem[crate::ix::U((((q).wrapping_add(r)).wrapping_add(1i32)) as usize)].int();
                        }
                    } else {
                        pen = self.eqtb[crate::ix::U(((29290i32) - 1) as usize)].int();
                    }
                    q = self.eqtb[crate::ix::U(((club_penalties_loc) - 1) as usize)].hh().rh();
                    if (q != null) {
                        {
                            r = (cur_line).wrapping_sub(self.cur_list.pg_field);
                            if (r > self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) {
                                r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            }
                            pen = (pen).wrapping_add(self.mem[crate::ix::U((((q).wrapping_add(r)).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        if (cur_line == (self.cur_list.pg_field).wrapping_add(1i32)) {
                            pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((29282i32) - 1) as usize)].int());
                        }
                    }
                    if d {
                        q = self.eqtb[crate::ix::U(((display_widow_penalties_loc) - 1) as usize)].hh().rh();
                    } else {
                        q = self.eqtb[crate::ix::U(((widow_penalties_loc) - 1) as usize)].hh().rh();
                    }
                    if (q != null) {
                        {
                            r = ((self.best_line).wrapping_sub(cur_line)).wrapping_sub(1i32);
                            if (r > self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) {
                                r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            }
                            pen = (pen).wrapping_add(self.mem[crate::ix::U((((q).wrapping_add(r)).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        if ((cur_line).wrapping_add(2i32) == self.best_line) {
                            if d {
                                pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((29284i32) - 1) as usize)].int());
                            } else {
                                pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((29283i32) - 1) as usize)].int());
                            }
                        }
                    }
                    if disc_break {
                        pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((29285i32) - 1) as usize)].int());
                    }
                    if (pen != 0i32) {
                        {
                            r = self.new_penalty(pen);
                            { let __ix1245 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1245) as usize)].set_hh_rh(r); }
                            self.cur_list.tail_field = r;
                        }
                    }
                }
            }
            // §1053
            cur_line = (cur_line).wrapping_add(1i32);
            self.cur_p = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
            if (self.cur_p != null) {
                if (!post_disc_break) {
                    // §1055
                    {
                        'l_done1_f: {
                            r = temp_head;
                            while true {
                                {
                                    q = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    if (q == self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh()) {
                                        break 'l_done1_f;
                                    }
                                    if (q >= self.hi_mem_min) {
                                        break 'l_done1_f;
                                    }
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() < math_node) {
                                        break 'l_done1_f;
                                    }
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == kern_node) {
                                        if (self.mem[crate::ix::U((q) as usize)].hh().b1() != explicit) {
                                            break 'l_done1_f;
                                        }
                                    }
                                    r = q;
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        if (self.eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                                            // §1708
                                            if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                                {
                                                    if (LR_ptr != null) {
                                                        if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                            {
                                                                self.temp_ptr = LR_ptr;
                                                                LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                {
                                                                    { let __ix1246 = self.temp_ptr; let __v1247 = self.avail; self.mem[crate::ix::U((__ix1246) as usize)].set_hh_rh(__v1247); }
                                                                    self.avail = self.temp_ptr;
                                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.temp_ptr = self.get_avail();
                                                    { let __ix1248 = self.temp_ptr; let __v1249 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1248) as usize)].set_hh_lh(__v1249); }
                                                    { let __ix1250 = self.temp_ptr; self.mem[crate::ix::U((__ix1250) as usize)].set_hh_rh(LR_ptr); }
                                                    LR_ptr = self.temp_ptr;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §1055
                        if (r != temp_head) {
                            {
                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(null);
                                self.flush_node_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
                                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(q);
                            }
                        }
                    }
                }
            }
            if (self.cur_p == null) { break; }
        }
        // §1053
        if ((cur_line != self.best_line) || (self.mem[crate::ix::U((temp_head) as usize)].hh().rh() != null)) {
            self.confusion(1353i32);
        }
        self.cur_list.pg_field = (self.best_line).wrapping_sub(1i32);
        self.cur_list.eTeX_aux_field = LR_ptr;
    }

    /// @<Declare the function called `reconstitute`
    // §1083
    pub fn reconstitute(&mut self, mut j: small_number, mut n: small_number, mut bchar: halfword, mut hchar: halfword) -> small_number {
        let mut reconstitute: small_number = 0;
        let mut p: halfword = 0; // §1083
        let mut t: halfword = 0; // §1083
        let mut q: four_quarters = four_quarters::default(); // §1083
        let mut cur_rh: halfword = 0; // §1083
        let mut test_char: halfword = 0; // §1083
        let mut w: scaled = 0; // §1083
        let mut k: font_index = 0; // §1083
        // goto labels: continue, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.hyphen_passed = 0i32;
                t = hold_head;
                w = 0i32;
                self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh(null);
                // §1085
                self.cur_l = (self.hu[crate::ix::U((j) as usize)]).wrapping_add(0i32);
                self.cur_q = t;
                if (j == 0i32) {
                    {
                        self.ligature_present = self.init_lig;
                        p = self.init_list;
                        if self.ligature_present {
                            self.lft_hit = self.init_lft;
                        }
                        while (p > null) {
                            {
                                {
                                    { let __v1251 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1251); }
                                    t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                    { let __v1252 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1252); }
                                    { let __v1253 = self.mem[crate::ix::U((p) as usize)].hh().b1(); self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1253); }
                                }
                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            }
                        }
                    }
                } else {
                    if (self.cur_l < non_char) {
                        {
                            { let __v1254 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1254); }
                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                            { let __v1255 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1255); }
                            { let __v1256 = self.cur_l; self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1256); }
                        }
                    }
                }
                self.lig_stack = null;
                {
                    if (j < n) {
                        self.cur_r = (self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]).wrapping_add(0i32);
                    } else {
                        self.cur_r = bchar;
                    }
                    if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                        cur_rh = hchar;
                    } else {
                        cur_rh = non_char;
                    }
                }
            }
            if __goto_1 <= 1 { // continue
                // §1083
                if (self.cur_l == non_char) {
                    // §1086
                    {
                        k = self.bchar_label[crate::ix::U((self.hf) as usize)];
                        if (k == non_address) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        } else {
                            q = self.font_info[crate::ix::U((k) as usize)].qqqq();
                        }
                    }
                } else {
                    {
                        q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.hf) as usize)]).wrapping_add(self.cur_l)) as usize)].qqqq();
                        if (((q.b2()).wrapping_sub(0i32) % 4i32) != lig_tag) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                        k = (self.lig_kern_base[crate::ix::U((self.hf) as usize)]).wrapping_add(q.b3());
                        q = self.font_info[crate::ix::U((k) as usize)].qqqq();
                        if (q.b0() > stop_flag) {
                            {
                                k = ((((self.lig_kern_base[crate::ix::U((self.hf) as usize)]).wrapping_add((256i32).wrapping_mul(q.b2()))).wrapping_add(q.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                q = self.font_info[crate::ix::U((k) as usize)].qqqq();
                            }
                        }
                    }
                }
                if (cur_rh < non_char) {
                    test_char = cur_rh;
                } else {
                    test_char = self.cur_r;
                }
                while true {
                    {
                        if (q.b1() == test_char) {
                            if (q.b0() <= stop_flag) {
                                if (cur_rh < non_char) {
                                    {
                                        self.hyphen_passed = j;
                                        hchar = non_char;
                                        cur_rh = non_char;
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    {
                                        if (hchar < non_char) {
                                            if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                                {
                                                    self.hyphen_passed = j;
                                                    hchar = non_char;
                                                }
                                            }
                                        }
                                        if (q.b2() < kern_flag) {
                                            // §1088
                                            {
                                                if (self.cur_l == non_char) {
                                                    self.lft_hit = true;
                                                }
                                                if (j == n) {
                                                    if (self.lig_stack == null) {
                                                        self.rt_hit = true;
                                                    }
                                                }
                                                {
                                                    if (self.interrupt != 0i32) {
                                                        self.pause_for_instructions();
                                                    }
                                                }
                                                match q.b2() {
                                                    1 | 5 => {
                                                        {
                                                            self.cur_l = q.b3();
                                                            self.ligature_present = true;
                                                        }
                                                    }
                                                    2 | 6 => {
                                                        {
                                                            self.cur_r = q.b3();
                                                            if (self.lig_stack > null) {
                                                                { let __ix1257 = self.lig_stack; let __v1258 = self.cur_r; self.mem[crate::ix::U((__ix1257) as usize)].set_hh_b1(__v1258); }
                                                            } else {
                                                                {
                                                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                                                    if (j == n) {
                                                                        bchar = non_char;
                                                                    } else {
                                                                        {
                                                                            p = self.get_avail();
                                                                            { let __ix1259 = (self.lig_stack).wrapping_add(1i32); self.mem[crate::ix::U((__ix1259) as usize)].set_hh_rh(p); }
                                                                            { let __v1260 = (self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]).wrapping_add(0i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1260); }
                                                                            { let __v1261 = self.hf; self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1261); }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    3 => {
                                                        {
                                                            self.cur_r = q.b3();
                                                            p = self.lig_stack;
                                                            self.lig_stack = self.new_lig_item(self.cur_r);
                                                            { let __ix1262 = self.lig_stack; self.mem[crate::ix::U((__ix1262) as usize)].set_hh_rh(p); }
                                                        }
                                                    }
                                                    7 | 11 => {
                                                        {
                                                            if self.ligature_present {
                                                                {
                                                                    p = self.new_ligature(self.hf, self.cur_l, self.mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                                                    if self.lft_hit {
                                                                        {
                                                                            self.mem[crate::ix::U((p) as usize)].set_hh_b1(2i32);
                                                                            self.lft_hit = false;
                                                                        }
                                                                    }
                                                                    if false {
                                                                        if (self.lig_stack == null) {
                                                                            {
                                                                                { let __v1263 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1263); }
                                                                                self.rt_hit = false;
                                                                            }
                                                                        }
                                                                    }
                                                                    { let __ix1264 = self.cur_q; self.mem[crate::ix::U((__ix1264) as usize)].set_hh_rh(p); }
                                                                    t = p;
                                                                    self.ligature_present = false;
                                                                }
                                                            }
                                                            self.cur_q = t;
                                                            self.cur_l = q.b3();
                                                            self.ligature_present = true;
                                                        }
                                                    }
                                                    _ => {
                                                        {
                                                            self.cur_l = q.b3();
                                                            self.ligature_present = true;
                                                            if (self.lig_stack > null) {
                                                                {
                                                                    if (self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh() > null) {
                                                                        {
                                                                            { let __v1265 = self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1265); }
                                                                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                                            j = (j).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                    p = self.lig_stack;
                                                                    self.lig_stack = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                    self.free_node(p, small_node_size);
                                                                    if (self.lig_stack == null) {
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = (self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]).wrapping_add(0i32);
                                                                            } else {
                                                                                self.cur_r = bchar;
                                                                            }
                                                                            if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                                                                cur_rh = hchar;
                                                                            } else {
                                                                                cur_rh = non_char;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        self.cur_r = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                                                                    }
                                                                }
                                                            } else {
                                                                if (j == n) {
                                                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                                                } else {
                                                                    {
                                                                        {
                                                                            { let __v1266 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1266); }
                                                                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                                            { let __v1267 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1267); }
                                                                            { let __v1268 = self.cur_r; self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1268); }
                                                                        }
                                                                        j = (j).wrapping_add(1i32);
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = (self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]).wrapping_add(0i32);
                                                                            } else {
                                                                                self.cur_r = bchar;
                                                                            }
                                                                            if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                                                                cur_rh = hchar;
                                                                            } else {
                                                                                cur_rh = non_char;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                if (q.b2() > 4i32) {
                                                    if (q.b2() != 7i32) {
                                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                            }
                                        }
                                        // §1086
                                        w = self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((self.hf) as usize)]).wrapping_add((256i32).wrapping_mul(q.b2()))).wrapping_add(q.b3())) as usize)].int();
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                        }
                        if (q.b0() >= stop_flag) {
                            if (cur_rh == non_char) {
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            } else {
                                {
                                    cur_rh = non_char;
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                        }
                        k = ((k).wrapping_add(q.b0())).wrapping_add(1i32);
                        q = self.font_info[crate::ix::U((k) as usize)].qqqq();
                    }
                }
            }
            if __goto_1 <= 2 { // done
                // §1087
                if self.ligature_present {
                    {
                        p = self.new_ligature(self.hf, self.cur_l, self.mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                        if self.lft_hit {
                            {
                                self.mem[crate::ix::U((p) as usize)].set_hh_b1(2i32);
                                self.lft_hit = false;
                            }
                        }
                        if self.rt_hit {
                            if (self.lig_stack == null) {
                                {
                                    { let __v1269 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1269); }
                                    self.rt_hit = false;
                                }
                            }
                        }
                        { let __ix1270 = self.cur_q; self.mem[crate::ix::U((__ix1270) as usize)].set_hh_rh(p); }
                        t = p;
                        self.ligature_present = false;
                    }
                }
                if (w != 0i32) {
                    {
                        { let __v1271 = self.new_kern(w); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1271); }
                        t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                        w = 0i32;
                        self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(0i32);
                    }
                }
                if (self.lig_stack > null) {
                    {
                        self.cur_q = t;
                        self.cur_l = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                        self.ligature_present = true;
                        {
                            if (self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh() > null) {
                                {
                                    { let __v1272 = self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1272); }
                                    t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                    j = (j).wrapping_add(1i32);
                                }
                            }
                            p = self.lig_stack;
                            self.lig_stack = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            self.free_node(p, small_node_size);
                            if (self.lig_stack == null) {
                                {
                                    if (j < n) {
                                        self.cur_r = (self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]).wrapping_add(0i32);
                                    } else {
                                        self.cur_r = bchar;
                                    }
                                    if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                        cur_rh = hchar;
                                    } else {
                                        cur_rh = non_char;
                                    }
                                }
                            } else {
                                self.cur_r = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                            }
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                // §1083
                reconstitute = j;
            }
            break 'l_dispatch_1;
        }
        reconstitute
    }

    /// @<Declare subprocedures for `line_break`
    // §1072
    pub fn hyphenate(&mut self) {
        let mut i: i32 = 0; // §1078
        let mut j: i32 = 0; // §1078
        let mut l: i32 = 0; // §1078
        let mut q: halfword = 0; // §1078
        let mut r: halfword = 0; // §1078
        let mut s: halfword = 0; // §1078
        let mut bchar: halfword = 0; // §1078
        let mut major_tail: halfword = 0; // §1089
        let mut minor_tail: halfword = 0; // §1089
        let mut c: ASCII_code = 0; // §1089
        let mut c_loc: i32 = 0; // §1089
        let mut r_count: i32 = 0; // §1089
        let mut hyf_node: halfword = 0; // §1089
        let mut z: trie_pointer = 0; // §1099
        let mut v: i32 = 0; // §1099
        let mut h: hyph_pointer = 0; // §1106
        let mut k: str_number = 0; // §1106
        let mut u: pool_pointer = 0; // §1106
        'l_exit_f: {
            'l_common_ending_f: {
                'l_found2_f: {
                    'l_found1_f: {
                        'l_found_f: {
                            'l_not_found_f: {
                                // §1100
                                {
                                    let __for_end_8 = self.hn;
                                    j = 0i32;
                                    while j <= __for_end_8 {
                                        self.hyf[crate::ix::U((j) as usize)] = 0i32;
                                        j = j.wrapping_add(1);
                                    }
                                }
                                // §1107
                                h = self.hc[crate::ix::U((1i32) as usize)];
                                self.hn = (self.hn).wrapping_add(1i32);
                                { let __ix1273 = self.hn; let __v1274 = self.cur_lang; self.hc[crate::ix::U((__ix1273) as usize)] = __v1274; }
                                {
                                    let __for_end_8 = self.hn;
                                    j = 2i32;
                                    while j <= __for_end_8 {
                                        h = (((h).wrapping_add(h)).wrapping_add(self.hc[crate::ix::U((j) as usize)]) % hyph_size);
                                        j = j.wrapping_add(1);
                                    }
                                }
                                while true {
                                    {
                                        'l_done_f: {
                                            // §1108
                                            k = self.hyph_word[crate::ix::U((h) as usize)];
                                            if (k == 0i32) {
                                                break 'l_not_found_f;
                                            }
                                            if ((self.str_start[crate::ix::U(((k).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((k) as usize)]) < self.hn) {
                                                break 'l_not_found_f;
                                            }
                                            if ((self.str_start[crate::ix::U(((k).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((k) as usize)]) == self.hn) {
                                                {
                                                    j = 1i32;
                                                    u = self.str_start[crate::ix::U((k) as usize)];
                                                    loop {
                                                        if (self.str_pool[crate::ix::U((u) as usize)] < self.hc[crate::ix::U((j) as usize)]) {
                                                            break 'l_not_found_f;
                                                        }
                                                        if (self.str_pool[crate::ix::U((u) as usize)] > self.hc[crate::ix::U((j) as usize)]) {
                                                            break 'l_done_f;
                                                        }
                                                        j = (j).wrapping_add(1i32);
                                                        u = (u).wrapping_add(1i32);
                                                        if (j > self.hn) { break; }
                                                    }
                                                    // §1109
                                                    s = self.hyph_list[crate::ix::U((h) as usize)];
                                                    while (s != null) {
                                                        {
                                                            self.hyf[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().lh()) as usize)] = 1i32;
                                                            s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                        }
                                                    }
                                                    // §1108
                                                    self.hn = (self.hn).wrapping_sub(1i32);
                                                    break 'l_found_f;
                                                }
                                            }
                                        }
                                        // §1107
                                        if (h > 0i32) {
                                            h = (h).wrapping_sub(1i32);
                                        } else {
                                            h = hyph_size;
                                        }
                                    }
                                }
                            }
                            self.hn = (self.hn).wrapping_sub(1i32);
                            // §1100
                            if (self.trie[crate::ix::U(((self.cur_lang).wrapping_add(1i32)) as usize)].b1() != (self.cur_lang).wrapping_add(0i32)) {
                                break 'l_exit_f;
                            }
                            self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                            self.hc[crate::ix::U(((self.hn).wrapping_add(1i32)) as usize)] = 0i32;
                            self.hc[crate::ix::U(((self.hn).wrapping_add(2i32)) as usize)] = 256i32;
                            {
                                let __for_end_7 = ((self.hn).wrapping_sub(self.r_hyf)).wrapping_add(1i32);
                                j = 0i32;
                                while j <= __for_end_7 {
                                    {
                                        z = (self.trie[crate::ix::U(((self.cur_lang).wrapping_add(1i32)) as usize)].rh()).wrapping_add(self.hc[crate::ix::U((j) as usize)]);
                                        l = j;
                                        while (self.hc[crate::ix::U((l) as usize)] == (self.trie[crate::ix::U((z) as usize)].b1()).wrapping_sub(0i32)) {
                                            {
                                                if (self.trie[crate::ix::U((z) as usize)].b0() != min_quarterword) {
                                                    // §1101
                                                    {
                                                        v = self.trie[crate::ix::U((z) as usize)].b0();
                                                        loop {
                                                            v = (v).wrapping_add(self.op_start[crate::ix::U((self.cur_lang) as usize)]);
                                                            i = (l).wrapping_sub(self.hyf_distance[crate::ix::U(((v) - 1) as usize)]);
                                                            if (self.hyf_num[crate::ix::U(((v) - 1) as usize)] > self.hyf[crate::ix::U((i) as usize)]) {
                                                                { let __v1275 = self.hyf_num[crate::ix::U(((v) - 1) as usize)]; self.hyf[crate::ix::U((i) as usize)] = __v1275; }
                                                            }
                                                            v = self.hyf_next[crate::ix::U(((v) - 1) as usize)];
                                                            if (v == min_quarterword) { break; }
                                                        }
                                                    }
                                                }
                                                // §1100
                                                l = (l).wrapping_add(1i32);
                                                z = (self.trie[crate::ix::U((z) as usize)].rh()).wrapping_add(self.hc[crate::ix::U((l) as usize)]);
                                            }
                                        }
                                    }
                                    j = j.wrapping_add(1);
                                }
                            }
                        }
                        {
                            let __for_end_6 = (self.l_hyf).wrapping_sub(1i32);
                            j = 0i32;
                            while j <= __for_end_6 {
                                self.hyf[crate::ix::U((j) as usize)] = 0i32;
                                j = j.wrapping_add(1);
                            }
                        }
                        {
                            let __for_end_6 = (self.r_hyf).wrapping_sub(1i32);
                            j = 0i32;
                            while j <= __for_end_6 {
                                self.hyf[crate::ix::U(((self.hn).wrapping_sub(j)) as usize)] = 0i32;
                                j = j.wrapping_add(1);
                            }
                        }
                        // §1079
                        {
                            let __for_end_6 = (self.hn).wrapping_sub(self.r_hyf);
                            j = self.l_hyf;
                            while j <= __for_end_6 {
                                if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                    break 'l_found1_f;
                                }
                                j = j.wrapping_add(1);
                            }
                        }
                        break 'l_exit_f;
                    }
                    // §1080
                    q = self.mem[crate::ix::U((self.hb) as usize)].hh().rh();
                    { let __ix1276 = self.hb; self.mem[crate::ix::U((__ix1276) as usize)].set_hh_rh(null); }
                    r = self.mem[crate::ix::U((self.ha) as usize)].hh().rh();
                    { let __ix1277 = self.ha; self.mem[crate::ix::U((__ix1277) as usize)].set_hh_rh(null); }
                    bchar = self.hyf_bchar;
                    if (self.ha >= self.hi_mem_min) {
                        if (self.mem[crate::ix::U((self.ha) as usize)].hh().b0() != self.hf) {
                            break 'l_found2_f;
                        } else {
                            {
                                self.init_list = self.ha;
                                self.init_lig = false;
                                { let __v1278 = (self.mem[crate::ix::U((self.ha) as usize)].hh().b1()).wrapping_sub(0i32); self.hu[crate::ix::U((0i32) as usize)] = __v1278; }
                            }
                        }
                    } else {
                        if (self.mem[crate::ix::U((self.ha) as usize)].hh().b0() == ligature_node) {
                            if (self.mem[crate::ix::U(((self.ha).wrapping_add(1i32)) as usize)].hh().b0() != self.hf) {
                                break 'l_found2_f;
                            } else {
                                {
                                    self.init_list = self.mem[crate::ix::U(((self.ha).wrapping_add(1i32)) as usize)].hh().rh();
                                    self.init_lig = true;
                                    self.init_lft = (self.mem[crate::ix::U((self.ha) as usize)].hh().b1() > 1i32);
                                    { let __v1279 = (self.mem[crate::ix::U(((self.ha).wrapping_add(1i32)) as usize)].hh().b1()).wrapping_sub(0i32); self.hu[crate::ix::U((0i32) as usize)] = __v1279; }
                                    if (self.init_list == null) {
                                        if self.init_lft {
                                            {
                                                self.hu[crate::ix::U((0i32) as usize)] = 256i32;
                                                self.init_lig = false;
                                            }
                                        }
                                    }
                                    self.free_node(self.ha, small_node_size);
                                }
                            }
                        } else {
                            {
                                if (!(r >= self.hi_mem_min)) {
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node) {
                                        if (self.mem[crate::ix::U((r) as usize)].hh().b1() > 1i32) {
                                            break 'l_found2_f;
                                        }
                                    }
                                }
                                j = 1i32;
                                s = self.ha;
                                self.init_list = null;
                                break 'l_common_ending_f;
                            }
                        }
                    }
                    s = self.cur_p;
                    while (self.mem[crate::ix::U((s) as usize)].hh().rh() != self.ha) {
                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                    }
                    j = 0i32;
                    break 'l_common_ending_f;
                }
                s = self.ha;
                j = 0i32;
                self.hu[crate::ix::U((0i32) as usize)] = 256i32;
                self.init_lig = false;
                self.init_list = null;
            }
            self.flush_node_list(r);
            // §1090
            loop {
                l = j;
                j = (self.reconstitute(j, self.hn, bchar, (self.hyf_char).wrapping_add(0i32))).wrapping_add(1i32);
                if (self.hyphen_passed == 0i32) {
                    {
                        { let __v1280 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1280); }
                        while (self.mem[crate::ix::U((s) as usize)].hh().rh() > null) {
                            s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                        }
                        if (((self.hyf[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]) % 2) != 0) {
                            {
                                l = j;
                                self.hyphen_passed = (j).wrapping_sub(1i32);
                                self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh(null);
                            }
                        }
                    }
                }
                if (self.hyphen_passed > 0i32) {
                    // §1091
                    loop {
                        r = self.get_node(small_node_size);
                        { let __v1281 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1281); }
                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(disc_node);
                        major_tail = r;
                        r_count = 0i32;
                        while (self.mem[crate::ix::U((major_tail) as usize)].hh().rh() > null) {
                            {
                                major_tail = self.mem[crate::ix::U((major_tail) as usize)].hh().rh();
                                r_count = (r_count).wrapping_add(1i32);
                            }
                        }
                        i = self.hyphen_passed;
                        self.hyf[crate::ix::U((i) as usize)] = 0i32;
                        // §1092
                        minor_tail = null;
                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(null);
                        hyf_node = self.new_character(self.hf, self.hyf_char);
                        if (hyf_node != null) {
                            {
                                i = (i).wrapping_add(1i32);
                                c = self.hu[crate::ix::U((i) as usize)];
                                { let __v1282 = self.hyf_char; self.hu[crate::ix::U((i) as usize)] = __v1282; }
                                {
                                    { let __v1283 = self.avail; self.mem[crate::ix::U((hyf_node) as usize)].set_hh_rh(__v1283); }
                                    self.avail = hyf_node;
                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                }
                            }
                        }
                        while (l <= i) {
                            {
                                l = (self.reconstitute(l, i, self.font_bchar[crate::ix::U((self.hf) as usize)], non_char)).wrapping_add(1i32);
                                if (self.mem[crate::ix::U((hold_head) as usize)].hh().rh() > null) {
                                    {
                                        if (minor_tail == null) {
                                            { let __v1284 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v1284); }
                                        } else {
                                            { let __v1285 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((minor_tail) as usize)].set_hh_rh(__v1285); }
                                        }
                                        minor_tail = self.mem[crate::ix::U((hold_head) as usize)].hh().rh();
                                        while (self.mem[crate::ix::U((minor_tail) as usize)].hh().rh() > null) {
                                            minor_tail = self.mem[crate::ix::U((minor_tail) as usize)].hh().rh();
                                        }
                                    }
                                }
                            }
                        }
                        if (hyf_node != null) {
                            {
                                self.hu[crate::ix::U((i) as usize)] = c;
                                l = i;
                                i = (i).wrapping_sub(1i32);
                            }
                        }
                        // §1093
                        minor_tail = null;
                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                        c_loc = 0i32;
                        if (self.bchar_label[crate::ix::U((self.hf) as usize)] != non_address) {
                            {
                                l = (l).wrapping_sub(1i32);
                                c = self.hu[crate::ix::U((l) as usize)];
                                c_loc = l;
                                self.hu[crate::ix::U((l) as usize)] = 256i32;
                            }
                        }
                        while (l < j) {
                            {
                                loop {
                                    l = (self.reconstitute(l, self.hn, bchar, non_char)).wrapping_add(1i32);
                                    if (c_loc > 0i32) {
                                        {
                                            self.hu[crate::ix::U((c_loc) as usize)] = c;
                                            c_loc = 0i32;
                                        }
                                    }
                                    if (self.mem[crate::ix::U((hold_head) as usize)].hh().rh() > null) {
                                        {
                                            if (minor_tail == null) {
                                                { let __v1286 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v1286); }
                                            } else {
                                                { let __v1287 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((minor_tail) as usize)].set_hh_rh(__v1287); }
                                            }
                                            minor_tail = self.mem[crate::ix::U((hold_head) as usize)].hh().rh();
                                            while (self.mem[crate::ix::U((minor_tail) as usize)].hh().rh() > null) {
                                                minor_tail = self.mem[crate::ix::U((minor_tail) as usize)].hh().rh();
                                            }
                                        }
                                    }
                                    if (l >= j) { break; }
                                }
                                while (l > j) {
                                    // §1094
                                    {
                                        j = (self.reconstitute(j, self.hn, bchar, non_char)).wrapping_add(1i32);
                                        { let __v1288 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((major_tail) as usize)].set_hh_rh(__v1288); }
                                        while (self.mem[crate::ix::U((major_tail) as usize)].hh().rh() > null) {
                                            {
                                                major_tail = self.mem[crate::ix::U((major_tail) as usize)].hh().rh();
                                                r_count = (r_count).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §1095
                        if (r_count > 127i32) {
                            {
                                { let __v1289 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1289); }
                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(null);
                                self.flush_node_list(r);
                            }
                        } else {
                            {
                                self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                self.mem[crate::ix::U((r) as usize)].set_hh_b1(r_count);
                            }
                        }
                        s = major_tail;
                        // §1091
                        self.hyphen_passed = (j).wrapping_sub(1i32);
                        self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh(null);
                        if (!(((self.hyf[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]) % 2) != 0)) { break; }
                    }
                }
                if (j > self.hn) { break; }
            }
            // §1090
            self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
            // §1080
            self.flush_list(self.init_list);
        }
        // §1072
    }

    /// It's tempting to remove the `overflow` stops in the following procedure;
    /// `new_trie_op` could return `min_quarterword` (thereby simply ignoring
    /// part of a hyphenation pattern) instead of aborting the job. However, that would
    /// lead to different hyphenation results on different installations of \TeX\
    /// using the same patterns. The `overflow` stops are necessary for portability
    /// of patterns.
    /// @<Declare procedures for preprocessing hyph...
    // §1121
    pub fn new_trie_op(&mut self, mut d: small_number, mut n: small_number, mut v: quarterword) -> quarterword {
        let mut new_trie_op: quarterword = 0;
        let mut h: i32 = 0; // §1121
        let mut u: quarterword = 0; // §1121
        let mut l: i32 = 0; // §1121
        'l_exit_f: {
            h = ((((((n).wrapping_add((313i32).wrapping_mul(d))).wrapping_add((361i32).wrapping_mul(v))).wrapping_add((1009i32).wrapping_mul(self.cur_lang))).wrapping_abs() % (trie_op_size).wrapping_add(trie_op_size))).wrapping_sub(trie_op_size);
            while true {
                {
                    l = self.trie_op_hash[crate::ix::U(((h) + 35111) as usize)];
                    if (l == 0i32) {
                        {
                            if (self.trie_op_ptr == trie_op_size) {
                                self.overflow(1363i32, trie_op_size);
                            }
                            u = self.trie_used[crate::ix::U((self.cur_lang) as usize)];
                            if (u == max_trie_op) {
                                self.overflow(1364i32, 65535i32);
                            }
                            self.trie_op_ptr = (self.trie_op_ptr).wrapping_add(1i32);
                            u = (u).wrapping_add(1i32);
                            self.trie_used[crate::ix::U((self.cur_lang) as usize)] = u;
                            self.hyf_distance[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = d;
                            self.hyf_num[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = n;
                            self.hyf_next[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = v;
                            { let __ix1290 = self.trie_op_ptr; let __v1291 = self.cur_lang; self.trie_op_lang[crate::ix::U(((__ix1290) - 1) as usize)] = __v1291; }
                            { let __v1292 = self.trie_op_ptr; self.trie_op_hash[crate::ix::U(((h) + 35111) as usize)] = __v1292; }
                            self.trie_op_val[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = u;
                            new_trie_op = u;
                            break 'l_exit_f;
                        }
                    }
                    if ((((self.hyf_distance[crate::ix::U(((l) - 1) as usize)] == d) && (self.hyf_num[crate::ix::U(((l) - 1) as usize)] == n)) && (self.hyf_next[crate::ix::U(((l) - 1) as usize)] == v)) && (self.trie_op_lang[crate::ix::U(((l) - 1) as usize)] == self.cur_lang)) {
                        {
                            new_trie_op = self.trie_op_val[crate::ix::U(((l) - 1) as usize)];
                            break 'l_exit_f;
                        }
                    }
                    if (h > (trie_op_size).wrapping_neg()) {
                        h = (h).wrapping_sub(1i32);
                    } else {
                        h = trie_op_size;
                    }
                }
            }
        }
        new_trie_op
    }

    /// Let us suppose that a linked trie has already been constructed.
    /// Experience shows that we can often reduce its size by recognizing common
    /// subtries; therefore another hash table is introduced for this purpose,
    /// somewhat similar to `trie_op_hash`. The new hash table will be
    /// initialized to zero.
    /// The function `trie_node(p)` returns `p` if `p` is distinct from other nodes
    /// that it has seen, otherwise it returns the number of the first equivalent
    /// node that it has seen.
    /// Notice that we might make subtries equivalent even if they correspond to
    /// patterns for different languages, in which the trie ops might mean quite
    /// different things. That's perfectly all right.
    /// @<Declare procedures for preprocessing hyph...
    // §1125
    pub fn trie_node(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut trie_node: trie_pointer = 0;
        let mut h: trie_pointer = 0; // §1125
        let mut q: trie_pointer = 0; // §1125
        'l_exit_f: {
            h = (((((self.trie_c[crate::ix::U((p) as usize)]).wrapping_add((1009i32).wrapping_mul(self.trie_o[crate::ix::U((p) as usize)]))).wrapping_add((2718i32).wrapping_mul(self.trie_l[crate::ix::U((p) as usize)]))).wrapping_add((3142i32).wrapping_mul(self.trie_r[crate::ix::U((p) as usize)]))).wrapping_abs() % trie_size);
            while true {
                {
                    q = self.trie_hash[crate::ix::U((h) as usize)];
                    if (q == 0i32) {
                        {
                            self.trie_hash[crate::ix::U((h) as usize)] = p;
                            trie_node = p;
                            break 'l_exit_f;
                        }
                    }
                    if ((((self.trie_c[crate::ix::U((q) as usize)] == self.trie_c[crate::ix::U((p) as usize)]) && (self.trie_o[crate::ix::U((q) as usize)] == self.trie_o[crate::ix::U((p) as usize)])) && (self.trie_l[crate::ix::U((q) as usize)] == self.trie_l[crate::ix::U((p) as usize)])) && (self.trie_r[crate::ix::U((q) as usize)] == self.trie_r[crate::ix::U((p) as usize)])) {
                        {
                            trie_node = q;
                            break 'l_exit_f;
                        }
                    }
                    if (h > 0i32) {
                        h = (h).wrapping_sub(1i32);
                    } else {
                        h = trie_size;
                    }
                }
            }
        }
        trie_node
    }

    /// A neat recursive procedure is now able to compress a trie by
    /// traversing it and applying `trie_node` to its nodes in ``bottom up''
    /// fashion. We will compress the entire trie by clearing `trie_hash` to
    /// zero and then saying ``trie_root:=compress_trie(trie_root)`'.
    /// @<Declare procedures for preprocessing hyph...
    // §1126
    pub fn compress_trie(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut compress_trie: trie_pointer = 0;
        if (p == 0i32) {
            compress_trie = 0i32;
        } else {
            {
                { let __v1293 = self.compress_trie(self.trie_l[crate::ix::U((p) as usize)]); self.trie_l[crate::ix::U((p) as usize)] = __v1293; }
                { let __v1294 = self.compress_trie(self.trie_r[crate::ix::U((p) as usize)]); self.trie_r[crate::ix::U((p) as usize)] = __v1294; }
                compress_trie = self.trie_node(p);
            }
        }
        compress_trie
    }

    /// The `first_fit` procedure finds the smallest hole `z` in `trie` such that
    /// a trie family starting at a given node `p` will fit into vacant positions
    /// starting at `z`. If `c=trie_c[p]`, this means that location `z-c` must
    /// not already be taken by some other family, and that `z-c+@t$c^\prime$@>`
    /// must be vacant for all characters $c^\prime$ in the family. The procedure
    /// sets `trie_ref[p]` to `z-c` when the first fit has been found.
    /// @<Declare procedures for preprocessing hyph...
    // §1130
    pub fn first_fit(&mut self, mut p: trie_pointer) {
        let mut h: trie_pointer = 0; // §1130
        let mut z: trie_pointer = 0; // §1130
        let mut q: trie_pointer = 0; // §1130
        let mut c: ASCII_code = 0; // §1130
        let mut l: trie_pointer = 0; // §1130
        let mut r: trie_pointer = 0; // §1130
        let mut ll: i32 = 0; // §1130
        'l_found_f: {
            c = self.trie_c[crate::ix::U((p) as usize)];
            z = self.trie_min[crate::ix::U((c) as usize)];
            while true {
                {
                    'l_not_found_f: {
                        h = (z).wrapping_sub(c);
                        // §1131
                        if (self.trie_max < (h).wrapping_add(256i32)) {
                            {
                                if (trie_size <= (h).wrapping_add(256i32)) {
                                    self.overflow(1365i32, trie_size);
                                }
                                loop {
                                    self.trie_max = (self.trie_max).wrapping_add(1i32);
                                    { let __ix1295 = self.trie_max; let __v1296 = false; self.trie_taken[crate::ix::U(((__ix1295) - 1) as usize)] = __v1296; }
                                    { let __ix1297 = self.trie_max; let __v1298 = (self.trie_max).wrapping_add(1i32); self.trie[crate::ix::U((__ix1297) as usize)].set_rh(__v1298); }
                                    { let __ix1299 = self.trie_max; let __v1300 = (self.trie_max).wrapping_sub(1i32); self.trie[crate::ix::U((__ix1299) as usize)].set_lh(__v1300); }
                                    if (self.trie_max == (h).wrapping_add(256i32)) { break; }
                                }
                            }
                        }
                        // §1130
                        if self.trie_taken[crate::ix::U(((h) - 1) as usize)] {
                            break 'l_not_found_f;
                        }
                        // §1132
                        q = self.trie_r[crate::ix::U((p) as usize)];
                        while (q > 0i32) {
                            {
                                if (self.trie[crate::ix::U(((h).wrapping_add(self.trie_c[crate::ix::U((q) as usize)])) as usize)].rh() == 0i32) {
                                    break 'l_not_found_f;
                                }
                                q = self.trie_r[crate::ix::U((q) as usize)];
                            }
                        }
                        break 'l_found_f;
                    }
                    // §1130
                    z = self.trie[crate::ix::U((z) as usize)].rh();
                }
            }
        }
        { let __v1301 = true; self.trie_taken[crate::ix::U(((h) - 1) as usize)] = __v1301; }
        // §1133
        self.trie_hash[crate::ix::U((p) as usize)] = h;
        q = p;
        loop {
            z = (h).wrapping_add(self.trie_c[crate::ix::U((q) as usize)]);
            l = self.trie[crate::ix::U((z) as usize)].lh();
            r = self.trie[crate::ix::U((z) as usize)].rh();
            self.trie[crate::ix::U((r) as usize)].set_lh(l);
            self.trie[crate::ix::U((l) as usize)].set_rh(r);
            self.trie[crate::ix::U((z) as usize)].set_rh(0i32);
            if (l < 256i32) {
                {
                    if (z < 256i32) {
                        ll = z;
                    } else {
                        ll = 256i32;
                    }
                    loop {
                        self.trie_min[crate::ix::U((l) as usize)] = r;
                        l = (l).wrapping_add(1i32);
                        if (l == ll) { break; }
                    }
                }
            }
            q = self.trie_r[crate::ix::U((q) as usize)];
            if (q == 0i32) { break; }
        }
    }

    /// To pack the entire linked trie, we use the following recursive procedure.
    /// @<Declare procedures for preprocessing hyph...
    // §1134
    pub fn trie_pack(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §1134
        loop {
            q = self.trie_l[crate::ix::U((p) as usize)];
            if ((q > 0i32) && (self.trie_hash[crate::ix::U((q) as usize)] == 0i32)) {
                {
                    self.first_fit(q);
                    self.trie_pack(q);
                }
            }
            p = self.trie_r[crate::ix::U((p) as usize)];
            if (p == 0i32) { break; }
        }
    }

    /// The fixing-up procedure is, of course, recursive. Since the linked trie
    /// usually has overlapping subtries, the same data may be moved several
    /// times; but that causes no harm, and at most as much work is done as it
    /// took to build the uncompressed trie.
    /// @<Declare procedures for preprocessing hyph...
    // §1136
    pub fn trie_fix(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §1136
        let mut c: ASCII_code = 0; // §1136
        let mut z: trie_pointer = 0; // §1136
        z = self.trie_hash[crate::ix::U((p) as usize)];
        loop {
            q = self.trie_l[crate::ix::U((p) as usize)];
            c = self.trie_c[crate::ix::U((p) as usize)];
            { let __v1302 = self.trie_hash[crate::ix::U((q) as usize)]; self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_rh(__v1302); }
            self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_b1((c).wrapping_add(0i32));
            { let __v1303 = self.trie_o[crate::ix::U((p) as usize)]; self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_b0(__v1303); }
            if (q > 0i32) {
                self.trie_fix(q);
            }
            p = self.trie_r[crate::ix::U((p) as usize)];
            if (p == 0i32) { break; }
        }
    }

    /// Now let's go back to the easier problem, of building the linked
    /// trie.  When \.{INITEX} has scanned the `\.{\\patterns}' control
    /// sequence, it calls on `new_patterns` to do the right thing.
    /// @<Declare procedures for preprocessing hyph...
    // §1137
    pub fn new_patterns(&mut self) {
        let mut k: i32 = 0; // §1137
        let mut l: i32 = 0; // §1137
        let mut digit_sensed: bool = false; // §1137
        let mut v: quarterword = 0; // §1137
        let mut p: trie_pointer = 0; // §1137
        let mut q: trie_pointer = 0; // §1137
        let mut first_child: bool = false; // §1137
        let mut c: ASCII_code = 0; // §1137
        if self.trie_not_ready {
            {
                'l_done_f: {
                    if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() <= 0i32) {
                        self.cur_lang = 0i32;
                    } else {
                        if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() > 255i32) {
                            self.cur_lang = 0i32;
                        } else {
                            self.cur_lang = self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int();
                        }
                    }
                    self.scan_left_brace();
                    // §1138
                    k = 0i32;
                    self.hyf[crate::ix::U((0i32) as usize)] = 0i32;
                    digit_sensed = false;
                    while true {
                        {
                            self.get_x_token();
                            match self.cur_cmd {
                                letter | other_char => {
                                    // §1139
                                    if ((digit_sensed || (self.cur_chr < 48i32)) || (self.cur_chr > 57i32)) {
                                        {
                                            if (self.cur_chr == 46i32) {
                                                self.cur_chr = 0i32;
                                            } else {
                                                {
                                                    self.cur_chr = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                    if (self.cur_chr == 0i32) {
                                                        {
                                                            {
                                                                self.dg_mark();
                                                                if (self.interaction == error_stop_mode) {
                                                                }
                                                                if self.file_line_error_style_p {
                                                                    self.print_file_line();
                                                                } else {
                                                                    self.print_nl(264i32);
                                                                }
                                                                self.print(1371i32);
                                                            }
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[crate::ix::U((0i32) as usize)] = 1370i32;
                                                            }
                                                            self.error();
                                                        }
                                                    }
                                                }
                                            }
                                            if (k < 63i32) {
                                                {
                                                    k = (k).wrapping_add(1i32);
                                                    { let __v1304 = self.cur_chr; self.hc[crate::ix::U((k) as usize)] = __v1304; }
                                                    self.hyf[crate::ix::U((k) as usize)] = 0i32;
                                                    digit_sensed = false;
                                                }
                                            }
                                        }
                                    } else {
                                        if (k < 63i32) {
                                            {
                                                { let __v1305 = (self.cur_chr).wrapping_sub(48i32); self.hyf[crate::ix::U((k) as usize)] = __v1305; }
                                                digit_sensed = true;
                                            }
                                        }
                                    }
                                }
                                spacer | right_brace => {
                                    // §1138
                                    {
                                        if (k > 0i32) {
                                            // §1140
                                            {
                                                'l_done1_f: {
                                                    // §1142
                                                    if (self.hc[crate::ix::U((1i32) as usize)] == 0i32) {
                                                        self.hyf[crate::ix::U((0i32) as usize)] = 0i32;
                                                    }
                                                    if (self.hc[crate::ix::U((k) as usize)] == 0i32) {
                                                        self.hyf[crate::ix::U((k) as usize)] = 0i32;
                                                    }
                                                    l = k;
                                                    v = min_quarterword;
                                                    while true {
                                                        {
                                                            if (self.hyf[crate::ix::U((l) as usize)] != 0i32) {
                                                                v = self.new_trie_op((k).wrapping_sub(l), self.hyf[crate::ix::U((l) as usize)], v);
                                                            }
                                                            if (l > 0i32) {
                                                                l = (l).wrapping_sub(1i32);
                                                            } else {
                                                                break 'l_done1_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §1140
                                                q = 0i32;
                                                { let __v1306 = self.cur_lang; self.hc[crate::ix::U((0i32) as usize)] = __v1306; }
                                                while (l <= k) {
                                                    {
                                                        c = self.hc[crate::ix::U((l) as usize)];
                                                        l = (l).wrapping_add(1i32);
                                                        p = self.trie_l[crate::ix::U((q) as usize)];
                                                        first_child = true;
                                                        while ((p > 0i32) && (c > self.trie_c[crate::ix::U((p) as usize)])) {
                                                            {
                                                                q = p;
                                                                p = self.trie_r[crate::ix::U((q) as usize)];
                                                                first_child = false;
                                                            }
                                                        }
                                                        if ((p == 0i32) || (c < self.trie_c[crate::ix::U((p) as usize)])) {
                                                            // §1141
                                                            {
                                                                if (self.trie_ptr == trie_size) {
                                                                    self.overflow(1365i32, trie_size);
                                                                }
                                                                self.trie_ptr = (self.trie_ptr).wrapping_add(1i32);
                                                                self.trie_r[crate::ix::U((self.trie_ptr) as usize)] = p;
                                                                p = self.trie_ptr;
                                                                self.trie_l[crate::ix::U((p) as usize)] = 0i32;
                                                                if first_child {
                                                                    self.trie_l[crate::ix::U((q) as usize)] = p;
                                                                } else {
                                                                    self.trie_r[crate::ix::U((q) as usize)] = p;
                                                                }
                                                                self.trie_c[crate::ix::U((p) as usize)] = c;
                                                                self.trie_o[crate::ix::U((p) as usize)] = min_quarterword;
                                                            }
                                                        }
                                                        // §1140
                                                        q = p;
                                                    }
                                                }
                                                if (self.trie_o[crate::ix::U((q) as usize)] != min_quarterword) {
                                                    {
                                                        {
                                                            self.dg_mark();
                                                            if (self.interaction == error_stop_mode) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(264i32);
                                                            }
                                                            self.print(1372i32);
                                                        }
                                                        {
                                                            self.help_ptr = 1i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 1370i32;
                                                        }
                                                        self.error();
                                                    }
                                                }
                                                self.trie_o[crate::ix::U((q) as usize)] = v;
                                            }
                                        }
                                        // §1138
                                        if (self.cur_cmd == right_brace) {
                                            break 'l_done_f;
                                        }
                                        k = 0i32;
                                        self.hyf[crate::ix::U((0i32) as usize)] = 0i32;
                                        digit_sensed = false;
                                    }
                                }
                                _ => {
                                    {
                                        {
                                            self.dg_mark();
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(264i32);
                                            }
                                            self.print(1369i32);
                                        }
                                        self.print_esc(1367i32);
                                        {
                                            self.help_ptr = 1i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 1370i32;
                                        }
                                        self.error();
                                    }
                                }
                            }
                        }
                    }
                }
                // §1137
                if (self.eqtb[crate::ix::U(((29387i32) - 1) as usize)].int() > 0i32) {
                    // §1855
                    {
                        c = self.cur_lang;
                        first_child = false;
                        p = 0i32;
                        loop {
                            q = p;
                            p = self.trie_r[crate::ix::U((q) as usize)];
                            if ((p == 0i32) || (c <= self.trie_c[crate::ix::U((p) as usize)])) { break; }
                        }
                        if ((p == 0i32) || (c < self.trie_c[crate::ix::U((p) as usize)])) {
                            // §1141
                            {
                                if (self.trie_ptr == trie_size) {
                                    self.overflow(1365i32, trie_size);
                                }
                                self.trie_ptr = (self.trie_ptr).wrapping_add(1i32);
                                self.trie_r[crate::ix::U((self.trie_ptr) as usize)] = p;
                                p = self.trie_ptr;
                                self.trie_l[crate::ix::U((p) as usize)] = 0i32;
                                if first_child {
                                    self.trie_l[crate::ix::U((q) as usize)] = p;
                                } else {
                                    self.trie_r[crate::ix::U((q) as usize)] = p;
                                }
                                self.trie_c[crate::ix::U((p) as usize)] = c;
                                self.trie_o[crate::ix::U((p) as usize)] = min_quarterword;
                            }
                        }
                        // §1855
                        q = p;
                        // §1856
                        p = self.trie_l[crate::ix::U((q) as usize)];
                        first_child = true;
                        {
                            let __for_end_6 = 255i32;
                            c = 0i32;
                            while c <= __for_end_6 {
                                if ((self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh() > 0i32) || ((c == 255i32) && first_child)) {
                                    {
                                        if (p == 0i32) {
                                            // §1141
                                            {
                                                if (self.trie_ptr == trie_size) {
                                                    self.overflow(1365i32, trie_size);
                                                }
                                                self.trie_ptr = (self.trie_ptr).wrapping_add(1i32);
                                                self.trie_r[crate::ix::U((self.trie_ptr) as usize)] = p;
                                                p = self.trie_ptr;
                                                self.trie_l[crate::ix::U((p) as usize)] = 0i32;
                                                if first_child {
                                                    self.trie_l[crate::ix::U((q) as usize)] = p;
                                                } else {
                                                    self.trie_r[crate::ix::U((q) as usize)] = p;
                                                }
                                                self.trie_c[crate::ix::U((p) as usize)] = c;
                                                self.trie_o[crate::ix::U((p) as usize)] = min_quarterword;
                                            }
                                        } else {
                                            // §1856
                                            self.trie_c[crate::ix::U((p) as usize)] = c;
                                        }
                                        { let __v1307 = (self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh()).wrapping_add(0i32); self.trie_o[crate::ix::U((p) as usize)] = __v1307; }
                                        q = p;
                                        p = self.trie_r[crate::ix::U((q) as usize)];
                                        first_child = false;
                                    }
                                }
                                c = c.wrapping_add(1);
                            }
                        }
                        if first_child {
                            self.trie_l[crate::ix::U((q) as usize)] = 0i32;
                        } else {
                            self.trie_r[crate::ix::U((q) as usize)] = 0i32;
                        }
                    }
                }
            }
        } else {
            // §1137
            {
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1366i32);
                }
                self.print_esc(1367i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1368i32;
                }
                self.error();
                { let __v1308 = self.scan_toks(false, false); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v1308); }
                self.flush_list(self.def_ref);
            }
        }
    }

    /// Finally we put everything together: Here is how the trie gets to its
    /// final, efficient form.
    /// The following packing routine is rigged so that the root of the linked
    /// tree gets mapped into location 1 of `trie`, as required by the hyphenation
    /// algorithm. This happens because the first call of `first_fit` will
    /// ``take'' location~1.
    /// @<Declare procedures for preprocessing hyphenation patterns
    // §1143
    pub fn init_trie(&mut self) {
        let mut p: trie_pointer = 0; // §1143
        let mut j: i32 = 0; // §1143
        let mut k: i32 = 0; // §1143
        let mut t: i32 = 0; // §1143
        let mut r: trie_pointer = 0; // §1143
        let mut s: trie_pointer = 0; // §1143
        let mut h: two_halves = two_halves::default(); // §1143
        // §1122
        self.op_start[crate::ix::U((0i32) as usize)] = (0i32).wrapping_neg();
        {
            let __for_end_2 = 255i32;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v1309 = ((self.op_start[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]).wrapping_add(self.trie_used[crate::ix::U(((j).wrapping_sub(1i32)) as usize)])).wrapping_sub(0i32); self.op_start[crate::ix::U((j) as usize)] = __v1309; }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v1310 = (self.op_start[crate::ix::U((self.trie_op_lang[crate::ix::U(((j) - 1) as usize)]) as usize)]).wrapping_add(self.trie_op_val[crate::ix::U(((j) - 1) as usize)]); self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)] = __v1310; }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            j = 1i32;
            while j <= __for_end_2 {
                while (self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)] > j) {
                    {
                        k = self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)];
                        t = self.hyf_distance[crate::ix::U(((k) - 1) as usize)];
                        { let __v1311 = self.hyf_distance[crate::ix::U(((j) - 1) as usize)]; self.hyf_distance[crate::ix::U(((k) - 1) as usize)] = __v1311; }
                        self.hyf_distance[crate::ix::U(((j) - 1) as usize)] = t;
                        t = self.hyf_num[crate::ix::U(((k) - 1) as usize)];
                        { let __v1312 = self.hyf_num[crate::ix::U(((j) - 1) as usize)]; self.hyf_num[crate::ix::U(((k) - 1) as usize)] = __v1312; }
                        self.hyf_num[crate::ix::U(((j) - 1) as usize)] = t;
                        t = self.hyf_next[crate::ix::U(((k) - 1) as usize)];
                        { let __v1313 = self.hyf_next[crate::ix::U(((j) - 1) as usize)]; self.hyf_next[crate::ix::U(((k) - 1) as usize)] = __v1313; }
                        self.hyf_next[crate::ix::U(((j) - 1) as usize)] = t;
                        { let __v1314 = self.trie_op_hash[crate::ix::U(((k) + 35111) as usize)]; self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)] = __v1314; }
                        self.trie_op_hash[crate::ix::U(((k) + 35111) as usize)] = k;
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        // §1129
        {
            let __for_end_2 = trie_size;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[crate::ix::U((p) as usize)] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        { let __v1315 = self.compress_trie(self.trie_r[crate::ix::U((0i32) as usize)]); self.trie_r[crate::ix::U((0i32) as usize)] = __v1315; }
        { let __v1316 = self.compress_trie(self.trie_l[crate::ix::U((0i32) as usize)]); self.trie_l[crate::ix::U((0i32) as usize)] = __v1316; }
        {
            let __for_end_2 = self.trie_ptr;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[crate::ix::U((p) as usize)] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_min[crate::ix::U((p) as usize)] = (p).wrapping_add(1i32);
                p = p.wrapping_add(1);
            }
        }
        self.trie[crate::ix::U((0i32) as usize)].set_rh(1i32);
        self.trie_max = 0i32;
        // §1143
        if (self.trie_l[crate::ix::U((0i32) as usize)] != 0i32) {
            {
                self.first_fit(self.trie_l[crate::ix::U((0i32) as usize)]);
                self.trie_pack(self.trie_l[crate::ix::U((0i32) as usize)]);
            }
        }
        if (self.trie_r[crate::ix::U((0i32) as usize)] != 0i32) {
            // §1857
            {
                if (self.trie_l[crate::ix::U((0i32) as usize)] == 0i32) {
                    {
                        let __for_end_5 = 255i32;
                        p = 0i32;
                        while p <= __for_end_5 {
                            self.trie_min[crate::ix::U((p) as usize)] = (p).wrapping_add(2i32);
                            p = p.wrapping_add(1);
                        }
                    }
                }
                self.first_fit(self.trie_r[crate::ix::U((0i32) as usize)]);
                self.trie_pack(self.trie_r[crate::ix::U((0i32) as usize)]);
                self.hyph_start = self.trie_hash[crate::ix::U((self.trie_r[crate::ix::U((0i32) as usize)]) as usize)];
            }
        }
        // §1135
        h.set_rh(0i32);
        h.set_b0(min_quarterword);
        h.set_b1(min_quarterword);
        if (self.trie_max == 0i32) {
            {
                {
                    let __for_end_4 = 256i32;
                    r = 0i32;
                    while r <= __for_end_4 {
                        self.trie[crate::ix::U((r) as usize)] = h;
                        r = r.wrapping_add(1);
                    }
                }
                self.trie_max = 256i32;
            }
        } else {
            {
                if (self.trie_r[crate::ix::U((0i32) as usize)] > 0i32) {
                    self.trie_fix(self.trie_r[crate::ix::U((0i32) as usize)]);
                }
                if (self.trie_l[crate::ix::U((0i32) as usize)] > 0i32) {
                    self.trie_fix(self.trie_l[crate::ix::U((0i32) as usize)]);
                }
                r = 0i32;
                loop {
                    s = self.trie[crate::ix::U((r) as usize)].rh();
                    self.trie[crate::ix::U((r) as usize)] = h;
                    r = s;
                    if (r > self.trie_max) { break; }
                }
            }
        }
        self.trie[crate::ix::U((0i32) as usize)].set_b1(63i32);
        // §1143
        self.trie_not_ready = false;
    }

    /// Since `line_break` is a rather lengthy procedure---sort of a small world unto
    /// itself---we must build it up little by little, somewhat more cautiously
    /// than we have done with the simpler procedures of \TeX. Here is the
    /// general outline.
    // §991
    pub fn line_break(&mut self, mut d: bool) {
        let mut q: halfword = 0; // §1038
        let mut r: halfword = 0; // §1038
        let mut s: halfword = 0; // §1038
        let mut prev_s: halfword = 0; // §1038
        let mut f: internal_font_number = 0; // §1038
        let mut j: small_number = 0; // §1070
        let mut c: i32 = 0; // §1070
        'l_done_f: {
            self.pack_begin_line = self.cur_list.ml_field;
            // §992
            { let __v1317 = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(); self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(__v1317); }
            if (self.cur_list.tail_field >= self.hi_mem_min) {
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1318 = self.cur_list.tail_field; let __v1319 = self.new_penalty(inf_penalty); self.mem[crate::ix::U((__ix1318) as usize)].set_hh_rh(__v1319); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            } else {
                if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() != glue_node) {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1320 = self.cur_list.tail_field; let __v1321 = self.new_penalty(inf_penalty); self.mem[crate::ix::U((__ix1320) as usize)].set_hh_rh(__v1321); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                } else {
                    {
                        { let __ix1322 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1322) as usize)].set_hh_b0(penalty_node); }
                        self.delete_glue_ref(self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().lh());
                        self.flush_node_list(self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().rh());
                        { let __ix1323 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1323) as usize)].set_int(inf_penalty); }
                    }
                }
            }
            { let __ix1324 = self.cur_list.tail_field; let __v1325 = self.new_param_glue(par_fill_skip_code); self.mem[crate::ix::U((__ix1324) as usize)].set_hh_rh(__v1325); }
            self.last_line_fill = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
            self.init_cur_lang = (self.cur_list.pg_field % 65536i32);
            self.init_l_hyf = (self.cur_list.pg_field / 4194304i32);
            self.init_r_hyf = ((self.cur_list.pg_field / 65536i32) % 64i32);
            self.pop_nest();
            // §1003
            self.no_shrink_error_yet = true;
            if ((self.mem[crate::ix::U((self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                {
                    { let __v1326 = self.finite_shrink(self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh()); self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].set_hh_rh(__v1326); }
                }
            }
            if ((self.mem[crate::ix::U((self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                {
                    { let __v1327 = self.finite_shrink(self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh()); self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].set_hh_rh(__v1327); }
                }
            }
            q = self.eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh();
            r = self.eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh();
            { let __v1328 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.background[crate::ix::U(((1i32) - 1) as usize)] = __v1328; }
            self.background[crate::ix::U(((2i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((3i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((4i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((5i32) - 1) as usize)] = 0i32;
            { let __ix1329 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1330 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.background[crate::ix::U(((__ix1329) - 1) as usize)] = __v1330; }
            { let __ix1331 = (2i32).wrapping_add(self.mem[crate::ix::U((r) as usize)].hh().b0()); let __v1332 = (self.background[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((r) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.background[crate::ix::U(((__ix1331) - 1) as usize)] = __v1332; }
            { let __v1333 = (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.background[crate::ix::U(((6i32) - 1) as usize)] = __v1333; }
            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                {
                    self.background[crate::ix::U(((7i32) - 1) as usize)] = 0i32;
                    self.background[crate::ix::U(((8i32) - 1) as usize)] = 0i32;
                    self.max_stretch_ratio = (1i32).wrapping_neg();
                    self.max_shrink_ratio = (1i32).wrapping_neg();
                    self.cur_font_step = (1i32).wrapping_neg();
                    self.prev_char_p = null;
                }
            }
            // §1843
            self.do_last_line_fit = false;
            self.active_node_size = active_node_size_normal;
            if (self.eqtb[crate::ix::U(((29385i32) - 1) as usize)].int() > 0i32) {
                {
                    q = self.mem[crate::ix::U(((self.last_line_fill).wrapping_add(1i32)) as usize)].hh().lh();
                    if ((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() > 0i32) && (self.mem[crate::ix::U((q) as usize)].hh().b0() > normal)) {
                        if (((self.background[crate::ix::U(((3i32) - 1) as usize)] == 0i32) && (self.background[crate::ix::U(((4i32) - 1) as usize)] == 0i32)) && (self.background[crate::ix::U(((5i32) - 1) as usize)] == 0i32)) {
                            {
                                self.do_last_line_fit = true;
                                self.active_node_size = active_node_size_extended;
                                self.fill_width[crate::ix::U((0i32) as usize)] = 0i32;
                                self.fill_width[crate::ix::U((1i32) as usize)] = 0i32;
                                self.fill_width[crate::ix::U((2i32) as usize)] = 0i32;
                                { let __ix1334 = (self.mem[crate::ix::U((q) as usize)].hh().b0()).wrapping_sub(1i32); let __v1335 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.fill_width[crate::ix::U((__ix1334) as usize)] = __v1335; }
                            }
                        }
                    }
                }
            }
            // §1010
            self.minimum_demerits = awful_bad;
            self.minimal_demerits[crate::ix::U((tight_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((decent_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((loose_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((very_loose_fit) as usize)] = awful_bad;
            // §1024
            if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) {
                if (self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() == 0i32) {
                    {
                        self.last_special_line = 0i32;
                        self.second_width = self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int();
                        self.second_indent = 0i32;
                    }
                } else {
                    // §1025
                    {
                        self.last_special_line = (self.eqtb[crate::ix::U(((29318i32) - 1) as usize)].int()).wrapping_abs();
                        if (self.eqtb[crate::ix::U(((29318i32) - 1) as usize)].int() < 0i32) {
                            {
                                self.first_width = (self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int()).wrapping_sub((self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int()).wrapping_abs());
                                if (self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() >= 0i32) {
                                    self.first_indent = self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int();
                                } else {
                                    self.first_indent = 0i32;
                                }
                                self.second_width = self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int();
                                self.second_indent = 0i32;
                            }
                        } else {
                            {
                                self.first_width = self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int();
                                self.first_indent = 0i32;
                                self.second_width = (self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int()).wrapping_sub((self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int()).wrapping_abs());
                                if (self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() >= 0i32) {
                                    self.second_indent = self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int();
                                } else {
                                    self.second_indent = 0i32;
                                }
                            }
                        }
                    }
                }
            } else {
                // §1024
                {
                    self.last_special_line = (self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_sub(1i32);
                    self.second_width = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul((self.last_special_line).wrapping_add(1i32)))) as usize)].int();
                    self.second_indent = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(self.last_special_line))).wrapping_add(1i32)) as usize)].int();
                }
            }
            if (self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int() == 0i32) {
                self.easy_line = self.last_special_line;
            } else {
                self.easy_line = max_halfword;
            }
            // §1039
            self.threshold = self.eqtb[crate::ix::U(((29277i32) - 1) as usize)].int();
            if (self.threshold >= 0i32) {
                {
                    if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(1347i32);
                        }
                    }
                    self.second_pass = false;
                    self.final_pass = false;
                }
            } else {
                {
                    self.threshold = self.eqtb[crate::ix::U(((29278i32) - 1) as usize)].int();
                    self.second_pass = true;
                    self.final_pass = (self.eqtb[crate::ix::U(((29923i32) - 1) as usize)].int() <= 0i32);
                    if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                        self.begin_diagnostic();
                    }
                }
            }
            while true {
                {
                    if (self.threshold > inf_bad) {
                        self.threshold = inf_bad;
                    }
                    if self.second_pass {
                        // §1068
                        {
                            if self.trie_not_ready {
                                self.init_trie();
                            }
                            self.cur_lang = self.init_cur_lang;
                            self.l_hyf = self.init_l_hyf;
                            self.r_hyf = self.init_r_hyf;
                            if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != (self.cur_lang).wrapping_add(0i32)) {
                                self.hyph_index = 0i32;
                            } else {
                                self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                            }
                        }
                    }
                    // §1040
                    q = self.get_node(self.active_node_size);
                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(unhyphenated);
                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(decent_fit);
                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(last_active);
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                    { let __v1336 = (self.cur_list.pg_field).wrapping_add(1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1336); }
                    self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
                    self.mem[crate::ix::U((active) as usize)].set_hh_rh(q);
                    if self.do_last_line_fit {
                        // §1845
                        {
                            self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
                            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(0i32);
                        }
                    }
                    // §1040
                    { let __v1337 = self.background[crate::ix::U(((1i32) - 1) as usize)]; self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1337; }
                    { let __v1338 = self.background[crate::ix::U(((2i32) - 1) as usize)]; self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1338; }
                    { let __v1339 = self.background[crate::ix::U(((3i32) - 1) as usize)]; self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1339; }
                    { let __v1340 = self.background[crate::ix::U(((4i32) - 1) as usize)]; self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1340; }
                    { let __v1341 = self.background[crate::ix::U(((5i32) - 1) as usize)]; self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1341; }
                    { let __v1342 = self.background[crate::ix::U(((6i32) - 1) as usize)]; self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1342; }
                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                        {
                            { let __v1343 = self.background[crate::ix::U(((7i32) - 1) as usize)]; self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1343; }
                            { let __v1344 = self.background[crate::ix::U(((8i32) - 1) as usize)]; self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1344; }
                        }
                    }
                    self.passive = null;
                    self.printed_node = temp_head;
                    self.pass_number = 0i32;
                    self.font_in_short_display = null_font;
                    // §1039
                    self.cur_p = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                    self.auto_breaking = true;
                    self.prev_p = self.cur_p;
                    self.prev_char_p = null;
                    self.prev_legal = null;
                    self.rejected_cur_p = null;
                    self.try_prev_break = false;
                    self.before_rejected_cur_p = false;
                    self.first_p = self.cur_p;
                    while ((self.cur_p != null) && (self.mem[crate::ix::U((active) as usize)].hh().rh() != last_active)) {
                        // §1042
                        {
                            'l_done5_f: {
                                if (self.cur_p >= self.hi_mem_min) {
                                    // §1043
                                    {
                                        self.prev_p = self.cur_p;
                                        loop {
                                            f = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0();
                                            { let __v1345 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1345; }
                                            if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                {
                                                    self.prev_char_p = self.cur_p;
                                                    { let __v1346 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1())); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1346; }
                                                    { let __v1347 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1())); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1347; }
                                                }
                                            }
                                            self.cur_p = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                            if (!(self.cur_p >= self.hi_mem_min)) { break; }
                                        }
                                    }
                                }
                                // §1042
                                match self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() {
                                    hlist_node | vlist_node | rule_node => {
                                        { let __v1348 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1348; }
                                    }
                                    whatsit_node => {
                                        // §1609
                                        {
                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == language_node) {
                                                {
                                                    self.cur_lang = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                                                    self.l_hyf = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b0();
                                                    self.r_hyf = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1();
                                                }
                                            }
                                            if ((self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == pdf_refxform_node) || (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == pdf_refximage_node)) {
                                                { let __v1349 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1349; }
                                            }
                                        }
                                    }
                                    glue_node => {
                                        // §1042
                                        {
                                            // §1044
                                            if self.auto_breaking {
                                                {
                                                    if (self.prev_p >= self.hi_mem_min) {
                                                        self.try_break(0i32, unhyphenated);
                                                    } else {
                                                        if (self.mem[crate::ix::U((self.prev_p) as usize)].hh().b0() < math_node) {
                                                            self.try_break(0i32, unhyphenated);
                                                        } else {
                                                            if ((self.mem[crate::ix::U((self.prev_p) as usize)].hh().b0() == kern_node) && (self.mem[crate::ix::U((self.prev_p) as usize)].hh().b1() != explicit)) {
                                                                self.try_break(0i32, unhyphenated);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if ((self.mem[crate::ix::U((self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                {
                                                    { let __ix1350 = (self.cur_p).wrapping_add(1i32); let __v1351 = self.finite_shrink(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix1350) as usize)].set_hh_lh(__v1351); }
                                                }
                                            }
                                            q = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
                                            { let __v1352 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1352; }
                                            { let __ix1353 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1354 = (self.active_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((__ix1353) - 1) as usize)] = __v1354; }
                                            { let __v1355 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1355; }
                                            // §1042
                                            if (self.second_pass && self.auto_breaking) {
                                                // §1071
                                                {
                                                    'l_done1_f: {
                                                        prev_s = self.cur_p;
                                                        s = self.mem[crate::ix::U((prev_s) as usize)].hh().rh();
                                                        if (s != null) {
                                                            {
                                                                'l_done4_f: {
                                                                    'l_done3_f: {
                                                                        'l_done2_f: {
                                                                            // §1073
                                                                            while true {
                                                                                {
                                                                                    'l_continue_f: {
                                                                                        if (s >= self.hi_mem_min) {
                                                                                            {
                                                                                                c = (self.mem[crate::ix::U((s) as usize)].hh().b1()).wrapping_sub(0i32);
                                                                                                self.hf = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                                            }
                                                                                        } else {
                                                                                            if (self.mem[crate::ix::U((s) as usize)].hh().b0() == ligature_node) {
                                                                                                if (self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh() == null) {
                                                                                                    break 'l_continue_f;
                                                                                                } else {
                                                                                                    {
                                                                                                        q = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                                        c = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(0i32);
                                                                                                        self.hf = self.mem[crate::ix::U((q) as usize)].hh().b0();
                                                                                                    }
                                                                                                }
                                                                                            } else {
                                                                                                if ((self.mem[crate::ix::U((s) as usize)].hh().b0() == kern_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() == normal)) {
                                                                                                    break 'l_continue_f;
                                                                                                } else {
                                                                                                    if ((self.mem[crate::ix::U((s) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() >= L_code)) {
                                                                                                        break 'l_continue_f;
                                                                                                    } else {
                                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b0() == whatsit_node) {
                                                                                                            {
                                                                                                                // §1610
                                                                                                                if (self.mem[crate::ix::U((s) as usize)].hh().b1() == language_node) {
                                                                                                                    {
                                                                                                                        self.cur_lang = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                                                        self.l_hyf = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                                                        self.r_hyf = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1();
                                                                                                                        if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != (self.cur_lang).wrapping_add(0i32)) {
                                                                                                                            self.hyph_index = 0i32;
                                                                                                                        } else {
                                                                                                                            self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                                // §1073
                                                                                                                break 'l_continue_f;
                                                                                                            }
                                                                                                        } else {
                                                                                                            break 'l_done1_f;
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        if (self.hyph_index == 0i32) {
                                                                                            { let __v1356 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1356; }
                                                                                        } else {
                                                                                            if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != (c).wrapping_add(0i32)) {
                                                                                                self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                            } else {
                                                                                                { let __v1357 = (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0()).wrapping_sub(0i32); self.hc[crate::ix::U((0i32) as usize)] = __v1357; }
                                                                                            }
                                                                                        }
                                                                                        if (self.hc[crate::ix::U((0i32) as usize)] != 0i32) {
                                                                                            if ((self.hc[crate::ix::U((0i32) as usize)] == c) || (self.eqtb[crate::ix::U(((29315i32) - 1) as usize)].int() > 0i32)) {
                                                                                                break 'l_done2_f;
                                                                                            } else {
                                                                                                break 'l_done1_f;
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    prev_s = s;
                                                                                    s = self.mem[crate::ix::U((prev_s) as usize)].hh().rh();
                                                                                }
                                                                            }
                                                                        }
                                                                        self.hyf_char = self.hyphen_char[crate::ix::U((self.hf) as usize)];
                                                                        if (self.hyf_char < 0i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        if (self.hyf_char > 255i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        self.ha = prev_s;
                                                                        // §1071
                                                                        if ((self.l_hyf).wrapping_add(self.r_hyf) > 63i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        // §1074
                                                                        self.hn = 0i32;
                                                                        while true {
                                                                            {
                                                                                if (s >= self.hi_mem_min) {
                                                                                    {
                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b0() != self.hf) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        self.hyf_bchar = self.mem[crate::ix::U((s) as usize)].hh().b1();
                                                                                        c = (self.hyf_bchar).wrapping_sub(0i32);
                                                                                        if (self.hyph_index == 0i32) {
                                                                                            { let __v1358 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1358; }
                                                                                        } else {
                                                                                            if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != (c).wrapping_add(0i32)) {
                                                                                                self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                            } else {
                                                                                                { let __v1359 = (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0()).wrapping_sub(0i32); self.hc[crate::ix::U((0i32) as usize)] = __v1359; }
                                                                                            }
                                                                                        }
                                                                                        if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        if (self.hn == 63i32) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        self.hb = s;
                                                                                        self.hn = (self.hn).wrapping_add(1i32);
                                                                                        self.hu[crate::ix::U((self.hn) as usize)] = c;
                                                                                        { let __ix1360 = self.hn; let __v1361 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((__ix1360) as usize)] = __v1361; }
                                                                                        self.hyf_bchar = non_char;
                                                                                    }
                                                                                } else {
                                                                                    if (self.mem[crate::ix::U((s) as usize)].hh().b0() == ligature_node) {
                                                                                        // §1075
                                                                                        {
                                                                                            if (self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0() != self.hf) {
                                                                                                break 'l_done3_f;
                                                                                            }
                                                                                            j = self.hn;
                                                                                            q = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                            if (q > null) {
                                                                                                self.hyf_bchar = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                                                            }
                                                                                            while (q > null) {
                                                                                                {
                                                                                                    c = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(0i32);
                                                                                                    if (self.hyph_index == 0i32) {
                                                                                                        { let __v1362 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1362; }
                                                                                                    } else {
                                                                                                        if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != (c).wrapping_add(0i32)) {
                                                                                                            self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                                        } else {
                                                                                                            { let __v1363 = (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0()).wrapping_sub(0i32); self.hc[crate::ix::U((0i32) as usize)] = __v1363; }
                                                                                                        }
                                                                                                    }
                                                                                                    if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                                                                                        break 'l_done3_f;
                                                                                                    }
                                                                                                    if (j == 63i32) {
                                                                                                        break 'l_done3_f;
                                                                                                    }
                                                                                                    j = (j).wrapping_add(1i32);
                                                                                                    self.hu[crate::ix::U((j) as usize)] = c;
                                                                                                    { let __v1364 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((j) as usize)] = __v1364; }
                                                                                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                                                }
                                                                                            }
                                                                                            self.hb = s;
                                                                                            self.hn = j;
                                                                                            if (((self.mem[crate::ix::U((s) as usize)].hh().b1()) % 2) != 0) {
                                                                                                self.hyf_bchar = self.font_bchar[crate::ix::U((self.hf) as usize)];
                                                                                            } else {
                                                                                                self.hyf_bchar = non_char;
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        // §1074
                                                                                        if ((self.mem[crate::ix::U((s) as usize)].hh().b0() == kern_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() == normal)) {
                                                                                            {
                                                                                                self.hb = s;
                                                                                                self.hyf_bchar = self.font_bchar[crate::ix::U((self.hf) as usize)];
                                                                                            }
                                                                                        } else {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                    }
                                                                                }
                                                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                            }
                                                                        }
                                                                    }
                                                                    // §1076
                                                                    if (self.hn < (self.l_hyf).wrapping_add(self.r_hyf)) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    while true {
                                                                        {
                                                                            if (!(s >= self.hi_mem_min)) {
                                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                                    ligature_node => {
                                                                                    }
                                                                                    kern_node => {
                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b1() != normal) {
                                                                                            break 'l_done4_f;
                                                                                        }
                                                                                    }
                                                                                    whatsit_node | glue_node | penalty_node | ins_node | adjust_node | mark_node => {
                                                                                        break 'l_done4_f;
                                                                                    }
                                                                                    math_node => {
                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b1() >= L_code) {
                                                                                            break 'l_done4_f;
                                                                                        } else {
                                                                                            break 'l_done1_f;
                                                                                        }
                                                                                    }
                                                                                    _ => {
                                                                                        break 'l_done1_f;
                                                                                    }
                                                                                }
                                                                            }
                                                                            s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                        }
                                                                    }
                                                                }
                                                                // §1071
                                                                self.dl_hyph_begin(self.ha);
                                                                self.hyphenate();
                                                                self.dl_hyph_end();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    kern_node => {
                                        // §1042
                                        if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == explicit) {
                                            {
                                                if ((!(self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh() >= self.hi_mem_min)) && self.auto_breaking) {
                                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh()) as usize)].hh().b0() == glue_node) {
                                                        self.try_break(0i32, unhyphenated);
                                                    }
                                                }
                                                { let __v1365 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1365; }
                                            }
                                        } else {
                                            {
                                                { let __v1366 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1366; }
                                                if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == normal)) {
                                                    {
                                                        { let __v1367 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.kern_stretch(self.cur_p)); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1367; }
                                                        { let __v1368 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.kern_shrink(self.cur_p)); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1368; }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ligature_node => {
                                        {
                                            f = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b0();
                                            { let __v1369 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1369; }
                                            if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                {
                                                    self.prev_char_p = self.cur_p;
                                                    { let __v1370 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1())); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1370; }
                                                    { let __v1371 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1())); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1371; }
                                                }
                                            }
                                        }
                                    }
                                    disc_node => {
                                        // §1045
                                        {
                                            s = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
                                            self.disc_width[crate::ix::U(((1i32) - 1) as usize)] = 0i32;
                                            if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                {
                                                    self.disc_width[crate::ix::U(((7i32) - 1) as usize)] = 0i32;
                                                    self.disc_width[crate::ix::U(((8i32) - 1) as usize)] = 0i32;
                                                }
                                            }
                                            if (s == null) {
                                                self.try_break(self.eqtb[crate::ix::U(((29281i32) - 1) as usize)].int(), hyphenated);
                                            } else {
                                                {
                                                    loop {
                                                        // §1046
                                                        if (s >= self.hi_mem_min) {
                                                            {
                                                                f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                { let __v1372 = (self.disc_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((s) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.disc_width[crate::ix::U(((1i32) - 1) as usize)] = __v1372; }
                                                                if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                    {
                                                                        self.prev_char_p = s;
                                                                        { let __v1373 = (self.disc_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.disc_width[crate::ix::U(((7i32) - 1) as usize)] = __v1373; }
                                                                        { let __v1374 = (self.disc_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.disc_width[crate::ix::U(((8i32) - 1) as usize)] = __v1374; }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                ligature_node => {
                                                                    {
                                                                        f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                        { let __v1375 = (self.disc_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.disc_width[crate::ix::U(((1i32) - 1) as usize)] = __v1375; }
                                                                        if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                            {
                                                                                self.prev_char_p = s;
                                                                                { let __v1376 = (self.disc_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.disc_width[crate::ix::U(((7i32) - 1) as usize)] = __v1376; }
                                                                                { let __v1377 = (self.disc_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.disc_width[crate::ix::U(((8i32) - 1) as usize)] = __v1377; }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                hlist_node | vlist_node | rule_node | kern_node => {
                                                                    {
                                                                        { let __v1378 = (self.disc_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.disc_width[crate::ix::U(((1i32) - 1) as usize)] = __v1378; }
                                                                        if (((self.mem[crate::ix::U((s) as usize)].hh().b0() == kern_node) && (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32)) && (self.mem[crate::ix::U((s) as usize)].hh().b1() == normal)) {
                                                                            {
                                                                                { let __v1379 = (self.disc_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.kern_stretch(s)); self.disc_width[crate::ix::U(((7i32) - 1) as usize)] = __v1379; }
                                                                                { let __v1380 = (self.disc_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.kern_shrink(s)); self.disc_width[crate::ix::U(((8i32) - 1) as usize)] = __v1380; }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                _ => {
                                                                    self.confusion(1351i32);
                                                                }
                                                            }
                                                        }
                                                        // §1045
                                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                        if (s == null) { break; }
                                                    }
                                                    { let __v1381 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((1i32) - 1) as usize)]); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1381; }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1382 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((7i32) - 1) as usize)]); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1382; }
                                                            { let __v1383 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.disc_width[crate::ix::U(((8i32) - 1) as usize)]); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1383; }
                                                        }
                                                    }
                                                    self.try_break(self.eqtb[crate::ix::U(((29280i32) - 1) as usize)].int(), hyphenated);
                                                    { let __v1384 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.disc_width[crate::ix::U(((1i32) - 1) as usize)]); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1384; }
                                                    if (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) {
                                                        {
                                                            { let __v1385 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_sub(self.disc_width[crate::ix::U(((7i32) - 1) as usize)]); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1385; }
                                                            { let __v1386 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_sub(self.disc_width[crate::ix::U(((8i32) - 1) as usize)]); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1386; }
                                                        }
                                                    }
                                                }
                                            }
                                            r = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1();
                                            s = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                            while (r > 0i32) {
                                                {
                                                    // §1047
                                                    if (s >= self.hi_mem_min) {
                                                        {
                                                            f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                            { let __v1387 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((s) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1387; }
                                                            if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                {
                                                                    self.prev_char_p = s;
                                                                    { let __v1388 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1388; }
                                                                    { let __v1389 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U((s) as usize)].hh().b1())); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1389; }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                            ligature_node => {
                                                                {
                                                                    f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                    { let __v1390 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1390; }
                                                                    if ((self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32) && self.check_expand_pars(f)) {
                                                                        {
                                                                            self.prev_char_p = s;
                                                                            { let __v1391 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1391; }
                                                                            { let __v1392 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1())); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1392; }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            hlist_node | vlist_node | rule_node | kern_node => {
                                                                {
                                                                    { let __v1393 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1393; }
                                                                    if (((self.mem[crate::ix::U((s) as usize)].hh().b0() == kern_node) && (self.eqtb[crate::ix::U(((29360i32) - 1) as usize)].int() > 1i32)) && (self.mem[crate::ix::U((s) as usize)].hh().b1() == normal)) {
                                                                        {
                                                                            { let __v1394 = (self.active_width[crate::ix::U(((7i32) - 1) as usize)]).wrapping_add(self.kern_stretch(s)); self.active_width[crate::ix::U(((7i32) - 1) as usize)] = __v1394; }
                                                                            { let __v1395 = (self.active_width[crate::ix::U(((8i32) - 1) as usize)]).wrapping_add(self.kern_shrink(s)); self.active_width[crate::ix::U(((8i32) - 1) as usize)] = __v1395; }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            _ => {
                                                                self.confusion(1352i32);
                                                            }
                                                        }
                                                    }
                                                    // §1045
                                                    r = (r).wrapping_sub(1i32);
                                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                }
                                            }
                                            self.prev_p = self.cur_p;
                                            self.cur_p = s;
                                            break 'l_done5_f;
                                        }
                                    }
                                    math_node => {
                                        // §1042
                                        {
                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() < L_code) {
                                                self.auto_breaking = (((self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1()) % 2) != 0);
                                            }
                                            {
                                                if ((!(self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh() >= self.hi_mem_min)) && self.auto_breaking) {
                                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh()) as usize)].hh().b0() == glue_node) {
                                                        self.try_break(0i32, unhyphenated);
                                                    }
                                                }
                                                { let __v1396 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1396; }
                                            }
                                        }
                                    }
                                    penalty_node => {
                                        self.try_break(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int(), unhyphenated);
                                    }
                                    mark_node | ins_node | adjust_node => {
                                    }
                                    _ => {
                                        self.confusion(1350i32);
                                    }
                                }
                                self.prev_p = self.cur_p;
                                self.cur_p = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                            }
                        }
                    }
                    // §1039
                    if (self.cur_p == null) {
                        // §1049
                        {
                            self.try_break((10000i32).wrapping_neg(), hyphenated);
                            if (self.mem[crate::ix::U((active) as usize)].hh().rh() != last_active) {
                                {
                                    // §1050
                                    r = self.mem[crate::ix::U((active) as usize)].hh().rh();
                                    self.fewest_demerits = awful_bad;
                                    loop {
                                        if (self.mem[crate::ix::U((r) as usize)].hh().b0() != delta_node) {
                                            if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int() < self.fewest_demerits) {
                                                {
                                                    self.fewest_demerits = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int();
                                                    self.best_bet = r;
                                                }
                                            }
                                        }
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                        if (r == last_active) { break; }
                                    }
                                    self.best_line = self.mem[crate::ix::U(((self.best_bet).wrapping_add(1i32)) as usize)].hh().lh();
                                    // §1049
                                    if (self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int() == 0i32) {
                                        break 'l_done_f;
                                    }
                                    // §1051
                                    {
                                        r = self.mem[crate::ix::U((active) as usize)].hh().rh();
                                        self.actual_looseness = 0i32;
                                        loop {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() != delta_node) {
                                                {
                                                    self.line_diff = (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(self.best_line);
                                                    if (((self.line_diff < self.actual_looseness) && (self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int() <= self.line_diff)) || ((self.line_diff > self.actual_looseness) && (self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int() >= self.line_diff))) {
                                                        {
                                                            self.best_bet = r;
                                                            self.actual_looseness = self.line_diff;
                                                            self.fewest_demerits = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int();
                                                        }
                                                    } else {
                                                        if ((self.line_diff == self.actual_looseness) && (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int() < self.fewest_demerits)) {
                                                            {
                                                                self.best_bet = r;
                                                                self.fewest_demerits = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                            if (r == last_active) { break; }
                                        }
                                        self.best_line = self.mem[crate::ix::U(((self.best_bet).wrapping_add(1i32)) as usize)].hh().lh();
                                    }
                                    // §1049
                                    if ((self.actual_looseness == self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int()) || self.final_pass) {
                                        break 'l_done_f;
                                    }
                                }
                            }
                        }
                    }
                    // §1041
                    q = self.mem[crate::ix::U((active) as usize)].hh().rh();
                    while (q != last_active) {
                        {
                            self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            if (self.mem[crate::ix::U((q) as usize)].hh().b0() == delta_node) {
                                self.free_node(q, delta_node_size);
                            } else {
                                self.free_node(q, self.active_node_size);
                            }
                            q = self.cur_p;
                        }
                    }
                    q = self.passive;
                    while (q != null) {
                        {
                            self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            self.free_node(q, passive_node_size);
                            q = self.cur_p;
                        }
                    }
                    // §1039
                    if (!self.second_pass) {
                        {
                            if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                                self.print_nl(1348i32);
                            }
                            self.threshold = self.eqtb[crate::ix::U(((29278i32) - 1) as usize)].int();
                            self.second_pass = true;
                            self.final_pass = (self.eqtb[crate::ix::U(((29923i32) - 1) as usize)].int() <= 0i32);
                        }
                    } else {
                        {
                            if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
                                self.print_nl(1349i32);
                            }
                            { let __v1397 = (self.background[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.eqtb[crate::ix::U(((29923i32) - 1) as usize)].int()); self.background[crate::ix::U(((2i32) - 1) as usize)] = __v1397; }
                            self.final_pass = true;
                        }
                    }
                }
            }
        }
        if (self.eqtb[crate::ix::U(((29309i32) - 1) as usize)].int() > 0i32) {
            {
                self.end_diagnostic(true);
                self.normalize_selector();
            }
        }
        if self.do_last_line_fit {
            // §1853
            if (self.mem[crate::ix::U(((self.best_bet).wrapping_add(3i32)) as usize)].int() == 0i32) {
                self.do_last_line_fit = false;
            } else {
                {
                    q = self.new_spec(self.mem[crate::ix::U(((self.last_line_fill).wrapping_add(1i32)) as usize)].hh().lh());
                    self.delete_glue_ref(self.mem[crate::ix::U(((self.last_line_fill).wrapping_add(1i32)) as usize)].hh().lh());
                    { let __v1398 = ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.best_bet).wrapping_add(3i32)) as usize)].int())).wrapping_sub(self.mem[crate::ix::U(((self.best_bet).wrapping_add(4i32)) as usize)].int()); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1398); }
                    self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
                    { let __ix1399 = (self.last_line_fill).wrapping_add(1i32); self.mem[crate::ix::U((__ix1399) as usize)].set_hh_lh(q); }
                }
            }
        }
        // §1052
        self.post_line_break(d);
        // §1041
        q = self.mem[crate::ix::U((active) as usize)].hh().rh();
        while (q != last_active) {
            {
                self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == delta_node) {
                    self.free_node(q, delta_node_size);
                } else {
                    self.free_node(q, self.active_node_size);
                }
                q = self.cur_p;
            }
        }
        q = self.passive;
        while (q != null) {
            {
                self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                self.free_node(q, passive_node_size);
                q = self.cur_p;
            }
        }
        // §991
        self.pack_begin_line = 0i32;
    }

    /// The `eTeX_enabled` function simply returns its first argument as
    /// result.  This argument is `true` if an optional \eTeX\ feature is
    /// currently enabled; otherwise, if the argument is `false`, the function
    /// gives an error message.
    /// @<Declare \eTeX\ procedures for use...
    // §1656
    pub fn eTeX_enabled(&mut self, mut b: bool, mut j: quarterword, mut k: halfword) -> bool {
        let mut eTeX_enabled: bool = false;
        if (!b) {
            {
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(780i32);
                }
                self.print_cmd_chr(j, k);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1966i32;
                }
                self.error();
            }
        }
        eTeX_enabled = b;
        eTeX_enabled
    }

    /// The modifications of \TeX\ required for the display produced by the
    /// `show_save_groups` procedure were first discussed by Donald~E. Knuth in
    /// {\sl TUGboat\/} {\bf 11}, 165--170 and 499--511, 1990.
    /// In order to understand a group type we also have to know its mode.
    /// Since unrestricted horizontal modes are not associated with grouping,
    /// they are skipped when traversing the semantic nest.
    /// @<Declare \eTeX\ procedures for use...
    // §1679
    pub fn show_save_groups(&mut self) {
        let mut p: i32 = 0; // §1679
        let mut m: i32 = 0; // §1679
        let mut v: save_pointer = 0; // §1679
        let mut l: quarterword = 0; // §1679
        let mut c: group_code = 0; // §1679
        let mut a: i32 = 0; // §1679
        let mut i: i32 = 0; // §1679
        let mut j: quarterword = 0; // §1679
        let mut s: str_number = 0; // §1679
        'l_done_f: {
            p = self.nest_ptr;
            { let __v1400 = self.cur_list; self.nest[crate::ix::U((p) as usize)] = __v1400; }
            v = self.save_ptr;
            l = self.cur_level;
            c = self.cur_group;
            self.save_ptr = self.cur_boundary;
            self.cur_level = (self.cur_level).wrapping_sub(1i32);
            a = 1i32;
            self.print_nl(347i32);
            self.print_ln();
            while true {
                {
                    'l_found_f: {
                        'l_found2_f: {
                            'l_found1_f: {
                                self.print_nl(375i32);
                                self.print_group(true);
                                if (self.cur_group == bottom_level) {
                                    break 'l_done_f;
                                }
                                loop {
                                    m = self.nest[crate::ix::U((p) as usize)].mode_field;
                                    if (p > 0i32) {
                                        p = (p).wrapping_sub(1i32);
                                    } else {
                                        m = vmode;
                                    }
                                    if (m != hmode) { break; }
                                }
                                self.print(288i32);
                                match self.cur_group {
                                    simple_group => {
                                        {
                                            p = (p).wrapping_add(1i32);
                                            break 'l_found2_f;
                                        }
                                    }
                                    hbox_group | adjusted_hbox_group => {
                                        s = 1474i32;
                                    }
                                    vbox_group => {
                                        s = 1383i32;
                                    }
                                    vtop_group => {
                                        s = 1473i32;
                                    }
                                    align_group => {
                                        if (a == 0i32) {
                                            {
                                                if (m == (1i32).wrapping_neg()) {
                                                    s = 597i32;
                                                } else {
                                                    s = 616i32;
                                                }
                                                a = 1i32;
                                                break 'l_found1_f;
                                            }
                                        } else {
                                            {
                                                if (a == 1i32) {
                                                    self.print(2004i32);
                                                } else {
                                                    self.print_esc(1309i32);
                                                }
                                                if (p >= a) {
                                                    p = (p).wrapping_sub(a);
                                                }
                                                a = 0i32;
                                                break 'l_found_f;
                                            }
                                        }
                                    }
                                    no_align_group => {
                                        {
                                            p = (p).wrapping_add(1i32);
                                            a = (1i32).wrapping_neg();
                                            self.print_esc(604i32);
                                            break 'l_found2_f;
                                        }
                                    }
                                    output_group => {
                                        {
                                            self.print_esc(409i32);
                                            break 'l_found_f;
                                        }
                                    }
                                    math_group => {
                                        break 'l_found2_f;
                                    }
                                    disc_group | math_choice_group => {
                                        {
                                            if (self.cur_group == disc_group) {
                                                self.print_esc(360i32);
                                            } else {
                                                self.print_esc(602i32);
                                            }
                                            {
                                                let __for_end_11 = 3i32;
                                                i = 1i32;
                                                while i <= __for_end_11 {
                                                    if (i <= self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int()) {
                                                        self.print(1269i32);
                                                    }
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                            break 'l_found2_f;
                                        }
                                    }
                                    insert_group => {
                                        {
                                            if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int() == 255i32) {
                                                self.print_esc(363i32);
                                            } else {
                                                {
                                                    self.print_esc(339i32);
                                                    self.print_int(((self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int()) as i64));
                                                }
                                            }
                                            break 'l_found2_f;
                                        }
                                    }
                                    vcenter_group => {
                                        {
                                            s = 617i32;
                                            break 'l_found1_f;
                                        }
                                    }
                                    semi_simple_group => {
                                        {
                                            p = (p).wrapping_add(1i32);
                                            self.print_esc(587i32);
                                            break 'l_found_f;
                                        }
                                    }
                                    math_shift_group => {
                                        {
                                            if (m == mmode) {
                                                self.print_char(36i32);
                                            } else {
                                                if (self.nest[crate::ix::U((p) as usize)].mode_field == mmode) {
                                                    {
                                                        self.print_cmd_chr(eq_no, self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int());
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            self.print_char(36i32);
                                            break 'l_found_f;
                                        }
                                    }
                                    math_left_group => {
                                        {
                                            if (self.mem[crate::ix::U((self.nest[crate::ix::U(((p).wrapping_add(1i32)) as usize)].eTeX_aux_field) as usize)].hh().b0() == left_noad) {
                                                self.print_esc(1285i32);
                                            } else {
                                                self.print_esc(1287i32);
                                            }
                                            break 'l_found_f;
                                        }
                                    }
                                    _ => {}
                                }
                                // §1681
                                i = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(4i32)) as usize)].int();
                                if (i != 0i32) {
                                    if (i < box_flag) {
                                        {
                                            if ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() == vmode) {
                                                j = hmove;
                                            } else {
                                                j = vmove;
                                            }
                                            if (i > 0i32) {
                                                self.print_cmd_chr(j, 0i32);
                                            } else {
                                                self.print_cmd_chr(j, 1i32);
                                            }
                                            self.print_scaled((i).wrapping_abs());
                                            self.print(314i32);
                                        }
                                    } else {
                                        if (i < ship_out_flag) {
                                            {
                                                if (i >= global_box_flag) {
                                                    {
                                                        self.print_esc(1590i32);
                                                        i = (i).wrapping_sub(32768i32);
                                                    }
                                                }
                                                self.print_esc(614i32);
                                                self.print_int((((i).wrapping_sub(1073741824i32)) as i64));
                                                self.print_char(61i32);
                                            }
                                        } else {
                                            self.print_cmd_chr(leader_ship, (i).wrapping_sub(1073807261i32));
                                        }
                                    }
                                }
                            }
                            // §1679
                            self.print_esc(s);
                            // §1680
                            if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int() != 0i32) {
                                {
                                    self.print_char(32i32);
                                    if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(3i32)) as usize)].int() == exactly) {
                                        self.print(1245i32);
                                    } else {
                                        self.print(1246i32);
                                    }
                                    self.print_scaled(self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int());
                                    self.print(314i32);
                                }
                            }
                        }
                        // §1679
                        self.print_char(123i32);
                    }
                    self.print_char(41i32);
                    self.cur_level = (self.cur_level).wrapping_sub(1i32);
                    self.cur_group = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                    self.save_ptr = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh();
                }
            }
        }
        self.save_ptr = v;
        self.cur_level = l;
        self.cur_group = c;
    }

    /// We have now completed the hyphenation routine, so the `line_break` procedure
    /// is finished at last. Since the hyphenation exception table is fresh in our
    /// minds, it's a good time to deal with the routine that adds new entries to it.
    /// When \TeX\ has scanned `\.{\\hyphenation}', it calls on a procedure named
    /// `new_hyph_exceptions` to do the right thing.
    // §1111
    pub fn new_hyph_exceptions(&mut self) {
        let mut n: i32 = 0; // §1111
        let mut j: i32 = 0; // §1111
        let mut h: hyph_pointer = 0; // §1111
        let mut k: str_number = 0; // §1111
        let mut p: halfword = 0; // §1111
        let mut q: halfword = 0; // §1111
        let mut s: str_number = 0; // §1111
        let mut t: str_number = 0; // §1111
        let mut u: pool_pointer = 0; // §1111
        let mut v: pool_pointer = 0; // §1111
        'l_exit_f: {
            'l_not_found1_f: {
                self.scan_left_brace();
                if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() <= 0i32) {
                    self.cur_lang = 0i32;
                } else {
                    if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() > 255i32) {
                        self.cur_lang = 0i32;
                    } else {
                        self.cur_lang = self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int();
                    }
                }
                if self.trie_not_ready {
                    {
                        self.hyph_index = 0i32;
                        break 'l_not_found1_f;
                    }
                }
                if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != (self.cur_lang).wrapping_add(0i32)) {
                    self.hyph_index = 0i32;
                } else {
                    self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                }
            }
            n = 0i32;
            // §1112
            p = null;
            while true {
                {
                    self.get_x_token();
                    'l_reswitch_b: loop {
                        match self.cur_cmd {
                            letter | other_char | char_given => {
                                // §1114
                                if (self.cur_chr == 45i32) {
                                    // §1115
                                    {
                                        if (n < 63i32) {
                                            {
                                                q = self.get_avail();
                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                self.mem[crate::ix::U((q) as usize)].set_hh_lh(n);
                                                p = q;
                                            }
                                        }
                                    }
                                } else {
                                    // §1114
                                    {
                                        if (self.hyph_index == 0i32) {
                                            { let __v1401 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1401; }
                                        } else {
                                            if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(self.cur_chr)) as usize)].b1() != (self.cur_chr).wrapping_add(0i32)) {
                                                self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                            } else {
                                                { let __v1402 = (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(self.cur_chr)) as usize)].b0()).wrapping_sub(0i32); self.hc[crate::ix::U((0i32) as usize)] = __v1402; }
                                            }
                                        }
                                        if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                            {
                                                {
                                                    self.dg_mark();
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(264i32);
                                                    }
                                                    self.print(1359i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 1360i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 1361i32;
                                                }
                                                self.error();
                                            }
                                        } else {
                                            if (n < 63i32) {
                                                {
                                                    n = (n).wrapping_add(1i32);
                                                    { let __v1403 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((n) as usize)] = __v1403; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            char_num => {
                                // §1112
                                {
                                    self.scan_char_num();
                                    self.cur_chr = self.cur_val;
                                    self.cur_cmd = char_given;
                                    continue 'l_reswitch_b;
                                }
                            }
                            spacer | right_brace => {
                                {
                                    if (n > 1i32) {
                                        // §1116
                                        {
                                            'l_found1_f: {
                                                n = (n).wrapping_add(1i32);
                                                { let __v1404 = self.cur_lang; self.hc[crate::ix::U((n) as usize)] = __v1404; }
                                                {
                                                    if ((self.pool_ptr).wrapping_add(n) > pool_size) {
                                                        self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                                    }
                                                }
                                                h = 0i32;
                                                {
                                                    let __for_end_12 = n;
                                                    j = 1i32;
                                                    while j <= __for_end_12 {
                                                        {
                                                            h = (((h).wrapping_add(h)).wrapping_add(self.hc[crate::ix::U((j) as usize)]) % hyph_size);
                                                            {
                                                                { let __ix1405 = self.pool_ptr; let __v1406 = self.hc[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U((__ix1405) as usize)] = __v1406; }
                                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                        j = j.wrapping_add(1);
                                                    }
                                                }
                                                s = self.make_string();
                                                // §1117
                                                if (self.hyph_count == hyph_size) {
                                                    self.overflow(1362i32, hyph_size);
                                                }
                                                self.hyph_count = (self.hyph_count).wrapping_add(1i32);
                                                while (self.hyph_word[crate::ix::U((h) as usize)] != 0i32) {
                                                    {
                                                        'l_not_found_f: {
                                                            'l_found_f: {
                                                                // §1118
                                                                k = self.hyph_word[crate::ix::U((h) as usize)];
                                                                if ((self.str_start[crate::ix::U(((k).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((k) as usize)]) < (self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)])) {
                                                                    break 'l_found_f;
                                                                }
                                                                if ((self.str_start[crate::ix::U(((k).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((k) as usize)]) > (self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)])) {
                                                                    break 'l_not_found_f;
                                                                }
                                                                u = self.str_start[crate::ix::U((k) as usize)];
                                                                v = self.str_start[crate::ix::U((s) as usize)];
                                                                loop {
                                                                    if (self.str_pool[crate::ix::U((u) as usize)] < self.str_pool[crate::ix::U((v) as usize)]) {
                                                                        break 'l_found_f;
                                                                    }
                                                                    if (self.str_pool[crate::ix::U((u) as usize)] > self.str_pool[crate::ix::U((v) as usize)]) {
                                                                        break 'l_not_found_f;
                                                                    }
                                                                    u = (u).wrapping_add(1i32);
                                                                    v = (v).wrapping_add(1i32);
                                                                    if (u == self.str_start[crate::ix::U(((k).wrapping_add(1i32)) as usize)]) { break; }
                                                                }
                                                                {
                                                                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                                                    self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                                                                }
                                                                s = self.hyph_word[crate::ix::U((h) as usize)];
                                                                self.hyph_count = (self.hyph_count).wrapping_sub(1i32);
                                                                break 'l_found1_f;
                                                            }
                                                            q = self.hyph_list[crate::ix::U((h) as usize)];
                                                            self.hyph_list[crate::ix::U((h) as usize)] = p;
                                                            p = q;
                                                            t = self.hyph_word[crate::ix::U((h) as usize)];
                                                            self.hyph_word[crate::ix::U((h) as usize)] = s;
                                                            s = t;
                                                        }
                                                        // §1117
                                                        if (h > 0i32) {
                                                            h = (h).wrapping_sub(1i32);
                                                        } else {
                                                            h = hyph_size;
                                                        }
                                                    }
                                                }
                                            }
                                            self.hyph_word[crate::ix::U((h) as usize)] = s;
                                            self.hyph_list[crate::ix::U((h) as usize)] = p;
                                        }
                                    }
                                    // §1112
                                    if (self.cur_cmd == right_brace) {
                                        break 'l_exit_f;
                                    }
                                    n = 0i32;
                                    p = null;
                                }
                            }
                            _ => {
                                // §1113
                                {
                                    {
                                        self.dg_mark();
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(780i32);
                                    }
                                    self.print_esc(1355i32);
                                    self.print(1356i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 1357i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1358i32;
                                    }
                                    self.error();
                                }
                            }
                        }
                        break 'l_reswitch_b;
                    }
                }
            }
        }
        // §1111
    }

    /// A subroutine called `prune_page_top` takes a pointer to a vlist and
    /// returns a pointer to a modified vlist in which all glue, kern, and penalty nodes
    /// have been deleted before the first box or rule node. However, the first
    /// box or rule is actually preceded by a newly created glue node designed so that
    /// the topmost baseline will be at distance `split_top_skip` from the top,
    /// whenever this is possible without backspacing.
    /// When the second argument `s` is `false` the deleted nodes are destroyed,
    /// otherwise they are collected in a list starting at `split_disc`.
    /// In this routine and those that follow, we make use of the fact that a
    /// vertical list contains no character nodes, hence the `type` field exists
    /// for each node in the list.
    // §1145
    pub fn prune_page_top(&mut self, mut p: halfword, mut s: bool) -> halfword {
        let mut prune_page_top: halfword = 0;
        let mut prev_p: halfword = 0; // §1145
        let mut q: halfword = 0; // §1145
        let mut r: halfword = 0; // §1145
        prev_p = temp_head;
        self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
        while (p != null) {
            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                hlist_node | vlist_node | rule_node => {
                    // §1146
                    {
                        q = self.new_skip_param(split_top_skip_code);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(q);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()) {
                            { let __ix1407 = (self.temp_ptr).wrapping_add(1i32); let __v1408 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1407) as usize)].set_int(__v1408); }
                        } else {
                            { let __ix1409 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1409) as usize)].set_int(0i32); }
                        }
                        p = null;
                    }
                }
                whatsit_node | mark_node | ins_node => {
                    // §1145
                    {
                        if ((self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) && ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_comp_node))) {
                            {
                                self.print(1373i32);
                                q = p;
                                p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                                self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                                if s {
                                    {
                                        if (self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] == null) {
                                            self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] = q;
                                        } else {
                                            self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                                        }
                                        r = q;
                                    }
                                } else {
                                    self.flush_node_list(q);
                                }
                            }
                        } else {
                            {
                                prev_p = p;
                                p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                            }
                        }
                    }
                }
                glue_node | kern_node | penalty_node => {
                    {
                        q = p;
                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        if s {
                            {
                                if (self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] == null) {
                                    self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] = q;
                                } else {
                                    self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                                }
                                r = q;
                            }
                        } else {
                            self.flush_node_list(q);
                        }
                    }
                }
                _ => {
                    self.confusion(1375i32);
                }
            }
        }
        prune_page_top = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
        prune_page_top
    }

    /// The next subroutine finds the best place to break a given vertical list
    /// so as to obtain a box of height~`h`, with maximum depth~`d`.
    /// A pointer to the beginning of the vertical list is given,
    /// and a pointer to the optimum breakpoint is returned. The list is effectively
    /// followed by a forced break, i.e., a penalty node with the `eject_penalty`;
    /// if the best break occurs at this artificial node, the value `null` is returned.
    /// An array of six `scaled` distances is used to keep track of the height
    /// from the beginning of the list to the current place, just as in `line_break`.
    /// In fact, we use one of the same arrays, only changing its name to reflect
    /// its new significance.
    // §1147
    pub fn vert_break(&mut self, mut p: halfword, mut h: scaled, mut d: scaled) -> halfword {
        let mut vert_break: halfword = 0;
        let mut prev_p: halfword = 0; // §1147
        let mut q: halfword = 0; // §1147
        let mut r: halfword = 0; // §1147
        let mut pi: i32 = 0; // §1147
        let mut b: i32 = 0; // §1147
        let mut least_cost: i32 = 0; // §1147
        let mut best_place: halfword = 0; // §1147
        let mut prev_dp: scaled = 0; // §1147
        let mut t: small_number = 0; // §1147
        'l_done_f: {
            prev_p = p;
            least_cost = awful_bad;
            self.active_width[crate::ix::U(((1i32) - 1) as usize)] = 0i32;
            self.active_width[crate::ix::U(((2i32) - 1) as usize)] = 0i32;
            self.active_width[crate::ix::U(((3i32) - 1) as usize)] = 0i32;
            self.active_width[crate::ix::U(((4i32) - 1) as usize)] = 0i32;
            self.active_width[crate::ix::U(((5i32) - 1) as usize)] = 0i32;
            self.active_width[crate::ix::U(((6i32) - 1) as usize)] = 0i32;
            prev_dp = 0i32;
            while true {
                {
                    'l_not_found_f: {
                        'l_L90_f: {
                            // §1149
                            if (p == null) {
                                pi = (10000i32).wrapping_neg();
                            } else {
                                // §1150
                                match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                    hlist_node | vlist_node | rule_node => {
                                        {
                                            { let __v1410 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1410; }
                                            prev_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                            break 'l_not_found_f;
                                        }
                                    }
                                    whatsit_node => {
                                        // §1612
                                        {
                                            if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                                {
                                                    { let __v1411 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1411; }
                                                    prev_dp = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                }
                                            }
                                            break 'l_not_found_f;
                                        }
                                    }
                                    glue_node => {
                                        // §1150
                                        if (self.mem[crate::ix::U((prev_p) as usize)].hh().b0() < math_node) {
                                            pi = 0i32;
                                        } else {
                                            break 'l_L90_f;
                                        }
                                    }
                                    kern_node => {
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().rh() == null) {
                                                t = penalty_node;
                                            } else {
                                                t = self.mem[crate::ix::U((self.mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b0();
                                            }
                                            if (t == glue_node) {
                                                pi = 0i32;
                                            } else {
                                                break 'l_L90_f;
                                            }
                                        }
                                    }
                                    penalty_node => {
                                        pi = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                    }
                                    mark_node | ins_node => {
                                        break 'l_not_found_f;
                                    }
                                    _ => {
                                        self.confusion(1376i32);
                                    }
                                }
                            }
                            // §1151
                            if (pi < inf_penalty) {
                                {
                                    // §1152
                                    if (self.active_width[crate::ix::U(((1i32) - 1) as usize)] < h) {
                                        if (((self.active_width[crate::ix::U(((3i32) - 1) as usize)] != 0i32) || (self.active_width[crate::ix::U(((4i32) - 1) as usize)] != 0i32)) || (self.active_width[crate::ix::U(((5i32) - 1) as usize)] != 0i32)) {
                                            b = 0i32;
                                        } else {
                                            b = self.badness((h).wrapping_sub(self.active_width[crate::ix::U(((1i32) - 1) as usize)]), self.active_width[crate::ix::U(((2i32) - 1) as usize)]);
                                        }
                                    } else {
                                        if ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(h) > self.active_width[crate::ix::U(((6i32) - 1) as usize)]) {
                                            b = awful_bad;
                                        } else {
                                            b = self.badness((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(h), self.active_width[crate::ix::U(((6i32) - 1) as usize)]);
                                        }
                                    }
                                    // §1151
                                    if (b < awful_bad) {
                                        if (pi <= (10000i32).wrapping_neg()) {
                                            b = pi;
                                        } else {
                                            if (b < inf_bad) {
                                                b = (b).wrapping_add(pi);
                                            } else {
                                                b = deplorable;
                                            }
                                        }
                                    }
                                    if (b <= least_cost) {
                                        {
                                            best_place = p;
                                            least_cost = b;
                                            self.best_height_plus_depth = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp);
                                        }
                                    }
                                    if ((b == awful_bad) || (pi <= (10000i32).wrapping_neg())) {
                                        break 'l_done_f;
                                    }
                                }
                            }
                            // §1149
                            if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > kern_node)) {
                                break 'l_not_found_f;
                            }
                        }
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                            // §1153
                            q = p;
                        } else {
                            {
                                q = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1412 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1413 = (self.active_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((__ix1412) - 1) as usize)] = __v1413; }
                                { let __v1414 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1414; }
                                if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                    {
                                        if self.is_bit_set(self.eqtb[crate::ix::U(((29388i32) - 1) as usize)].int(), 1i32) {
                                            {
                                                self.old_selector_ignored_err = self.selector;
                                                self.selector = log_only;
                                                {
                                                    crate::system::wr_ln(&mut self.log_file);
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "ignored: ");
                                                }
                                                self.print(1377i32);
                                                self.selector = self.old_selector_ignored_err;
                                            }
                                        } else {
                                            {
                                                {
                                                    self.dg_mark();
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(264i32);
                                                    }
                                                    self.print(1377i32);
                                                }
                                                {
                                                    self.help_ptr = 4i32;
                                                    self.help_line[crate::ix::U((3i32) as usize)] = 1378i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 1379i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 1380i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 1332i32;
                                                }
                                                self.error();
                                            }
                                        }
                                        r = self.new_spec(q);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b1(normal);
                                        self.delete_glue_ref(q);
                                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(r);
                                        q = r;
                                    }
                                }
                            }
                        }
                        { let __v1415 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1415; }
                        prev_dp = 0i32;
                    }
                    // §1149
                    if (prev_dp > d) {
                        {
                            { let __v1416 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_sub(d); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1416; }
                            prev_dp = d;
                        }
                    }
                    // §1147
                    prev_p = p;
                    p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                }
            }
        }
        vert_break = best_place;
        vert_break
    }

    /// The current marks for all mark classes are maintained by the `vsplit`
    /// and `fire_up` routines and are finally destroyed (for \.{INITEX} only)
    /// by the `final_cleanup` routine.  Apart from updating the current marks
    /// when mark nodes are encountered, these routines perform certain actions
    /// on all existing mark classes.  The recursive `do_marks` procedure walks
    /// through the whole tree or a subtree of existing mark class nodes and
    /// preforms certain actions indicted by its first parameter `a`, the action
    /// code.  The second parameter `l` indicates the level of recursion (at
    /// most four); the third parameter points to a nonempty tree or subtree.
    /// The result is `true` if the complete tree or subtree has been deleted.
    // §1825
    pub fn do_marks(&mut self, mut a: small_number, mut l: small_number, mut q: halfword) -> bool {
        let mut do_marks: bool = false;
        let mut i: small_number = 0; // §1825
        if (l < 4i32) {
            {
                {
                    let __for_end_4 = 15i32;
                    i = 0i32;
                    while i <= __for_end_4 {
                        {
                            if (((i) % 2) != 0) {
                                self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                            } else {
                                self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                            }
                            if (self.cur_ptr != null) {
                                if self.do_marks(a, (l).wrapping_add(1i32), self.cur_ptr) {
                                    {
                                        if (((i) % 2) != 0) {
                                            self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                                        } else {
                                            self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(null);
                                        }
                                        { let __v1417 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1417); }
                                    }
                                }
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }
                if (self.mem[crate::ix::U((q) as usize)].hh().b1() == 0i32) {
                    {
                        self.free_node(q, index_node_size);
                        q = null;
                    }
                }
            }
        } else {
            {
                match a {
                    vsplit_init => {
                        // §1826
                        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() != null) {
                            {
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh());
                                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_rh(null);
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().lh());
                                self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_hh_lh(null);
                            }
                        }
                    }
                    fire_up_init => {
                        // §1828
                        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh() != null) {
                            {
                                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                    self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh());
                                }
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh());
                                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                                if (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().rh() == null) {
                                    {
                                        self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh());
                                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_lh(null);
                                    }
                                } else {
                                    { let __ix1418 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh(); let __v1419 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1418) as usize)].set_hh_lh(__v1419); }
                                }
                                { let __v1420 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1420); }
                            }
                        }
                    }
                    fire_up_done => {
                        // §1829
                        if ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != null) && (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == null)) {
                            {
                                { let __v1421 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v1421); }
                                { let __ix1422 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh(); let __v1423 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1422) as usize)].set_hh_lh(__v1423); }
                            }
                        }
                    }
                    destroy_marks => {
                        // §1831
                        {
                            let __for_end_6 = split_bot_mark_code;
                            i = top_mark_code;
                            while i <= __for_end_6 {
                                {
                                    if (((i) % 2) != 0) {
                                        self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                    } else {
                                        self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                    }
                                    if (self.cur_ptr != null) {
                                        {
                                            self.delete_token_ref(self.cur_ptr);
                                            if (((i) % 2) != 0) {
                                                self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                                            } else {
                                                self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(null);
                                            }
                                        }
                                    }
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    _ => {}
                }
                // §1825
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh() == null) {
                    if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().lh() == null) {
                        {
                            self.free_node(q, mark_class_node_size);
                            q = null;
                        }
                    }
                }
            }
        }
        do_marks = (q == null);
        do_marks
    }

    /// Now we are ready to consider `vsplit` itself. Most of
    /// its work is accomplished by the two subroutines that we have just considered.
    /// Given the number of a vlist box `n`, and given a desired page height `h`,
    /// the `vsplit` function finds the best initial segment of the vlist and
    /// returns a box for a page of height~`h`. The remainder of the vlist, if
    /// any, replaces the original box, after removing glue and penalties and
    /// adjusting for `split_top_skip`. Mark nodes in the split-off box are used to
    /// set the values of `split_first_mark` and `split_bot_mark`; we use the
    /// fact that `split_first_mark=null` if and only if `split_bot_mark=null`.
    /// The original box becomes ``void'' if and only if it has been entirely
    /// extracted.  The extracted box is ``void'' if and only if the original
    /// box was void (or if it was, erroneously, an hlist box).
    // §1154
    pub fn vsplit(&mut self, mut n: halfword, mut h: scaled) -> halfword {
        let mut vsplit: halfword = 0;
        let mut v: halfword = 0; // §1154
        let mut p: halfword = 0; // §1154
        let mut q: halfword = 0; // §1154
        'l_exit_f: {
            'l_done_f: {
                self.cur_val = n;
                if (self.cur_val < 256i32) {
                    v = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                } else {
                    {
                        self.find_sa_element(box_val, self.cur_val, false);
                        if (self.cur_ptr == null) {
                            v = null;
                        } else {
                            v = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                        }
                    }
                }
                self.flush_node_list(self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)]);
                self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] = null;
                if (self.sa_root[crate::ix::U((mark_val) as usize)] != null) {
                    if self.do_marks(vsplit_init, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                        self.sa_root[crate::ix::U((mark_val) as usize)] = null;
                    }
                }
                if (self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] != null) {
                    {
                        self.delete_token_ref(self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]);
                        self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] = null;
                        self.delete_token_ref(self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]);
                        self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = null;
                    }
                }
                // §1155
                if (v == null) {
                    {
                        vsplit = null;
                        break 'l_exit_f;
                    }
                }
                if (self.mem[crate::ix::U((v) as usize)].hh().b0() != vlist_node) {
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(347i32);
                        }
                        self.print_esc(1381i32);
                        self.print(1382i32);
                        self.print_esc(1383i32);
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1384i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1385i32;
                        }
                        self.error();
                        vsplit = null;
                        break 'l_exit_f;
                    }
                }
                // §1154
                q = self.vert_break(self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh(), h, self.eqtb[crate::ix::U(((29909i32) - 1) as usize)].int());
                // §1156
                p = self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh();
                if (p == q) {
                    self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(null);
                } else {
                    while true {
                        {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node) {
                                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                    // §1827
                                    {
                                        self.find_sa_element(mark_val, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), true);
                                        if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().rh() == null) {
                                            {
                                                { let __ix1424 = (self.cur_ptr).wrapping_add(2i32); let __v1425 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1424) as usize)].set_hh_rh(__v1425); }
                                                { let __ix1426 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1427 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1426) as usize)].set_hh_lh(__v1427); }
                                            }
                                        } else {
                                            self.delete_token_ref(self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(3i32)) as usize)].hh().lh());
                                        }
                                        { let __ix1428 = (self.cur_ptr).wrapping_add(3i32); let __v1429 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1428) as usize)].set_hh_lh(__v1429); }
                                        { let __ix1430 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1431 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1430) as usize)].set_hh_lh(__v1431); }
                                    }
                                } else {
                                    // §1156
                                    if (self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] == null) {
                                        {
                                            { let __v1432 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] = __v1432; }
                                            { let __v1433 = self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]; self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = __v1433; }
                                            { let __ix1434 = self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]; let __v1435 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(2i32); self.mem[crate::ix::U((__ix1434) as usize)].set_hh_lh(__v1435); }
                                        }
                                    } else {
                                        {
                                            self.delete_token_ref(self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]);
                                            { let __v1436 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = __v1436; }
                                            { let __ix1437 = self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]; let __v1438 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1437) as usize)].set_hh_lh(__v1438); }
                                        }
                                    }
                                }
                            }
                            if (self.mem[crate::ix::U((p) as usize)].hh().rh() == q) {
                                {
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                                    break 'l_done_f;
                                }
                            }
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §1154
            q = self.prune_page_top(q, (self.eqtb[crate::ix::U(((29386i32) - 1) as usize)].int() > 0i32));
            p = self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh();
            self.free_node(v, box_node_size);
            if (q != null) {
                q = self.vpackage(q, 0i32, additional, max_dimen);
            }
            if (self.cur_val < 256i32) {
                { let __ix1439 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1439) - 1) as usize)].set_hh_rh(q); }
            } else {
                {
                    self.find_sa_element(box_val, self.cur_val, false);
                    if (self.cur_ptr != null) {
                        {
                            { let __ix1440 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1440) as usize)].set_hh_rh(q); }
                            { let __ix1441 = (self.cur_ptr).wrapping_add(1i32); let __v1442 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1441) as usize)].set_hh_lh(__v1442); }
                            self.delete_sa_ref(self.cur_ptr);
                        }
                    }
                }
            }
            vsplit = self.vpackage(p, h, exactly, self.eqtb[crate::ix::U(((29909i32) - 1) as usize)].int());
        }
        vsplit
    }

    // §1162
    pub fn print_totals(&mut self) {
        self.print_scaled(self.page_so_far[crate::ix::U((1i32) as usize)]);
        if (self.page_so_far[crate::ix::U((2i32) as usize)] != 0i32) {
            {
                self.print(319i32);
                self.print_scaled(self.page_so_far[crate::ix::U((2i32) as usize)]);
                self.print(347i32);
            }
        }
        if (self.page_so_far[crate::ix::U((3i32) as usize)] != 0i32) {
            {
                self.print(319i32);
                self.print_scaled(self.page_so_far[crate::ix::U((3i32) as usize)]);
                self.print(318i32);
            }
        }
        if (self.page_so_far[crate::ix::U((4i32) as usize)] != 0i32) {
            {
                self.print(319i32);
                self.print_scaled(self.page_so_far[crate::ix::U((4i32) as usize)]);
                self.print(1394i32);
            }
        }
        if (self.page_so_far[crate::ix::U((5i32) as usize)] != 0i32) {
            {
                self.print(319i32);
                self.print_scaled(self.page_so_far[crate::ix::U((5i32) as usize)]);
                self.print(1395i32);
            }
        }
        if (self.page_so_far[crate::ix::U((6i32) as usize)] != 0i32) {
            {
                self.print(320i32);
                self.print_scaled(self.page_so_far[crate::ix::U((6i32) as usize)]);
            }
        }
    }

    /// Here is a procedure that is called when the `page_contents` is changing
    /// from `empty` to `inserts_only` or `box_there`.
    // §1164
    pub fn freeze_page_specs(&mut self, mut s: small_number) {
        self.page_contents = s;
        { let __v1443 = self.eqtb[crate::ix::U(((29907i32) - 1) as usize)].int(); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1443; }
        self.page_max_depth = self.eqtb[crate::ix::U(((29908i32) - 1) as usize)].int();
        self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((1i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((2i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((3i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((4i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((5i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((6i32) as usize)] = 0i32;
        self.least_page_cost = awful_bad;
        if (self.eqtb[crate::ix::U(((29310i32) - 1) as usize)].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_nl(1403i32);
                self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                self.print(1404i32);
                self.print_scaled(self.page_max_depth);
                self.end_diagnostic(false);
            }
        }
    }

    /// At certain times box 255 is supposed to be void (i.e., `null`),
    /// or an insertion box is supposed to be ready to accept a vertical list.
    /// If not, an error message is printed, and the following subroutine
    /// flushes the unwanted contents, reporting them to the user.
    // §1169
    pub fn box_error(&mut self, mut n: eight_bits) {
        self.error();
        self.begin_diagnostic();
        self.print_nl(998i32);
        self.show_box(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        self.end_diagnostic(true);
        self.flush_node_list(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh(null);
    }

    /// The following procedure guarantees that a given box register
    /// does not contain an \.{\\hbox}.
    // §1170
    pub fn ensure_vbox(&mut self, mut n: eight_bits) {
        let mut p: halfword = 0; // §1170
        p = self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh();
        if (p != null) {
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) {
                {
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(1405i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 1406i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1407i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1408i32;
                    }
                    self.box_error(n);
                }
            }
        }
    }

    /// When the page builder has looked at as much material as could appear before
    /// the next page break, it makes its decision. The break that gave minimum
    /// badness will be used to put a completed ``page'' into box 255, with insertions
    /// appended to their other boxes.
    /// We also set the values of `top_mark`, `first_mark`, and `bot_mark`. The
    /// program uses the fact that `bot_mark<>null` implies `first_mark<>null`;
    /// it also knows that `bot_mark=null` implies `top_mark=first_mark=null`.
    /// The `fire_up` subroutine prepares to output the current page at the best
    /// place; then it fires up the user's output routine, if there is one,
    /// or it simply ships out the page. There is one parameter, `c`, which represents
    /// the node that was being contributed to the page when the decision to
    /// force an output was made.
    /// @<Declare the procedure called `fire_up`
    // §1189
    pub fn fire_up(&mut self, mut c: halfword) {
        let mut p: halfword = 0; // §1189
        let mut q: halfword = 0; // §1189
        let mut r: halfword = 0; // §1189
        let mut s: halfword = 0; // §1189
        let mut prev_p: halfword = 0; // §1189
        let mut n: i32 = 0; // §1189
        let mut wait: bool = false; // §1189
        let mut save_vbadness: i32 = 0; // §1189
        let mut save_vfuzz: scaled = 0; // §1189
        let mut save_split_top_skip: halfword = 0; // §1189
        'l_exit_f: {
            // §1190
            if (self.mem[crate::ix::U((self.best_page_break) as usize)].hh().b0() == penalty_node) {
                {
                    self.geq_word_define(29316i32, self.mem[crate::ix::U(((self.best_page_break).wrapping_add(1i32)) as usize)].int());
                    { let __ix1444 = (self.best_page_break).wrapping_add(1i32); self.mem[crate::ix::U((__ix1444) as usize)].set_int(inf_penalty); }
                }
            } else {
                self.geq_word_define(29316i32, inf_penalty);
            }
            // §1189
            if (self.sa_root[crate::ix::U((mark_val) as usize)] != null) {
                if self.do_marks(fire_up_init, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                    self.sa_root[crate::ix::U((mark_val) as usize)] = null;
                }
            }
            if (self.cur_mark[crate::ix::U((bot_mark_code) as usize)] != null) {
                {
                    if (self.cur_mark[crate::ix::U((top_mark_code) as usize)] != null) {
                        self.delete_token_ref(self.cur_mark[crate::ix::U((top_mark_code) as usize)]);
                    }
                    { let __v1445 = self.cur_mark[crate::ix::U((bot_mark_code) as usize)]; self.cur_mark[crate::ix::U((top_mark_code) as usize)] = __v1445; }
                    { let __ix1446 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; let __v1447 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((top_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1446) as usize)].set_hh_lh(__v1447); }
                    self.delete_token_ref(self.cur_mark[crate::ix::U((first_mark_code) as usize)]);
                    self.cur_mark[crate::ix::U((first_mark_code) as usize)] = null;
                }
            }
            // §1191
            if (c == self.best_page_break) {
                self.best_page_break = null;
            }
            // §1192
            if (self.eqtb[crate::ix::U(((27688i32) - 1) as usize)].hh().rh() != null) {
                {
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(347i32);
                    }
                    self.print_esc(424i32);
                    self.print(1418i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1419i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1408i32;
                    }
                    self.box_error(255i32);
                }
            }
            // §1191
            self.insert_penalties = 0i32;
            save_split_top_skip = self.eqtb[crate::ix::U(((26638i32) - 1) as usize)].hh().rh();
            if (self.eqtb[crate::ix::U(((29330i32) - 1) as usize)].int() <= 0i32) {
                // §1195
                {
                    r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                    while (r != page_ins_head) {
                        {
                            if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() != null) {
                                {
                                    n = (self.mem[crate::ix::U((r) as usize)].hh().b1()).wrapping_sub(0i32);
                                    self.ensure_vbox(n);
                                    if (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh() == null) {
                                        { let __v1448 = self.new_null_box(); self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh(__v1448); }
                                    }
                                    p = (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(5i32);
                                    while (self.mem[crate::ix::U((p) as usize)].hh().rh() != null) {
                                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    }
                                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_rh(p);
                                }
                            }
                            r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §1191
            q = hold_head;
            self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
            prev_p = page_head;
            p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
            while (p != self.best_page_break) {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) {
                        {
                            if (self.eqtb[crate::ix::U(((29330i32) - 1) as usize)].int() <= 0i32) {
                                // §1197
                                {
                                    r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                    while (self.mem[crate::ix::U((r) as usize)].hh().b1() != self.mem[crate::ix::U((p) as usize)].hh().b1()) {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                    if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() == null) {
                                        wait = true;
                                    } else {
                                        {
                                            wait = false;
                                            s = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().rh();
                                            { let __v1449 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1449); }
                                            if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() == p) {
                                                // §1198
                                                {
                                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == split_up) {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == p) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() != null)) {
                                                            {
                                                                while (self.mem[crate::ix::U((s) as usize)].hh().rh() != self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh()) {
                                                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                }
                                                                self.mem[crate::ix::U((s) as usize)].set_hh_rh(null);
                                                                { let __v1450 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(); self.eqtb[crate::ix::U(((26638i32) - 1) as usize)].set_hh_rh(__v1450); }
                                                                { let __v1451 = self.prune_page_top(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh(), false); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_lh(__v1451); }
                                                                if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh() != null) {
                                                                    {
                                                                        self.temp_ptr = self.vpackage(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh(), 0i32, additional, max_dimen);
                                                                        { let __v1452 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v1452); }
                                                                        self.free_node(self.temp_ptr, box_node_size);
                                                                        wait = true;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(null);
                                                    n = (self.mem[crate::ix::U((r) as usize)].hh().b1()).wrapping_sub(0i32);
                                                    self.temp_ptr = self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(5i32)) as usize)].hh().rh();
                                                    self.free_node(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh(), box_node_size);
                                                    { let __v1453 = self.vpackage(self.temp_ptr, 0i32, additional, max_dimen); self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh(__v1453); }
                                                }
                                            } else {
                                                // §1197
                                                {
                                                    while (self.mem[crate::ix::U((s) as usize)].hh().rh() != null) {
                                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                    }
                                                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_rh(s);
                                                }
                                            }
                                        }
                                    }
                                    // §1199
                                    { let __v1454 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(__v1454); }
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                                    if wait {
                                        {
                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                            q = p;
                                            self.insert_penalties = (self.insert_penalties).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            self.delete_glue_ref(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh());
                                            self.free_node(p, ins_node_size);
                                        }
                                    }
                                    p = prev_p;
                                }
                            }
                        }
                    } else {
                        // §1191
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node) {
                            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                // §1830
                                {
                                    self.find_sa_element(mark_val, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), true);
                                    if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh() == null) {
                                        {
                                            { let __ix1455 = (self.cur_ptr).wrapping_add(1i32); let __v1456 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1455) as usize)].set_hh_rh(__v1456); }
                                            { let __ix1457 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1458 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1457) as usize)].set_hh_lh(__v1458); }
                                        }
                                    }
                                    if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().lh() != null) {
                                        self.delete_token_ref(self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().lh());
                                    }
                                    { let __ix1459 = (self.cur_ptr).wrapping_add(2i32); let __v1460 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1459) as usize)].set_hh_lh(__v1460); }
                                    { let __ix1461 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1462 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1461) as usize)].set_hh_lh(__v1462); }
                                }
                            } else {
                                // §1193
                                {
                                    if (self.cur_mark[crate::ix::U((first_mark_code) as usize)] == null) {
                                        {
                                            { let __v1463 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((first_mark_code) as usize)] = __v1463; }
                                            { let __ix1464 = self.cur_mark[crate::ix::U((first_mark_code) as usize)]; let __v1465 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((first_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1464) as usize)].set_hh_lh(__v1465); }
                                        }
                                    }
                                    if (self.cur_mark[crate::ix::U((bot_mark_code) as usize)] != null) {
                                        self.delete_token_ref(self.cur_mark[crate::ix::U((bot_mark_code) as usize)]);
                                    }
                                    { let __v1466 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((bot_mark_code) as usize)] = __v1466; }
                                    { let __ix1467 = self.cur_mark[crate::ix::U((bot_mark_code) as usize)]; let __v1468 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((bot_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1467) as usize)].set_hh_lh(__v1468); }
                                }
                            }
                        }
                    }
                    // §1191
                    prev_p = p;
                    p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                }
            }
            self.eqtb[crate::ix::U(((26638i32) - 1) as usize)].set_hh_rh(save_split_top_skip);
            // §1194
            if (p != null) {
                {
                    if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == null) {
                        if (self.nest_ptr == 0i32) {
                            self.cur_list.tail_field = self.page_tail;
                        } else {
                            self.nest[crate::ix::U((0i32) as usize)].tail_field = self.page_tail;
                        }
                    }
                    { let __ix1469 = self.page_tail; let __v1470 = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1469) as usize)].set_hh_rh(__v1470); }
                    self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(p);
                    self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(null);
                }
            }
            save_vbadness = self.eqtb[crate::ix::U(((29304i32) - 1) as usize)].int();
            self.eqtb[crate::ix::U(((29304i32) - 1) as usize)].set_int(inf_bad);
            save_vfuzz = self.eqtb[crate::ix::U(((29912i32) - 1) as usize)].int();
            self.eqtb[crate::ix::U(((29912i32) - 1) as usize)].set_int(max_dimen);
            { let __v1471 = self.vpackage(self.mem[crate::ix::U((page_head) as usize)].hh().rh(), self.best_size, exactly, self.page_max_depth); self.eqtb[crate::ix::U(((27688i32) - 1) as usize)].set_hh_rh(__v1471); }
            self.eqtb[crate::ix::U(((29304i32) - 1) as usize)].set_int(save_vbadness);
            self.eqtb[crate::ix::U(((29912i32) - 1) as usize)].set_int(save_vfuzz);
            if (self.last_glue != max_halfword) {
                self.delete_glue_ref(self.last_glue);
            }
            // §1168
            self.page_contents = empty;
            self.page_tail = page_head;
            self.mem[crate::ix::U((page_head) as usize)].set_hh_rh(null);
            self.last_glue = max_halfword;
            self.last_penalty = 0i32;
            self.last_kern = 0i32;
            self.last_node_type = (1i32).wrapping_neg();
            self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
            self.page_max_depth = 0i32;
            // §1194
            if (q != hold_head) {
                {
                    { let __v1472 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((page_head) as usize)].set_hh_rh(__v1472); }
                    self.page_tail = q;
                }
            }
            // §1196
            r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
            while (r != page_ins_head) {
                {
                    q = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    self.free_node(r, page_ins_node_size);
                    r = q;
                }
            }
            self.mem[crate::ix::U((page_ins_head) as usize)].set_hh_rh(page_ins_head);
            // §1189
            if (self.sa_root[crate::ix::U((mark_val) as usize)] != null) {
                if self.do_marks(fire_up_done, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                    self.sa_root[crate::ix::U((mark_val) as usize)] = null;
                }
            }
            if ((self.cur_mark[crate::ix::U((top_mark_code) as usize)] != null) && (self.cur_mark[crate::ix::U((first_mark_code) as usize)] == null)) {
                {
                    { let __v1473 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; self.cur_mark[crate::ix::U((first_mark_code) as usize)] = __v1473; }
                    { let __ix1474 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; let __v1475 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((top_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1474) as usize)].set_hh_lh(__v1475); }
                }
            }
            if (self.eqtb[crate::ix::U(((output_routine_loc) - 1) as usize)].hh().rh() != null) {
                if (self.dead_cycles >= self.eqtb[crate::ix::U(((29317i32) - 1) as usize)].int()) {
                    // §1201
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1420i32);
                        }
                        self.print_int(((self.dead_cycles) as i64));
                        self.print(1421i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1422i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1423i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1424i32;
                        }
                        self.error();
                    }
                } else {
                    // §1202
                    {
                        self.output_active = true;
                        self.dead_cycles = (self.dead_cycles).wrapping_add(1i32);
                        self.push_nest();
                        self.cur_list.mode_field = (1i32).wrapping_neg();
                        { let __v1476 = self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v1476); }
                        self.cur_list.ml_field = (self.line).wrapping_neg();
                        self.begin_token_list(self.eqtb[crate::ix::U(((output_routine_loc) - 1) as usize)].hh().rh(), output_text);
                        self.new_save_level(output_group);
                        self.normal_paragraph();
                        self.scan_left_brace();
                        break 'l_exit_f;
                    }
                }
            }
            // §1200
            {
                if (self.mem[crate::ix::U((page_head) as usize)].hh().rh() != null) {
                    {
                        if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == null) {
                            if (self.nest_ptr == 0i32) {
                                self.cur_list.tail_field = self.page_tail;
                            } else {
                                self.nest[crate::ix::U((0i32) as usize)].tail_field = self.page_tail;
                            }
                        } else {
                            { let __ix1477 = self.page_tail; let __v1478 = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1477) as usize)].set_hh_rh(__v1478); }
                        }
                        { let __v1479 = self.mem[crate::ix::U((page_head) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1479); }
                        self.mem[crate::ix::U((page_head) as usize)].set_hh_rh(null);
                        self.page_tail = page_head;
                    }
                }
                self.flush_node_list(self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)]);
                self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = null;
                self.ship_out(self.eqtb[crate::ix::U(((27688i32) - 1) as usize)].hh().rh());
                self.eqtb[crate::ix::U(((27688i32) - 1) as usize)].set_hh_rh(null);
            }
        }
        // §1189
    }

    /// \TeX\ is not always in vertical mode at the time `build_page`
    /// is called; the current mode reflects what \TeX\ should return to, after
    /// the contribution list has been emptied. A call on `build_page` should
    /// be immediately followed by ``goto big_switch`', which is \TeX's central
    /// control point.
    // §1171
    pub fn build_page(&mut self) {
        let mut p: halfword = 0; // §1171
        let mut q: halfword = 0; // §1171
        let mut r: halfword = 0; // §1171
        let mut b: i32 = 0; // §1171
        let mut c: i32 = 0; // §1171
        let mut pi: i32 = 0; // §1171
        let mut n: i32 = 0; // §1171
        let mut delta: scaled = 0; // §1171
        let mut h: scaled = 0; // §1171
        let mut w: scaled = 0; // §1171
        'l_exit_f: {
            if ((self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == null) || self.output_active) {
                break 'l_exit_f;
            }
            loop {
                // goto labels: continue, L90, L80, done1, done
                let mut __goto_1: i32 = 0;
                'l_dispatch_1: loop {
                    if __goto_1 <= 0 {
                        p = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh();
                        // §1173
                        if (self.last_glue != max_halfword) {
                            self.delete_glue_ref(self.last_glue);
                        }
                        self.last_penalty = 0i32;
                        self.last_kern = 0i32;
                        self.last_node_type = (self.mem[crate::ix::U((p) as usize)].hh().b0()).wrapping_add(1i32);
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) {
                            {
                                self.last_glue = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1480 = self.last_glue; let __v1481 = (self.mem[crate::ix::U((self.last_glue) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1480) as usize)].set_hh_rh(__v1481); }
                            }
                        } else {
                            {
                                self.last_glue = max_halfword;
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node) {
                                    self.last_penalty = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                } else {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                                        self.last_kern = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                    }
                                }
                            }
                        }
                        // §1177
                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                            hlist_node | vlist_node | rule_node => {
                                if (self.page_contents < box_there) {
                                    // §1178
                                    {
                                        if (self.page_contents == empty) {
                                            self.freeze_page_specs(box_there);
                                        } else {
                                            self.page_contents = box_there;
                                        }
                                        q = self.new_skip_param(top_skip_code);
                                        if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()) {
                                            { let __ix1482 = (self.temp_ptr).wrapping_add(1i32); let __v1483 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1482) as usize)].set_int(__v1483); }
                                        } else {
                                            { let __ix1484 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1484) as usize)].set_int(0i32); }
                                        }
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                        self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(q);
                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    // §1179
                                    {
                                        { let __v1485 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1485; }
                                        { let __v1486 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.page_so_far[crate::ix::U((7i32) as usize)] = __v1486; }
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            whatsit_node => {
                                // §1177
                                if ((self.page_contents < box_there) && ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_comp_node))) {
                                    {
                                        self.print(1373i32);
                                        { __goto_1 = 3; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    // §1611
                                    {
                                        if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                            {
                                                { let __v1487 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1487; }
                                                { let __v1488 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.page_so_far[crate::ix::U((7i32) as usize)] = __v1488; }
                                            }
                                        }
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            glue_node => {
                                // §1177
                                if (self.page_contents < box_there) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    if (self.mem[crate::ix::U((self.page_tail) as usize)].hh().b0() < math_node) {
                                        pi = 0i32;
                                    } else {
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            kern_node => {
                                if (self.page_contents < box_there) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().rh() == null) {
                                        break 'l_exit_f;
                                    } else {
                                        if (self.mem[crate::ix::U((self.mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b0() == glue_node) {
                                            pi = 0i32;
                                        } else {
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            }
                            penalty_node => {
                                if (self.page_contents < box_there) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    pi = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                }
                            }
                            mark_node => {
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            }
                            ins_node => {
                                // §1185
                                {
                                    if (self.page_contents == empty) {
                                        self.freeze_page_specs(inserts_only);
                                    }
                                    n = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    r = page_ins_head;
                                    while (n >= self.mem[crate::ix::U((self.mem[crate::ix::U((r) as usize)].hh().rh()) as usize)].hh().b1()) {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                    n = (n).wrapping_sub(0i32);
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b1() != (n).wrapping_add(0i32)) {
                                        // §1186
                                        {
                                            q = self.get_node(page_ins_node_size);
                                            { let __v1489 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1489); }
                                            self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                                            r = q;
                                            self.mem[crate::ix::U((r) as usize)].set_hh_b1((n).wrapping_add(0i32));
                                            self.mem[crate::ix::U((r) as usize)].set_hh_b0(inserting);
                                            self.ensure_vbox(n);
                                            if (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh() == null) {
                                                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(0i32);
                                            } else {
                                                { let __v1490 = (self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1490); }
                                            }
                                            self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(null);
                                            q = self.eqtb[crate::ix::U((((skip_base).wrapping_add(n)) - 1) as usize)].hh().rh();
                                            if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() == 1000i32) {
                                                h = self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int();
                                            } else {
                                                h = (self.x_over_n(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int(), 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int());
                                            }
                                            { let __v1491 = ((self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(h)).wrapping_sub(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1491; }
                                            { let __ix1492 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1493 = (self.page_so_far[crate::ix::U(((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.page_so_far[crate::ix::U((__ix1492) as usize)] = __v1493; }
                                            { let __v1494 = (self.page_so_far[crate::ix::U((6i32) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((6i32) as usize)] = __v1494; }
                                            if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                {
                                                    {
                                                        self.dg_mark();
                                                        if (self.interaction == error_stop_mode) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(264i32);
                                                        }
                                                        self.print(1413i32);
                                                    }
                                                    self.print_esc(407i32);
                                                    self.print_int(((n) as i64));
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[crate::ix::U((2i32) as usize)] = 1414i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 1415i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 1332i32;
                                                    }
                                                    self.error();
                                                }
                                            }
                                        }
                                    }
                                    // §1185
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == split_up) {
                                        self.insert_penalties = (self.insert_penalties).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    } else {
                                        {
                                            self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_rh(p);
                                            delta = (((self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(self.page_so_far[crate::ix::U((1i32) as usize)])).wrapping_sub(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.page_so_far[crate::ix::U((6i32) as usize)]);
                                            if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() == 1000i32) {
                                                h = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                            } else {
                                                h = (self.x_over_n(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int());
                                            }
                                            if (((h <= 0i32) || (h <= delta)) && ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()) <= self.eqtb[crate::ix::U((((scaled_base).wrapping_add(n)) - 1) as usize)].int())) {
                                                {
                                                    { let __v1495 = (self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(h); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1495; }
                                                    { let __v1496 = (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1496); }
                                                }
                                            } else {
                                                // §1187
                                                {
                                                    if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() <= 0i32) {
                                                        w = max_dimen;
                                                    } else {
                                                        {
                                                            w = ((self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(self.page_so_far[crate::ix::U((1i32) as usize)])).wrapping_sub(self.page_so_far[crate::ix::U((7i32) as usize)]);
                                                            if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() != 1000i32) {
                                                                w = (self.x_over_n(w, self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int())).wrapping_mul(1000i32);
                                                            }
                                                        }
                                                    }
                                                    if (w > (self.eqtb[crate::ix::U((((scaled_base).wrapping_add(n)) - 1) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int())) {
                                                        w = (self.eqtb[crate::ix::U((((scaled_base).wrapping_add(n)) - 1) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int());
                                                    }
                                                    q = self.vert_break(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh(), w, self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    { let __v1497 = (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.best_height_plus_depth); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1497); }
                                                    if (self.eqtb[crate::ix::U(((29310i32) - 1) as usize)].int() > 0i32) {
                                                        // §1188
                                                        {
                                                            self.begin_diagnostic();
                                                            self.print_nl(1416i32);
                                                            self.print_int(((n) as i64));
                                                            self.print(1417i32);
                                                            self.print_scaled(w);
                                                            self.print_char(44i32);
                                                            self.print_scaled(self.best_height_plus_depth);
                                                            self.print(1345i32);
                                                            if (q == null) {
                                                                self.print_int((((10000i32).wrapping_neg()) as i64));
                                                            } else {
                                                                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) {
                                                                    self.print_int(((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) as i64));
                                                                } else {
                                                                    self.print_char(48i32);
                                                                }
                                                            }
                                                            self.end_diagnostic(false);
                                                        }
                                                    }
                                                    // §1187
                                                    if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() != 1000i32) {
                                                        self.best_height_plus_depth = (self.x_over_n(self.best_height_plus_depth, 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int());
                                                    }
                                                    { let __v1498 = (self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(self.best_height_plus_depth); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1498; }
                                                    self.mem[crate::ix::U((r) as usize)].set_hh_b0(split_up);
                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(q);
                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                                                    if (q == null) {
                                                        self.insert_penalties = (self.insert_penalties).wrapping_sub(10000i32);
                                                    } else {
                                                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) {
                                                            self.insert_penalties = (self.insert_penalties).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // §1185
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            _ => {
                                // §1177
                                self.confusion(888i32);
                            }
                        }
                        // §1182
                        if (pi < inf_penalty) {
                            {
                                // §1184
                                if (self.page_so_far[crate::ix::U((1i32) as usize)] < self.page_so_far[crate::ix::U((0i32) as usize)]) {
                                    if (((self.page_so_far[crate::ix::U((3i32) as usize)] != 0i32) || (self.page_so_far[crate::ix::U((4i32) as usize)] != 0i32)) || (self.page_so_far[crate::ix::U((5i32) as usize)] != 0i32)) {
                                        b = 0i32;
                                    } else {
                                        b = self.badness((self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(self.page_so_far[crate::ix::U((1i32) as usize)]), self.page_so_far[crate::ix::U((2i32) as usize)]);
                                    }
                                } else {
                                    if ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_sub(self.page_so_far[crate::ix::U((0i32) as usize)]) > self.page_so_far[crate::ix::U((6i32) as usize)]) {
                                        b = awful_bad;
                                    } else {
                                        b = self.badness((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_sub(self.page_so_far[crate::ix::U((0i32) as usize)]), self.page_so_far[crate::ix::U((6i32) as usize)]);
                                    }
                                }
                                // §1182
                                if (b < awful_bad) {
                                    if (pi <= (10000i32).wrapping_neg()) {
                                        c = pi;
                                    } else {
                                        if (b < inf_bad) {
                                            c = ((b).wrapping_add(pi)).wrapping_add(self.insert_penalties);
                                        } else {
                                            c = deplorable;
                                        }
                                    }
                                } else {
                                    c = b;
                                }
                                if (self.insert_penalties >= 10000i32) {
                                    c = awful_bad;
                                }
                                if (self.eqtb[crate::ix::U(((29310i32) - 1) as usize)].int() > 0i32) {
                                    // §1183
                                    {
                                        self.begin_diagnostic();
                                        self.print_nl(37i32);
                                        self.print(1341i32);
                                        self.print_totals();
                                        self.print(1411i32);
                                        self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                                        self.print(1344i32);
                                        if (b == awful_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(((b) as i64));
                                        }
                                        self.print(1345i32);
                                        self.print_int(((pi) as i64));
                                        self.print(1412i32);
                                        if (c == awful_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(((c) as i64));
                                        }
                                        if (c <= self.least_page_cost) {
                                            self.print_char(35i32);
                                        }
                                        self.end_diagnostic(false);
                                    }
                                }
                                // §1182
                                if (c <= self.least_page_cost) {
                                    {
                                        self.best_page_break = p;
                                        self.best_size = self.page_so_far[crate::ix::U((0i32) as usize)];
                                        self.least_page_cost = c;
                                        r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                        while (r != page_ins_head) {
                                            {
                                                { let __v1499 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(__v1499); }
                                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                            }
                                        }
                                    }
                                }
                                if ((c == awful_bad) || (pi <= (10000i32).wrapping_neg())) {
                                    {
                                        self.fire_up(p);
                                        if self.output_active {
                                            break 'l_exit_f;
                                        }
                                        { __goto_1 = 4; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                        }
                        // §1174
                        if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > kern_node)) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    if __goto_1 <= 1 { // L90
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                            // §1181
                            q = p;
                        } else {
                            {
                                q = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1500 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1501 = (self.page_so_far[crate::ix::U(((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.page_so_far[crate::ix::U((__ix1500) as usize)] = __v1501; }
                                { let __v1502 = (self.page_so_far[crate::ix::U((6i32) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((6i32) as usize)] = __v1502; }
                                if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                    {
                                        {
                                            self.dg_mark();
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(264i32);
                                            }
                                            self.print(1409i32);
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[crate::ix::U((3i32) as usize)] = 1410i32;
                                            self.help_line[crate::ix::U((2i32) as usize)] = 1379i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 1380i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 1332i32;
                                        }
                                        self.error();
                                        r = self.new_spec(q);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b1(normal);
                                        self.delete_glue_ref(q);
                                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(r);
                                        q = r;
                                    }
                                }
                            }
                        }
                        { let __v1503 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1503; }
                        self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
                    }
                    if __goto_1 <= 2 { // L80
                        // §1174
                        if (self.page_so_far[crate::ix::U((7i32) as usize)] > self.page_max_depth) {
                            // §1180
                            {
                                { let __v1504 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_sub(self.page_max_depth); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1504; }
                                { let __v1505 = self.page_max_depth; self.page_so_far[crate::ix::U((7i32) as usize)] = __v1505; }
                            }
                        }
                        // §1175
                        { let __ix1506 = self.page_tail; self.mem[crate::ix::U((__ix1506) as usize)].set_hh_rh(p); }
                        self.page_tail = p;
                        { let __v1507 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1507); }
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                        { __goto_1 = 4; continue 'l_dispatch_1; }
                    }
                    if __goto_1 <= 3 { // done1
                        // §1174
                        { let __v1508 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1508); }
                        // §1176
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                        if (self.eqtb[crate::ix::U(((29386i32) - 1) as usize)].int() > 0i32) {
                            {
                                if (self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] == null) {
                                    self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = p;
                                } else {
                                    { let __ix1509 = self.disc_ptr[crate::ix::U(((copy_code) - 1) as usize)]; self.mem[crate::ix::U((__ix1509) as usize)].set_hh_rh(p); }
                                }
                                self.disc_ptr[crate::ix::U(((copy_code) - 1) as usize)] = p;
                            }
                        } else {
                            self.flush_node_list(p);
                        }
                    }
                    if __goto_1 <= 4 { // done
                        // §1174
                    }
                    break 'l_dispatch_1;
                }
                if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == null) { break; }
            }
            // §1172
            if (self.nest_ptr == 0i32) {
                self.cur_list.tail_field = contrib_head;
            } else {
                self.nest[crate::ix::U((0i32) as usize)].tail_field = contrib_head;
            }
        }
        // §1171
        if (self.ckpt_on_segment != 0i32) {
            if (self.ckpt_request == 0i32) {
                if (!self.output_active) {
                    self.ckpt_request = self.ckpt_on_segment;
                }
            }
        }
    }

    /// @<Declare act...
    // §1221
    pub fn app_space(&mut self) {
        let mut q: halfword = 0; // §1221
        if ((self.cur_list.aux_field.hh().lh() >= 2000i32) && (self.eqtb[crate::ix::U(((26641i32) - 1) as usize)].hh().rh() != zero_glue)) {
            q = self.new_param_glue(xspace_skip_code);
        } else {
            {
                if (self.eqtb[crate::ix::U(((26640i32) - 1) as usize)].hh().rh() != zero_glue) {
                    self.main_p = self.eqtb[crate::ix::U(((26640i32) - 1) as usize)].hh().rh();
                } else {
                    // §1220
                    {
                        self.main_p = self.font_glue[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)];
                        if (self.main_p == null) {
                            {
                                self.main_p = self.new_spec(zero_glue);
                                self.main_k = (self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)]).wrapping_add(2i32);
                                { let __ix1510 = (self.main_p).wrapping_add(1i32); let __v1511 = self.font_info[crate::ix::U((self.main_k) as usize)].int(); self.mem[crate::ix::U((__ix1510) as usize)].set_int(__v1511); }
                                { let __ix1512 = (self.main_p).wrapping_add(2i32); let __v1513 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix1512) as usize)].set_int(__v1513); }
                                { let __ix1514 = (self.main_p).wrapping_add(3i32); let __v1515 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U((__ix1514) as usize)].set_int(__v1515); }
                                { let __ix1516 = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(); let __v1517 = self.main_p; self.font_glue[crate::ix::U((__ix1516) as usize)] = __v1517; }
                            }
                        }
                    }
                }
                // §1221
                self.main_p = self.new_spec(self.main_p);
                // §1222
                if (self.cur_list.aux_field.hh().lh() >= 2000i32) {
                    { let __ix1518 = (self.main_p).wrapping_add(1i32); let __v1519 = (self.mem[crate::ix::U(((self.main_p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((extra_space_code).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); self.mem[crate::ix::U((__ix1518) as usize)].set_int(__v1519); }
                }
                { let __ix1520 = (self.main_p).wrapping_add(2i32); let __v1521 = self.xn_over_d(self.mem[crate::ix::U(((self.main_p).wrapping_add(2i32)) as usize)].int(), self.cur_list.aux_field.hh().lh(), 1000i32); self.mem[crate::ix::U((__ix1520) as usize)].set_int(__v1521); }
                { let __ix1522 = (self.main_p).wrapping_add(3i32); let __v1523 = self.xn_over_d(self.mem[crate::ix::U(((self.main_p).wrapping_add(3i32)) as usize)].int(), 1000i32, self.cur_list.aux_field.hh().lh()); self.mem[crate::ix::U((__ix1522) as usize)].set_int(__v1523); }
                // §1221
                q = self.new_glue(self.main_p);
                { let __ix1524 = self.main_p; self.mem[crate::ix::U((__ix1524) as usize)].set_hh_rh(null); }
            }
        }
        { let __ix1525 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1525) as usize)].set_hh_rh(q); }
        self.cur_list.tail_field = q;
    }

    /// @<Declare action...
    // §1225
    pub fn insert_dollar_sign(&mut self) {
        self.back_input();
        self.cur_tok = 804i32;
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1431i32);
        }
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1432i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1433i32;
        }
        self.ins_error();
    }

    /// The ``you_cant`' procedure prints a line saying that the current command
    /// is illegal in the current mode; it identifies these things symbolically.
    /// @<Declare action...
    // §1227
    pub fn you_cant(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(785i32);
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        self.print(1434i32);
        self.print_mode(self.cur_list.mode_field);
    }

    /// @<Declare act...
    // §1228
    pub fn report_illegal_case(&mut self) {
        self.you_cant();
        {
            self.help_ptr = 4i32;
            self.help_line[crate::ix::U((3i32) as usize)] = 1435i32;
            self.help_line[crate::ix::U((2i32) as usize)] = 1436i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1437i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1438i32;
        }
        self.error();
    }

    /// Some operations are allowed only in privileged modes, i.e., in cases
    /// that `mode>0`. The `privileged` function is used to detect violations
    /// of this rule; it issues an error message and returns `false` if the
    /// current `mode` is negative.
    /// @<Declare act...
    // §1229
    pub fn privileged(&mut self) -> bool {
        let mut privileged: bool = false;
        if (self.cur_list.mode_field > 0i32) {
            privileged = true;
        } else {
            {
                self.report_illegal_case();
                privileged = false;
            }
        }
        privileged
    }

    /// We don't want to leave `main_control` immediately when a `stop` command
    /// is sensed, because it may be necessary to invoke an \.{\\output} routine
    /// several times before things really grind to a halt. (The output routine
    /// might even say `\.{\\gdef\\end\{...\}}', to prolong the life of the job.)
    /// Therefore `its_all_over` is `true` only when the current page
    /// and contribution list are empty, and when the last output was not a
    /// ``dead cycle.''
    /// @<Declare act...
    // §1232
    pub fn its_all_over(&mut self) -> bool {
        let mut its_all_over: bool = false;
        'l_exit_f: {
            if self.privileged() {
                {
                    if (((page_head == self.page_tail) && (self.cur_list.head_field == self.cur_list.tail_field)) && (self.dead_cycles == 0i32)) {
                        {
                            its_all_over = true;
                            break 'l_exit_f;
                        }
                    }
                    self.back_input();
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1526 = self.cur_list.tail_field; let __v1527 = self.new_null_box(); self.mem[crate::ix::U((__ix1526) as usize)].set_hh_rh(__v1527); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix1528 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1529 = self.eqtb[crate::ix::U(((29906i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1528) as usize)].set_int(__v1529); }
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1530 = self.cur_list.tail_field; let __v1531 = self.new_glue(fill_glue); self.mem[crate::ix::U((__ix1530) as usize)].set_hh_rh(__v1531); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1532 = self.cur_list.tail_field; let __v1533 = self.new_penalty((1073741824i32).wrapping_neg()); self.mem[crate::ix::U((__ix1532) as usize)].set_hh_rh(__v1533); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    self.build_page();
                }
            }
            its_all_over = false;
        }
        its_all_over
    }

    /// All the work relating to glue creation has been relegated to the
    /// following subroutine. It does not call `build_page`, because it is
    /// used in at least one place where that would be a mistake.
    /// @<Declare action...
    // §1238
    pub fn append_glue(&mut self) {
        let mut s: small_number = 0; // §1238
        s = self.cur_chr;
        match s {
            fil_code => {
                self.cur_val = fil_glue;
            }
            fill_code => {
                self.cur_val = fill_glue;
            }
            ss_code => {
                self.cur_val = ss_glue;
            }
            fil_neg_code => {
                self.cur_val = fil_neg_glue;
            }
            skip_code => {
                self.scan_glue(glue_val);
            }
            mskip_code => {
                self.scan_glue(mu_val);
            }
            _ => {}
        }
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1534 = self.cur_list.tail_field; let __v1535 = self.new_glue(self.cur_val); self.mem[crate::ix::U((__ix1534) as usize)].set_hh_rh(__v1535); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (s >= skip_code) {
            {
                { let __ix1536 = self.cur_val; let __v1537 = (self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix1536) as usize)].set_hh_rh(__v1537); }
                if (s > skip_code) {
                    { let __ix1538 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1538) as usize)].set_hh_b1(mu_glue); }
                }
            }
        }
    }

    /// @<Declare act...
    // §1239
    pub fn append_kern(&mut self) {
        let mut s: quarterword = 0; // §1239
        s = self.cur_chr;
        self.scan_dimen((s == mu_glue), false, false);
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1539 = self.cur_list.tail_field; let __v1540 = self.new_kern(self.cur_val); self.mem[crate::ix::U((__ix1539) as usize)].set_hh_rh(__v1540); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix1541 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1541) as usize)].set_hh_b1(s); }
    }

    /// We have to deal with errors in which braces and such things are not
    /// properly nested. Sometimes the user makes an error of commission by
    /// inserting an extra symbol, but sometimes the user makes an error of omission.
    /// \TeX\ can't always tell one from the other, so it makes a guess and tries
    /// to avoid getting into a loop.
    /// The `off_save` routine is called when the current group code is wrong. It tries
    /// to insert something into the user's input that will help clean off
    /// the top level.
    /// @<Declare act...
    // §1242
    pub fn off_save(&mut self) {
        let mut p: halfword = 0; // §1242
        if (self.cur_group == bottom_level) {
            // §1244
            {
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(932i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1456i32;
                }
                self.error();
            }
        } else {
            // §1242
            {
                self.back_input();
                p = self.get_avail();
                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(711i32);
                }
                // §1243
                match self.cur_group {
                    semi_simple_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(19611i32);
                            self.print_esc(591i32);
                        }
                    }
                    math_shift_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(804i32);
                            self.print_char(36i32);
                        }
                    }
                    math_left_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(19612i32);
                            { let __v1542 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v1542); }
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(3118i32);
                            self.print_esc(1455i32);
                        }
                    }
                    _ => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                            self.print_char(125i32);
                        }
                    }
                }
                // §1242
                self.print(712i32);
                self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                {
                    self.help_ptr = 5i32;
                    self.help_line[crate::ix::U((4i32) as usize)] = 1450i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 1451i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1452i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1453i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1454i32;
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1247
    pub fn extra_right_brace(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1461i32);
        }
        match self.cur_group {
            semi_simple_group => {
                self.print_esc(591i32);
            }
            math_shift_group => {
                self.print_char(36i32);
            }
            math_left_group => {
                self.print_esc(1286i32);
            }
            _ => {}
        }
        {
            self.help_ptr = 5i32;
            self.help_line[crate::ix::U((4i32) as usize)] = 1462i32;
            self.help_line[crate::ix::U((3i32) as usize)] = 1463i32;
            self.help_line[crate::ix::U((2i32) as usize)] = 1464i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1465i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1466i32;
        }
        self.error();
        self.align_state = (self.align_state).wrapping_add(1i32);
    }

    /// Here is where we clear the parameters that are supposed to revert to their
    /// default values after every paragraph and when internal vertical mode is entered.
    /// @<Declare act...
    // §1248
    pub fn normal_paragraph(&mut self) {
        if (self.eqtb[crate::ix::U(((29296i32) - 1) as usize)].int() != 0i32) {
            self.eq_word_define(29296i32, 0i32);
        }
        if (self.eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() != 0i32) {
            self.eq_word_define(29920i32, 0i32);
        }
        if (self.eqtb[crate::ix::U(((29318i32) - 1) as usize)].int() != 1i32) {
            self.eq_word_define(29318i32, 1i32);
        }
        if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() != null) {
            self.eq_define(par_shape_loc, shape_ref, null);
        }
        if (self.eqtb[crate::ix::U(((inter_line_penalties_loc) - 1) as usize)].hh().rh() != null) {
            self.eq_define(inter_line_penalties_loc, shape_ref, null);
        }
    }

    /// The `box_end` procedure does the right thing with `cur_box`, if
    /// `box_context` represents the context as explained above.
    /// @<Declare act...
    // §1253
    pub fn box_end(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1253
        let mut a: small_number = 0; // §1253
        if (box_context < box_flag) {
            // §1254
            {
                if (self.cur_box != null) {
                    {
                        { let __ix1543 = (self.cur_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1543) as usize)].set_int(box_context); }
                        if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                            {
                                if (self.pre_adjust_tail != null) {
                                    {
                                        if (pre_adjust_head != self.pre_adjust_tail) {
                                            {
                                                { let __ix1544 = self.cur_list.tail_field; let __v1545 = self.mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1544) as usize)].set_hh_rh(__v1545); }
                                                self.cur_list.tail_field = self.pre_adjust_tail;
                                            }
                                        }
                                        self.pre_adjust_tail = null;
                                    }
                                }
                                self.append_to_vlist(self.cur_box);
                                if (self.adjust_tail != null) {
                                    {
                                        if (adjust_head != self.adjust_tail) {
                                            {
                                                { let __ix1546 = self.cur_list.tail_field; let __v1547 = self.mem[crate::ix::U((adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1546) as usize)].set_hh_rh(__v1547); }
                                                self.cur_list.tail_field = self.adjust_tail;
                                            }
                                        }
                                        self.adjust_tail = null;
                                    }
                                }
                                if (self.cur_list.mode_field > 0i32) {
                                    self.build_page();
                                }
                            }
                        } else {
                            {
                                if ((self.cur_list.mode_field).wrapping_abs() == hmode) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                } else {
                                    {
                                        p = self.new_noad();
                                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
                                        { let __v1548 = self.cur_box; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v1548); }
                                        self.cur_box = p;
                                    }
                                }
                                { let __ix1549 = self.cur_list.tail_field; let __v1550 = self.cur_box; self.mem[crate::ix::U((__ix1549) as usize)].set_hh_rh(__v1550); }
                                self.cur_list.tail_field = self.cur_box;
                            }
                        }
                    }
                }
            }
        } else {
            // §1253
            if (box_context < ship_out_flag) {
                // §1255
                {
                    if (box_context < global_box_flag) {
                        {
                            self.cur_val = (box_context).wrapping_sub(1073741824i32);
                            a = 0i32;
                        }
                    } else {
                        {
                            self.cur_val = (box_context).wrapping_sub(1073774592i32);
                            a = 4i32;
                        }
                    }
                    if (self.cur_val < 256i32) {
                        if (a >= 4i32) {
                            self.geq_define((box_base).wrapping_add(self.cur_val), box_ref, self.cur_box);
                        } else {
                            self.eq_define((box_base).wrapping_add(self.cur_val), box_ref, self.cur_box);
                        }
                    } else {
                        {
                            self.find_sa_element(box_val, self.cur_val, true);
                            if (a >= 4i32) {
                                self.gsa_def(self.cur_ptr, self.cur_box);
                            } else {
                                self.sa_def(self.cur_ptr, self.cur_box);
                            }
                        }
                    }
                }
            } else {
                // §1253
                if (self.cur_box != null) {
                    if (box_context > ship_out_flag) {
                        // §1256
                        {
                            // §430
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                            }
                            // §1256
                            if (((self.cur_cmd == hskip) && ((self.cur_list.mode_field).wrapping_abs() != vmode)) || ((self.cur_cmd == vskip) && ((self.cur_list.mode_field).wrapping_abs() == vmode))) {
                                {
                                    self.append_glue();
                                    { let __ix1551 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1551) as usize)].set_hh_b1((box_context).wrapping_sub(1073807261i32)); }
                                    { let __ix1552 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1553 = self.cur_box; self.mem[crate::ix::U((__ix1552) as usize)].set_hh_rh(__v1553); }
                                }
                            } else {
                                {
                                    {
                                        self.dg_mark();
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(1479i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 1480i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 1481i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1482i32;
                                    }
                                    self.back_error();
                                    self.flush_node_list(self.cur_box);
                                }
                            }
                        }
                    } else {
                        // §1253
                        self.ship_out(self.cur_box);
                    }
                }
            }
        }
    }

    /// Now that we can see what eventually happens to boxes, we can consider
    /// the first steps in their creation. The `begin_box` routine is called when
    /// `box_context` is a context specification, `cur_chr` specifies the type of
    /// box desired, and `cur_cmd=make_box`.
    /// @<Declare act...
    // §1257
    pub fn begin_box(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1257
        let mut q: halfword = 0; // §1257
        let mut r: halfword = 0; // §1257
        let mut fm: bool = false; // §1257
        let mut tx: halfword = 0; // §1257
        let mut m: quarterword = 0; // §1257
        let mut k: halfword = 0; // §1257
        let mut n: halfword = 0; // §1257
        'l_exit_f: {
            match self.cur_chr {
                box_code => {
                    {
                        self.scan_register_num();
                        if (self.cur_val < 256i32) {
                            self.cur_box = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr == null) {
                                    self.cur_box = null;
                                } else {
                                    self.cur_box = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                        if (self.cur_val < 256i32) {
                            { let __ix1554 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1554) - 1) as usize)].set_hh_rh(null); }
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr != null) {
                                    {
                                        { let __ix1555 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1555) as usize)].set_hh_rh(null); }
                                        { let __ix1556 = (self.cur_ptr).wrapping_add(1i32); let __v1557 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1556) as usize)].set_hh_lh(__v1557); }
                                        self.delete_sa_ref(self.cur_ptr);
                                    }
                                }
                            }
                        }
                    }
                }
                copy_code => {
                    {
                        self.scan_register_num();
                        if (self.cur_val < 256i32) {
                            q = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr == null) {
                                    q = null;
                                } else {
                                    q = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                        self.cur_box = self.copy_node_list(q);
                    }
                }
                last_box_code => {
                    // §1258
                    {
                        self.cur_box = null;
                        if ((self.cur_list.mode_field).wrapping_abs() == mmode) {
                            {
                                self.you_cant();
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1484i32;
                                }
                                self.error();
                            }
                        } else {
                            if ((self.cur_list.mode_field == vmode) && (self.cur_list.head_field == self.cur_list.tail_field)) {
                                {
                                    self.you_cant();
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 1485i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1486i32;
                                    }
                                    self.error();
                                }
                            } else {
                                {
                                    'l_done_f: {
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
                                        if (!(tx >= self.hi_mem_min)) {
                                            if ((self.mem[crate::ix::U((tx) as usize)].hh().b0() == hlist_node) || (self.mem[crate::ix::U((tx) as usize)].hh().b0() == vlist_node)) {
                                                // §1259
                                                {
                                                    q = self.cur_list.head_field;
                                                    p = null;
                                                    loop {
                                                        r = p;
                                                        p = q;
                                                        fm = false;
                                                        if (!(q >= self.hi_mem_min)) {
                                                            if (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node) {
                                                                {
                                                                    {
                                                                        let __for_end_17 = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                                        m = 1i32;
                                                                        while m <= __for_end_17 {
                                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                            m = m.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    if (p == tx) {
                                                                        break 'l_done_f;
                                                                    }
                                                                }
                                                            } else {
                                                                if ((self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == begin_M_code)) {
                                                                    fm = true;
                                                                }
                                                            }
                                                        }
                                                        q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                        if (q == tx) { break; }
                                                    }
                                                    q = self.mem[crate::ix::U((tx) as usize)].hh().rh();
                                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                    self.mem[crate::ix::U((tx) as usize)].set_hh_rh(null);
                                                    if (q == null) {
                                                        if fm {
                                                            self.confusion(1483i32);
                                                        } else {
                                                            self.cur_list.tail_field = p;
                                                        }
                                                    } else {
                                                        if fm {
                                                            {
                                                                self.cur_list.tail_field = r;
                                                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(null);
                                                                self.flush_node_list(p);
                                                            }
                                                        }
                                                    }
                                                    self.cur_box = tx;
                                                    { let __ix1558 = (self.cur_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1558) as usize)].set_int(0i32); }
                                                }
                                            }
                                        }
                                    }
                                    // §1258
                                }
                            }
                        }
                    }
                }
                vsplit_code => {
                    // §1260
                    {
                        self.scan_register_num();
                        n = self.cur_val;
                        if (!self.scan_keyword(1245i32)) {
                            {
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1487i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1488i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1489i32;
                                }
                                self.error();
                            }
                        }
                        self.scan_dimen(false, false, false);
                        self.cur_box = self.vsplit(n, self.cur_val);
                    }
                }
                _ => {
                    // §1261
                    {
                        k = (self.cur_chr).wrapping_sub(4i32);
                        { let __ix1559 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix1559) as usize)].set_int(box_context); }
                        if (k == hmode) {
                            if ((box_context < box_flag) && ((self.cur_list.mode_field).wrapping_abs() == vmode)) {
                                self.scan_spec(adjusted_hbox_group, true);
                            } else {
                                self.scan_spec(hbox_group, true);
                            }
                        } else {
                            {
                                if (k == vmode) {
                                    self.scan_spec(vbox_group, true);
                                } else {
                                    {
                                        self.scan_spec(vtop_group, true);
                                        k = vmode;
                                    }
                                }
                                self.normal_paragraph();
                            }
                        }
                        self.push_nest();
                        self.cur_list.mode_field = (k).wrapping_neg();
                        if (k == vmode) {
                            {
                                { let __v1560 = self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v1560); }
                                if (self.eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh() != null) {
                                    self.begin_token_list(self.eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh(), every_vbox_text);
                                }
                            }
                        } else {
                            {
                                self.cur_list.aux_field.set_hh_lh(1000i32);
                                if (self.eqtb[crate::ix::U(((every_hbox_loc) - 1) as usize)].hh().rh() != null) {
                                    self.begin_token_list(self.eqtb[crate::ix::U(((every_hbox_loc) - 1) as usize)].hh().rh(), every_hbox_text);
                                }
                            }
                        }
                        break 'l_exit_f;
                    }
                }
            }
            // §1257
            self.box_end(box_context);
        }
    }

    /// @<Declare act...
    // §1262
    pub fn scan_box(&mut self, mut box_context: i32) {
        // §430
        loop {
            self.get_x_token();
            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
        }
        // §1262
        if (self.cur_cmd == make_box) {
            self.begin_box(box_context);
        } else {
            if ((box_context >= leader_flag) && ((self.cur_cmd == hrule) || (self.cur_cmd == vrule))) {
                {
                    self.cur_box = self.scan_rule_spec();
                    self.box_end(box_context);
                }
            } else {
                {
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(1490i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 1491i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1492i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1493i32;
                    }
                    self.back_error();
                }
            }
        }
    }

    /// @<Declare action...
    // §1264
    pub fn package(&mut self, mut c: small_number) {
        let mut h: scaled = 0; // §1264
        let mut p: halfword = 0; // §1264
        let mut d: scaled = 0; // §1264
        d = self.eqtb[crate::ix::U(((29910i32) - 1) as usize)].int();
        self.unsave();
        self.save_ptr = (self.save_ptr).wrapping_sub(3i32);
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            self.cur_box = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(2i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int());
        } else {
            {
                self.cur_box = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(2i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), d);
                if (c == vtop_code) {
                    // §1265
                    {
                        h = 0i32;
                        p = self.mem[crate::ix::U(((self.cur_box).wrapping_add(5i32)) as usize)].hh().rh();
                        if (p != null) {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() <= rule_node) {
                                h = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                            }
                        }
                        { let __ix1561 = (self.cur_box).wrapping_add(2i32); let __v1562 = ((self.mem[crate::ix::U(((self.cur_box).wrapping_add(2i32)) as usize)].int()).wrapping_sub(h)).wrapping_add(self.mem[crate::ix::U(((self.cur_box).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1561) as usize)].set_int(__v1562); }
                        { let __ix1563 = (self.cur_box).wrapping_add(3i32); self.mem[crate::ix::U((__ix1563) as usize)].set_int(h); }
                    }
                }
            }
        }
        // §1264
        self.pop_nest();
        self.box_end(self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int());
    }

    /// @<Declare act...
    // §1269
    pub fn norm_min(&mut self, mut h: i32) -> small_number {
        let mut norm_min: small_number = 0;
        if (h <= 0i32) {
            norm_min = 1i32;
        } else {
            if (h >= 63i32) {
                norm_min = 63i32;
            } else {
                norm_min = h;
            }
        }
        norm_min
    }

    /// @<Declare act...
    // §1269
    pub fn new_graf(&mut self, mut indented: bool) {
        self.cur_list.pg_field = 0i32;
        if ((self.cur_list.mode_field == vmode) || (self.cur_list.head_field != self.cur_list.tail_field)) {
            {
                self.prev_tail = self.cur_list.tail_field;
                { let __ix1564 = self.cur_list.tail_field; let __v1565 = self.new_param_glue(par_skip_code); self.mem[crate::ix::U((__ix1564) as usize)].set_hh_rh(__v1565); }
                self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
            }
        }
        self.push_nest();
        self.cur_list.mode_field = hmode;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() > 255i32) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = self.eqtb[crate::ix::U(((29327i32) - 1) as usize)].int();
            }
        }
        { let __v1566 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v1566); }
        self.cur_list.pg_field = ((((self.norm_min(self.eqtb[crate::ix::U(((29328i32) - 1) as usize)].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(self.eqtb[crate::ix::U(((29329i32) - 1) as usize)].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        if indented {
            {
                self.cur_list.tail_field = self.new_null_box();
                { let __ix1567 = self.cur_list.head_field; let __v1568 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1567) as usize)].set_hh_rh(__v1568); }
                { let __ix1569 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1570 = self.eqtb[crate::ix::U(((29903i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1569) as usize)].set_int(__v1570); }
            }
        }
        if (self.eqtb[crate::ix::U(((every_par_loc) - 1) as usize)].hh().rh() != null) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_par_loc) - 1) as usize)].hh().rh(), every_par_text);
        }
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// @<Declare act...
    // §1271
    pub fn indent_in_hmode(&mut self) {
        let mut p: halfword = 0; // §1271
        let mut q: halfword = 0; // §1271
        if (self.cur_chr > 0i32) {
            {
                p = self.new_null_box();
                { let __v1571 = self.eqtb[crate::ix::U(((29903i32) - 1) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v1571); }
                if ((self.cur_list.mode_field).wrapping_abs() == hmode) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    {
                        q = self.new_noad();
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                        p = q;
                    }
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1572 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1572) as usize)].set_hh_rh(p); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            }
        }
    }

    /// @<Declare act...
    // §1273
    pub fn head_for_vmode(&mut self) {
        if (self.cur_list.mode_field < 0i32) {
            if (self.cur_cmd != hrule) {
                self.off_save();
            } else {
                {
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(785i32);
                    }
                    self.print_esc(598i32);
                    self.print(1497i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1498i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1499i32;
                    }
                    self.error();
                }
            }
        } else {
            {
                self.back_input();
                self.cur_tok = self.par_token;
                self.back_input();
                self.cur_input.index_field = inserted;
            }
        }
    }

    /// @<Declare act...
    // §1274
    pub fn end_graf(&mut self) {
        if (self.cur_list.mode_field == hmode) {
            {
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    self.pop_nest();
                } else {
                    self.line_break(false);
                }
                if (self.cur_list.eTeX_aux_field != null) {
                    {
                        self.flush_list(self.cur_list.eTeX_aux_field);
                        self.cur_list.eTeX_aux_field = null;
                    }
                }
                self.normal_paragraph();
                self.error_count = 0i32;
            }
        }
    }

    /// @<Declare act...
    // §1277
    pub fn begin_insert_or_adjust(&mut self) {
        if (self.cur_cmd == vadjust) {
            self.cur_val = 255i32;
        } else {
            {
                self.scan_eight_bit_int();
                if (self.cur_val == 255i32) {
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1500i32);
                        }
                        self.print_esc(339i32);
                        self.print_int(((255i32) as i64));
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1501i32;
                        }
                        self.error();
                        self.cur_val = 0i32;
                    }
                }
            }
        }
        { let __ix1573 = (self.save_ptr).wrapping_add(0i32); let __v1574 = self.cur_val; self.save_stack[crate::ix::U((__ix1573) as usize)].set_int(__v1574); }
        if ((self.cur_cmd == vadjust) && self.scan_keyword(1502i32)) {
            { let __ix1575 = (self.save_ptr).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1575) as usize)].set_int(1i32); }
        } else {
            { let __ix1576 = (self.save_ptr).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1576) as usize)].set_int(0i32); }
        }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(insert_group);
        self.scan_left_brace();
        self.normal_paragraph();
        self.push_nest();
        self.cur_list.mode_field = (1i32).wrapping_neg();
        { let __v1577 = self.eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v1577); }
    }

    /// @<Declare act...
    // §1279
    pub fn make_mark(&mut self) {
        let mut p: halfword = 0; // §1279
        let mut c: halfword = 0; // §1279
        if (self.cur_chr == 0i32) {
            c = 0i32;
        } else {
            {
                self.scan_register_num();
                c = self.cur_val;
            }
        }
        p = self.scan_toks(false, true);
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(c);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(mark_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        { let __v1578 = self.def_ref; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v1578); }
        { let __ix1579 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1579) as usize)].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// @<Declare action...
    // §1281
    pub fn append_penalty(&mut self) {
        self.scan_int();
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1580 = self.cur_list.tail_field; let __v1581 = self.new_penalty(self.cur_val); self.mem[crate::ix::U((__ix1580) as usize)].set_hh_rh(__v1581); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (self.cur_list.mode_field == vmode) {
            self.build_page();
        }
    }

    /// When `delete_last` is called, `cur_chr` is the `type` of node that
    /// will be deleted, if present.
    /// @<Declare action...
    // §1283
    pub fn delete_last(&mut self) {
        let mut p: halfword = 0; // §1283
        let mut q: halfword = 0; // §1283
        let mut r: halfword = 0; // §1283
        let mut fm: bool = false; // §1283
        let mut tx: halfword = 0; // §1283
        let mut m: quarterword = 0; // §1283
        'l_exit_f: {
            if ((self.cur_list.mode_field == vmode) && (self.cur_list.tail_field == self.cur_list.head_field)) {
                // §1284
                {
                    if ((self.cur_chr != glue_node) || (self.last_glue != max_halfword)) {
                        {
                            self.you_cant();
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1485i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1503i32;
                            }
                            if (self.cur_chr == kern_node) {
                                self.help_line[crate::ix::U((0i32) as usize)] = 1504i32;
                            } else {
                                if (self.cur_chr != glue_node) {
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1505i32;
                                }
                            }
                            self.error();
                        }
                    }
                }
            } else {
                // §1283
                {
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
                    if (!(tx >= self.hi_mem_min)) {
                        if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == self.cur_chr) {
                            {
                                q = self.cur_list.head_field;
                                p = null;
                                loop {
                                    r = p;
                                    p = q;
                                    fm = false;
                                    if (!(q >= self.hi_mem_min)) {
                                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node) {
                                            {
                                                {
                                                    let __for_end_12 = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                    m = 1i32;
                                                    while m <= __for_end_12 {
                                                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                        m = m.wrapping_add(1);
                                                    }
                                                }
                                                if (p == tx) {
                                                    break 'l_exit_f;
                                                }
                                            }
                                        } else {
                                            if ((self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == begin_M_code)) {
                                                fm = true;
                                            }
                                        }
                                    }
                                    q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (q == tx) { break; }
                                }
                                q = self.mem[crate::ix::U((tx) as usize)].hh().rh();
                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                self.mem[crate::ix::U((tx) as usize)].set_hh_rh(null);
                                if (q == null) {
                                    if fm {
                                        self.confusion(1483i32);
                                    } else {
                                        self.cur_list.tail_field = p;
                                    }
                                } else {
                                    if fm {
                                        {
                                            self.cur_list.tail_field = r;
                                            self.mem[crate::ix::U((r) as usize)].set_hh_rh(null);
                                            self.flush_node_list(p);
                                        }
                                    }
                                }
                                self.flush_node_list(tx);
                            }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare act...
    // §1288
    pub fn unpackage(&mut self) {
        let mut p: halfword = 0; // §1288
        let mut r: halfword = 0; // §1288
        let mut c: i32 = 0; // §1288
        'l_exit_f: {
            'l_done_f: {
                if (self.cur_chr > copy_code) {
                    // §1863
                    {
                        { let __ix1582 = self.cur_list.tail_field; let __v1583 = self.disc_ptr[crate::ix::U(((self.cur_chr) - 1) as usize)]; self.mem[crate::ix::U((__ix1582) as usize)].set_hh_rh(__v1583); }
                        self.disc_ptr[crate::ix::U(((self.cur_chr) - 1) as usize)] = null;
                        break 'l_done_f;
                    }
                }
                // §1288
                c = self.cur_chr;
                self.scan_register_num();
                if (self.cur_val < 256i32) {
                    p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                } else {
                    {
                        self.find_sa_element(box_val, self.cur_val, false);
                        if (self.cur_ptr == null) {
                            p = null;
                        } else {
                            p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                        }
                    }
                }
                if (p == null) {
                    break 'l_exit_f;
                }
                if ((((self.cur_list.mode_field).wrapping_abs() == mmode) || (((self.cur_list.mode_field).wrapping_abs() == vmode) && (self.mem[crate::ix::U((p) as usize)].hh().b0() != vlist_node))) || (((self.cur_list.mode_field).wrapping_abs() == hmode) && (self.mem[crate::ix::U((p) as usize)].hh().b0() != hlist_node))) {
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1513i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1514i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1515i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1516i32;
                        }
                        self.error();
                        break 'l_exit_f;
                    }
                }
                if (c == copy_code) {
                    { let __ix1584 = self.cur_list.tail_field; let __v1585 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()); self.mem[crate::ix::U((__ix1584) as usize)].set_hh_rh(__v1585); }
                } else {
                    {
                        { let __ix1586 = self.cur_list.tail_field; let __v1587 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1586) as usize)].set_hh_rh(__v1587); }
                        if (self.cur_val < 256i32) {
                            { let __ix1588 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1588) - 1) as usize)].set_hh_rh(null); }
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr != null) {
                                    {
                                        { let __ix1589 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1589) as usize)].set_hh_rh(null); }
                                        { let __ix1590 = (self.cur_ptr).wrapping_add(1i32); let __v1591 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1590) as usize)].set_hh_lh(__v1591); }
                                        self.delete_sa_ref(self.cur_ptr);
                                    }
                                }
                            }
                        }
                        self.free_node(p, box_node_size);
                    }
                }
            }
            while (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh() != null) {
                {
                    r = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    if ((!(r >= self.hi_mem_min)) && (self.mem[crate::ix::U((r) as usize)].hh().b0() == margin_kern_node)) {
                        {
                            { let __ix1592 = self.cur_list.tail_field; let __v1593 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1592) as usize)].set_hh_rh(__v1593); }
                            {
                                { let __ix1594 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh(); let __v1595 = self.avail; self.mem[crate::ix::U((__ix1594) as usize)].set_hh_rh(__v1595); }
                                self.avail = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh();
                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                            }
                            self.free_node(r, margin_kern_node_size);
                        }
                    }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            }
        }
    }

    /// @<Declare act...
    // §1291
    pub fn append_italic_correction(&mut self) {
        let mut p: halfword = 0; // §1291
        let mut f: internal_font_number = 0; // §1291
        'l_exit_f: {
            if (self.cur_list.tail_field != self.cur_list.head_field) {
                {
                    if (self.cur_list.tail_field >= self.hi_mem_min) {
                        p = self.cur_list.tail_field;
                    } else {
                        if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == ligature_node) {
                            p = (self.cur_list.tail_field).wrapping_add(1i32);
                        } else {
                            break 'l_exit_f;
                        }
                    }
                    f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1596 = self.cur_list.tail_field; let __v1597 = self.new_kern(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b1())) as usize)].qqqq().b2()).wrapping_sub(0i32) / 4i32))) as usize)].int()); self.mem[crate::ix::U((__ix1596) as usize)].set_hh_rh(__v1597); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix1598 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1598) as usize)].set_hh_b1(explicit); }
                }
            }
        }
    }

    /// The space factor does not change when we append a discretionary node,
    /// but it starts out as 1000 in the subsidiary lists.
    /// @<Declare act...
    // §1295
    pub fn append_discretionary(&mut self) {
        let mut c: i32 = 0; // §1295
        let mut app_kern: halfword = 0; // §1295
        let mut pre_kern: halfword = 0; // §1295
        let mut p: halfword = 0; // §1295
        let mut c_node: halfword = 0; // §1295
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1599 = self.cur_list.tail_field; let __v1600 = self.new_disc(); self.mem[crate::ix::U((__ix1599) as usize)].set_hh_rh(__v1600); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (self.cur_chr == 1i32) {
            {
                c = self.hyphen_char[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)];
                if (c >= 0i32) {
                    if (c < 256i32) {
                        {
                            pre_kern = self.get_auto_kern(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), non_char, c);
                            app_kern = self.get_auto_kern(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), c, non_char);
                            c_node = self.new_character(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), c);
                            if ((app_kern == null) && (pre_kern == null)) {
                                { let __ix1601 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1601) as usize)].set_hh_lh(c_node); }
                            } else {
                                {
                                    if (pre_kern == null) {
                                        { let __ix1602 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1602) as usize)].set_hh_lh(c_node); }
                                    } else {
                                        {
                                            { let __ix1603 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1603) as usize)].set_hh_lh(pre_kern); }
                                            self.mem[crate::ix::U((pre_kern) as usize)].set_hh_rh(c_node);
                                        }
                                    }
                                    if (app_kern != null) {
                                        self.mem[crate::ix::U((c_node) as usize)].set_hh_rh(app_kern);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            {
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix1604 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[crate::ix::U((__ix1604) as usize)].set_int(0i32); }
                self.new_save_level(disc_group);
                self.scan_left_brace();
                self.push_nest();
                self.cur_list.mode_field = (105i32).wrapping_neg();
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1297
    pub fn build_discretionary(&mut self) {
        let mut p: halfword = 0; // §1297
        let mut q: halfword = 0; // §1297
        let mut n: i32 = 0; // §1297
        'l_exit_f: {
            'l_done_f: {
                self.unsave();
                // §1299
                q = self.cur_list.head_field;
                p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                n = 0i32;
                while (p != null) {
                    {
                        if (!(p >= self.hi_mem_min)) {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() > rule_node) {
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() != kern_node) {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() != ligature_node) {
                                        {
                                            {
                                                self.dg_mark();
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(1523i32);
                                            }
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 1524i32;
                                            }
                                            self.error();
                                            self.begin_diagnostic();
                                            self.print_nl(1525i32);
                                            self.show_box(p);
                                            self.end_diagnostic(true);
                                            self.flush_node_list(p);
                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                                            break 'l_done_f;
                                        }
                                    }
                                }
                            }
                        }
                        q = p;
                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        n = (n).wrapping_add(1i32);
                    }
                }
            }
            // §1297
            p = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
            self.pop_nest();
            match self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int() {
                0 => {
                    { let __ix1605 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1605) as usize)].set_hh_lh(p); }
                }
                1 => {
                    { let __ix1606 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1606) as usize)].set_hh_rh(p); }
                }
                2 => {
                    // §1298
                    {
                        if ((n > 0i32) && ((self.cur_list.mode_field).wrapping_abs() == mmode)) {
                            {
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1517i32);
                                }
                                self.print_esc(360i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1518i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1519i32;
                                }
                                self.flush_node_list(p);
                                n = 0i32;
                                self.error();
                            }
                        } else {
                            { let __ix1607 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1607) as usize)].set_hh_rh(p); }
                        }
                        if (n <= max_quarterword) {
                            { let __ix1608 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1608) as usize)].set_hh_b1(n); }
                        } else {
                            {
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1520i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1521i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1522i32;
                                }
                                self.error();
                            }
                        }
                        if (n > 0i32) {
                            self.cur_list.tail_field = q;
                        }
                        self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                        break 'l_exit_f;
                    }
                }
                _ => {}
            }
            // §1297
            { let __ix1609 = (self.save_ptr).wrapping_sub(1i32); let __v1610 = (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1609) as usize)].set_int(__v1610); }
            self.new_save_level(disc_group);
            self.scan_left_brace();
            self.push_nest();
            self.cur_list.mode_field = (105i32).wrapping_neg();
            self.cur_list.aux_field.set_hh_lh(1000i32);
        }
    }

    /// The positioning of accents is straightforward but tedious. Given an accent
    /// of width `a`, designed for characters of height `x` and slant `s`;
    /// and given a character of width `w`, height `h`, and slant `t`: We will shift
    /// the accent down by `x-h`, and we will insert kern nodes that have the effect of
    /// centering the accent over the character and shifting the accent to the
    /// right by $\delta={1\over2}(w-a)+h\cdot t-x\cdot s$.  If either character is
    /// absent from the font, we will simply use the other, without shifting.
    /// @<Declare act...
    // §1301
    pub fn make_accent(&mut self) {
        let mut s: f64 = 0.0; // §1301
        let mut t: f64 = 0.0; // §1301
        let mut p: halfword = 0; // §1301
        let mut q: halfword = 0; // §1301
        let mut r: halfword = 0; // §1301
        let mut f: internal_font_number = 0; // §1301
        let mut a: scaled = 0; // §1301
        let mut h: scaled = 0; // §1301
        let mut x: scaled = 0; // §1301
        let mut w: scaled = 0; // §1301
        let mut delta: scaled = 0; // §1301
        let mut i: four_quarters = four_quarters::default(); // §1301
        self.scan_char_num();
        f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
        p = self.new_character(f, self.cur_val);
        if (p != null) {
            {
                x = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
                s = (((self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) as f64) / 65536.0f64);
                a = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int();
                self.do_assignments();
                // §1302
                q = null;
                f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
                if (((self.cur_cmd == letter) || (self.cur_cmd == other_char)) || (self.cur_cmd == char_given)) {
                    q = self.new_character(f, self.cur_chr);
                } else {
                    if (self.cur_cmd == char_num) {
                        {
                            self.scan_char_num();
                            q = self.new_character(f, self.cur_val);
                        }
                    } else {
                        self.back_input();
                    }
                }
                // §1301
                if (q != null) {
                    // §1303
                    {
                        t = (((self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) as f64) / 65536.0f64);
                        i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b1())) as usize)].qqqq();
                        w = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int();
                        h = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add(((i.b1()).wrapping_sub(0i32) / 16i32))) as usize)].int();
                        if (h != x) {
                            {
                                p = self.hpack(p, 0i32, additional);
                                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int((x).wrapping_sub(h));
                            }
                        }
                        delta = crate::system::pas_round(((((((w).wrapping_sub(a)) as f64) / 2.0f64) + (((h) as f64) * t)) - (((x) as f64) * s)));
                        r = self.new_kern(delta);
                        self.mem[crate::ix::U((r) as usize)].set_hh_b1(acc_kern);
                        { let __ix1611 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1611) as usize)].set_hh_rh(r); }
                        self.mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                        self.cur_list.tail_field = self.new_kern(((a).wrapping_neg()).wrapping_sub(delta));
                        { let __ix1612 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1612) as usize)].set_hh_b1(acc_kern); }
                        { let __v1613 = self.cur_list.tail_field; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v1613); }
                        p = q;
                    }
                }
                // §1301
                { let __ix1614 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1614) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1305
    pub fn align_error(&mut self) {
        if ((self.align_state).wrapping_abs() > 2i32) {
            // §1306
            {
                {
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1530i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                if (self.cur_tok == 1062i32) {
                    {
                        {
                            self.help_ptr = 6i32;
                            self.help_line[crate::ix::U((5i32) as usize)] = 1531i32;
                            self.help_line[crate::ix::U((4i32) as usize)] = 1532i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 1533i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1534i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1535i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1536i32;
                        }
                    }
                } else {
                    {
                        {
                            self.help_ptr = 5i32;
                            self.help_line[crate::ix::U((4i32) as usize)] = 1531i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 1537i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1534i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1535i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1536i32;
                        }
                    }
                }
                self.error();
            }
        } else {
            // §1305
            {
                self.back_input();
                if (self.align_state < 0i32) {
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(743i32);
                        }
                        self.align_state = (self.align_state).wrapping_add(1i32);
                        self.cur_tok = 379i32;
                    }
                } else {
                    {
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1526i32);
                        }
                        self.align_state = (self.align_state).wrapping_sub(1i32);
                        self.cur_tok = 637i32;
                    }
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1527i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1528i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1529i32;
                }
                self.ins_error();
            }
        }
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1307
    pub fn no_align_error(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1530i32);
        }
        self.print_esc(604i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1538i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1539i32;
        }
        self.error();
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1307
    pub fn omit_error(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1530i32);
        }
        self.print_esc(607i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1540i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1539i32;
        }
        self.error();
    }

    /// An `align_group` code is supposed to remain on the `save_stack`
    /// during an entire alignment, until `fin_align` removes it.
    /// A devious user might force an `endv` command to occur just about anywhere;
    /// we must defeat such hacks.
    /// @<Declare act...
    // §1309
    pub fn do_endv(&mut self) {
        self.base_ptr = self.input_ptr;
        { let __ix1615 = self.base_ptr; let __v1616 = self.cur_input; self.input_stack[crate::ix::U((__ix1615) as usize)] = __v1616; }
        while (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field == null)) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == token_list)) {
            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
        }
        if (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field != null)) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field != token_list)) {
            self.fatal_error(679i32);
        }
        if (self.cur_group == align_group) {
            {
                self.end_graf();
                if self.fin_col() {
                    self.fin_row();
                }
            }
        } else {
            self.off_save();
        }
    }

}
