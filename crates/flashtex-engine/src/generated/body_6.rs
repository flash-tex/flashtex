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
    /// @<Declare act...
    // §1050
    pub fn report_illegal_case(&mut self) {
        self.you_cant();
        {
            self.help_ptr = 4i32;
            self.help_line[(3i32) as usize] = 1021i32;
            self.help_line[(2i32) as usize] = 1022i32;
            self.help_line[(1i32) as usize] = 1023i32;
            self.help_line[(0i32) as usize] = 1024i32;
        }
        self.error();
    }

    /// Some operations are allowed only in privileged modes, i.e., in cases
    /// that `mode>0`. The `privileged` function is used to detect violations
    /// of this rule; it issues an error message and returns `false` if the
    /// current `mode` is negative.
    /// @<Declare act...
    // §1051
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
    // §1054
    pub fn its_all_over(&mut self) -> bool {
        let mut its_all_over: bool = false;
        'l_exit_f: {
            if self.privileged() {
                {
                    if (((4999997i32 == self.page_tail) && (self.cur_list.head_field == self.cur_list.tail_field)) && (self.dead_cycles == 0i32)) {
                        {
                            its_all_over = true;
                            break 'l_exit_f;
                        }
                    }
                    self.back_input();
                    {
                        { let __ix789 = self.cur_list.tail_field; let __v790 = self.new_null_box(); self.mem[(__ix789) as usize].set_hh_rh(__v790); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                    { let __ix791 = (self.cur_list.tail_field).wrapping_add(1i32); let __v792 = self.eqtb[((618733i32) - 1) as usize].int(); self.mem[(__ix791) as usize].set_int(__v792); }
                    {
                        { let __ix793 = self.cur_list.tail_field; let __v794 = self.new_glue(8i32); self.mem[(__ix793) as usize].set_hh_rh(__v794); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                    {
                        { let __ix795 = self.cur_list.tail_field; let __v796 = self.new_penalty((1073741824i32).wrapping_neg()); self.mem[(__ix795) as usize].set_hh_rh(__v796); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
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
    // §1060
    pub fn append_glue(&mut self) {
        let mut s: small_number = 0; // §1060
        s = self.cur_chr;
        match s {
            0 => {
                self.cur_val = 4i32;
            }
            1 => {
                self.cur_val = 8i32;
            }
            2 => {
                self.cur_val = 12i32;
            }
            3 => {
                self.cur_val = 16i32;
            }
            4 => {
                self.scan_glue(2i32);
            }
            5 => {
                self.scan_glue(3i32);
            }
            _ => {}
        }
        {
            { let __ix797 = self.cur_list.tail_field; let __v798 = self.new_glue(self.cur_val); self.mem[(__ix797) as usize].set_hh_rh(__v798); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        if (s >= 4i32) {
            {
                { let __ix799 = self.cur_val; let __v800 = (self.mem[(self.cur_val) as usize].hh().rh()).wrapping_sub(1i32); self.mem[(__ix799) as usize].set_hh_rh(__v800); }
                if (s > 4i32) {
                    { let __ix801 = self.cur_list.tail_field; self.mem[(__ix801) as usize].set_hh_b1(99i32); }
                }
            }
        }
    }

    /// @<Declare act...
    // §1061
    pub fn append_kern(&mut self) {
        let mut s: quarterword = 0; // §1061
        s = self.cur_chr;
        self.scan_dimen((s == 99i32), false, false);
        {
            { let __ix802 = self.cur_list.tail_field; let __v803 = self.new_kern(self.cur_val); self.mem[(__ix802) as usize].set_hh_rh(__v803); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix804 = self.cur_list.tail_field; self.mem[(__ix804) as usize].set_hh_b1(s); }
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
    // §1064
    pub fn off_save(&mut self) {
        let mut p: halfword = 0; // §1064
        if (self.cur_group == 0i32) {
            // §1066
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(777i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 1043i32;
                }
                self.error();
            }
        } else {
            // §1064
            {
                self.back_input();
                p = self.get_avail();
                self.mem[(4999996i32) as usize].set_hh_rh(p);
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(625i32);
                }
                // §1065
                match self.cur_group {
                    14 => {
                        {
                            self.mem[(p) as usize].set_hh_lh(619611i32);
                            self.print_esc(516i32);
                        }
                    }
                    15 => {
                        {
                            self.mem[(p) as usize].set_hh_lh(804i32);
                            self.print_char(36i32);
                        }
                    }
                    16 => {
                        {
                            self.mem[(p) as usize].set_hh_lh(619612i32);
                            { let __v805 = self.get_avail(); self.mem[(p) as usize].set_hh_rh(__v805); }
                            p = self.mem[(p) as usize].hh().rh();
                            self.mem[(p) as usize].set_hh_lh(3118i32);
                            self.print_esc(1042i32);
                        }
                    }
                    _ => {
                        {
                            self.mem[(p) as usize].set_hh_lh(637i32);
                            self.print_char(125i32);
                        }
                    }
                }
                // §1064
                self.print(626i32);
                self.begin_token_list(self.mem[(4999996i32) as usize].hh().rh(), 4i32);
                {
                    self.help_ptr = 5i32;
                    self.help_line[(4i32) as usize] = 1037i32;
                    self.help_line[(3i32) as usize] = 1038i32;
                    self.help_line[(2i32) as usize] = 1039i32;
                    self.help_line[(1i32) as usize] = 1040i32;
                    self.help_line[(0i32) as usize] = 1041i32;
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1069
    pub fn extra_right_brace(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(1048i32);
        }
        match self.cur_group {
            14 => {
                self.print_esc(516i32);
            }
            15 => {
                self.print_char(36i32);
            }
            16 => {
                self.print_esc(877i32);
            }
            _ => {}
        }
        {
            self.help_ptr = 5i32;
            self.help_line[(4i32) as usize] = 1049i32;
            self.help_line[(3i32) as usize] = 1050i32;
            self.help_line[(2i32) as usize] = 1051i32;
            self.help_line[(1i32) as usize] = 1052i32;
            self.help_line[(0i32) as usize] = 1053i32;
        }
        self.error();
        self.align_state = (self.align_state).wrapping_add(1i32);
    }

    /// Here is where we clear the parameters that are supposed to revert to their
    /// default values after every paragraph and when internal vertical mode is entered.
    /// @<Declare act...
    // §1070
    pub fn normal_paragraph(&mut self) {
        if (self.eqtb[((618182i32) - 1) as usize].int() != 0i32) {
            self.eq_word_define(618182i32, 0i32);
        }
        if (self.eqtb[((618747i32) - 1) as usize].int() != 0i32) {
            self.eq_word_define(618747i32, 0i32);
        }
        if (self.eqtb[((618204i32) - 1) as usize].int() != 1i32) {
            self.eq_word_define(618204i32, 1i32);
        }
        if (self.eqtb[((616312i32) - 1) as usize].hh().rh() != 0i32) {
            self.eq_define(616312i32, 118i32, 0i32);
        }
    }

    /// The `box_end` procedure does the right thing with `cur_box`, if
    /// `box_context` represents the context as explained above.
    /// @<Declare act...
    // §1075
    pub fn box_end(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1075
        if (box_context < 1073741824i32) {
            // §1076
            {
                if (self.cur_box != 0i32) {
                    {
                        { let __ix806 = (self.cur_box).wrapping_add(4i32); self.mem[(__ix806) as usize].set_int(box_context); }
                        if ((self.cur_list.mode_field).wrapping_abs() == 1i32) {
                            {
                                self.append_to_vlist(self.cur_box);
                                if (self.adjust_tail != 0i32) {
                                    {
                                        if (4999994i32 != self.adjust_tail) {
                                            {
                                                { let __ix807 = self.cur_list.tail_field; let __v808 = self.mem[(4999994i32) as usize].hh().rh(); self.mem[(__ix807) as usize].set_hh_rh(__v808); }
                                                self.cur_list.tail_field = self.adjust_tail;
                                            }
                                        }
                                        self.adjust_tail = 0i32;
                                    }
                                }
                                if (self.cur_list.mode_field > 0i32) {
                                    self.build_page();
                                }
                            }
                        } else {
                            {
                                if ((self.cur_list.mode_field).wrapping_abs() == 102i32) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                } else {
                                    {
                                        p = self.new_noad();
                                        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
                                        { let __v809 = self.cur_box; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(__v809); }
                                        self.cur_box = p;
                                    }
                                }
                                { let __ix810 = self.cur_list.tail_field; let __v811 = self.cur_box; self.mem[(__ix810) as usize].set_hh_rh(__v811); }
                                self.cur_list.tail_field = self.cur_box;
                            }
                        }
                    }
                }
            }
        } else {
            // §1075
            if (box_context < 1073742336i32) {
                // §1077
                if (box_context < 1073742080i32) {
                    self.eq_define(((1073125246i32).wrapping_neg()).wrapping_add(box_context), 119i32, self.cur_box);
                } else {
                    self.geq_define(((1073125502i32).wrapping_neg()).wrapping_add(box_context), 119i32, self.cur_box);
                }
            } else {
                // §1075
                if (self.cur_box != 0i32) {
                    if (box_context > 1073742336i32) {
                        // §1078
                        {
                            // §404
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                            }
                            // §1078
                            if (((self.cur_cmd == 26i32) && ((self.cur_list.mode_field).wrapping_abs() != 1i32)) || ((self.cur_cmd == 27i32) && ((self.cur_list.mode_field).wrapping_abs() == 1i32))) {
                                {
                                    self.append_glue();
                                    { let __ix812 = self.cur_list.tail_field; self.mem[(__ix812) as usize].set_hh_b1((box_context).wrapping_sub(1073742237i32)); }
                                    { let __ix813 = (self.cur_list.tail_field).wrapping_add(1i32); let __v814 = self.cur_box; self.mem[(__ix813) as usize].set_hh_rh(__v814); }
                                }
                            } else {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(1066i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[(2i32) as usize] = 1067i32;
                                        self.help_line[(1i32) as usize] = 1068i32;
                                        self.help_line[(0i32) as usize] = 1069i32;
                                    }
                                    self.back_error();
                                    self.flush_node_list(self.cur_box);
                                }
                            }
                        }
                    } else {
                        // §1075
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
    // §1079
    pub fn begin_box(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1079
        let mut q: halfword = 0; // §1079
        let mut m: quarterword = 0; // §1079
        let mut k: halfword = 0; // §1079
        let mut n: eight_bits = 0; // §1079
        'l_exit_f: {
            match self.cur_chr {
                0 => {
                    {
                        self.scan_eight_bit_int();
                        self.cur_box = self.eqtb[(((616578i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                        { let __ix815 = (616578i32).wrapping_add(self.cur_val); self.eqtb[((__ix815) - 1) as usize].set_hh_rh(0i32); }
                    }
                }
                1 => {
                    {
                        self.scan_eight_bit_int();
                        self.cur_box = self.copy_node_list(self.eqtb[(((616578i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh());
                    }
                }
                2 => {
                    // §1080
                    {
                        self.cur_box = 0i32;
                        if ((self.cur_list.mode_field).wrapping_abs() == 203i32) {
                            {
                                self.you_cant();
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1070i32;
                                }
                                self.error();
                            }
                        } else {
                            if ((self.cur_list.mode_field == 1i32) && (self.cur_list.head_field == self.cur_list.tail_field)) {
                                {
                                    self.you_cant();
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 1071i32;
                                        self.help_line[(0i32) as usize] = 1072i32;
                                    }
                                    self.error();
                                }
                            } else {
                                {
                                    if (!(self.cur_list.tail_field >= self.hi_mem_min)) {
                                        if ((self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 0i32) || (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 1i32)) {
                                            // §1081
                                            {
                                                'l_done_f: {
                                                    q = self.cur_list.head_field;
                                                    loop {
                                                        p = q;
                                                        if (!(q >= self.hi_mem_min)) {
                                                            if (self.mem[(q) as usize].hh().b0() == 7i32) {
                                                                {
                                                                    {
                                                                        let __for_end_17 = self.mem[(q) as usize].hh().b1();
                                                                        m = 1i32;
                                                                        while m <= __for_end_17 {
                                                                            p = self.mem[(p) as usize].hh().rh();
                                                                            m = m.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    if (p == self.cur_list.tail_field) {
                                                                        break 'l_done_f;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        q = self.mem[(p) as usize].hh().rh();
                                                        if (q == self.cur_list.tail_field) { break; }
                                                    }
                                                    self.cur_box = self.cur_list.tail_field;
                                                    { let __ix816 = (self.cur_box).wrapping_add(4i32); self.mem[(__ix816) as usize].set_int(0i32); }
                                                    self.cur_list.tail_field = p;
                                                    self.mem[(p) as usize].set_hh_rh(0i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                3 => {
                    // §1082
                    {
                        self.scan_eight_bit_int();
                        n = self.cur_val;
                        if (!self.scan_keyword(842i32)) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1073i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1074i32;
                                    self.help_line[(0i32) as usize] = 1075i32;
                                }
                                self.error();
                            }
                        }
                        self.scan_dimen(false, false, false);
                        self.cur_box = self.vsplit(n, self.cur_val);
                    }
                }
                _ => {
                    // §1083
                    {
                        k = (self.cur_chr).wrapping_sub(4i32);
                        { let __ix817 = (self.save_ptr).wrapping_add(0i32); self.save_stack[(__ix817) as usize].set_int(box_context); }
                        if (k == 102i32) {
                            if ((box_context < 1073741824i32) && ((self.cur_list.mode_field).wrapping_abs() == 1i32)) {
                                self.scan_spec(3i32, true);
                            } else {
                                self.scan_spec(2i32, true);
                            }
                        } else {
                            {
                                if (k == 1i32) {
                                    self.scan_spec(4i32, true);
                                } else {
                                    {
                                        self.scan_spec(5i32, true);
                                        k = 1i32;
                                    }
                                }
                                self.normal_paragraph();
                            }
                        }
                        self.push_nest();
                        self.cur_list.mode_field = (k).wrapping_neg();
                        if (k == 1i32) {
                            {
                                self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                                if (self.eqtb[((616318i32) - 1) as usize].hh().rh() != 0i32) {
                                    self.begin_token_list(self.eqtb[((616318i32) - 1) as usize].hh().rh(), 11i32);
                                }
                            }
                        } else {
                            {
                                self.cur_list.aux_field.set_hh_lh(1000i32);
                                if (self.eqtb[((616317i32) - 1) as usize].hh().rh() != 0i32) {
                                    self.begin_token_list(self.eqtb[((616317i32) - 1) as usize].hh().rh(), 10i32);
                                }
                            }
                        }
                        break 'l_exit_f;
                    }
                }
            }
            // §1079
            self.box_end(box_context);
        }
    }

    /// @<Declare act...
    // §1084
    pub fn scan_box(&mut self, mut box_context: i32) {
        // §404
        loop {
            self.get_x_token();
            if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
        }
        // §1084
        if (self.cur_cmd == 20i32) {
            self.begin_box(box_context);
        } else {
            if ((box_context >= 1073742337i32) && ((self.cur_cmd == 36i32) || (self.cur_cmd == 35i32))) {
                {
                    self.cur_box = self.scan_rule_spec();
                    self.box_end(box_context);
                }
            } else {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1076i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 1077i32;
                        self.help_line[(1i32) as usize] = 1078i32;
                        self.help_line[(0i32) as usize] = 1079i32;
                    }
                    self.back_error();
                }
            }
        }
    }

    /// @<Declare action...
    // §1086
    pub fn package(&mut self, mut c: small_number) {
        let mut h: scaled = 0; // §1086
        let mut p: halfword = 0; // §1086
        let mut d: scaled = 0; // §1086
        d = self.eqtb[((618737i32) - 1) as usize].int();
        self.unsave();
        self.save_ptr = (self.save_ptr).wrapping_sub(3i32);
        if (self.cur_list.mode_field == (102i32).wrapping_neg()) {
            self.cur_box = self.hpack(self.mem[(self.cur_list.head_field) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(2i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int());
        } else {
            {
                self.cur_box = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(2i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(), d);
                if (c == 4i32) {
                    // §1087
                    {
                        h = 0i32;
                        p = self.mem[((self.cur_box).wrapping_add(5i32)) as usize].hh().rh();
                        if (p != 0i32) {
                            if (self.mem[(p) as usize].hh().b0() <= 2i32) {
                                h = self.mem[((p).wrapping_add(3i32)) as usize].int();
                            }
                        }
                        { let __ix818 = (self.cur_box).wrapping_add(2i32); let __v819 = ((self.mem[((self.cur_box).wrapping_add(2i32)) as usize].int()).wrapping_sub(h)).wrapping_add(self.mem[((self.cur_box).wrapping_add(3i32)) as usize].int()); self.mem[(__ix818) as usize].set_int(__v819); }
                        { let __ix820 = (self.cur_box).wrapping_add(3i32); self.mem[(__ix820) as usize].set_int(h); }
                    }
                }
            }
        }
        // §1086
        self.pop_nest();
        self.box_end(self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int());
    }

    /// @<Declare act...
    // §1091
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
    // §1091
    pub fn new_graf(&mut self, mut indented: bool) {
        self.cur_list.pg_field = 0i32;
        if ((self.cur_list.mode_field == 1i32) || (self.cur_list.head_field != self.cur_list.tail_field)) {
            {
                { let __ix821 = self.cur_list.tail_field; let __v822 = self.new_param_glue(2i32); self.mem[(__ix821) as usize].set_hh_rh(__v822); }
                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
            }
        }
        self.push_nest();
        self.cur_list.mode_field = 102i32;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (self.eqtb[((618213i32) - 1) as usize].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (self.eqtb[((618213i32) - 1) as usize].int() > 255i32) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = self.eqtb[((618213i32) - 1) as usize].int();
            }
        }
        { let __v823 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v823); }
        self.cur_list.pg_field = ((((self.norm_min(self.eqtb[((618214i32) - 1) as usize].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(self.eqtb[((618215i32) - 1) as usize].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        if indented {
            {
                self.cur_list.tail_field = self.new_null_box();
                { let __ix824 = self.cur_list.head_field; let __v825 = self.cur_list.tail_field; self.mem[(__ix824) as usize].set_hh_rh(__v825); }
                { let __ix826 = (self.cur_list.tail_field).wrapping_add(1i32); let __v827 = self.eqtb[((618730i32) - 1) as usize].int(); self.mem[(__ix826) as usize].set_int(__v827); }
            }
        }
        if (self.eqtb[((616314i32) - 1) as usize].hh().rh() != 0i32) {
            self.begin_token_list(self.eqtb[((616314i32) - 1) as usize].hh().rh(), 7i32);
        }
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// @<Declare act...
    // §1093
    pub fn indent_in_hmode(&mut self) {
        let mut p: halfword = 0; // §1093
        let mut q: halfword = 0; // §1093
        if (self.cur_chr > 0i32) {
            {
                p = self.new_null_box();
                { let __v828 = self.eqtb[((618730i32) - 1) as usize].int(); self.mem[((p).wrapping_add(1i32)) as usize].set_int(__v828); }
                if ((self.cur_list.mode_field).wrapping_abs() == 102i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    {
                        q = self.new_noad();
                        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
                        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(p);
                        p = q;
                    }
                }
                {
                    { let __ix829 = self.cur_list.tail_field; self.mem[(__ix829) as usize].set_hh_rh(p); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
            }
        }
    }

    /// @<Declare act...
    // §1095
    pub fn head_for_vmode(&mut self) {
        if (self.cur_list.mode_field < 0i32) {
            if (self.cur_cmd != 36i32) {
                self.off_save();
            } else {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(685i32);
                    }
                    self.print_esc(521i32);
                    self.print(1082i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 1083i32;
                        self.help_line[(0i32) as usize] = 1084i32;
                    }
                    self.error();
                }
            }
        } else {
            {
                self.back_input();
                self.cur_tok = self.par_token;
                self.back_input();
                self.cur_input.index_field = 4i32;
            }
        }
    }

    /// @<Declare act...
    // §1096
    pub fn end_graf(&mut self) {
        if (self.cur_list.mode_field == 102i32) {
            {
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    self.pop_nest();
                } else {
                    self.line_break(self.eqtb[((618169i32) - 1) as usize].int());
                }
                self.normal_paragraph();
                self.error_count = 0i32;
            }
        }
    }

    /// @<Declare act...
    // §1099
    pub fn begin_insert_or_adjust(&mut self) {
        if (self.cur_cmd == 38i32) {
            self.cur_val = 255i32;
        } else {
            {
                self.scan_eight_bit_int();
                if (self.cur_val == 255i32) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(1085i32);
                        }
                        self.print_esc(330i32);
                        self.print_int(255i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1086i32;
                        }
                        self.error();
                        self.cur_val = 0i32;
                    }
                }
            }
        }
        { let __ix830 = (self.save_ptr).wrapping_add(0i32); let __v831 = self.cur_val; self.save_stack[(__ix830) as usize].set_int(__v831); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        self.new_save_level(11i32);
        self.scan_left_brace();
        self.normal_paragraph();
        self.push_nest();
        self.cur_list.mode_field = (1i32).wrapping_neg();
        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
    }

    /// @<Declare act...
    // §1101
    pub fn make_mark(&mut self) {
        let mut p: halfword = 0; // §1101
        p = self.scan_toks(false, true);
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(4i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        { let __v832 = self.def_ref; self.mem[((p).wrapping_add(1i32)) as usize].set_int(__v832); }
        { let __ix833 = self.cur_list.tail_field; self.mem[(__ix833) as usize].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// @<Declare action...
    // §1103
    pub fn append_penalty(&mut self) {
        self.scan_int();
        {
            { let __ix834 = self.cur_list.tail_field; let __v835 = self.new_penalty(self.cur_val); self.mem[(__ix834) as usize].set_hh_rh(__v835); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        if (self.cur_list.mode_field == 1i32) {
            self.build_page();
        }
    }

    /// When `delete_last` is called, `cur_chr` is the `type` of node that
    /// will be deleted, if present.
    /// @<Declare action...
    // §1105
    pub fn delete_last(&mut self) {
        let mut p: halfword = 0; // §1105
        let mut q: halfword = 0; // §1105
        let mut m: quarterword = 0; // §1105
        'l_exit_f: {
            if ((self.cur_list.mode_field == 1i32) && (self.cur_list.tail_field == self.cur_list.head_field)) {
                // §1106
                {
                    if ((self.cur_chr != 10i32) || (self.last_glue != 268435455i32)) {
                        {
                            self.you_cant();
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1071i32;
                                self.help_line[(0i32) as usize] = 1087i32;
                            }
                            if (self.cur_chr == 11i32) {
                                self.help_line[(0i32) as usize] = 1088i32;
                            } else {
                                if (self.cur_chr != 10i32) {
                                    self.help_line[(0i32) as usize] = 1089i32;
                                }
                            }
                            self.error();
                        }
                    }
                }
            } else {
                // §1105
                {
                    if (!(self.cur_list.tail_field >= self.hi_mem_min)) {
                        if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == self.cur_chr) {
                            {
                                q = self.cur_list.head_field;
                                loop {
                                    p = q;
                                    if (!(q >= self.hi_mem_min)) {
                                        if (self.mem[(q) as usize].hh().b0() == 7i32) {
                                            {
                                                {
                                                    let __for_end_12 = self.mem[(q) as usize].hh().b1();
                                                    m = 1i32;
                                                    while m <= __for_end_12 {
                                                        p = self.mem[(p) as usize].hh().rh();
                                                        m = m.wrapping_add(1);
                                                    }
                                                }
                                                if (p == self.cur_list.tail_field) {
                                                    break 'l_exit_f;
                                                }
                                            }
                                        }
                                    }
                                    q = self.mem[(p) as usize].hh().rh();
                                    if (q == self.cur_list.tail_field) { break; }
                                }
                                self.mem[(p) as usize].set_hh_rh(0i32);
                                self.flush_node_list(self.cur_list.tail_field);
                                self.cur_list.tail_field = p;
                            }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare act...
    // §1110
    pub fn unpackage(&mut self) {
        let mut p: halfword = 0; // §1110
        let mut c: i32 = 0; // §1110
        'l_exit_f: {
            c = self.cur_chr;
            self.scan_eight_bit_int();
            p = self.eqtb[(((616578i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
            if (p == 0i32) {
                break 'l_exit_f;
            }
            if ((((self.cur_list.mode_field).wrapping_abs() == 203i32) || (((self.cur_list.mode_field).wrapping_abs() == 1i32) && (self.mem[(p) as usize].hh().b0() != 1i32))) || (((self.cur_list.mode_field).wrapping_abs() == 102i32) && (self.mem[(p) as usize].hh().b0() != 0i32))) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1097i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 1098i32;
                        self.help_line[(1i32) as usize] = 1099i32;
                        self.help_line[(0i32) as usize] = 1100i32;
                    }
                    self.error();
                    break 'l_exit_f;
                }
            }
            if (c == 1i32) {
                { let __ix836 = self.cur_list.tail_field; let __v837 = self.copy_node_list(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh()); self.mem[(__ix836) as usize].set_hh_rh(__v837); }
            } else {
                {
                    { let __ix838 = self.cur_list.tail_field; let __v839 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(__ix838) as usize].set_hh_rh(__v839); }
                    { let __ix840 = (616578i32).wrapping_add(self.cur_val); self.eqtb[((__ix840) - 1) as usize].set_hh_rh(0i32); }
                    self.free_node(p, 7i32);
                }
            }
            while (self.mem[(self.cur_list.tail_field) as usize].hh().rh() != 0i32) {
                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
            }
        }
    }

    /// @<Declare act...
    // §1113
    pub fn append_italic_correction(&mut self) {
        let mut p: halfword = 0; // §1113
        let mut f: internal_font_number = 0; // §1113
        'l_exit_f: {
            if (self.cur_list.tail_field != self.cur_list.head_field) {
                {
                    if (self.cur_list.tail_field >= self.hi_mem_min) {
                        p = self.cur_list.tail_field;
                    } else {
                        if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 6i32) {
                            p = (self.cur_list.tail_field).wrapping_add(1i32);
                        } else {
                            break 'l_exit_f;
                        }
                    }
                    f = self.mem[(p) as usize].hh().b0();
                    {
                        { let __ix841 = self.cur_list.tail_field; let __v842 = self.new_kern(self.font_info[((self.italic_base[(f) as usize]).wrapping_add(((self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq().b2()).wrapping_sub(0i32) / 4i32))) as usize].int()); self.mem[(__ix841) as usize].set_hh_rh(__v842); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                    { let __ix843 = self.cur_list.tail_field; self.mem[(__ix843) as usize].set_hh_b1(1i32); }
                }
            }
        }
    }

    /// The space factor does not change when we append a discretionary node,
    /// but it starts out as 1000 in the subsidiary lists.
    /// @<Declare act...
    // §1117
    pub fn append_discretionary(&mut self) {
        let mut c: i32 = 0; // §1117
        {
            { let __ix844 = self.cur_list.tail_field; let __v845 = self.new_disc(); self.mem[(__ix844) as usize].set_hh_rh(__v845); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        if (self.cur_chr == 1i32) {
            {
                c = self.hyphen_char[(self.eqtb[((616834i32) - 1) as usize].hh().rh()) as usize];
                if (c >= 0i32) {
                    if (c < 256i32) {
                        { let __ix846 = (self.cur_list.tail_field).wrapping_add(1i32); let __v847 = self.new_character(self.eqtb[((616834i32) - 1) as usize].hh().rh(), c); self.mem[(__ix846) as usize].set_hh_lh(__v847); }
                    }
                }
            }
        } else {
            {
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix848 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[(__ix848) as usize].set_int(0i32); }
                self.new_save_level(10i32);
                self.scan_left_brace();
                self.push_nest();
                self.cur_list.mode_field = (102i32).wrapping_neg();
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1119
    pub fn build_discretionary(&mut self) {
        let mut p: halfword = 0; // §1119
        let mut q: halfword = 0; // §1119
        let mut n: i32 = 0; // §1119
        'l_exit_f: {
            'l_done_f: {
                self.unsave();
                // §1121
                q = self.cur_list.head_field;
                p = self.mem[(q) as usize].hh().rh();
                n = 0i32;
                while (p != 0i32) {
                    {
                        if (!(p >= self.hi_mem_min)) {
                            if (self.mem[(p) as usize].hh().b0() > 2i32) {
                                if (self.mem[(p) as usize].hh().b0() != 11i32) {
                                    if (self.mem[(p) as usize].hh().b0() != 6i32) {
                                        {
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                self.print_nl(262i32);
                                                self.print(1107i32);
                                            }
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[(0i32) as usize] = 1108i32;
                                            }
                                            self.error();
                                            self.begin_diagnostic();
                                            self.print_nl(1109i32);
                                            self.show_box(p);
                                            self.end_diagnostic(true);
                                            self.flush_node_list(p);
                                            self.mem[(q) as usize].set_hh_rh(0i32);
                                            break 'l_done_f;
                                        }
                                    }
                                }
                            }
                        }
                        q = p;
                        p = self.mem[(q) as usize].hh().rh();
                        n = (n).wrapping_add(1i32);
                    }
                }
            }
            // §1119
            p = self.mem[(self.cur_list.head_field) as usize].hh().rh();
            self.pop_nest();
            match self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int() {
                0 => {
                    { let __ix849 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix849) as usize].set_hh_lh(p); }
                }
                1 => {
                    { let __ix850 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix850) as usize].set_hh_rh(p); }
                }
                2 => {
                    // §1120
                    {
                        if ((n > 0i32) && ((self.cur_list.mode_field).wrapping_abs() == 203i32)) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1101i32);
                                }
                                self.print_esc(349i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1102i32;
                                    self.help_line[(0i32) as usize] = 1103i32;
                                }
                                self.flush_node_list(p);
                                n = 0i32;
                                self.error();
                            }
                        } else {
                            { let __ix851 = self.cur_list.tail_field; self.mem[(__ix851) as usize].set_hh_rh(p); }
                        }
                        if (n <= 255i32) {
                            { let __ix852 = self.cur_list.tail_field; self.mem[(__ix852) as usize].set_hh_b1(n); }
                        } else {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1104i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1105i32;
                                    self.help_line[(0i32) as usize] = 1106i32;
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
            // §1119
            { let __ix853 = (self.save_ptr).wrapping_sub(1i32); let __v854 = (self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int()).wrapping_add(1i32); self.save_stack[(__ix853) as usize].set_int(__v854); }
            self.new_save_level(10i32);
            self.scan_left_brace();
            self.push_nest();
            self.cur_list.mode_field = (102i32).wrapping_neg();
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
    // §1123
    pub fn make_accent(&mut self) {
        let mut s: f64 = 0.0; // §1123
        let mut t: f64 = 0.0; // §1123
        let mut p: halfword = 0; // §1123
        let mut q: halfword = 0; // §1123
        let mut r: halfword = 0; // §1123
        let mut f: internal_font_number = 0; // §1123
        let mut a: scaled = 0; // §1123
        let mut h: scaled = 0; // §1123
        let mut x: scaled = 0; // §1123
        let mut w: scaled = 0; // §1123
        let mut delta: scaled = 0; // §1123
        let mut i: four_quarters = four_quarters::default(); // §1123
        self.scan_char_num();
        f = self.eqtb[((616834i32) - 1) as usize].hh().rh();
        p = self.new_character(f, self.cur_val);
        if (p != 0i32) {
            {
                x = self.font_info[((5i32).wrapping_add(self.param_base[(f) as usize])) as usize].int();
                s = (((self.font_info[((1i32).wrapping_add(self.param_base[(f) as usize])) as usize].int()) as f64) / 65536.0f64);
                a = self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int();
                self.do_assignments();
                // §1124
                q = 0i32;
                f = self.eqtb[((616834i32) - 1) as usize].hh().rh();
                if (((self.cur_cmd == 11i32) || (self.cur_cmd == 12i32)) || (self.cur_cmd == 68i32)) {
                    q = self.new_character(f, self.cur_chr);
                } else {
                    if (self.cur_cmd == 16i32) {
                        {
                            self.scan_char_num();
                            q = self.new_character(f, self.cur_val);
                        }
                    } else {
                        self.back_input();
                    }
                }
                // §1123
                if (q != 0i32) {
                    // §1125
                    {
                        t = (((self.font_info[((1i32).wrapping_add(self.param_base[(f) as usize])) as usize].int()) as f64) / 65536.0f64);
                        i = self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(q) as usize].hh().b1())) as usize].qqqq();
                        w = self.font_info[((self.width_base[(f) as usize]).wrapping_add(i.b0())) as usize].int();
                        h = self.font_info[((self.height_base[(f) as usize]).wrapping_add(((i.b1()).wrapping_sub(0i32) / 16i32))) as usize].int();
                        if (h != x) {
                            {
                                p = self.hpack(p, 0i32, 1i32);
                                self.mem[((p).wrapping_add(4i32)) as usize].set_int((x).wrapping_sub(h));
                            }
                        }
                        delta = crate::system::pas_round(((((((w).wrapping_sub(a)) as f64) / 2.0f64) + (((h) as f64) * t)) - (((x) as f64) * s)));
                        r = self.new_kern(delta);
                        self.mem[(r) as usize].set_hh_b1(2i32);
                        { let __ix855 = self.cur_list.tail_field; self.mem[(__ix855) as usize].set_hh_rh(r); }
                        self.mem[(r) as usize].set_hh_rh(p);
                        self.cur_list.tail_field = self.new_kern(((a).wrapping_neg()).wrapping_sub(delta));
                        { let __ix856 = self.cur_list.tail_field; self.mem[(__ix856) as usize].set_hh_b1(2i32); }
                        { let __v857 = self.cur_list.tail_field; self.mem[(p) as usize].set_hh_rh(__v857); }
                        p = q;
                    }
                }
                // §1123
                { let __ix858 = self.cur_list.tail_field; self.mem[(__ix858) as usize].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1127
    pub fn align_error(&mut self) {
        if ((self.align_state).wrapping_abs() > 2i32) {
            // §1128
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1114i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                if (self.cur_tok == 1062i32) {
                    {
                        {
                            self.help_ptr = 6i32;
                            self.help_line[(5i32) as usize] = 1115i32;
                            self.help_line[(4i32) as usize] = 1116i32;
                            self.help_line[(3i32) as usize] = 1117i32;
                            self.help_line[(2i32) as usize] = 1118i32;
                            self.help_line[(1i32) as usize] = 1119i32;
                            self.help_line[(0i32) as usize] = 1120i32;
                        }
                    }
                } else {
                    {
                        {
                            self.help_ptr = 5i32;
                            self.help_line[(4i32) as usize] = 1115i32;
                            self.help_line[(3i32) as usize] = 1121i32;
                            self.help_line[(2i32) as usize] = 1118i32;
                            self.help_line[(1i32) as usize] = 1119i32;
                            self.help_line[(0i32) as usize] = 1120i32;
                        }
                    }
                }
                self.error();
            }
        } else {
            // §1127
            {
                self.back_input();
                if (self.align_state < 0i32) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(657i32);
                        }
                        self.align_state = (self.align_state).wrapping_add(1i32);
                        self.cur_tok = 379i32;
                    }
                } else {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(1110i32);
                        }
                        self.align_state = (self.align_state).wrapping_sub(1i32);
                        self.cur_tok = 637i32;
                    }
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1111i32;
                    self.help_line[(1i32) as usize] = 1112i32;
                    self.help_line[(0i32) as usize] = 1113i32;
                }
                self.ins_error();
            }
        }
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1129
    pub fn no_align_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(1114i32);
        }
        self.print_esc(527i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 1122i32;
            self.help_line[(0i32) as usize] = 1123i32;
        }
        self.error();
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1129
    pub fn omit_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(1114i32);
        }
        self.print_esc(530i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 1124i32;
            self.help_line[(0i32) as usize] = 1123i32;
        }
        self.error();
    }

    /// An `align_group` code is supposed to remain on the `save_stack`
    /// during an entire alignment, until `fin_align` removes it.
    /// A devious user might force an `endv` command to occur just about anywhere;
    /// we must defeat such hacks.
    /// @<Declare act...
    // §1131
    pub fn do_endv(&mut self) {
        self.base_ptr = self.input_ptr;
        { let __ix859 = self.base_ptr; let __v860 = self.cur_input; self.input_stack[(__ix859) as usize] = __v860; }
        while (((self.input_stack[(self.base_ptr) as usize].index_field != 2i32) && (self.input_stack[(self.base_ptr) as usize].loc_field == 0i32)) && (self.input_stack[(self.base_ptr) as usize].state_field == 0i32)) {
            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
        }
        if (((self.input_stack[(self.base_ptr) as usize].index_field != 2i32) || (self.input_stack[(self.base_ptr) as usize].loc_field != 0i32)) || (self.input_stack[(self.base_ptr) as usize].state_field != 0i32)) {
            self.fatal_error(595i32);
        }
        if (self.cur_group == 6i32) {
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

    /// @<Declare act...
    // §1135
    pub fn cs_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(777i32);
        }
        self.print_esc(505i32);
        {
            self.help_ptr = 1i32;
            self.help_line[(0i32) as usize] = 1126i32;
        }
        self.error();
    }

    /// \[48] Building math lists.
    /// The routines that \TeX\ uses to create mlists are similar to those we have
    /// just seen for the generation of hlists and vlists. But it is necessary to
    /// make ``noads'' as well as nodes, so the reader should review the
    /// discussion of math mode data structures before trying to make sense out of
    /// the following program.
    /// Here is a little routine that needs to be done whenever a subformula
    /// is about to be processed. The parameter is a code like `math_group`.
    /// @<Declare act...
    // §1136
    pub fn push_math(&mut self, mut c: group_code) {
        self.push_nest();
        self.cur_list.mode_field = (203i32).wrapping_neg();
        self.cur_list.aux_field.set_int(0i32);
        self.new_save_level(c);
    }

    /// @<Declare act...
    // §1138
    pub fn init_math(&mut self) {
        let mut w: scaled = 0; // §1138
        let mut l: scaled = 0; // §1138
        let mut s: scaled = 0; // §1138
        let mut p: halfword = 0; // §1138
        let mut q: halfword = 0; // §1138
        let mut f: internal_font_number = 0; // §1138
        let mut n: i32 = 0; // §1138
        let mut v: scaled = 0; // §1138
        let mut d: scaled = 0; // §1138
        self.get_token();
        if ((self.cur_cmd == 3i32) && (self.cur_list.mode_field > 0i32)) {
            // §1145
            {
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    {
                        self.pop_nest();
                        w = (1073741823i32).wrapping_neg();
                    }
                } else {
                    {
                        'l_done_f: {
                            self.line_break(self.eqtb[((618170i32) - 1) as usize].int());
                            // §1146
                            v = (self.mem[((self.just_box).wrapping_add(4i32)) as usize].int()).wrapping_add((2i32).wrapping_mul(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[((616834i32) - 1) as usize].hh().rh()) as usize])) as usize].int()));
                            w = (1073741823i32).wrapping_neg();
                            p = self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().rh();
                            while (p != 0i32) {
                                {
                                    // goto labels: reswitch, found, not_found
                                    let mut __goto_1: i32 = 0;
                                    'l_dispatch_1: loop {
                                        if __goto_1 <= 0 {
                                            // §1147
                                            if (p >= self.hi_mem_min) {
                                                {
                                                    f = self.mem[(p) as usize].hh().b0();
                                                    d = self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int();
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                            match self.mem[(p) as usize].hh().b0() {
                                                0 | 1 | 2 => {
                                                    {
                                                        d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                6 => {
                                                    // §652
                                                    {
                                                        { let __v861 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[(4999987i32) as usize] = __v861; }
                                                        { let __v862 = self.mem[(p) as usize].hh().rh(); self.mem[(4999987i32) as usize].set_hh_rh(__v862); }
                                                        p = 4999987i32;
                                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                11 | 9 => {
                                                    // §1147
                                                    d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                }
                                                10 => {
                                                    // §1148
                                                    {
                                                        q = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                                        d = self.mem[((q).wrapping_add(1i32)) as usize].int();
                                                        if (self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b0() == 1i32) {
                                                            {
                                                                if ((self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b1() == self.mem[(q) as usize].hh().b0()) && (self.mem[((q).wrapping_add(2i32)) as usize].int() != 0i32)) {
                                                                    v = 1073741823i32;
                                                                }
                                                            }
                                                        } else {
                                                            if (self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b0() == 2i32) {
                                                                {
                                                                    if ((self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b1() == self.mem[(q) as usize].hh().b1()) && (self.mem[((q).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                                                        v = 1073741823i32;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                                        }
                                                    }
                                                }
                                                8 => {
                                                    // §1361
                                                    d = 0i32;
                                                }
                                                _ => {
                                                    // §1147
                                                    d = 0i32;
                                                }
                                            }
                                            // §1146
                                            if (v < 1073741823i32) {
                                                v = (v).wrapping_add(d);
                                            }
                                            { __goto_1 = 2; continue 'l_dispatch_1; }
                                        }
                                        if __goto_1 <= 1 { // found
                                            if (v < 1073741823i32) {
                                                {
                                                    v = (v).wrapping_add(d);
                                                    w = v;
                                                }
                                            } else {
                                                {
                                                    w = 1073741823i32;
                                                    break 'l_done_f;
                                                }
                                            }
                                        }
                                        if __goto_1 <= 2 { // not_found
                                            p = self.mem[(p) as usize].hh().rh();
                                        }
                                        break 'l_dispatch_1;
                                    }
                                }
                            }
                        }
                    }
                }
                // §1149
                if (self.eqtb[((616312i32) - 1) as usize].hh().rh() == 0i32) {
                    if ((self.eqtb[((618747i32) - 1) as usize].int() != 0i32) && (((self.eqtb[((618204i32) - 1) as usize].int() >= 0i32) && ((self.cur_list.pg_field).wrapping_add(2i32) > self.eqtb[((618204i32) - 1) as usize].int())) || ((self.cur_list.pg_field).wrapping_add(1i32) < (self.eqtb[((618204i32) - 1) as usize].int()).wrapping_neg()))) {
                        {
                            l = (self.eqtb[((618733i32) - 1) as usize].int()).wrapping_sub((self.eqtb[((618747i32) - 1) as usize].int()).wrapping_abs());
                            if (self.eqtb[((618747i32) - 1) as usize].int() > 0i32) {
                                s = self.eqtb[((618747i32) - 1) as usize].int();
                            } else {
                                s = 0i32;
                            }
                        }
                    } else {
                        {
                            l = self.eqtb[((618733i32) - 1) as usize].int();
                            s = 0i32;
                        }
                    }
                } else {
                    {
                        n = self.mem[(self.eqtb[((616312i32) - 1) as usize].hh().rh()) as usize].hh().lh();
                        if ((self.cur_list.pg_field).wrapping_add(2i32) >= n) {
                            p = (self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(n));
                        } else {
                            p = (self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul((self.cur_list.pg_field).wrapping_add(2i32)));
                        }
                        s = self.mem[((p).wrapping_sub(1i32)) as usize].int();
                        l = self.mem[(p) as usize].int();
                    }
                }
                // §1145
                self.push_math(15i32);
                self.cur_list.mode_field = 203i32;
                self.eq_word_define(618207i32, (1i32).wrapping_neg());
                self.eq_word_define(618743i32, w);
                self.eq_word_define(618744i32, l);
                self.eq_word_define(618745i32, s);
                if (self.eqtb[((616316i32) - 1) as usize].hh().rh() != 0i32) {
                    self.begin_token_list(self.eqtb[((616316i32) - 1) as usize].hh().rh(), 9i32);
                }
                if (self.nest_ptr == 1i32) {
                    self.build_page();
                }
            }
        } else {
            // §1138
            {
                self.back_input();
                // §1139
                {
                    self.push_math(15i32);
                    self.eq_word_define(618207i32, (1i32).wrapping_neg());
                    if (self.eqtb[((616315i32) - 1) as usize].hh().rh() != 0i32) {
                        self.begin_token_list(self.eqtb[((616315i32) - 1) as usize].hh().rh(), 8i32);
                    }
                }
            }
        }
    }

    /// When \TeX\ is in display math mode, `cur_group=math_shift_group`,
    /// so it is not necessary for the `start_eq_no` procedure to test for
    /// this condition.
    /// @<Declare act...
    // §1142
    pub fn start_eq_no(&mut self) {
        { let __ix863 = (self.save_ptr).wrapping_add(0i32); let __v864 = self.cur_chr; self.save_stack[(__ix863) as usize].set_int(__v864); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        // §1139
        {
            self.push_math(15i32);
            self.eq_word_define(618207i32, (1i32).wrapping_neg());
            if (self.eqtb[((616315i32) - 1) as usize].hh().rh() != 0i32) {
                self.begin_token_list(self.eqtb[((616315i32) - 1) as usize].hh().rh(), 8i32);
            }
        }
    }

    /// Recall that the `nucleus`, `subscr`, and `supscr` fields in a noad are
    /// broken down into subfields called `math_type` and either `info` or
    /// `(fam,character)`. The job of `scan_math` is to figure out what to place
    /// in one of these principal fields; it looks at the subformula that
    /// comes next in the input, and places an encoding of that subformula
    /// into a given word of `mem`.
    // §1151
    pub fn scan_math(&mut self, mut p: halfword) {
        let mut c: i32 = 0; // §1151
        // goto labels: restart, reswitch, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                loop {
                    // §404
                    self.get_x_token();
                    if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                }
            }
            if __goto_1 <= 1 { // reswitch
                // §1151
                match self.cur_cmd {
                    11 | 12 | 68 => {
                        {
                            c = (self.eqtb[(((617907i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32);
                            if (c == 32768i32) {
                                {
                                    // §1152
                                    {
                                        self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                        self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                        self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                        self.x_token();
                                        self.back_input();
                                    }
                                    // §1151
                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                }
                            }
                        }
                    }
                    16 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.cur_cmd = 68i32;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    17 => {
                        {
                            self.scan_fifteen_bit_int();
                            c = self.cur_val;
                        }
                    }
                    69 => {
                        c = self.cur_chr;
                    }
                    15 => {
                        {
                            self.scan_twenty_seven_bit_int();
                            c = (self.cur_val / 4096i32);
                        }
                    }
                    _ => {
                        // §1153
                        {
                            self.back_input();
                            self.scan_left_brace();
                            { let __ix865 = (self.save_ptr).wrapping_add(0i32); self.save_stack[(__ix865) as usize].set_int(p); }
                            self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                            self.push_math(9i32);
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §1151
                self.mem[(p) as usize].set_hh_rh(1i32);
                self.mem[(p) as usize].set_hh_b1(((c % 256i32)).wrapping_add(0i32));
                if ((c >= 28672i32) && ((self.eqtb[((618207i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((618207i32) - 1) as usize].int() < 16i32))) {
                    { let __v866 = self.eqtb[((618207i32) - 1) as usize].int(); self.mem[(p) as usize].set_hh_b0(__v866); }
                } else {
                    self.mem[(p) as usize].set_hh_b0(((c / 256i32) % 16i32));
                }
            }
            if __goto_1 <= 2 { // exit
            }
            break 'l_dispatch_1;
        }
    }

    /// The `set_math_char` procedure creates a new noad appropriate to a given
    /// math code, and appends it to the current mlist. However, if the math code
    /// is sufficiently large, the `cur_chr` is treated as an active character and
    /// nothing is appended.
    /// @<Declare act...
    // §1155
    pub fn set_math_char(&mut self, mut c: i32) {
        let mut p: halfword = 0; // §1155
        if (c >= 32768i32) {
            // §1152
            {
                self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                self.x_token();
                self.back_input();
            }
        } else {
            // §1155
            {
                p = self.new_noad();
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(1i32);
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b1(((c % 256i32)).wrapping_add(0i32));
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b0(((c / 256i32) % 16i32));
                if (c >= 28672i32) {
                    {
                        if ((self.eqtb[((618207i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((618207i32) - 1) as usize].int() < 16i32)) {
                            { let __v867 = self.eqtb[((618207i32) - 1) as usize].int(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b0(__v867); }
                        }
                        self.mem[(p) as usize].set_hh_b0(16i32);
                    }
                } else {
                    self.mem[(p) as usize].set_hh_b0((16i32).wrapping_add((c / 4096i32)));
                }
                { let __ix868 = self.cur_list.tail_field; self.mem[(__ix868) as usize].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
    }

    /// @<Declare act...
    // §1159
    pub fn math_limit_switch(&mut self) {
        'l_exit_f: {
            if (self.cur_list.head_field != self.cur_list.tail_field) {
                if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 17i32) {
                    {
                        { let __ix869 = self.cur_list.tail_field; let __v870 = self.cur_chr; self.mem[(__ix869) as usize].set_hh_b1(__v870); }
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (self.interaction == 3i32) {
                }
                self.print_nl(262i32);
                self.print(1130i32);
            }
            {
                self.help_ptr = 1i32;
                self.help_line[(0i32) as usize] = 1131i32;
            }
            self.error();
        }
    }

    /// Delimiter fields of noads are filled in by the `scan_delimiter` routine.
    /// The first parameter of this procedure is the `mem` address where the
    /// delimiter is to be placed; the second tells if this delimiter follows
    /// \.{\\radical} or not.
    /// @<Declare act...
    // §1160
    pub fn scan_delimiter(&mut self, mut p: halfword, mut r: bool) {
        if r {
            self.scan_twenty_seven_bit_int();
        } else {
            {
                // §404
                loop {
                    self.get_x_token();
                    if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                }
                // §1160
                match self.cur_cmd {
                    11 | 12 => {
                        self.cur_val = self.eqtb[(((618474i32).wrapping_add(self.cur_chr)) - 1) as usize].int();
                    }
                    15 => {
                        self.scan_twenty_seven_bit_int();
                    }
                    _ => {
                        self.cur_val = (1i32).wrapping_neg();
                    }
                }
            }
        }
        if (self.cur_val < 0i32) {
            // §1161
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1132i32);
                }
                {
                    self.help_ptr = 6i32;
                    self.help_line[(5i32) as usize] = 1133i32;
                    self.help_line[(4i32) as usize] = 1134i32;
                    self.help_line[(3i32) as usize] = 1135i32;
                    self.help_line[(2i32) as usize] = 1136i32;
                    self.help_line[(1i32) as usize] = 1137i32;
                    self.help_line[(0i32) as usize] = 1138i32;
                }
                self.back_error();
                self.cur_val = 0i32;
            }
        }
        // §1160
        { let __v871 = ((self.cur_val / 1048576i32) % 16i32); self.mem[(p) as usize].set_qqqq_b0(__v871); }
        { let __v872 = (((self.cur_val / 4096i32) % 256i32)).wrapping_add(0i32); self.mem[(p) as usize].set_qqqq_b1(__v872); }
        { let __v873 = ((self.cur_val / 256i32) % 16i32); self.mem[(p) as usize].set_qqqq_b2(__v873); }
        { let __v874 = ((self.cur_val % 256i32)).wrapping_add(0i32); self.mem[(p) as usize].set_qqqq_b3(__v874); }
    }

    /// @<Declare act...
    // §1163
    pub fn math_radical(&mut self) {
        {
            { let __ix875 = self.cur_list.tail_field; let __v876 = self.get_node(5i32); self.mem[(__ix875) as usize].set_hh_rh(__v876); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix877 = self.cur_list.tail_field; self.mem[(__ix877) as usize].set_hh_b0(24i32); }
        { let __ix878 = self.cur_list.tail_field; self.mem[(__ix878) as usize].set_hh_b1(0i32); }
        { let __ix879 = (self.cur_list.tail_field).wrapping_add(1i32); let __v880 = self.empty_field; self.mem[(__ix879) as usize].set_hh(__v880); }
        { let __ix881 = (self.cur_list.tail_field).wrapping_add(3i32); let __v882 = self.empty_field; self.mem[(__ix881) as usize].set_hh(__v882); }
        { let __ix883 = (self.cur_list.tail_field).wrapping_add(2i32); let __v884 = self.empty_field; self.mem[(__ix883) as usize].set_hh(__v884); }
        self.scan_delimiter((self.cur_list.tail_field).wrapping_add(4i32), true);
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// @<Declare act...
    // §1165
    pub fn math_ac(&mut self) {
        if (self.cur_cmd == 45i32) {
            // §1166
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1139i32);
                }
                self.print_esc(523i32);
                self.print(1140i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 1141i32;
                    self.help_line[(0i32) as usize] = 1142i32;
                }
                self.error();
            }
        }
        // §1165
        {
            { let __ix885 = self.cur_list.tail_field; let __v886 = self.get_node(5i32); self.mem[(__ix885) as usize].set_hh_rh(__v886); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix887 = self.cur_list.tail_field; self.mem[(__ix887) as usize].set_hh_b0(28i32); }
        { let __ix888 = self.cur_list.tail_field; self.mem[(__ix888) as usize].set_hh_b1(0i32); }
        { let __ix889 = (self.cur_list.tail_field).wrapping_add(1i32); let __v890 = self.empty_field; self.mem[(__ix889) as usize].set_hh(__v890); }
        { let __ix891 = (self.cur_list.tail_field).wrapping_add(3i32); let __v892 = self.empty_field; self.mem[(__ix891) as usize].set_hh(__v892); }
        { let __ix893 = (self.cur_list.tail_field).wrapping_add(2i32); let __v894 = self.empty_field; self.mem[(__ix893) as usize].set_hh(__v894); }
        { let __ix895 = (self.cur_list.tail_field).wrapping_add(4i32); self.mem[(__ix895) as usize].set_hh_rh(1i32); }
        self.scan_fifteen_bit_int();
        { let __ix896 = (self.cur_list.tail_field).wrapping_add(4i32); let __v897 = ((self.cur_val % 256i32)).wrapping_add(0i32); self.mem[(__ix896) as usize].set_hh_b1(__v897); }
        if ((self.cur_val >= 28672i32) && ((self.eqtb[((618207i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((618207i32) - 1) as usize].int() < 16i32))) {
            { let __ix898 = (self.cur_list.tail_field).wrapping_add(4i32); let __v899 = self.eqtb[((618207i32) - 1) as usize].int(); self.mem[(__ix898) as usize].set_hh_b0(__v899); }
        } else {
            { let __ix900 = (self.cur_list.tail_field).wrapping_add(4i32); let __v901 = ((self.cur_val / 256i32) % 16i32); self.mem[(__ix900) as usize].set_hh_b0(__v901); }
        }
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

}
