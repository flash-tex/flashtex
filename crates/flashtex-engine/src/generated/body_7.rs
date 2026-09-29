// GENERATED FILE -- DO NOT EDIT.
// Translated WEB procedures and functions.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(dead_code, unreachable_code, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// The routine that scans the four mlists of a \.{\\mathchoice} is very
    /// much like the routine that builds discretionary nodes.
    /// @<Declare act...
    // §1172
    pub fn append_choices(&mut self) {
        {
            { let __ix902 = self.cur_list.tail_field; let __v903 = self.new_choice(); self.mem[(__ix902) as usize].set_hh_rh(__v903); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        { let __ix904 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[(__ix904) as usize].set_int(0i32); }
        self.push_math(13i32);
        self.scan_left_brace();
    }

    /// At the end of a math formula or subformula, the `fin_mlist` routine is
    /// called upon to return a pointer to the newly completed mlist, and to
    /// pop the nest back to the enclosing semantic level. The parameter to
    /// `fin_mlist`, if not null, points to a `right_noad` that ends the
    /// current mlist; this `right_noad` has not yet been appended.
    /// @<Declare the function called `fin_mlist`
    // §1184
    pub fn fin_mlist(&mut self, mut p: halfword) -> halfword {
        let mut fin_mlist: halfword = 0;
        let mut q: halfword = 0; // §1184
        if (self.cur_list.aux_field.int() != 0i32) {
            // §1185
            {
                { let __ix905 = (self.cur_list.aux_field.int()).wrapping_add(3i32); self.mem[(__ix905) as usize].set_hh_rh(3i32); }
                { let __ix906 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v907 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix906) as usize].set_hh_lh(__v907); }
                if (p == 0i32) {
                    q = self.cur_list.aux_field.int();
                } else {
                    {
                        q = self.mem[((self.cur_list.aux_field.int()).wrapping_add(2i32)) as usize].hh().lh();
                        if (self.mem[(q) as usize].hh().b0() != 30i32) {
                            self.confusion(877i32);
                        }
                        { let __ix908 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v909 = self.mem[(q) as usize].hh().rh(); self.mem[(__ix908) as usize].set_hh_lh(__v909); }
                        { let __v910 = self.cur_list.aux_field.int(); self.mem[(q) as usize].set_hh_rh(__v910); }
                        { let __ix911 = self.cur_list.aux_field.int(); self.mem[(__ix911) as usize].set_hh_rh(p); }
                    }
                }
            }
        } else {
            // §1184
            {
                { let __ix912 = self.cur_list.tail_field; self.mem[(__ix912) as usize].set_hh_rh(p); }
                q = self.mem[(self.cur_list.head_field) as usize].hh().rh();
            }
        }
        self.pop_nest();
        fin_mlist = q;
        fin_mlist
    }

    /// @<Declare act...
    // §1174
    pub fn build_choices(&mut self) {
        let mut p: halfword = 0; // §1174
        'l_exit_f: {
            self.unsave();
            p = self.fin_mlist(0i32);
            match self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int() {
                0 => {
                    { let __ix913 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix913) as usize].set_hh_lh(p); }
                }
                1 => {
                    { let __ix914 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix914) as usize].set_hh_rh(p); }
                }
                2 => {
                    { let __ix915 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix915) as usize].set_hh_lh(p); }
                }
                3 => {
                    {
                        { let __ix916 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix916) as usize].set_hh_rh(p); }
                        self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                        break 'l_exit_f;
                    }
                }
                _ => {}
            }
            { let __ix917 = (self.save_ptr).wrapping_sub(1i32); let __v918 = (self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int()).wrapping_add(1i32); self.save_stack[(__ix917) as usize].set_int(__v918); }
            self.push_math(13i32);
            self.scan_left_brace();
        }
    }

    /// @<Declare act...
    // §1176
    pub fn sub_sup(&mut self) {
        let mut t: small_number = 0; // §1176
        let mut p: halfword = 0; // §1176
        t = 0i32;
        p = 0i32;
        if (self.cur_list.tail_field != self.cur_list.head_field) {
            if ((self.mem[(self.cur_list.tail_field) as usize].hh().b0() >= 16i32) && (self.mem[(self.cur_list.tail_field) as usize].hh().b0() < 30i32)) {
                {
                    p = (((self.cur_list.tail_field).wrapping_add(2i32)).wrapping_add(self.cur_cmd)).wrapping_sub(7i32);
                    t = self.mem[(p) as usize].hh().rh();
                }
            }
        }
        if ((p == 0i32) || (t != 0i32)) {
            // §1177
            {
                {
                    { let __ix919 = self.cur_list.tail_field; let __v920 = self.new_noad(); self.mem[(__ix919) as usize].set_hh_rh(__v920); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                p = (((self.cur_list.tail_field).wrapping_add(2i32)).wrapping_add(self.cur_cmd)).wrapping_sub(7i32);
                if (t != 0i32) {
                    {
                        if (self.cur_cmd == 7i32) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1143i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1144i32;
                                }
                            }
                        } else {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1145i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1146i32;
                                }
                            }
                        }
                        self.error();
                    }
                }
            }
        }
        // §1176
        self.scan_math(p);
    }

    /// @<Declare act...
    // §1181
    pub fn math_fraction(&mut self) {
        let mut c: small_number = 0; // §1181
        c = self.cur_chr;
        if (self.cur_list.aux_field.int() != 0i32) {
            // §1183
            {
                if (c >= 3i32) {
                    {
                        self.scan_delimiter(29988i32, false);
                        self.scan_delimiter(29988i32, false);
                    }
                }
                if ((c % 3i32) == 0i32) {
                    self.scan_dimen(false, false, false);
                }
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1153i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1154i32;
                    self.help_line[(1i32) as usize] = 1155i32;
                    self.help_line[(0i32) as usize] = 1156i32;
                }
                self.error();
            }
        } else {
            // §1181
            {
                { let __v921 = self.get_node(6i32); self.cur_list.aux_field.set_int(__v921); }
                { let __ix922 = self.cur_list.aux_field.int(); self.mem[(__ix922) as usize].set_hh_b0(25i32); }
                { let __ix923 = self.cur_list.aux_field.int(); self.mem[(__ix923) as usize].set_hh_b1(0i32); }
                { let __ix924 = (self.cur_list.aux_field.int()).wrapping_add(2i32); self.mem[(__ix924) as usize].set_hh_rh(3i32); }
                { let __ix925 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v926 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix925) as usize].set_hh_lh(__v926); }
                { let __ix927 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v928 = self.empty_field; self.mem[(__ix927) as usize].set_hh(__v928); }
                { let __ix929 = (self.cur_list.aux_field.int()).wrapping_add(4i32); let __v930 = self.null_delimiter; self.mem[(__ix929) as usize].set_qqqq(__v930); }
                { let __ix931 = (self.cur_list.aux_field.int()).wrapping_add(5i32); let __v932 = self.null_delimiter; self.mem[(__ix931) as usize].set_qqqq(__v932); }
                { let __ix933 = self.cur_list.head_field; self.mem[(__ix933) as usize].set_hh_rh(0i32); }
                self.cur_list.tail_field = self.cur_list.head_field;
                // §1182
                if (c >= 3i32) {
                    {
                        self.scan_delimiter((self.cur_list.aux_field.int()).wrapping_add(4i32), false);
                        self.scan_delimiter((self.cur_list.aux_field.int()).wrapping_add(5i32), false);
                    }
                }
                match (c % 3i32) {
                    0 => {
                        {
                            self.scan_dimen(false, false, false);
                            { let __ix934 = (self.cur_list.aux_field.int()).wrapping_add(1i32); let __v935 = self.cur_val; self.mem[(__ix934) as usize].set_int(__v935); }
                        }
                    }
                    1 => {
                        { let __ix936 = (self.cur_list.aux_field.int()).wrapping_add(1i32); self.mem[(__ix936) as usize].set_int(1073741824i32); }
                    }
                    2 => {
                        { let __ix937 = (self.cur_list.aux_field.int()).wrapping_add(1i32); self.mem[(__ix937) as usize].set_int(0i32); }
                    }
                    _ => {}
                }
            }
        }
    }

    /// @<Declare act...
    // §1191
    pub fn math_left_right(&mut self) {
        let mut t: small_number = 0; // §1191
        let mut p: halfword = 0; // §1191
        t = self.cur_chr;
        if ((t == 31i32) && (self.cur_group != 16i32)) {
            // §1192
            {
                if (self.cur_group == 15i32) {
                    {
                        self.scan_delimiter(29988i32, false);
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(777i32);
                        }
                        self.print_esc(877i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1157i32;
                        }
                        self.error();
                    }
                } else {
                    self.off_save();
                }
            }
        } else {
            // §1191
            {
                p = self.new_noad();
                self.mem[(p) as usize].set_hh_b0(t);
                self.scan_delimiter((p).wrapping_add(1i32), false);
                if (t == 30i32) {
                    {
                        self.push_math(16i32);
                        { let __ix938 = self.cur_list.head_field; self.mem[(__ix938) as usize].set_hh_rh(p); }
                        self.cur_list.tail_field = p;
                    }
                } else {
                    {
                        p = self.fin_mlist(p);
                        self.unsave();
                        {
                            { let __ix939 = self.cur_list.tail_field; let __v940 = self.new_noad(); self.mem[(__ix939) as usize].set_hh_rh(__v940); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                        { let __ix941 = self.cur_list.tail_field; self.mem[(__ix941) as usize].set_hh_b0(23i32); }
                        { let __ix942 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix942) as usize].set_hh_rh(3i32); }
                        { let __ix943 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix943) as usize].set_hh_lh(p); }
                    }
                }
            }
        }
    }

    /// @<Declare act...
    // §1194
    pub fn after_math(&mut self) {
        let mut l: bool = false; // §1194
        let mut danger: bool = false; // §1194
        let mut m: i32 = 0; // §1194
        let mut p: halfword = 0; // §1194
        let mut a: halfword = 0; // §1194
        let mut b: halfword = 0; // §1198
        let mut w: scaled = 0; // §1198
        let mut z: scaled = 0; // §1198
        let mut e: scaled = 0; // §1198
        let mut q: scaled = 0; // §1198
        let mut d: scaled = 0; // §1198
        let mut s: scaled = 0; // §1198
        let mut g1: small_number = 0; // §1198
        let mut g2: small_number = 0; // §1198
        let mut r: halfword = 0; // §1198
        let mut t: halfword = 0; // §1198
        danger = false;
        // §1195
        if (((self.font_params[(self.eqtb[((3937i32) - 1) as usize].hh().rh()) as usize] < 22i32) || (self.font_params[(self.eqtb[((3953i32) - 1) as usize].hh().rh()) as usize] < 22i32)) || (self.font_params[(self.eqtb[((3969i32) - 1) as usize].hh().rh()) as usize] < 22i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1158i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1159i32;
                    self.help_line[(1i32) as usize] = 1160i32;
                    self.help_line[(0i32) as usize] = 1161i32;
                }
                self.error();
                self.flush_math();
                danger = true;
            }
        } else {
            if (((self.font_params[(self.eqtb[((3938i32) - 1) as usize].hh().rh()) as usize] < 13i32) || (self.font_params[(self.eqtb[((3954i32) - 1) as usize].hh().rh()) as usize] < 13i32)) || (self.font_params[(self.eqtb[((3970i32) - 1) as usize].hh().rh()) as usize] < 13i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1162i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 1163i32;
                        self.help_line[(1i32) as usize] = 1164i32;
                        self.help_line[(0i32) as usize] = 1165i32;
                    }
                    self.error();
                    self.flush_math();
                    danger = true;
                }
            }
        }
        // §1194
        m = self.cur_list.mode_field;
        l = false;
        p = self.fin_mlist(0i32);
        if (self.cur_list.mode_field == (m).wrapping_neg()) {
            {
                // §1197
                {
                    self.get_x_token();
                    if (self.cur_cmd != 3i32) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(1166i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1167i32;
                                self.help_line[(0i32) as usize] = 1168i32;
                            }
                            self.back_error();
                        }
                    }
                }
                // §1194
                self.cur_mlist = p;
                self.cur_style = 2i32;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                a = self.hpack(self.mem[(29997i32) as usize].hh().rh(), 0i32, 1i32);
                self.unsave();
                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() == 1i32) {
                    l = true;
                }
                danger = false;
                // §1195
                if (((self.font_params[(self.eqtb[((3937i32) - 1) as usize].hh().rh()) as usize] < 22i32) || (self.font_params[(self.eqtb[((3953i32) - 1) as usize].hh().rh()) as usize] < 22i32)) || (self.font_params[(self.eqtb[((3969i32) - 1) as usize].hh().rh()) as usize] < 22i32)) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(1158i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 1159i32;
                            self.help_line[(1i32) as usize] = 1160i32;
                            self.help_line[(0i32) as usize] = 1161i32;
                        }
                        self.error();
                        self.flush_math();
                        danger = true;
                    }
                } else {
                    if (((self.font_params[(self.eqtb[((3938i32) - 1) as usize].hh().rh()) as usize] < 13i32) || (self.font_params[(self.eqtb[((3954i32) - 1) as usize].hh().rh()) as usize] < 13i32)) || (self.font_params[(self.eqtb[((3970i32) - 1) as usize].hh().rh()) as usize] < 13i32)) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(1162i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1163i32;
                                self.help_line[(1i32) as usize] = 1164i32;
                                self.help_line[(0i32) as usize] = 1165i32;
                            }
                            self.error();
                            self.flush_math();
                            danger = true;
                        }
                    }
                }
                // §1194
                m = self.cur_list.mode_field;
                p = self.fin_mlist(0i32);
            }
        } else {
            a = 0i32;
        }
        if (m < 0i32) {
            // §1196
            {
                {
                    { let __ix944 = self.cur_list.tail_field; let __v945 = self.new_math(self.eqtb[((5831i32) - 1) as usize].int(), 0i32); self.mem[(__ix944) as usize].set_hh_rh(__v945); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                self.cur_mlist = p;
                self.cur_style = 2i32;
                self.mlist_penalties = (self.cur_list.mode_field > 0i32);
                self.mlist_to_hlist();
                { let __ix946 = self.cur_list.tail_field; let __v947 = self.mem[(29997i32) as usize].hh().rh(); self.mem[(__ix946) as usize].set_hh_rh(__v947); }
                while (self.mem[(self.cur_list.tail_field) as usize].hh().rh() != 0i32) {
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                {
                    { let __ix948 = self.cur_list.tail_field; let __v949 = self.new_math(self.eqtb[((5831i32) - 1) as usize].int(), 1i32); self.mem[(__ix948) as usize].set_hh_rh(__v949); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                self.cur_list.aux_field.set_hh_lh(1000i32);
                self.unsave();
            }
        } else {
            // §1194
            {
                if (a == 0i32) {
                    // §1197
                    {
                        self.get_x_token();
                        if (self.cur_cmd != 3i32) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1166i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1167i32;
                                    self.help_line[(0i32) as usize] = 1168i32;
                                }
                                self.back_error();
                            }
                        }
                    }
                }
                // §1199
                self.cur_mlist = p;
                self.cur_style = 0i32;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                p = self.mem[(29997i32) as usize].hh().rh();
                self.adjust_tail = 29995i32;
                b = self.hpack(p, 0i32, 1i32);
                p = self.mem[((b).wrapping_add(5i32)) as usize].hh().rh();
                t = self.adjust_tail;
                self.adjust_tail = 0i32;
                w = self.mem[((b).wrapping_add(1i32)) as usize].int();
                z = self.eqtb[((5844i32) - 1) as usize].int();
                s = self.eqtb[((5845i32) - 1) as usize].int();
                if ((a == 0i32) || danger) {
                    {
                        e = 0i32;
                        q = 0i32;
                    }
                } else {
                    {
                        e = self.mem[((a).wrapping_add(1i32)) as usize].int();
                        q = (e).wrapping_add(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[((3937i32) - 1) as usize].hh().rh()) as usize])) as usize].int());
                    }
                }
                if ((w).wrapping_add(q) > z) {
                    // §1201
                    {
                        if ((e != 0i32) && ((((((w).wrapping_sub(self.total_shrink[(0i32) as usize])).wrapping_add(q) <= z) || (self.total_shrink[(1i32) as usize] != 0i32)) || (self.total_shrink[(2i32) as usize] != 0i32)) || (self.total_shrink[(3i32) as usize] != 0i32))) {
                            {
                                self.free_node(b, 7i32);
                                b = self.hpack(p, (z).wrapping_sub(q), 0i32);
                            }
                        } else {
                            {
                                e = 0i32;
                                if (w > z) {
                                    {
                                        self.free_node(b, 7i32);
                                        b = self.hpack(p, z, 0i32);
                                    }
                                }
                            }
                        }
                        w = self.mem[((b).wrapping_add(1i32)) as usize].int();
                    }
                }
                // §1202
                d = self.half((z).wrapping_sub(w));
                if ((e > 0i32) && (d < (2i32).wrapping_mul(e))) {
                    {
                        d = self.half(((z).wrapping_sub(w)).wrapping_sub(e));
                        if (p != 0i32) {
                            if (!(p >= self.hi_mem_min)) {
                                if (self.mem[(p) as usize].hh().b0() == 10i32) {
                                    d = 0i32;
                                }
                            }
                        }
                    }
                }
                // §1203
                {
                    { let __ix950 = self.cur_list.tail_field; let __v951 = self.new_penalty(self.eqtb[((5274i32) - 1) as usize].int()); self.mem[(__ix950) as usize].set_hh_rh(__v951); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                if (((d).wrapping_add(s) <= self.eqtb[((5843i32) - 1) as usize].int()) || l) {
                    {
                        g1 = 3i32;
                        g2 = 4i32;
                    }
                } else {
                    {
                        g1 = 5i32;
                        g2 = 6i32;
                    }
                }
                if (l && (e == 0i32)) {
                    {
                        self.mem[((a).wrapping_add(4i32)) as usize].set_int(s);
                        self.append_to_vlist(a);
                        {
                            { let __ix952 = self.cur_list.tail_field; let __v953 = self.new_penalty(10000i32); self.mem[(__ix952) as usize].set_hh_rh(__v953); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                } else {
                    {
                        { let __ix954 = self.cur_list.tail_field; let __v955 = self.new_param_glue(g1); self.mem[(__ix954) as usize].set_hh_rh(__v955); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
                // §1204
                if (e != 0i32) {
                    {
                        r = self.new_kern((((z).wrapping_sub(w)).wrapping_sub(e)).wrapping_sub(d));
                        if l {
                            {
                                self.mem[(a) as usize].set_hh_rh(r);
                                self.mem[(r) as usize].set_hh_rh(b);
                                b = a;
                                d = 0i32;
                            }
                        } else {
                            {
                                self.mem[(b) as usize].set_hh_rh(r);
                                self.mem[(r) as usize].set_hh_rh(a);
                            }
                        }
                        b = self.hpack(b, 0i32, 1i32);
                    }
                }
                self.mem[((b).wrapping_add(4i32)) as usize].set_int((s).wrapping_add(d));
                self.append_to_vlist(b);
                // §1205
                if (((a != 0i32) && (e == 0i32)) && (!l)) {
                    {
                        {
                            { let __ix956 = self.cur_list.tail_field; let __v957 = self.new_penalty(10000i32); self.mem[(__ix956) as usize].set_hh_rh(__v957); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                        { let __v958 = ((s).wrapping_add(z)).wrapping_sub(self.mem[((a).wrapping_add(1i32)) as usize].int()); self.mem[((a).wrapping_add(4i32)) as usize].set_int(__v958); }
                        self.append_to_vlist(a);
                        g2 = 0i32;
                    }
                }
                if (t != 29995i32) {
                    {
                        { let __ix959 = self.cur_list.tail_field; let __v960 = self.mem[(29995i32) as usize].hh().rh(); self.mem[(__ix959) as usize].set_hh_rh(__v960); }
                        self.cur_list.tail_field = t;
                    }
                }
                {
                    { let __ix961 = self.cur_list.tail_field; let __v962 = self.new_penalty(self.eqtb[((5275i32) - 1) as usize].int()); self.mem[(__ix961) as usize].set_hh_rh(__v962); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                if (g2 > 0i32) {
                    {
                        { let __ix963 = self.cur_list.tail_field; let __v964 = self.new_param_glue(g2); self.mem[(__ix963) as usize].set_hh_rh(__v964); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
                // §1199
                self.resume_after_display();
            }
        }
    }

    /// @<Declare act...
    // §1200
    pub fn resume_after_display(&mut self) {
        if (self.cur_group != 15i32) {
            self.confusion(1169i32);
        }
        self.unsave();
        self.cur_list.pg_field = (self.cur_list.pg_field).wrapping_add(3i32);
        self.push_nest();
        self.cur_list.mode_field = 102i32;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (self.eqtb[((5313i32) - 1) as usize].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (self.eqtb[((5313i32) - 1) as usize].int() > 255i32) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = self.eqtb[((5313i32) - 1) as usize].int();
            }
        }
        { let __v965 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v965); }
        self.cur_list.pg_field = ((((self.norm_min(self.eqtb[((5314i32) - 1) as usize].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(self.eqtb[((5315i32) - 1) as usize].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        // §443
        {
            self.get_x_token();
            if (self.cur_cmd != 10i32) {
                self.back_input();
            }
        }
        // §1200
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// When a control sequence is to be defined, by \.{\\def} or \.{\\let} or
    /// something similar, the `get_r_token` routine will substitute a special
    /// control sequence for a token that is not redefinable.
    /// @<Declare subprocedures for `prefixed_command`
    // §1215
    pub fn get_r_token(&mut self) {
        'l_restart_b: loop {
            loop {
                self.get_token();
                if (self.cur_tok != 2592i32) { break; }
            }
            if ((self.cur_cs == 0i32) || (self.cur_cs > 2614i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1184i32);
                    }
                    {
                        self.help_ptr = 5i32;
                        self.help_line[(4i32) as usize] = 1185i32;
                        self.help_line[(3i32) as usize] = 1186i32;
                        self.help_line[(2i32) as usize] = 1187i32;
                        self.help_line[(1i32) as usize] = 1188i32;
                        self.help_line[(0i32) as usize] = 1189i32;
                    }
                    if (self.cur_cs == 0i32) {
                        self.back_input();
                    }
                    self.cur_tok = 6709i32;
                    self.ins_error();
                    continue 'l_restart_b;
                }
            }
            break 'l_restart_b;
        }
    }

    /// When a glue register or parameter becomes zero, it will always point to
    /// `zero_glue` because of the following procedure. (Exception: The tabskip
    /// glue isn't trapped while preambles are being scanned.)
    /// @<Declare subprocedures for `prefixed_command`
    // §1229
    pub fn trap_zero_glue(&mut self) {
        if (((self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int() == 0i32) && (self.mem[((self.cur_val).wrapping_add(2i32)) as usize].int() == 0i32)) && (self.mem[((self.cur_val).wrapping_add(3i32)) as usize].int() == 0i32)) {
            {
                { let __v966 = (self.mem[(0i32) as usize].hh().rh()).wrapping_add(1i32); self.mem[(0i32) as usize].set_hh_rh(__v966); }
                self.delete_glue_ref(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// We use the fact that `register<advance<multiply<divide`.
    /// @<Declare subprocedures for `prefixed_command`
    // §1236
    pub fn do_register_command(&mut self, mut a: small_number) {
        let mut l: halfword = 0; // §1236
        let mut q: halfword = 0; // §1236
        let mut r: halfword = 0; // §1236
        let mut s: halfword = 0; // §1236
        let mut p: i32 = 0; // §1236
        'l_exit_f: {
            'l_found_f: {
                q = self.cur_cmd;
                // §1237
                {
                    if (q != 89i32) {
                        {
                            self.get_x_token();
                            if ((self.cur_cmd >= 73i32) && (self.cur_cmd <= 76i32)) {
                                {
                                    l = self.cur_chr;
                                    p = (self.cur_cmd).wrapping_sub(73i32);
                                    break 'l_found_f;
                                }
                            }
                            if (self.cur_cmd != 89i32) {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(685i32);
                                    }
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print(686i32);
                                    self.print_cmd_chr(q, 0i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 1210i32;
                                    }
                                    self.error();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    p = self.cur_chr;
                    self.scan_eight_bit_int();
                    match p {
                        0 => {
                            l = (self.cur_val).wrapping_add(5318i32);
                        }
                        1 => {
                            l = (self.cur_val).wrapping_add(5851i32);
                        }
                        2 => {
                            l = (self.cur_val).wrapping_add(2900i32);
                        }
                        3 => {
                            l = (self.cur_val).wrapping_add(3156i32);
                        }
                        _ => {}
                    }
                }
            }
            // §1236
            if (q == 89i32) {
                self.scan_optional_equals();
            } else {
                if self.scan_keyword(1206i32) {
                }
            }
            self.arith_error = false;
            if (q < 91i32) {
                // §1238
                if (p < 2i32) {
                    {
                        if (p == 0i32) {
                            self.scan_int();
                        } else {
                            self.scan_dimen(false, false, false);
                        }
                        if (q == 90i32) {
                            self.cur_val = (self.cur_val).wrapping_add(self.eqtb[((l) - 1) as usize].int());
                        }
                    }
                } else {
                    {
                        self.scan_glue(p);
                        if (q == 90i32) {
                            // §1239
                            {
                                q = self.new_spec(self.cur_val);
                                r = self.eqtb[((l) - 1) as usize].hh().rh();
                                self.delete_glue_ref(self.cur_val);
                                { let __v967 = (self.mem[((q).wrapping_add(1i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v967); }
                                if (self.mem[((q).wrapping_add(2i32)) as usize].int() == 0i32) {
                                    self.mem[(q) as usize].set_hh_b0(0i32);
                                }
                                if (self.mem[(q) as usize].hh().b0() == self.mem[(r) as usize].hh().b0()) {
                                    { let __v968 = (self.mem[((q).wrapping_add(2i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v968); }
                                } else {
                                    if ((self.mem[(q) as usize].hh().b0() < self.mem[(r) as usize].hh().b0()) && (self.mem[((r).wrapping_add(2i32)) as usize].int() != 0i32)) {
                                        {
                                            { let __v969 = self.mem[((r).wrapping_add(2i32)) as usize].int(); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v969); }
                                            { let __v970 = self.mem[(r) as usize].hh().b0(); self.mem[(q) as usize].set_hh_b0(__v970); }
                                        }
                                    }
                                }
                                if (self.mem[((q).wrapping_add(3i32)) as usize].int() == 0i32) {
                                    self.mem[(q) as usize].set_hh_b1(0i32);
                                }
                                if (self.mem[(q) as usize].hh().b1() == self.mem[(r) as usize].hh().b1()) {
                                    { let __v971 = (self.mem[((q).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v971); }
                                } else {
                                    if ((self.mem[(q) as usize].hh().b1() < self.mem[(r) as usize].hh().b1()) && (self.mem[((r).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                        {
                                            { let __v972 = self.mem[((r).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v972); }
                                            { let __v973 = self.mem[(r) as usize].hh().b1(); self.mem[(q) as usize].set_hh_b1(__v973); }
                                        }
                                    }
                                }
                                self.cur_val = q;
                            }
                        }
                    }
                }
            } else {
                // §1240
                {
                    self.scan_int();
                    if (p < 2i32) {
                        if (q == 91i32) {
                            if (p == 0i32) {
                                self.cur_val = self.mult_and_add(self.eqtb[((l) - 1) as usize].int(), self.cur_val, 0i32, 2147483647i32);
                            } else {
                                self.cur_val = self.mult_and_add(self.eqtb[((l) - 1) as usize].int(), self.cur_val, 0i32, 1073741823i32);
                            }
                        } else {
                            self.cur_val = self.x_over_n(self.eqtb[((l) - 1) as usize].int(), self.cur_val);
                        }
                    } else {
                        {
                            s = self.eqtb[((l) - 1) as usize].hh().rh();
                            r = self.new_spec(s);
                            if (q == 91i32) {
                                {
                                    { let __v974 = self.mult_and_add(self.mem[((s).wrapping_add(1i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v974); }
                                    { let __v975 = self.mult_and_add(self.mem[((s).wrapping_add(2i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v975); }
                                    { let __v976 = self.mult_and_add(self.mem[((s).wrapping_add(3i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v976); }
                                }
                            } else {
                                {
                                    { let __v977 = self.x_over_n(self.mem[((s).wrapping_add(1i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v977); }
                                    { let __v978 = self.x_over_n(self.mem[((s).wrapping_add(2i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v978); }
                                    { let __v979 = self.x_over_n(self.mem[((s).wrapping_add(3i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v979); }
                                }
                            }
                            self.cur_val = r;
                        }
                    }
                }
            }
            // §1236
            if self.arith_error {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1207i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 1208i32;
                        self.help_line[(0i32) as usize] = 1209i32;
                    }
                    if (p >= 2i32) {
                        self.delete_glue_ref(self.cur_val);
                    }
                    self.error();
                    break 'l_exit_f;
                }
            }
            if (p < 2i32) {
                if (a >= 4i32) {
                    self.geq_word_define(l, self.cur_val);
                } else {
                    self.eq_word_define(l, self.cur_val);
                }
            } else {
                {
                    self.trap_zero_glue();
                    if (a >= 4i32) {
                        self.geq_define(l, 117i32, self.cur_val);
                    } else {
                        self.eq_define(l, 117i32, self.cur_val);
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1243
    pub fn alter_aux(&mut self) {
        let mut c: halfword = 0; // §1243
        if (self.cur_chr != (self.cur_list.mode_field).wrapping_abs()) {
            self.report_illegal_case();
        } else {
            {
                c = self.cur_chr;
                self.scan_optional_equals();
                if (c == 1i32) {
                    {
                        self.scan_dimen(false, false, false);
                        { let __v980 = self.cur_val; self.cur_list.aux_field.set_int(__v980); }
                    }
                } else {
                    {
                        self.scan_int();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32767i32)) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1213i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1214i32;
                                }
                                self.int_error(self.cur_val);
                            }
                        } else {
                            { let __v981 = self.cur_val; self.cur_list.aux_field.set_hh_lh(__v981); }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1244
    pub fn alter_prev_graf(&mut self) {
        let mut p: i32 = 0; // §1244
        { let __ix982 = self.nest_ptr; let __v983 = self.cur_list; self.nest[(__ix982) as usize] = __v983; }
        p = self.nest_ptr;
        while ((self.nest[(p) as usize].mode_field).wrapping_abs() != 1i32) {
            p = (p).wrapping_sub(1i32);
        }
        self.scan_optional_equals();
        self.scan_int();
        if (self.cur_val < 0i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(955i32);
                }
                self.print_esc(532i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 1215i32;
                }
                self.int_error(self.cur_val);
            }
        } else {
            {
                self.nest[(p) as usize].pg_field = self.cur_val;
                self.cur_list = self.nest[(self.nest_ptr) as usize];
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1245
    pub fn alter_page_so_far(&mut self) {
        let mut c: i32 = 0; // §1245
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        { let __v984 = self.cur_val; self.page_so_far[(c) as usize] = __v984; }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1246
    pub fn alter_integer(&mut self) {
        let mut c: i32 = 0; // §1246
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_int();
        if (c == 0i32) {
            self.dead_cycles = self.cur_val;
        } else {
            self.insert_penalties = self.cur_val;
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1247
    pub fn alter_box_dimen(&mut self) {
        let mut c: small_number = 0; // §1247
        let mut b: eight_bits = 0; // §1247
        c = self.cur_chr;
        self.scan_eight_bit_int();
        b = self.cur_val;
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        if (self.eqtb[(((3678i32).wrapping_add(b)) - 1) as usize].hh().rh() != 0i32) {
            { let __ix985 = (self.eqtb[(((3678i32).wrapping_add(b)) - 1) as usize].hh().rh()).wrapping_add(c); let __v986 = self.cur_val; self.mem[(__ix985) as usize].set_int(__v986); }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1257
    pub fn new_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §1257
        let mut s: scaled = 0; // §1257
        let mut f: internal_font_number = 0; // §1257
        let mut t: str_number = 0; // §1257
        let mut old_setting: i32 = 0; // §1257
        let mut flushable_string: str_number = 0; // §1257
        'l_common_ending_f: {
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            self.get_r_token();
            u = self.cur_cs;
            if (u >= 514i32) {
                t = self.hash[((u) - 514) as usize].rh();
            } else {
                if (u >= 257i32) {
                    if (u == 513i32) {
                        t = 1219i32;
                    } else {
                        t = (u).wrapping_sub(257i32);
                    }
                } else {
                    {
                        old_setting = self.selector;
                        self.selector = 21i32;
                        self.print(1219i32);
                        self.print((u).wrapping_sub(1i32));
                        self.selector = old_setting;
                        {
                            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                            }
                        }
                        t = self.make_string();
                    }
                }
            }
            if (a >= 4i32) {
                self.geq_define(u, 87i32, 0i32);
            } else {
                self.eq_define(u, 87i32, 0i32);
            }
            self.scan_optional_equals();
            self.scan_file_name();
            // §1258
            self.name_in_progress = true;
            if self.scan_keyword(1220i32) {
                // §1259
                {
                    self.scan_dimen(false, false, false);
                    s = self.cur_val;
                    if ((s <= 0i32) || (s >= 134217728i32)) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(1222i32);
                            }
                            self.print_scaled(s);
                            self.print(1223i32);
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1224i32;
                                self.help_line[(0i32) as usize] = 1225i32;
                            }
                            self.error();
                            s = (10i32).wrapping_mul(65536i32);
                        }
                    }
                }
            } else {
                // §1258
                if self.scan_keyword(1221i32) {
                    {
                        self.scan_int();
                        s = (self.cur_val).wrapping_neg();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32768i32)) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(552i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 553i32;
                                }
                                self.int_error(self.cur_val);
                                s = (1000i32).wrapping_neg();
                            }
                        }
                    }
                } else {
                    s = (1000i32).wrapping_neg();
                }
            }
            self.name_in_progress = false;
            // §1260
            flushable_string = (self.str_ptr).wrapping_sub(1i32);
            {
                let __for_end_3 = self.font_ptr;
                f = 1i32;
                while f <= __for_end_3 {
                    if (self.str_eq_str(self.font_name[(f) as usize], self.cur_name) && self.str_eq_str(self.font_area[(f) as usize], self.cur_area)) {
                        {
                            if (self.cur_name == flushable_string) {
                                {
                                    {
                                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                        self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                    }
                                    self.cur_name = self.font_name[(f) as usize];
                                }
                            }
                            if (s > 0i32) {
                                {
                                    if (s == self.font_size[(f) as usize]) {
                                        break 'l_common_ending_f;
                                    }
                                }
                            } else {
                                if (self.font_size[(f) as usize] == self.xn_over_d(self.font_dsize[(f) as usize], (s).wrapping_neg(), 1000i32)) {
                                    break 'l_common_ending_f;
                                }
                            }
                        }
                    }
                    f = f.wrapping_add(1);
                }
            }
            // §1257
            f = self.read_font_info(u, self.cur_name, self.cur_area, s);
        }
        self.eqtb[((u) - 1) as usize].set_hh_rh(f);
        { let __v987 = self.eqtb[((u) - 1) as usize]; self.eqtb[(((2624i32).wrapping_add(f)) - 1) as usize] = __v987; }
        self.hash[(((2624i32).wrapping_add(f)) - 514) as usize].set_rh(t);
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1265
    pub fn new_interaction(&mut self) {
        self.print_ln();
        self.interaction = self.cur_chr;
        // §75
        if (self.interaction == 0i32) {
            self.selector = 16i32;
        } else {
            self.selector = 17i32;
        }
        // §1265
        if self.log_opened {
            self.selector = (self.selector).wrapping_add(2i32);
        }
    }

    /// If the user says, e.g., `\.{\\global\\global}', the redundancy is
    /// silently accepted.
    /// @<Declare act...
    // §1211
    pub fn prefixed_command(&mut self) {
        let mut a: small_number = 0; // §1211
        let mut f: internal_font_number = 0; // §1211
        let mut j: halfword = 0; // §1211
        let mut k: font_index = 0; // §1211
        let mut p: halfword = 0; // §1211
        let mut q: halfword = 0; // §1211
        let mut n: i32 = 0; // §1211
        let mut e: bool = false; // §1211
        'l_exit_f: {
            'l_done_f: {
                a = 0i32;
                while (self.cur_cmd == 93i32) {
                    {
                        if (!((((a / self.cur_chr)) % 2) != 0)) {
                            a = (a).wrapping_add(self.cur_chr);
                        }
                        // §404
                        loop {
                            self.get_x_token();
                            if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                        }
                        // §1211
                        if (self.cur_cmd <= 70i32) {
                            // §1212
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1179i32);
                                }
                                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                self.print_char(39i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1180i32;
                                }
                                self.back_error();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                // §1213
                if ((self.cur_cmd != 97i32) && ((a % 4i32) != 0i32)) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(685i32);
                        }
                        self.print_esc(1171i32);
                        self.print(1181i32);
                        self.print_esc(1172i32);
                        self.print(1182i32);
                        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                        self.print_char(39i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1183i32;
                        }
                        self.error();
                    }
                }
                // §1214
                if (self.eqtb[((5306i32) - 1) as usize].int() != 0i32) {
                    if (self.eqtb[((5306i32) - 1) as usize].int() < 0i32) {
                        {
                            if (a >= 4i32) {
                                a = (a).wrapping_sub(4i32);
                            }
                        }
                    } else {
                        {
                            if (!(a >= 4i32)) {
                                a = (a).wrapping_add(4i32);
                            }
                        }
                    }
                }
                // §1211
                match self.cur_cmd {
                    87 => {
                        // §1217
                        if (a >= 4i32) {
                            self.geq_define(3934i32, 120i32, self.cur_chr);
                        } else {
                            self.eq_define(3934i32, 120i32, self.cur_chr);
                        }
                    }
                    97 => {
                        // §1218
                        {
                            if (((((self.cur_chr) % 2) != 0) && (!(a >= 4i32))) && (self.eqtb[((5306i32) - 1) as usize].int() >= 0i32)) {
                                a = (a).wrapping_add(4i32);
                            }
                            e = (self.cur_chr >= 2i32);
                            self.get_r_token();
                            p = self.cur_cs;
                            q = self.scan_toks(true, e);
                            if (a >= 4i32) {
                                self.geq_define(p, (111i32).wrapping_add((a % 4i32)), self.def_ref);
                            } else {
                                self.eq_define(p, (111i32).wrapping_add((a % 4i32)), self.def_ref);
                            }
                        }
                    }
                    94 => {
                        // §1221
                        {
                            n = self.cur_chr;
                            self.get_r_token();
                            p = self.cur_cs;
                            if (n == 0i32) {
                                {
                                    loop {
                                        self.get_token();
                                        if (self.cur_cmd != 10i32) { break; }
                                    }
                                    if (self.cur_tok == 3133i32) {
                                        {
                                            self.get_token();
                                            if (self.cur_cmd == 10i32) {
                                                self.get_token();
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    self.get_token();
                                    q = self.cur_tok;
                                    self.get_token();
                                    self.back_input();
                                    self.cur_tok = q;
                                    self.back_input();
                                }
                            }
                            if (self.cur_cmd >= 111i32) {
                                { let __ix988 = self.cur_chr; let __v989 = (self.mem[(self.cur_chr) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix988) as usize].set_hh_lh(__v989); }
                            }
                            if (a >= 4i32) {
                                self.geq_define(p, self.cur_cmd, self.cur_chr);
                            } else {
                                self.eq_define(p, self.cur_cmd, self.cur_chr);
                            }
                        }
                    }
                    95 => {
                        // §1224
                        {
                            n = self.cur_chr;
                            self.get_r_token();
                            p = self.cur_cs;
                            if (a >= 4i32) {
                                self.geq_define(p, 0i32, 256i32);
                            } else {
                                self.eq_define(p, 0i32, 256i32);
                            }
                            self.scan_optional_equals();
                            match n {
                                0 => {
                                    {
                                        self.scan_char_num();
                                        if (a >= 4i32) {
                                            self.geq_define(p, 68i32, self.cur_val);
                                        } else {
                                            self.eq_define(p, 68i32, self.cur_val);
                                        }
                                    }
                                }
                                1 => {
                                    {
                                        self.scan_fifteen_bit_int();
                                        if (a >= 4i32) {
                                            self.geq_define(p, 69i32, self.cur_val);
                                        } else {
                                            self.eq_define(p, 69i32, self.cur_val);
                                        }
                                    }
                                }
                                _ => {
                                    {
                                        self.scan_eight_bit_int();
                                        match n {
                                            2 => {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 73i32, (5318i32).wrapping_add(self.cur_val));
                                                } else {
                                                    self.eq_define(p, 73i32, (5318i32).wrapping_add(self.cur_val));
                                                }
                                            }
                                            3 => {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 74i32, (5851i32).wrapping_add(self.cur_val));
                                                } else {
                                                    self.eq_define(p, 74i32, (5851i32).wrapping_add(self.cur_val));
                                                }
                                            }
                                            4 => {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 75i32, (2900i32).wrapping_add(self.cur_val));
                                                } else {
                                                    self.eq_define(p, 75i32, (2900i32).wrapping_add(self.cur_val));
                                                }
                                            }
                                            5 => {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 76i32, (3156i32).wrapping_add(self.cur_val));
                                                } else {
                                                    self.eq_define(p, 76i32, (3156i32).wrapping_add(self.cur_val));
                                                }
                                            }
                                            6 => {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 72i32, (3422i32).wrapping_add(self.cur_val));
                                                } else {
                                                    self.eq_define(p, 72i32, (3422i32).wrapping_add(self.cur_val));
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    96 => {
                        // §1225
                        {
                            self.scan_int();
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
                                        self.help_line[(1i32) as usize] = 1200i32;
                                        self.help_line[(0i32) as usize] = 1201i32;
                                    }
                                    self.error();
                                }
                            }
                            self.get_r_token();
                            p = self.cur_cs;
                            self.read_toks(n, p);
                            if (a >= 4i32) {
                                self.geq_define(p, 111i32, self.cur_val);
                            } else {
                                self.eq_define(p, 111i32, self.cur_val);
                            }
                        }
                    }
                    71 | 72 => {
                        // §1226
                        {
                            q = self.cur_cs;
                            if (self.cur_cmd == 71i32) {
                                {
                                    self.scan_eight_bit_int();
                                    p = (3422i32).wrapping_add(self.cur_val);
                                }
                            } else {
                                p = self.cur_chr;
                            }
                            self.scan_optional_equals();
                            // §404
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                            }
                            // §1226
                            if (self.cur_cmd != 1i32) {
                                // §1227
                                {
                                    if (self.cur_cmd == 71i32) {
                                        {
                                            self.scan_eight_bit_int();
                                            self.cur_cmd = 72i32;
                                            self.cur_chr = (3422i32).wrapping_add(self.cur_val);
                                        }
                                    }
                                    if (self.cur_cmd == 72i32) {
                                        {
                                            q = self.eqtb[((self.cur_chr) - 1) as usize].hh().rh();
                                            if (q == 0i32) {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 101i32, 0i32);
                                                } else {
                                                    self.eq_define(p, 101i32, 0i32);
                                                }
                                            } else {
                                                {
                                                    { let __v990 = (self.mem[(q) as usize].hh().lh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_lh(__v990); }
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 111i32, q);
                                                    } else {
                                                        self.eq_define(p, 111i32, q);
                                                    }
                                                }
                                            }
                                            break 'l_done_f;
                                        }
                                    }
                                }
                            }
                            // §1226
                            self.back_input();
                            self.cur_cs = q;
                            q = self.scan_toks(false, false);
                            if (self.mem[(self.def_ref) as usize].hh().rh() == 0i32) {
                                {
                                    if (a >= 4i32) {
                                        self.geq_define(p, 101i32, 0i32);
                                    } else {
                                        self.eq_define(p, 101i32, 0i32);
                                    }
                                    {
                                        { let __ix991 = self.def_ref; let __v992 = self.avail; self.mem[(__ix991) as usize].set_hh_rh(__v992); }
                                        self.avail = self.def_ref;
                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                    }
                                }
                            } else {
                                {
                                    if (p == 3413i32) {
                                        {
                                            { let __v993 = self.get_avail(); self.mem[(q) as usize].set_hh_rh(__v993); }
                                            q = self.mem[(q) as usize].hh().rh();
                                            self.mem[(q) as usize].set_hh_lh(637i32);
                                            q = self.get_avail();
                                            self.mem[(q) as usize].set_hh_lh(379i32);
                                            { let __v994 = self.mem[(self.def_ref) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v994); }
                                            { let __ix995 = self.def_ref; self.mem[(__ix995) as usize].set_hh_rh(q); }
                                        }
                                    }
                                    if (a >= 4i32) {
                                        self.geq_define(p, 111i32, self.def_ref);
                                    } else {
                                        self.eq_define(p, 111i32, self.def_ref);
                                    }
                                }
                            }
                        }
                    }
                    73 => {
                        // §1228
                        {
                            p = self.cur_chr;
                            self.scan_optional_equals();
                            self.scan_int();
                            if (a >= 4i32) {
                                self.geq_word_define(p, self.cur_val);
                            } else {
                                self.eq_word_define(p, self.cur_val);
                            }
                        }
                    }
                    74 => {
                        {
                            p = self.cur_chr;
                            self.scan_optional_equals();
                            self.scan_dimen(false, false, false);
                            if (a >= 4i32) {
                                self.geq_word_define(p, self.cur_val);
                            } else {
                                self.eq_word_define(p, self.cur_val);
                            }
                        }
                    }
                    75 | 76 => {
                        {
                            p = self.cur_chr;
                            n = self.cur_cmd;
                            self.scan_optional_equals();
                            if (n == 76i32) {
                                self.scan_glue(3i32);
                            } else {
                                self.scan_glue(2i32);
                            }
                            self.trap_zero_glue();
                            if (a >= 4i32) {
                                self.geq_define(p, 117i32, self.cur_val);
                            } else {
                                self.eq_define(p, 117i32, self.cur_val);
                            }
                        }
                    }
                    85 => {
                        // §1232
                        {
                            // §1233
                            if (self.cur_chr == 3983i32) {
                                n = 15i32;
                            } else {
                                if (self.cur_chr == 5007i32) {
                                    n = 32768i32;
                                } else {
                                    if (self.cur_chr == 4751i32) {
                                        n = 32767i32;
                                    } else {
                                        if (self.cur_chr == 5574i32) {
                                            n = 16777215i32;
                                        } else {
                                            n = 255i32;
                                        }
                                    }
                                }
                            }
                            // §1232
                            p = self.cur_chr;
                            self.scan_char_num();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_int();
                            if (((self.cur_val < 0i32) && (p < 5574i32)) || (self.cur_val > n)) {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(1202i32);
                                    }
                                    self.print_int(self.cur_val);
                                    if (p < 5574i32) {
                                        self.print(1203i32);
                                    } else {
                                        self.print(1204i32);
                                    }
                                    self.print_int(n);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 1205i32;
                                    }
                                    self.error();
                                    self.cur_val = 0i32;
                                }
                            }
                            if (p < 5007i32) {
                                if (a >= 4i32) {
                                    self.geq_define(p, 120i32, self.cur_val);
                                } else {
                                    self.eq_define(p, 120i32, self.cur_val);
                                }
                            } else {
                                if (p < 5574i32) {
                                    if (a >= 4i32) {
                                        self.geq_define(p, 120i32, (self.cur_val).wrapping_add(0i32));
                                    } else {
                                        self.eq_define(p, 120i32, (self.cur_val).wrapping_add(0i32));
                                    }
                                } else {
                                    if (a >= 4i32) {
                                        self.geq_word_define(p, self.cur_val);
                                    } else {
                                        self.eq_word_define(p, self.cur_val);
                                    }
                                }
                            }
                        }
                    }
                    86 => {
                        // §1234
                        {
                            p = self.cur_chr;
                            self.scan_four_bit_int();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_font_ident();
                            if (a >= 4i32) {
                                self.geq_define(p, 120i32, self.cur_val);
                            } else {
                                self.eq_define(p, 120i32, self.cur_val);
                            }
                        }
                    }
                    89 | 90 | 91 | 92 => {
                        // §1235
                        self.do_register_command(a);
                    }
                    98 => {
                        // §1241
                        {
                            self.scan_eight_bit_int();
                            if (a >= 4i32) {
                                n = (256i32).wrapping_add(self.cur_val);
                            } else {
                                n = self.cur_val;
                            }
                            self.scan_optional_equals();
                            if self.set_box_allowed {
                                self.scan_box((1073741824i32).wrapping_add(n));
                            } else {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(680i32);
                                    }
                                    self.print_esc(536i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 1211i32;
                                        self.help_line[(0i32) as usize] = 1212i32;
                                    }
                                    self.error();
                                }
                            }
                        }
                    }
                    79 => {
                        // §1242
                        self.alter_aux();
                    }
                    80 => {
                        self.alter_prev_graf();
                    }
                    81 => {
                        self.alter_page_so_far();
                    }
                    82 => {
                        self.alter_integer();
                    }
                    83 => {
                        self.alter_box_dimen();
                    }
                    84 => {
                        // §1248
                        {
                            self.scan_optional_equals();
                            self.scan_int();
                            n = self.cur_val;
                            if (n <= 0i32) {
                                p = 0i32;
                            } else {
                                {
                                    p = self.get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                    self.mem[(p) as usize].set_hh_lh(n);
                                    {
                                        let __for_end_9 = n;
                                        j = 1i32;
                                        while j <= __for_end_9 {
                                            {
                                                self.scan_dimen(false, false, false);
                                                { let __v996 = self.cur_val; self.mem[(((p).wrapping_add((2i32).wrapping_mul(j))).wrapping_sub(1i32)) as usize].set_int(__v996); }
                                                self.scan_dimen(false, false, false);
                                                { let __v997 = self.cur_val; self.mem[((p).wrapping_add((2i32).wrapping_mul(j))) as usize].set_int(__v997); }
                                            }
                                            j = j.wrapping_add(1);
                                        }
                                    }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(3412i32, 118i32, p);
                            } else {
                                self.eq_define(3412i32, 118i32, p);
                            }
                        }
                    }
                    99 => {
                        // §1252
                        if (self.cur_chr == 1i32) {
                            {
                                self.new_patterns();
                                break 'l_done_f;
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(1216i32);
                                }
                                self.help_ptr = 0i32;
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_cmd == 2i32) { break; }
                                }
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                self.new_hyph_exceptions();
                                break 'l_done_f;
                            }
                        }
                    }
                    77 => {
                        // §1253
                        {
                            self.find_font_dimen(true);
                            k = self.cur_val;
                            self.scan_optional_equals();
                            self.scan_dimen(false, false, false);
                            { let __v998 = self.cur_val; self.font_info[(k) as usize].set_int(__v998); }
                        }
                    }
                    78 => {
                        {
                            n = self.cur_chr;
                            self.scan_font_ident();
                            f = self.cur_val;
                            self.scan_optional_equals();
                            self.scan_int();
                            if (n == 0i32) {
                                { let __v999 = self.cur_val; self.hyphen_char[(f) as usize] = __v999; }
                            } else {
                                { let __v1000 = self.cur_val; self.skew_char[(f) as usize] = __v1000; }
                            }
                        }
                    }
                    88 => {
                        // §1256
                        self.new_font(a);
                    }
                    100 => {
                        // §1264
                        self.new_interaction();
                    }
                    _ => {
                        // §1211
                        self.confusion(1178i32);
                    }
                }
            }
            if (self.after_token != 0i32) {
                // §1269
                {
                    self.cur_tok = self.after_token;
                    self.back_input();
                    self.after_token = 0i32;
                }
            }
        }
        // §1211
    }

    /// Here is a procedure that might be called `Get the next non-blank non-relax
    /// non-call non-assignment token'.
    /// @<Declare act...
    // §1270
    pub fn do_assignments(&mut self) {
        'l_exit_f: {
            while true {
                {
                    // §404
                    loop {
                        self.get_x_token();
                        if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                    }
                    // §1270
                    if (self.cur_cmd <= 70i32) {
                        break 'l_exit_f;
                    }
                    self.set_box_allowed = false;
                    self.prefixed_command();
                    self.set_box_allowed = true;
                }
            }
        }
    }

    /// @<Declare act...
    // §1275
    pub fn open_or_close_in(&mut self) {
        let mut c: i32 = 0; // §1275
        let mut n: i32 = 0; // §1275
        c = self.cur_chr;
        self.scan_four_bit_int();
        n = self.cur_val;
        if (self.read_open[(n) as usize] != 2i32) {
            {
                { let mut __f = ::core::mem::take(&mut self.read_file[(n) as usize]); let __r = self.a_close(&mut __f); self.read_file[(n) as usize] = __f; __r };
                self.read_open[(n) as usize] = 2i32;
            }
        }
        if (c != 0i32) {
            {
                self.scan_optional_equals();
                self.scan_file_name();
                if (self.cur_ext == 338i32) {
                    self.cur_ext = 791i32;
                }
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                if { let mut __f = ::core::mem::take(&mut self.read_file[(n) as usize]); let __r = self.a_open_in(&mut __f); self.read_file[(n) as usize] = __f; __r } {
                    self.read_open[(n) as usize] = 1i32;
                }
            }
        }
    }

    /// @<Declare act...
    // §1279
    pub fn issue_message(&mut self) {
        let mut old_setting: i32 = 0; // §1279
        let mut c: i32 = 0; // §1279
        let mut s: str_number = 0; // §1279
        c = self.cur_chr;
        { let __v1001 = self.scan_toks(false, true); self.mem[(29988i32) as usize].set_hh_rh(__v1001); }
        old_setting = self.selector;
        self.selector = 21i32;
        self.token_show(self.def_ref);
        self.selector = old_setting;
        self.flush_list(self.def_ref);
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        if (c == 0i32) {
            // §1280
            {
                if ((self.term_offset).wrapping_add((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) > (max_print_line).wrapping_sub(2i32)) {
                    self.print_ln();
                } else {
                    if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                        self.print_char(32i32);
                    }
                }
                self.slow_print(s);
                crate::system::break_out(&mut self.term_out);
            }
        } else {
            // §1283
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(338i32);
                }
                self.slow_print(s);
                if (self.eqtb[((3421i32) - 1) as usize].hh().rh() != 0i32) {
                    self.use_err_help = true;
                } else {
                    if self.long_help_seen {
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1232i32;
                        }
                    } else {
                        {
                            if (self.interaction < 3i32) {
                                self.long_help_seen = true;
                            }
                            {
                                self.help_ptr = 4i32;
                                self.help_line[(3i32) as usize] = 1233i32;
                                self.help_line[(2i32) as usize] = 1234i32;
                                self.help_line[(1i32) as usize] = 1235i32;
                                self.help_line[(0i32) as usize] = 1236i32;
                            }
                        }
                    }
                }
                self.error();
                self.use_err_help = false;
            }
        }
        // §1279
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
        }
    }

    /// @<Declare act...
    // §1288
    pub fn shift_case(&mut self) {
        let mut b: halfword = 0; // §1288
        let mut p: halfword = 0; // §1288
        let mut t: halfword = 0; // §1288
        let mut c: eight_bits = 0; // §1288
        b = self.cur_chr;
        p = self.scan_toks(false, false);
        p = self.mem[(self.def_ref) as usize].hh().rh();
        while (p != 0i32) {
            {
                // §1289
                t = self.mem[(p) as usize].hh().lh();
                if (t < 4352i32) {
                    {
                        c = (t % 256i32);
                        if (self.eqtb[(((b).wrapping_add(c)) - 1) as usize].hh().rh() != 0i32) {
                            { let __v1002 = ((t).wrapping_sub(c)).wrapping_add(self.eqtb[(((b).wrapping_add(c)) - 1) as usize].hh().rh()); self.mem[(p) as usize].set_hh_lh(__v1002); }
                        }
                    }
                }
                // §1288
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        self.begin_token_list(self.mem[(self.def_ref) as usize].hh().rh(), 3i32);
        {
            { let __ix1003 = self.def_ref; let __v1004 = self.avail; self.mem[(__ix1003) as usize].set_hh_rh(__v1004); }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
    }

    /// @<Declare act...
    // §1293
    pub fn show_whatever(&mut self) {
        let mut p: halfword = 0; // §1293
        'l_common_ending_f: {
            match self.cur_chr {
                3 => {
                    {
                        self.begin_diagnostic();
                        self.show_activities();
                    }
                }
                1 => {
                    // §1296
                    {
                        self.scan_eight_bit_int();
                        self.begin_diagnostic();
                        self.print_nl(1254i32);
                        self.print_int(self.cur_val);
                        self.print_char(61i32);
                        if (self.eqtb[(((3678i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh() == 0i32) {
                            self.print(410i32);
                        } else {
                            self.show_box(self.eqtb[(((3678i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh());
                        }
                    }
                }
                0 => {
                    // §1294
                    {
                        self.get_token();
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(1248i32);
                        if (self.cur_cs != 0i32) {
                            {
                                self.sprint_cs(self.cur_cs);
                                self.print_char(61i32);
                            }
                        }
                        self.print_meaning();
                        break 'l_common_ending_f;
                    }
                }
                _ => {
                    // §1297
                    {
                        p = self.the_toks();
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(1248i32);
                        self.token_show(29997i32);
                        self.flush_list(self.mem[(29997i32) as usize].hh().rh());
                        break 'l_common_ending_f;
                    }
                }
            }
            // §1298
            self.end_diagnostic(true);
            {
                if (self.interaction == 3i32) {
                }
                self.print_nl(262i32);
                self.print(1255i32);
            }
            if (self.selector == 19i32) {
                if (self.eqtb[((5292i32) - 1) as usize].int() <= 0i32) {
                    {
                        self.selector = 17i32;
                        self.print(1256i32);
                        self.selector = 19i32;
                    }
                }
            }
        }
        // §1293
        if (self.interaction < 3i32) {
            {
                self.help_ptr = 0i32;
                self.error_count = (self.error_count).wrapping_sub(1i32);
            }
        } else {
            if (self.eqtb[((5292i32) - 1) as usize].int() > 0i32) {
                {
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 1243i32;
                        self.help_line[(1i32) as usize] = 1244i32;
                        self.help_line[(0i32) as usize] = 1245i32;
                    }
                }
            } else {
                {
                    {
                        self.help_ptr = 5i32;
                        self.help_line[(4i32) as usize] = 1243i32;
                        self.help_line[(3i32) as usize] = 1244i32;
                        self.help_line[(2i32) as usize] = 1245i32;
                        self.help_line[(1i32) as usize] = 1246i32;
                        self.help_line[(0i32) as usize] = 1247i32;
                    }
                }
            }
        }
        self.error();
    }

    /// @<Declare act...
    // §1302
    pub fn store_fmt_file(&mut self) {
        let mut j: i32 = 0; // §1302
        let mut k: i32 = 0; // §1302
        let mut l: i32 = 0; // §1302
        let mut p: halfword = 0; // §1302
        let mut q: halfword = 0; // §1302
        let mut x: i32 = 0; // §1302
        let mut w: four_quarters = four_quarters::default(); // §1302
        // §1304
        if (self.save_ptr != 0i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1258i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 1259i32;
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
        }
        // §1328
        self.selector = 21i32;
        self.print(1272i32);
        self.print(self.job_name);
        self.print_char(32i32);
        self.print_int(self.eqtb[((5286i32) - 1) as usize].int());
        self.print_char(46i32);
        self.print_int(self.eqtb[((5285i32) - 1) as usize].int());
        self.print_char(46i32);
        self.print_int(self.eqtb[((5284i32) - 1) as usize].int());
        self.print_char(41i32);
        if (self.interaction == 0i32) {
            self.selector = 18i32;
        } else {
            self.selector = 19i32;
        }
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.format_ident = self.make_string();
        self.pack_job_name(786i32);
        while (!{ let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_out(&mut __f); self.fmt_file = __f; __r }) {
            self.prompt_file_name(1273i32, 786i32);
        }
        self.print_nl(1274i32);
        { let __a1005_0 = { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_make_name_string(&mut __f); self.fmt_file = __f; __r }; self.slow_print(__a1005_0) };
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
        }
        self.print_nl(338i32);
        self.slow_print(self.format_ident);
        // §1307
        {
            self.fmt_file.buf.set_int(504454778i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(0i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(30000i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(6106i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(1777i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(307i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1309
        {
            { let __v1006 = self.pool_ptr; self.fmt_file.buf.set_int(__v1006); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1007 = self.str_ptr; self.fmt_file.buf.set_int(__v1007); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.str_ptr;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1008 = self.str_start[(k) as usize]; self.fmt_file.buf.set_int(__v1008); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        k = 0i32;
        while ((k).wrapping_add(4i32) < self.pool_ptr) {
            {
                { let __v1009 = (self.str_pool[(k) as usize]).wrapping_add(0i32); w.set_b0(__v1009); }
                { let __v1010 = (self.str_pool[((k).wrapping_add(1i32)) as usize]).wrapping_add(0i32); w.set_b1(__v1010); }
                { let __v1011 = (self.str_pool[((k).wrapping_add(2i32)) as usize]).wrapping_add(0i32); w.set_b2(__v1011); }
                { let __v1012 = (self.str_pool[((k).wrapping_add(3i32)) as usize]).wrapping_add(0i32); w.set_b3(__v1012); }
                {
                    self.fmt_file.buf.set_qqqq(w);
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = (k).wrapping_add(4i32);
            }
        }
        k = (self.pool_ptr).wrapping_sub(4i32);
        { let __v1013 = (self.str_pool[(k) as usize]).wrapping_add(0i32); w.set_b0(__v1013); }
        { let __v1014 = (self.str_pool[((k).wrapping_add(1i32)) as usize]).wrapping_add(0i32); w.set_b1(__v1014); }
        { let __v1015 = (self.str_pool[((k).wrapping_add(2i32)) as usize]).wrapping_add(0i32); w.set_b2(__v1015); }
        { let __v1016 = (self.str_pool[((k).wrapping_add(3i32)) as usize]).wrapping_add(0i32); w.set_b3(__v1016); }
        {
            self.fmt_file.buf.set_qqqq(w);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(self.str_ptr);
        self.print(1260i32);
        self.print_int(self.pool_ptr);
        // §1311
        self.sort_avail();
        self.var_used = 0i32;
        {
            { let __v1017 = self.lo_mem_max; self.fmt_file.buf.set_int(__v1017); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1018 = self.rover; self.fmt_file.buf.set_int(__v1018); }
            crate::system::put_word(&mut self.fmt_file);
        }
        p = 0i32;
        q = self.rover;
        x = 0i32;
        loop {
            {
                let __for_end_3 = (q).wrapping_add(1i32);
                k = p;
                while k <= __for_end_3 {
                    {
                        self.fmt_file.buf = self.mem[(k) as usize];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
            x = (((x).wrapping_add(q)).wrapping_add(2i32)).wrapping_sub(p);
            self.var_used = ((self.var_used).wrapping_add(q)).wrapping_sub(p);
            p = (q).wrapping_add(self.mem[(q) as usize].hh().lh());
            q = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
            if (q == self.rover) { break; }
        }
        self.var_used = ((self.var_used).wrapping_add(self.lo_mem_max)).wrapping_sub(p);
        self.dyn_used = ((self.mem_end).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        {
            let __for_end_2 = self.lo_mem_max;
            k = p;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.mem[(k) as usize];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.lo_mem_max)).wrapping_add(1i32)).wrapping_sub(p);
        {
            { let __v1019 = self.hi_mem_min; self.fmt_file.buf.set_int(__v1019); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1020 = self.avail; self.fmt_file.buf.set_int(__v1020); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.mem_end;
            k = self.hi_mem_min;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.mem[(k) as usize];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.mem_end)).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        p = self.avail;
        while (p != 0i32) {
            {
                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        {
            { let __v1021 = self.var_used; self.fmt_file.buf.set_int(__v1021); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1022 = self.dyn_used; self.fmt_file.buf.set_int(__v1022); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(x);
        self.print(1261i32);
        self.print_int(self.var_used);
        self.print_char(38i32);
        self.print_int(self.dyn_used);
        // §1315
        k = 1i32;
        loop {
            'l_done1_f: {
                'l_found1_f: {
                    j = k;
                    while (j < 5262i32) {
                        {
                            if (((self.eqtb[((j) - 1) as usize].hh().rh() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().rh()) && (self.eqtb[((j) - 1) as usize].hh().b0() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b0())) && (self.eqtb[((j) - 1) as usize].hh().b1() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b1())) {
                                break 'l_found1_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 5263i32;
                    break 'l_done1_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < 5262i32) {
                    {
                        if (((self.eqtb[((j) - 1) as usize].hh().rh() != self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().rh()) || (self.eqtb[((j) - 1) as usize].hh().b0() != self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b0())) || (self.eqtb[((j) - 1) as usize].hh().b1() != self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b1())) {
                            break 'l_done1_f;
                        }
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
            {
                self.fmt_file.buf.set_int((l).wrapping_sub(k));
                crate::system::put_word(&mut self.fmt_file);
            }
            while (k < l) {
                {
                    {
                        self.fmt_file.buf = self.eqtb[((k) - 1) as usize];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = (k).wrapping_add(1i32);
                }
            }
            k = (j).wrapping_add(1i32);
            {
                self.fmt_file.buf.set_int((k).wrapping_sub(l));
                crate::system::put_word(&mut self.fmt_file);
            }
            if (k == 5263i32) { break; }
        }
        // §1316
        loop {
            'l_done2_f: {
                'l_found2_f: {
                    j = k;
                    while (j < 6106i32) {
                        {
                            if (self.eqtb[((j) - 1) as usize].int() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].int()) {
                                break 'l_found2_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 6107i32;
                    break 'l_done2_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < 6106i32) {
                    {
                        if (self.eqtb[((j) - 1) as usize].int() != self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].int()) {
                            break 'l_done2_f;
                        }
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
            {
                self.fmt_file.buf.set_int((l).wrapping_sub(k));
                crate::system::put_word(&mut self.fmt_file);
            }
            while (k < l) {
                {
                    {
                        self.fmt_file.buf = self.eqtb[((k) - 1) as usize];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = (k).wrapping_add(1i32);
                }
            }
            k = (j).wrapping_add(1i32);
            {
                self.fmt_file.buf.set_int((k).wrapping_sub(l));
                crate::system::put_word(&mut self.fmt_file);
            }
            if (k > 6106i32) { break; }
        }
        // §1313
        {
            { let __v1023 = self.par_loc; self.fmt_file.buf.set_int(__v1023); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1024 = self.write_loc; self.fmt_file.buf.set_int(__v1024); }
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1318
        {
            { let __v1025 = self.hash_used; self.fmt_file.buf.set_int(__v1025); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.cs_count = (2613i32).wrapping_sub(self.hash_used);
        {
            let __for_end_2 = self.hash_used;
            p = 514i32;
            while p <= __for_end_2 {
                if (self.hash[((p) - 514) as usize].rh() != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(p);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1026 = self.hash[((p) - 514) as usize]; self.fmt_file.buf.set_hh(__v1026); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        self.cs_count = (self.cs_count).wrapping_add(1i32);
                    }
                }
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 2880i32;
            p = (self.hash_used).wrapping_add(1i32);
            while p <= __for_end_2 {
                {
                    { let __v1027 = self.hash[((p) - 514) as usize]; self.fmt_file.buf.set_hh(__v1027); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        {
            { let __v1028 = self.cs_count; self.fmt_file.buf.set_int(__v1028); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(self.cs_count);
        self.print(1262i32);
        // §1320
        {
            { let __v1029 = self.fmem_ptr; self.fmt_file.buf.set_int(__v1029); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = (self.fmem_ptr).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.font_info[(k) as usize];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            { let __v1030 = self.font_ptr; self.fmt_file.buf.set_int(__v1030); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.font_ptr;
            k = 0i32;
            while k <= __for_end_2 {
                // §1322
                {
                    {
                        { let __v1031 = self.font_check[(k) as usize]; self.fmt_file.buf.set_qqqq(__v1031); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1032 = self.font_size[(k) as usize]; self.fmt_file.buf.set_int(__v1032); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1033 = self.font_dsize[(k) as usize]; self.fmt_file.buf.set_int(__v1033); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1034 = self.font_params[(k) as usize]; self.fmt_file.buf.set_int(__v1034); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1035 = self.hyphen_char[(k) as usize]; self.fmt_file.buf.set_int(__v1035); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1036 = self.skew_char[(k) as usize]; self.fmt_file.buf.set_int(__v1036); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1037 = self.font_name[(k) as usize]; self.fmt_file.buf.set_int(__v1037); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1038 = self.font_area[(k) as usize]; self.fmt_file.buf.set_int(__v1038); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1039 = self.font_bc[(k) as usize]; self.fmt_file.buf.set_int(__v1039); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1040 = self.font_ec[(k) as usize]; self.fmt_file.buf.set_int(__v1040); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1041 = self.char_base[(k) as usize]; self.fmt_file.buf.set_int(__v1041); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1042 = self.width_base[(k) as usize]; self.fmt_file.buf.set_int(__v1042); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1043 = self.height_base[(k) as usize]; self.fmt_file.buf.set_int(__v1043); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1044 = self.depth_base[(k) as usize]; self.fmt_file.buf.set_int(__v1044); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1045 = self.italic_base[(k) as usize]; self.fmt_file.buf.set_int(__v1045); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1046 = self.lig_kern_base[(k) as usize]; self.fmt_file.buf.set_int(__v1046); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1047 = self.kern_base[(k) as usize]; self.fmt_file.buf.set_int(__v1047); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1048 = self.exten_base[(k) as usize]; self.fmt_file.buf.set_int(__v1048); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1049 = self.param_base[(k) as usize]; self.fmt_file.buf.set_int(__v1049); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1050 = self.font_glue[(k) as usize]; self.fmt_file.buf.set_int(__v1050); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1051 = self.bchar_label[(k) as usize]; self.fmt_file.buf.set_int(__v1051); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1052 = self.font_bchar[(k) as usize]; self.fmt_file.buf.set_int(__v1052); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1053 = self.font_false_bchar[(k) as usize]; self.fmt_file.buf.set_int(__v1053); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    self.print_nl(1265i32);
                    self.print_esc(self.hash[(((2624i32).wrapping_add(k)) - 514) as usize].rh());
                    self.print_char(61i32);
                    self.print_file_name(self.font_name[(k) as usize], self.font_area[(k) as usize], 338i32);
                    if (self.font_size[(k) as usize] != self.font_dsize[(k) as usize]) {
                        {
                            self.print(741i32);
                            self.print_scaled(self.font_size[(k) as usize]);
                            self.print(397i32);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        // §1320
        self.print_ln();
        self.print_int((self.fmem_ptr).wrapping_sub(7i32));
        self.print(1263i32);
        self.print_int((self.font_ptr).wrapping_sub(0i32));
        self.print(1264i32);
        if (self.font_ptr != 1i32) {
            self.print_char(115i32);
        }
        // §1324
        {
            { let __v1054 = self.hyph_count; self.fmt_file.buf.set_int(__v1054); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = 307i32;
            k = 0i32;
            while k <= __for_end_2 {
                if (self.hyph_word[(k) as usize] != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1055 = self.hyph_word[(k) as usize]; self.fmt_file.buf.set_int(__v1055); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1056 = self.hyph_list[(k) as usize]; self.fmt_file.buf.set_int(__v1056); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.print_int(self.hyph_count);
        self.print(1266i32);
        if (self.hyph_count != 1i32) {
            self.print_char(115i32);
        }
        if self.trie_not_ready {
            self.init_trie();
        }
        {
            { let __v1057 = self.trie_max; self.fmt_file.buf.set_int(__v1057); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_max;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1058 = self.trie[(k) as usize]; self.fmt_file.buf.set_hh(__v1058); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            { let __v1059 = self.trie_op_ptr; self.fmt_file.buf.set_int(__v1059); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    {
                        { let __v1060 = self.hyf_distance[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1060); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1061 = self.hyf_num[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1061); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1062 = self.hyf_next[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1062); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_nl(1267i32);
        self.print_int(self.trie_max);
        self.print(1268i32);
        self.print_int(self.trie_op_ptr);
        self.print(1269i32);
        if (self.trie_op_ptr != 1i32) {
            self.print_char(115i32);
        }
        self.print(1270i32);
        self.print_int(trie_op_size);
        {
            let __for_end_2 = 0i32;
            k = 255i32;
            while k >= __for_end_2 {
                if (self.trie_used[(k) as usize] > 0i32) {
                    {
                        self.print_nl(800i32);
                        self.print_int((self.trie_used[(k) as usize]).wrapping_sub(0i32));
                        self.print(1271i32);
                        self.print_int(k);
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1063 = (self.trie_used[(k) as usize]).wrapping_sub(0i32); self.fmt_file.buf.set_int(__v1063); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_sub(1);
            }
        }
        // §1326
        {
            { let __v1064 = self.interaction; self.fmt_file.buf.set_int(__v1064); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1065 = self.format_ident; self.fmt_file.buf.set_int(__v1065); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(69069i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.eqtb[((5294i32) - 1) as usize].set_int(0i32);
        // §1329
        { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f); self.fmt_file = __f; __r };
    }

    /// Here is a subroutine that creates a whatsit node having a given `subtype`
    /// and a given number of words. It initializes only the first word of the whatsit,
    /// and appends it to the current list.
    /// @<Declare procedures needed in `do_extension`
    // §1349
    pub fn new_whatsit(&mut self, mut s: small_number, mut w: small_number) {
        let mut p: halfword = 0; // §1349
        p = self.get_node(w);
        self.mem[(p) as usize].set_hh_b0(8i32);
        self.mem[(p) as usize].set_hh_b1(s);
        { let __ix1066 = self.cur_list.tail_field; self.mem[(__ix1066) as usize].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// The next subroutine uses `cur_chr` to decide what sort of whatsit is
    /// involved, and also inserts a `write_stream` number.
    /// @<Declare procedures needed in `do_ext...
    // §1350
    pub fn new_write_whatsit(&mut self, mut w: small_number) {
        self.new_whatsit(self.cur_chr, w);
        if (w != 2i32) {
            self.scan_four_bit_int();
        } else {
            {
                self.scan_int();
                if (self.cur_val < 0i32) {
                    self.cur_val = 17i32;
                } else {
                    if (self.cur_val > 15i32) {
                        self.cur_val = 16i32;
                    }
                }
            }
        }
        { let __ix1067 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1068 = self.cur_val; self.mem[(__ix1067) as usize].set_hh_lh(__v1068); }
    }

    /// @<Declare act...
    // §1348
    pub fn do_extension(&mut self) {
        let mut i: i32 = 0; // §1348
        let mut j: i32 = 0; // §1348
        let mut k: i32 = 0; // §1348
        let mut p: halfword = 0; // §1348
        let mut q: halfword = 0; // §1348
        let mut r: halfword = 0; // §1348
        match self.cur_chr {
            0 => {
                // §1351
                {
                    self.new_write_whatsit(3i32);
                    self.scan_optional_equals();
                    self.scan_file_name();
                    { let __ix1069 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1070 = self.cur_name; self.mem[(__ix1069) as usize].set_hh_rh(__v1070); }
                    { let __ix1071 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1072 = self.cur_area; self.mem[(__ix1071) as usize].set_hh_lh(__v1072); }
                    { let __ix1073 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1074 = self.cur_ext; self.mem[(__ix1073) as usize].set_hh_rh(__v1074); }
                }
            }
            1 => {
                // §1352
                {
                    k = self.cur_cs;
                    self.new_write_whatsit(2i32);
                    self.cur_cs = k;
                    p = self.scan_toks(false, false);
                    { let __ix1075 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1076 = self.def_ref; self.mem[(__ix1075) as usize].set_hh_rh(__v1076); }
                }
            }
            2 => {
                // §1353
                {
                    self.new_write_whatsit(2i32);
                    { let __ix1077 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1077) as usize].set_hh_rh(0i32); }
                }
            }
            3 => {
                // §1354
                {
                    self.new_whatsit(3i32, 2i32);
                    { let __ix1078 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1078) as usize].set_hh_lh(0i32); }
                    p = self.scan_toks(false, true);
                    { let __ix1079 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1080 = self.def_ref; self.mem[(__ix1079) as usize].set_hh_rh(__v1080); }
                }
            }
            4 => {
                // §1375
                {
                    self.get_x_token();
                    if ((self.cur_cmd == 59i32) && (self.cur_chr <= 2i32)) {
                        {
                            p = self.cur_list.tail_field;
                            self.do_extension();
                            self.out_what(self.cur_list.tail_field);
                            self.flush_node_list(self.cur_list.tail_field);
                            self.cur_list.tail_field = p;
                            self.mem[(p) as usize].set_hh_rh(0i32);
                        }
                    } else {
                        self.back_input();
                    }
                }
            }
            5 => {
                // §1377
                if ((self.cur_list.mode_field).wrapping_abs() != 102i32) {
                    self.report_illegal_case();
                } else {
                    {
                        self.new_whatsit(4i32, 2i32);
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.cur_list.aux_field.set_hh_rh(0i32);
                        } else {
                            if (self.cur_val > 255i32) {
                                self.cur_list.aux_field.set_hh_rh(0i32);
                            } else {
                                { let __v1081 = self.cur_val; self.cur_list.aux_field.set_hh_rh(__v1081); }
                            }
                        }
                        { let __ix1082 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1083 = self.cur_list.aux_field.hh().rh(); self.mem[(__ix1082) as usize].set_hh_rh(__v1083); }
                        { let __ix1084 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1085 = self.norm_min(self.eqtb[((5314i32) - 1) as usize].int()); self.mem[(__ix1084) as usize].set_hh_b0(__v1085); }
                        { let __ix1086 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1087 = self.norm_min(self.eqtb[((5315i32) - 1) as usize].int()); self.mem[(__ix1086) as usize].set_hh_b1(__v1087); }
                    }
                }
            }
            _ => {
                // §1348
                self.confusion(1291i32);
            }
        }
    }

    /// The \.{\\language} extension is somewhat different.
    /// We need a subroutine that comes into play when a character of
    /// a non-`clang` language is being appended to the current paragraph.
    /// @<Declare action...
    // §1376
    pub fn fix_language(&mut self) {
        let mut l: ASCII_code = 0; // §1376
        if (self.eqtb[((5313i32) - 1) as usize].int() <= 0i32) {
            l = 0i32;
        } else {
            if (self.eqtb[((5313i32) - 1) as usize].int() > 255i32) {
                l = 0i32;
            } else {
                l = self.eqtb[((5313i32) - 1) as usize].int();
            }
        }
        if (l != self.cur_list.aux_field.hh().rh()) {
            {
                self.new_whatsit(4i32, 2i32);
                { let __ix1088 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1088) as usize].set_hh_rh(l); }
                self.cur_list.aux_field.set_hh_rh(l);
                { let __ix1089 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1090 = self.norm_min(self.eqtb[((5314i32) - 1) as usize].int()); self.mem[(__ix1089) as usize].set_hh_b0(__v1090); }
                { let __ix1091 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1092 = self.norm_min(self.eqtb[((5315i32) - 1) as usize].int()); self.mem[(__ix1091) as usize].set_hh_b1(__v1092); }
            }
        }
    }

    /// @<Declare the procedure called `handle_right_brace`
    // §1068
    pub fn handle_right_brace(&mut self) {
        let mut p: halfword = 0; // §1068
        let mut q: halfword = 0; // §1068
        let mut d: scaled = 0; // §1068
        let mut f: i32 = 0; // §1068
        match self.cur_group {
            1 => {
                self.unsave();
            }
            0 => {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(1044i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 1045i32;
                        self.help_line[(0i32) as usize] = 1046i32;
                    }
                    self.error();
                }
            }
            14 | 15 | 16 => {
                self.extra_right_brace();
            }
            2 => {
                // §1085
                self.package(0i32);
            }
            3 => {
                {
                    self.adjust_tail = 29995i32;
                    self.package(0i32);
                }
            }
            4 => {
                {
                    self.end_graf();
                    self.package(0i32);
                }
            }
            5 => {
                {
                    self.end_graf();
                    self.package(4i32);
                }
            }
            11 => {
                // §1100
                {
                    self.end_graf();
                    q = self.eqtb[((2892i32) - 1) as usize].hh().rh();
                    { let __v1093 = (self.mem[(q) as usize].hh().rh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_rh(__v1093); }
                    d = self.eqtb[((5836i32) - 1) as usize].int();
                    f = self.eqtb[((5305i32) - 1) as usize].int();
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    p = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32, 1073741823i32);
                    self.pop_nest();
                    if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() < 255i32) {
                        {
                            {
                                { let __ix1094 = self.cur_list.tail_field; let __v1095 = self.get_node(5i32); self.mem[(__ix1094) as usize].set_hh_rh(__v1095); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix1096 = self.cur_list.tail_field; self.mem[(__ix1096) as usize].set_hh_b0(3i32); }
                            { let __ix1097 = self.cur_list.tail_field; let __v1098 = (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int()).wrapping_add(0i32); self.mem[(__ix1097) as usize].set_hh_b1(__v1098); }
                            { let __ix1099 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1100 = (self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int()); self.mem[(__ix1099) as usize].set_int(__v1100); }
                            { let __ix1101 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1102 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(__ix1101) as usize].set_hh_lh(__v1102); }
                            { let __ix1103 = (self.cur_list.tail_field).wrapping_add(4i32); self.mem[(__ix1103) as usize].set_hh_rh(q); }
                            { let __ix1104 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix1104) as usize].set_int(d); }
                            { let __ix1105 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1105) as usize].set_int(f); }
                        }
                    } else {
                        {
                            {
                                { let __ix1106 = self.cur_list.tail_field; let __v1107 = self.get_node(2i32); self.mem[(__ix1106) as usize].set_hh_rh(__v1107); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix1108 = self.cur_list.tail_field; self.mem[(__ix1108) as usize].set_hh_b0(5i32); }
                            { let __ix1109 = self.cur_list.tail_field; self.mem[(__ix1109) as usize].set_hh_b1(0i32); }
                            { let __ix1110 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1111 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(__ix1110) as usize].set_int(__v1111); }
                            self.delete_glue_ref(q);
                        }
                    }
                    self.free_node(p, 7i32);
                    if (self.nest_ptr == 0i32) {
                        self.build_page();
                    }
                }
            }
            8 => {
                // §1026
                {
                    if ((self.cur_input.loc_field != 0i32) || ((self.cur_input.index_field != 6i32) && (self.cur_input.index_field != 3i32))) {
                        // §1027
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(1010i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1011i32;
                                self.help_line[(0i32) as usize] = 1012i32;
                            }
                            self.error();
                            loop {
                                self.get_token();
                                if (self.cur_input.loc_field == 0i32) { break; }
                            }
                        }
                    }
                    // §1026
                    self.end_token_list();
                    self.end_graf();
                    self.unsave();
                    self.output_active = false;
                    self.insert_penalties = 0i32;
                    // §1028
                    if (self.eqtb[((3933i32) - 1) as usize].hh().rh() != 0i32) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(1013i32);
                            }
                            self.print_esc(409i32);
                            self.print_int(255i32);
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1014i32;
                                self.help_line[(1i32) as usize] = 1015i32;
                                self.help_line[(0i32) as usize] = 1016i32;
                            }
                            self.box_error(255i32);
                        }
                    }
                    // §1026
                    if (self.cur_list.tail_field != self.cur_list.head_field) {
                        {
                            { let __ix1112 = self.page_tail; let __v1113 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix1112) as usize].set_hh_rh(__v1113); }
                            self.page_tail = self.cur_list.tail_field;
                        }
                    }
                    if (self.mem[(29998i32) as usize].hh().rh() != 0i32) {
                        {
                            if (self.mem[(29999i32) as usize].hh().rh() == 0i32) {
                                self.nest[(0i32) as usize].tail_field = self.page_tail;
                            }
                            { let __ix1114 = self.page_tail; let __v1115 = self.mem[(29999i32) as usize].hh().rh(); self.mem[(__ix1114) as usize].set_hh_rh(__v1115); }
                            { let __v1116 = self.mem[(29998i32) as usize].hh().rh(); self.mem[(29999i32) as usize].set_hh_rh(__v1116); }
                            self.mem[(29998i32) as usize].set_hh_rh(0i32);
                            self.page_tail = 29998i32;
                        }
                    }
                    self.pop_nest();
                    self.build_page();
                }
            }
            10 => {
                // §1118
                self.build_discretionary();
            }
            6 => {
                // §1132
                {
                    self.back_input();
                    self.cur_tok = 6710i32;
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(625i32);
                    }
                    self.print_esc(899i32);
                    self.print(626i32);
                    {
                        self.help_ptr = 1i32;
                        self.help_line[(0i32) as usize] = 1125i32;
                    }
                    self.ins_error();
                }
            }
            7 => {
                // §1133
                {
                    self.end_graf();
                    self.unsave();
                    self.align_peek();
                }
            }
            12 => {
                // §1168
                {
                    self.end_graf();
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                    p = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(), 1073741823i32);
                    self.pop_nest();
                    {
                        { let __ix1117 = self.cur_list.tail_field; let __v1118 = self.new_noad(); self.mem[(__ix1117) as usize].set_hh_rh(__v1118); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                    { let __ix1119 = self.cur_list.tail_field; self.mem[(__ix1119) as usize].set_hh_b0(29i32); }
                    { let __ix1120 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1120) as usize].set_hh_rh(2i32); }
                    { let __ix1121 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1121) as usize].set_hh_lh(p); }
                }
            }
            13 => {
                // §1173
                self.build_choices();
            }
            9 => {
                // §1186
                {
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    { let __ix1122 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); self.mem[(__ix1122) as usize].set_hh_rh(3i32); }
                    p = self.fin_mlist(0i32);
                    { let __ix1123 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); self.mem[(__ix1123) as usize].set_hh_lh(p); }
                    if (p != 0i32) {
                        if (self.mem[(p) as usize].hh().rh() == 0i32) {
                            if (self.mem[(p) as usize].hh().b0() == 16i32) {
                                {
                                    if (self.mem[((p).wrapping_add(3i32)) as usize].hh().rh() == 0i32) {
                                        if (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                                            {
                                                { let __ix1124 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); let __v1125 = self.mem[((p).wrapping_add(1i32)) as usize].hh(); self.mem[(__ix1124) as usize].set_hh(__v1125); }
                                                self.free_node(p, 4i32);
                                            }
                                        }
                                    }
                                }
                            } else {
                                if (self.mem[(p) as usize].hh().b0() == 28i32) {
                                    if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() == (self.cur_list.tail_field).wrapping_add(1i32)) {
                                        if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 16i32) {
                                            // §1187
                                            {
                                                q = self.cur_list.head_field;
                                                while (self.mem[(q) as usize].hh().rh() != self.cur_list.tail_field) {
                                                    q = self.mem[(q) as usize].hh().rh();
                                                }
                                                self.mem[(q) as usize].set_hh_rh(p);
                                                self.free_node(self.cur_list.tail_field, 4i32);
                                                self.cur_list.tail_field = p;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // §1068
                self.confusion(1047i32);
            }
        }
    }

    /// We shall concentrate first on the inner loop of `main_control`, deferring
    /// consideration of the other cases until later.
    // §1030
    pub fn main_control(&mut self) {
        let mut t: i32 = 0; // §1030
        // goto labels: L60, reswitch, L70, L80, L90, L91, L92, L100, L101, L110, L111, L112, L95, L120, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if (self.eqtb[((3419i32) - 1) as usize].hh().rh() != 0i32) {
                    self.begin_token_list(self.eqtb[((3419i32) - 1) as usize].hh().rh(), 12i32);
                }
            }
            if __goto_1 <= 1 { // L60
                self.get_x_token();
            }
            if __goto_1 <= 2 { // reswitch
                if (self.interrupt != 0i32) {
                    // §1031
                    if self.OK_to_interrupt {
                        {
                            self.back_input();
                            {
                                if (self.interrupt != 0i32) {
                                    self.pause_for_instructions();
                                }
                            }
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                }
                if (self.eqtb[((5299i32) - 1) as usize].int() > 0i32) {
                    self.show_cur_cmd_chr();
                }
                // §1030
                match ((self.cur_list.mode_field).wrapping_abs()).wrapping_add(self.cur_cmd) {
                    113 | 114 | 170 => {
                        { __goto_1 = 3; continue 'l_dispatch_1; }
                    }
                    118 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            { __goto_1 = 3; continue 'l_dispatch_1; }
                        }
                    }
                    167 => {
                        {
                            self.get_x_token();
                            if ((((self.cur_cmd == 11i32) || (self.cur_cmd == 12i32)) || (self.cur_cmd == 68i32)) || (self.cur_cmd == 16i32)) {
                                self.cancel_boundary = true;
                            }
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    112 => {
                        if (self.cur_list.aux_field.hh().lh() == 1000i32) {
                            { __goto_1 = 14; continue 'l_dispatch_1; }
                        } else {
                            self.app_space();
                        }
                    }
                    166 | 267 => {
                        { __goto_1 = 14; continue 'l_dispatch_1; }
                    }
                    1 | 102 | 203 | 11 | 213 | 268 => {
                        // §1045
                    }
                    40 | 141 | 242 => {
                        {
                            // §406
                            loop {
                                self.get_x_token();
                                if (self.cur_cmd != 10i32) { break; }
                            }
                            // §1045
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    15 => {
                        if self.its_all_over() {
                            { __goto_1 = 15; continue 'l_dispatch_1; }
                        }
                    }
                    23 | 123 | 224 | 71 | 172 | 273 | 39 | 45 | 49 | 150 | 7 | 108 | 209 => {
                        self.report_illegal_case();
                    }
                    8 | 109 | 9 | 110 | 18 | 119 | 70 | 171 | 51 | 152 | 16 | 117 | 50 | 151 | 53 | 154 | 67 | 168 | 54 | 155 | 55 | 156 | 57 | 158 | 56 | 157 | 31 | 132 | 52 | 153 | 29 | 130 | 47 | 148 | 212 | 216 | 217 | 230 | 227 | 236 | 239 => {
                        self.insert_dollar_sign();
                    }
                    37 | 137 | 238 => {
                        // §1056
                        {
                            {
                                { let __ix1126 = self.cur_list.tail_field; let __v1127 = self.scan_rule_spec(); self.mem[(__ix1126) as usize].set_hh_rh(__v1127); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            if ((self.cur_list.mode_field).wrapping_abs() == 1i32) {
                                self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                            } else {
                                if ((self.cur_list.mode_field).wrapping_abs() == 102i32) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                }
                            }
                        }
                    }
                    28 | 128 | 229 | 231 => {
                        // §1057
                        self.append_glue();
                    }
                    30 | 131 | 232 | 233 => {
                        self.append_kern();
                    }
                    2 | 103 => {
                        // §1063
                        self.new_save_level(1i32);
                    }
                    62 | 163 | 264 => {
                        self.new_save_level(14i32);
                    }
                    63 | 164 | 265 => {
                        if (self.cur_group == 14i32) {
                            self.unsave();
                        } else {
                            self.off_save();
                        }
                    }
                    3 | 104 | 205 => {
                        // §1067
                        self.handle_right_brace();
                    }
                    22 | 124 | 225 => {
                        // §1073
                        {
                            t = self.cur_chr;
                            self.scan_dimen(false, false, false);
                            if (t == 0i32) {
                                self.scan_box(self.cur_val);
                            } else {
                                self.scan_box((self.cur_val).wrapping_neg());
                            }
                        }
                    }
                    32 | 133 | 234 => {
                        self.scan_box((1073742237i32).wrapping_add(self.cur_chr));
                    }
                    21 | 122 | 223 => {
                        self.begin_box(0i32);
                    }
                    44 => {
                        // §1090
                        self.new_graf((self.cur_chr > 0i32));
                    }
                    12 | 13 | 17 | 69 | 4 | 24 | 36 | 46 | 48 | 27 | 34 | 65 | 66 => {
                        {
                            self.back_input();
                            self.new_graf(true);
                        }
                    }
                    145 | 246 => {
                        // §1092
                        self.indent_in_hmode();
                    }
                    14 => {
                        // §1094
                        {
                            self.normal_paragraph();
                            if (self.cur_list.mode_field > 0i32) {
                                self.build_page();
                            }
                        }
                    }
                    115 => {
                        {
                            if (self.align_state < 0i32) {
                                self.off_save();
                            }
                            self.end_graf();
                            if (self.cur_list.mode_field == 1i32) {
                                self.build_page();
                            }
                        }
                    }
                    116 | 129 | 138 | 126 | 134 => {
                        self.head_for_vmode();
                    }
                    38 | 139 | 240 | 140 | 241 => {
                        // §1097
                        self.begin_insert_or_adjust();
                    }
                    19 | 120 | 221 => {
                        self.make_mark();
                    }
                    43 | 144 | 245 => {
                        // §1102
                        self.append_penalty();
                    }
                    26 | 127 | 228 => {
                        // §1104
                        self.delete_last();
                    }
                    25 | 125 | 226 => {
                        // §1109
                        self.unpackage();
                    }
                    146 => {
                        // §1112
                        self.append_italic_correction();
                    }
                    247 => {
                        {
                            { let __ix1128 = self.cur_list.tail_field; let __v1129 = self.new_kern(0i32); self.mem[(__ix1128) as usize].set_hh_rh(__v1129); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                    149 | 250 => {
                        // §1116
                        self.append_discretionary();
                    }
                    147 => {
                        // §1122
                        self.make_accent();
                    }
                    6 | 107 | 208 | 5 | 106 | 207 => {
                        // §1126
                        self.align_error();
                    }
                    35 | 136 | 237 => {
                        self.no_align_error();
                    }
                    64 | 165 | 266 => {
                        self.omit_error();
                    }
                    33 | 135 => {
                        // §1130
                        self.init_align();
                    }
                    235 => {
                        if self.privileged() {
                            if (self.cur_group == 15i32) {
                                self.init_align();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    10 | 111 => {
                        self.do_endv();
                    }
                    68 | 169 | 270 => {
                        // §1134
                        self.cs_error();
                    }
                    105 => {
                        // §1137
                        self.init_math();
                    }
                    251 => {
                        // §1140
                        if self.privileged() {
                            if (self.cur_group == 15i32) {
                                self.start_eq_no();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    204 => {
                        // §1150
                        {
                            {
                                { let __ix1130 = self.cur_list.tail_field; let __v1131 = self.new_noad(); self.mem[(__ix1130) as usize].set_hh_rh(__v1131); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            self.back_input();
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    214 | 215 | 271 => {
                        // §1154
                        self.set_math_char((self.eqtb[(((5007i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32));
                    }
                    219 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.set_math_char((self.eqtb[(((5007i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32));
                        }
                    }
                    220 => {
                        {
                            self.scan_fifteen_bit_int();
                            self.set_math_char(self.cur_val);
                        }
                    }
                    272 => {
                        self.set_math_char(self.cur_chr);
                    }
                    218 => {
                        {
                            self.scan_twenty_seven_bit_int();
                            self.set_math_char((self.cur_val / 4096i32));
                        }
                    }
                    253 => {
                        // §1158
                        {
                            {
                                { let __ix1132 = self.cur_list.tail_field; let __v1133 = self.new_noad(); self.mem[(__ix1132) as usize].set_hh_rh(__v1133); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix1134 = self.cur_list.tail_field; let __v1135 = self.cur_chr; self.mem[(__ix1134) as usize].set_hh_b0(__v1135); }
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    254 => {
                        self.math_limit_switch();
                    }
                    269 => {
                        // §1162
                        self.math_radical();
                    }
                    248 | 249 => {
                        // §1164
                        self.math_ac();
                    }
                    259 => {
                        // §1167
                        {
                            self.scan_spec(12i32, false);
                            self.normal_paragraph();
                            self.push_nest();
                            self.cur_list.mode_field = (1i32).wrapping_neg();
                            self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                            if (self.eqtb[((3418i32) - 1) as usize].hh().rh() != 0i32) {
                                self.begin_token_list(self.eqtb[((3418i32) - 1) as usize].hh().rh(), 11i32);
                            }
                        }
                    }
                    256 => {
                        // §1171
                        {
                            { let __ix1136 = self.cur_list.tail_field; let __v1137 = self.new_style(self.cur_chr); self.mem[(__ix1136) as usize].set_hh_rh(__v1137); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                    258 => {
                        {
                            {
                                { let __ix1138 = self.cur_list.tail_field; let __v1139 = self.new_glue(0i32); self.mem[(__ix1138) as usize].set_hh_rh(__v1139); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix1140 = self.cur_list.tail_field; self.mem[(__ix1140) as usize].set_hh_b1(98i32); }
                        }
                    }
                    257 => {
                        self.append_choices();
                    }
                    211 | 210 => {
                        // §1175
                        self.sub_sup();
                    }
                    255 => {
                        // §1180
                        self.math_fraction();
                    }
                    252 => {
                        // §1190
                        self.math_left_right();
                    }
                    206 => {
                        // §1193
                        if (self.cur_group == 15i32) {
                            self.after_math();
                        } else {
                            self.off_save();
                        }
                    }
                    72 | 173 | 274 | 73 | 174 | 275 | 74 | 175 | 276 | 75 | 176 | 277 | 76 | 177 | 278 | 77 | 178 | 279 | 78 | 179 | 280 | 79 | 180 | 281 | 80 | 181 | 282 | 81 | 182 | 283 | 82 | 183 | 284 | 83 | 184 | 285 | 84 | 185 | 286 | 85 | 186 | 287 | 86 | 187 | 288 | 87 | 188 | 289 | 88 | 189 | 290 | 89 | 190 | 291 | 90 | 191 | 292 | 91 | 192 | 293 | 92 | 193 | 294 | 93 | 194 | 295 | 94 | 195 | 296 | 95 | 196 | 297 | 96 | 197 | 298 | 97 | 198 | 299 | 98 | 199 | 300 | 99 | 200 | 301 | 100 | 201 | 302 | 101 | 202 | 303 => {
                        // §1210
                        self.prefixed_command();
                    }
                    41 | 142 | 243 => {
                        // §1268
                        {
                            self.get_token();
                            self.after_token = self.cur_tok;
                        }
                    }
                    42 | 143 | 244 => {
                        // §1271
                        {
                            self.get_token();
                            self.save_for_after(self.cur_tok);
                        }
                    }
                    61 | 162 | 263 => {
                        // §1274
                        self.open_or_close_in();
                    }
                    59 | 160 | 261 => {
                        // §1276
                        self.issue_message();
                    }
                    58 | 159 | 260 => {
                        // §1285
                        self.shift_case();
                    }
                    20 | 121 | 222 => {
                        // §1290
                        self.show_whatever();
                    }
                    60 | 161 | 262 => {
                        // §1347
                        self.do_extension();
                    }
                    _ => {}
                }
                // §1030
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 3 { // L70
                self.main_s = self.eqtb[(((4751i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                // §1034
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v1141 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v1141); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v1142 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v1142); }
                        }
                    }
                }
                self.main_f = self.eqtb[((3934i32) - 1) as usize].hh().rh();
                self.bchar = self.font_bchar[(self.main_f) as usize];
                self.false_bchar = self.font_false_bchar[(self.main_f) as usize];
                if (self.cur_list.mode_field > 0i32) {
                    if (self.eqtb[((5313i32) - 1) as usize].int() != self.cur_list.aux_field.hh().rh()) {
                        self.fix_language();
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == 0i32) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail = self.mem[(self.lig_stack) as usize].hh().rh();
                            { let __ix1143 = self.lig_stack; self.mem[(__ix1143) as usize].set_hh_rh(0i32); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                { let __ix1144 = self.lig_stack; let __v1145 = self.main_f; self.mem[(__ix1144) as usize].set_hh_b0(__v1145); }
                self.cur_l = (self.cur_chr).wrapping_add(0i32);
                { let __ix1146 = self.lig_stack; let __v1147 = self.cur_l; self.mem[(__ix1146) as usize].set_hh_b1(__v1147); }
                self.cur_q = self.cur_list.tail_field;
                if self.cancel_boundary {
                    {
                        self.cancel_boundary = false;
                        self.main_k = 0i32;
                    }
                } else {
                    self.main_k = self.bchar_label[(self.main_f) as usize];
                }
                if (self.main_k == 0i32) {
                    { __goto_1 = 7; continue 'l_dispatch_1; }
                }
                self.cur_r = self.cur_l;
                self.cur_l = 256i32;
                { __goto_1 = 11; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 4 { // L80
                if (self.cur_l < 256i32) {
                    // §1035
                    {
                        if (self.mem[(self.cur_q) as usize].hh().rh() > 0i32) {
                            if (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == (self.hyphen_char[(self.main_f) as usize]).wrapping_add(0i32)) {
                                self.ins_disc = true;
                            }
                        }
                        if self.ligature_present {
                            {
                                self.main_p = self.new_ligature(self.main_f, self.cur_l, self.mem[(self.cur_q) as usize].hh().rh());
                                if self.lft_hit {
                                    {
                                        { let __ix1148 = self.main_p; self.mem[(__ix1148) as usize].set_hh_b1(2i32); }
                                        self.lft_hit = false;
                                    }
                                }
                                if self.rt_hit {
                                    if (self.lig_stack == 0i32) {
                                        {
                                            { let __ix1149 = self.main_p; let __v1150 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix1149) as usize].set_hh_b1(__v1150); }
                                            self.rt_hit = false;
                                        }
                                    }
                                }
                                { let __ix1151 = self.cur_q; let __v1152 = self.main_p; self.mem[(__ix1151) as usize].set_hh_rh(__v1152); }
                                self.cur_list.tail_field = self.main_p;
                                self.ligature_present = false;
                            }
                        }
                        if self.ins_disc {
                            {
                                self.ins_disc = false;
                                if (self.cur_list.mode_field > 0i32) {
                                    {
                                        { let __ix1153 = self.cur_list.tail_field; let __v1154 = self.new_disc(); self.mem[(__ix1153) as usize].set_hh_rh(__v1154); }
                                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 5 { // L90
                // §1034
                if (self.lig_stack == 0i32) {
                    // §1036
                    { __goto_1 = 2; continue 'l_dispatch_1; }
                }
                self.cur_q = self.cur_list.tail_field;
                self.cur_l = self.mem[(self.lig_stack) as usize].hh().b1();
            }
            if __goto_1 <= 6 { // L91
                if (!(self.lig_stack >= self.hi_mem_min)) {
                    { __goto_1 = 13; continue 'l_dispatch_1; }
                }
            }
            if __goto_1 <= 7 { // L92
                if ((self.cur_chr < self.font_bc[(self.main_f) as usize]) || (self.cur_chr > self.font_ec[(self.main_f) as usize])) {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            { let __ix1155 = self.lig_stack; let __v1156 = self.avail; self.mem[(__ix1155) as usize].set_hh_rh(__v1156); }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                self.main_i = self.font_info[((self.char_base[(self.main_f) as usize]).wrapping_add(self.cur_l)) as usize].qqqq();
                if (!(self.main_i.b0() > 0i32)) {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            { let __ix1157 = self.lig_stack; let __v1158 = self.avail; self.mem[(__ix1157) as usize].set_hh_rh(__v1158); }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                { let __ix1159 = self.cur_list.tail_field; let __v1160 = self.lig_stack; self.mem[(__ix1159) as usize].set_hh_rh(__v1160); }
                self.cur_list.tail_field = self.lig_stack;
            }
            if __goto_1 <= 8 { // L100
                // §1034
                self.get_next();
                // §1038
                if (self.cur_cmd == 11i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == 12i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == 68i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                self.x_token();
                if (self.cur_cmd == 11i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == 12i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == 68i32) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == 16i32) {
                    {
                        self.scan_char_num();
                        self.cur_chr = self.cur_val;
                        { __goto_1 = 9; continue 'l_dispatch_1; }
                    }
                }
                if (self.cur_cmd == 65i32) {
                    self.bchar = 256i32;
                }
                self.cur_r = self.bchar;
                self.lig_stack = 0i32;
                { __goto_1 = 10; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 9 { // L101
                self.main_s = self.eqtb[(((4751i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v1161 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v1161); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v1162 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v1162); }
                        }
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == 0i32) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail = self.mem[(self.lig_stack) as usize].hh().rh();
                            { let __ix1163 = self.lig_stack; self.mem[(__ix1163) as usize].set_hh_rh(0i32); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                { let __ix1164 = self.lig_stack; let __v1165 = self.main_f; self.mem[(__ix1164) as usize].set_hh_b0(__v1165); }
                self.cur_r = (self.cur_chr).wrapping_add(0i32);
                { let __ix1166 = self.lig_stack; let __v1167 = self.cur_r; self.mem[(__ix1166) as usize].set_hh_b1(__v1167); }
                if (self.cur_r == self.false_bchar) {
                    self.cur_r = 256i32;
                }
            }
            if __goto_1 <= 10 { // L110
                // §1034
                if (((self.main_i.b2()).wrapping_sub(0i32) % 4i32) != 1i32) {
                    // §1039
                    { __goto_1 = 4; continue 'l_dispatch_1; }
                }
                if (self.cur_r == 256i32) {
                    { __goto_1 = 4; continue 'l_dispatch_1; }
                }
                self.main_k = (self.lig_kern_base[(self.main_f) as usize]).wrapping_add(self.main_i.b3());
                self.main_j = self.font_info[(self.main_k) as usize].qqqq();
                if (self.main_j.b0() <= 128i32) {
                    { __goto_1 = 12; continue 'l_dispatch_1; }
                }
                self.main_k = ((((self.lig_kern_base[(self.main_f) as usize]).wrapping_add((256i32).wrapping_mul(self.main_j.b2()))).wrapping_add(self.main_j.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
            }
            if __goto_1 <= 11 { // L111
                self.main_j = self.font_info[(self.main_k) as usize].qqqq();
            }
            if __goto_1 <= 12 { // L112
                if (self.main_j.b1() == self.cur_r) {
                    if (self.main_j.b0() <= 128i32) {
                        // §1040
                        {
                            if (self.main_j.b2() >= 128i32) {
                                {
                                    if (self.cur_l < 256i32) {
                                        {
                                            if (self.mem[(self.cur_q) as usize].hh().rh() > 0i32) {
                                                if (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == (self.hyphen_char[(self.main_f) as usize]).wrapping_add(0i32)) {
                                                    self.ins_disc = true;
                                                }
                                            }
                                            if self.ligature_present {
                                                {
                                                    self.main_p = self.new_ligature(self.main_f, self.cur_l, self.mem[(self.cur_q) as usize].hh().rh());
                                                    if self.lft_hit {
                                                        {
                                                            { let __ix1168 = self.main_p; self.mem[(__ix1168) as usize].set_hh_b1(2i32); }
                                                            self.lft_hit = false;
                                                        }
                                                    }
                                                    if self.rt_hit {
                                                        if (self.lig_stack == 0i32) {
                                                            {
                                                                { let __ix1169 = self.main_p; let __v1170 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix1169) as usize].set_hh_b1(__v1170); }
                                                                self.rt_hit = false;
                                                            }
                                                        }
                                                    }
                                                    { let __ix1171 = self.cur_q; let __v1172 = self.main_p; self.mem[(__ix1171) as usize].set_hh_rh(__v1172); }
                                                    self.cur_list.tail_field = self.main_p;
                                                    self.ligature_present = false;
                                                }
                                            }
                                            if self.ins_disc {
                                                {
                                                    self.ins_disc = false;
                                                    if (self.cur_list.mode_field > 0i32) {
                                                        {
                                                            { let __ix1173 = self.cur_list.tail_field; let __v1174 = self.new_disc(); self.mem[(__ix1173) as usize].set_hh_rh(__v1174); }
                                                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        { let __ix1175 = self.cur_list.tail_field; let __v1176 = self.new_kern(self.font_info[(((self.kern_base[(self.main_f) as usize]).wrapping_add((256i32).wrapping_mul(self.main_j.b2()))).wrapping_add(self.main_j.b3())) as usize].int()); self.mem[(__ix1175) as usize].set_hh_rh(__v1176); }
                                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                    }
                                    { __goto_1 = 5; continue 'l_dispatch_1; }
                                }
                            }
                            if (self.cur_l == 256i32) {
                                self.lft_hit = true;
                            } else {
                                if (self.lig_stack == 0i32) {
                                    self.rt_hit = true;
                                }
                            }
                            {
                                if (self.interrupt != 0i32) {
                                    self.pause_for_instructions();
                                }
                            }
                            match self.main_j.b2() {
                                1 | 5 => {
                                    {
                                        self.cur_l = self.main_j.b3();
                                        self.main_i = self.font_info[((self.char_base[(self.main_f) as usize]).wrapping_add(self.cur_l)) as usize].qqqq();
                                        self.ligature_present = true;
                                    }
                                }
                                2 | 6 => {
                                    {
                                        self.cur_r = self.main_j.b3();
                                        if (self.lig_stack == 0i32) {
                                            {
                                                self.lig_stack = self.new_lig_item(self.cur_r);
                                                self.bchar = 256i32;
                                            }
                                        } else {
                                            if (self.lig_stack >= self.hi_mem_min) {
                                                {
                                                    self.main_p = self.lig_stack;
                                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                                    { let __ix1177 = (self.lig_stack).wrapping_add(1i32); let __v1178 = self.main_p; self.mem[(__ix1177) as usize].set_hh_rh(__v1178); }
                                                }
                                            } else {
                                                { let __ix1179 = self.lig_stack; let __v1180 = self.cur_r; self.mem[(__ix1179) as usize].set_hh_b1(__v1180); }
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    {
                                        self.cur_r = self.main_j.b3();
                                        self.main_p = self.lig_stack;
                                        self.lig_stack = self.new_lig_item(self.cur_r);
                                        { let __ix1181 = self.lig_stack; let __v1182 = self.main_p; self.mem[(__ix1181) as usize].set_hh_rh(__v1182); }
                                    }
                                }
                                7 | 11 => {
                                    {
                                        if (self.cur_l < 256i32) {
                                            {
                                                if (self.mem[(self.cur_q) as usize].hh().rh() > 0i32) {
                                                    if (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == (self.hyphen_char[(self.main_f) as usize]).wrapping_add(0i32)) {
                                                        self.ins_disc = true;
                                                    }
                                                }
                                                if self.ligature_present {
                                                    {
                                                        self.main_p = self.new_ligature(self.main_f, self.cur_l, self.mem[(self.cur_q) as usize].hh().rh());
                                                        if self.lft_hit {
                                                            {
                                                                { let __ix1183 = self.main_p; self.mem[(__ix1183) as usize].set_hh_b1(2i32); }
                                                                self.lft_hit = false;
                                                            }
                                                        }
                                                        if false {
                                                            if (self.lig_stack == 0i32) {
                                                                {
                                                                    { let __ix1184 = self.main_p; let __v1185 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix1184) as usize].set_hh_b1(__v1185); }
                                                                    self.rt_hit = false;
                                                                }
                                                            }
                                                        }
                                                        { let __ix1186 = self.cur_q; let __v1187 = self.main_p; self.mem[(__ix1186) as usize].set_hh_rh(__v1187); }
                                                        self.cur_list.tail_field = self.main_p;
                                                        self.ligature_present = false;
                                                    }
                                                }
                                                if self.ins_disc {
                                                    {
                                                        self.ins_disc = false;
                                                        if (self.cur_list.mode_field > 0i32) {
                                                            {
                                                                { let __ix1188 = self.cur_list.tail_field; let __v1189 = self.new_disc(); self.mem[(__ix1188) as usize].set_hh_rh(__v1189); }
                                                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.cur_q = self.cur_list.tail_field;
                                        self.cur_l = self.main_j.b3();
                                        self.main_i = self.font_info[((self.char_base[(self.main_f) as usize]).wrapping_add(self.cur_l)) as usize].qqqq();
                                        self.ligature_present = true;
                                    }
                                }
                                _ => {
                                    {
                                        self.cur_l = self.main_j.b3();
                                        self.ligature_present = true;
                                        if (self.lig_stack == 0i32) {
                                            { __goto_1 = 4; continue 'l_dispatch_1; }
                                        } else {
                                            { __goto_1 = 6; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            }
                            if (self.main_j.b2() > 4i32) {
                                if (self.main_j.b2() != 7i32) {
                                    { __goto_1 = 4; continue 'l_dispatch_1; }
                                }
                            }
                            if (self.cur_l < 256i32) {
                                { __goto_1 = 10; continue 'l_dispatch_1; }
                            }
                            self.main_k = self.bchar_label[(self.main_f) as usize];
                            { __goto_1 = 11; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §1039
                if (self.main_j.b0() == 0i32) {
                    self.main_k = (self.main_k).wrapping_add(1i32);
                } else {
                    {
                        if (self.main_j.b0() >= 128i32) {
                            { __goto_1 = 4; continue 'l_dispatch_1; }
                        }
                        self.main_k = ((self.main_k).wrapping_add(self.main_j.b0())).wrapping_add(1i32);
                    }
                }
                { __goto_1 = 11; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 13 { // L95
                // §1034
                self.main_p = self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh();
                // §1037
                if (self.main_p > 0i32) {
                    {
                        { let __ix1190 = self.cur_list.tail_field; let __v1191 = self.main_p; self.mem[(__ix1190) as usize].set_hh_rh(__v1191); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
                self.temp_ptr = self.lig_stack;
                self.lig_stack = self.mem[(self.temp_ptr) as usize].hh().rh();
                self.free_node(self.temp_ptr, 2i32);
                self.main_i = self.font_info[((self.char_base[(self.main_f) as usize]).wrapping_add(self.cur_l)) as usize].qqqq();
                self.ligature_present = true;
                if (self.lig_stack == 0i32) {
                    if (self.main_p > 0i32) {
                        { __goto_1 = 8; continue 'l_dispatch_1; }
                    } else {
                        self.cur_r = self.bchar;
                    }
                } else {
                    self.cur_r = self.mem[(self.lig_stack) as usize].hh().b1();
                }
                { __goto_1 = 10; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 14 { // L120
                // §1030
                if (self.eqtb[((2894i32) - 1) as usize].hh().rh() == 0i32) {
                    // §1041
                    {
                        // §1042
                        {
                            self.main_p = self.font_glue[(self.eqtb[((3934i32) - 1) as usize].hh().rh()) as usize];
                            if (self.main_p == 0i32) {
                                {
                                    self.main_p = self.new_spec(0i32);
                                    self.main_k = (self.param_base[(self.eqtb[((3934i32) - 1) as usize].hh().rh()) as usize]).wrapping_add(2i32);
                                    { let __ix1192 = (self.main_p).wrapping_add(1i32); let __v1193 = self.font_info[(self.main_k) as usize].int(); self.mem[(__ix1192) as usize].set_int(__v1193); }
                                    { let __ix1194 = (self.main_p).wrapping_add(2i32); let __v1195 = self.font_info[((self.main_k).wrapping_add(1i32)) as usize].int(); self.mem[(__ix1194) as usize].set_int(__v1195); }
                                    { let __ix1196 = (self.main_p).wrapping_add(3i32); let __v1197 = self.font_info[((self.main_k).wrapping_add(2i32)) as usize].int(); self.mem[(__ix1196) as usize].set_int(__v1197); }
                                    { let __ix1198 = self.eqtb[((3934i32) - 1) as usize].hh().rh(); let __v1199 = self.main_p; self.font_glue[(__ix1198) as usize] = __v1199; }
                                }
                            }
                        }
                        // §1041
                        self.temp_ptr = self.new_glue(self.main_p);
                    }
                } else {
                    self.temp_ptr = self.new_param_glue(12i32);
                }
                { let __ix1200 = self.cur_list.tail_field; let __v1201 = self.temp_ptr; self.mem[(__ix1200) as usize].set_hh_rh(__v1201); }
                self.cur_list.tail_field = self.temp_ptr;
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 15 { // exit
                // §1030
            }
            break 'l_dispatch_1;
        }
    }

    /// The `error` routine calls on `give_err_help` if help is requested from
    /// the `err_help` parameter.
    // §1284
    pub fn give_err_help(&mut self) {
        self.token_show(self.eqtb[((3421i32) - 1) as usize].hh().rh());
    }

    /// Here is the only place we use `pack_buffered_name`. This part of the program
    /// becomes active when a ``virgin'' \TeX\ is trying to get going, just after
    /// the preliminary initialization, or when the user is substituting another
    /// format file by typing `\.\&' after the initial `\.{**}' prompt.  The buffer
    /// contains the first line of input in `buffer[loc..(last-1)]`, where
    /// `loc<last` and `buffer[loc]<>" "`.
    /// @<Declare the function called `open_fmt_file`
    // §524
    pub fn open_fmt_file(&mut self) -> bool {
        let mut open_fmt_file: bool = false;
        let mut j: i32 = 0; // §524
        'l_exit_f: {
            'l_found_f: {
                j = self.cur_input.loc_field;
                if (self.buffer[(self.cur_input.loc_field) as usize] == 38i32) {
                    {
                        self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                        j = self.cur_input.loc_field;
                        self.buffer[(self.last) as usize] = 32i32;
                        while (self.buffer[(j) as usize] != 32i32) {
                            j = (j).wrapping_add(1i32);
                        }
                        self.pack_buffered_name(0i32, self.cur_input.loc_field, (j).wrapping_sub(1i32));
                        if { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_in(&mut __f); self.fmt_file = __f; __r } {
                            break 'l_found_f;
                        }
                        self.pack_buffered_name(11i32, self.cur_input.loc_field, (j).wrapping_sub(1i32));
                        if { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_in(&mut __f); self.fmt_file = __f; __r } {
                            break 'l_found_f;
                        }
                        {
                            crate::system::wr_str(&mut self.term_out, "Sorry, I can't find that format;");
                            crate::system::wr_str(&mut self.term_out, " will try PLAIN.");
                            crate::system::wr_ln(&mut self.term_out);
                        }
                        crate::system::break_out(&mut self.term_out);
                    }
                }
                self.pack_buffered_name(16i32, 1i32, 0i32);
                if (!{ let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_in(&mut __f); self.fmt_file = __f; __r }) {
                    {
                        {
                            crate::system::wr_str(&mut self.term_out, "I can't find the PLAIN format file!");
                            crate::system::wr_ln(&mut self.term_out);
                        }
                        open_fmt_file = false;
                        break 'l_exit_f;
                    }
                }
            }
            self.cur_input.loc_field = j;
            open_fmt_file = true;
        }
        open_fmt_file
    }

    /// Corresponding to the procedure that dumps a format file, we have a function
    /// that reads one in. The function returns `false` if the dumped format is
    /// incompatible with the present \TeX\ table sizes, etc.
    // §1303
    pub fn load_fmt_file(&mut self) -> bool {
        let mut load_fmt_file: bool = false;
        let mut j: i32 = 0; // §1303
        let mut k: i32 = 0; // §1303
        let mut p: halfword = 0; // §1303
        let mut q: halfword = 0; // §1303
        let mut x: i32 = 0; // §1303
        let mut w: four_quarters = four_quarters::default(); // §1303
        'l_exit_f: {
            'l_L6666_f: {
                // §1308
                x = self.fmt_file.buf.int();
                if (x != 504454778i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 0i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 30000i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 6106i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 1777i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 307i32) {
                    break 'l_L6666_f;
                }
                // §1310
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 0i32) {
                        break 'l_L6666_f;
                    }
                    if (x > pool_size) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "string pool size");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        self.pool_ptr = x;
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 0i32) {
                        break 'l_L6666_f;
                    }
                    if (x > max_strings) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "max strings");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        self.str_ptr = x;
                    }
                }
                {
                    let __for_end_4 = self.str_ptr;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                x = self.fmt_file.buf.int();
                            }
                            if ((x < 0i32) || (x > self.pool_ptr)) {
                                break 'l_L6666_f;
                            } else {
                                self.str_start[(k) as usize] = x;
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                k = 0i32;
                while ((k).wrapping_add(4i32) < self.pool_ptr) {
                    {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            w = self.fmt_file.buf.qqqq();
                        }
                        self.str_pool[(k) as usize] = (w.b0()).wrapping_sub(0i32);
                        self.str_pool[((k).wrapping_add(1i32)) as usize] = (w.b1()).wrapping_sub(0i32);
                        self.str_pool[((k).wrapping_add(2i32)) as usize] = (w.b2()).wrapping_sub(0i32);
                        self.str_pool[((k).wrapping_add(3i32)) as usize] = (w.b3()).wrapping_sub(0i32);
                        k = (k).wrapping_add(4i32);
                    }
                }
                k = (self.pool_ptr).wrapping_sub(4i32);
                {
                    crate::system::get_word(&mut self.fmt_file);
                    w = self.fmt_file.buf.qqqq();
                }
                self.str_pool[(k) as usize] = (w.b0()).wrapping_sub(0i32);
                self.str_pool[((k).wrapping_add(1i32)) as usize] = (w.b1()).wrapping_sub(0i32);
                self.str_pool[((k).wrapping_add(2i32)) as usize] = (w.b2()).wrapping_sub(0i32);
                self.str_pool[((k).wrapping_add(3i32)) as usize] = (w.b3()).wrapping_sub(0i32);
                self.init_str_ptr = self.str_ptr;
                self.init_pool_ptr = self.pool_ptr;
                // §1312
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1019i32) || (x > 29986i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.lo_mem_max = x;
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 20i32) || (x > self.lo_mem_max)) {
                        break 'l_L6666_f;
                    } else {
                        self.rover = x;
                    }
                }
                p = 0i32;
                q = self.rover;
                loop {
                    {
                        let __for_end_5 = (q).wrapping_add(1i32);
                        k = p;
                        while k <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1202 = self.fmt_file.buf; self.mem[(k) as usize] = __v1202; }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    p = (q).wrapping_add(self.mem[(q) as usize].hh().lh());
                    if ((p > self.lo_mem_max) || ((q >= self.mem[((q).wrapping_add(1i32)) as usize].hh().rh()) && (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() != self.rover))) {
                        break 'l_L6666_f;
                    }
                    q = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                    if (q == self.rover) { break; }
                }
                {
                    let __for_end_4 = self.lo_mem_max;
                    k = p;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v1203 = self.fmt_file.buf; self.mem[(k) as usize] = __v1203; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                if (mem_min < (2i32).wrapping_neg()) {
                    {
                        p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
                        q = (mem_min).wrapping_add(1i32);
                        { let __ix1204 = mem_min; self.mem[(__ix1204) as usize].set_hh_rh(0i32); }
                        { let __ix1205 = mem_min; self.mem[(__ix1205) as usize].set_hh_lh(0i32); }
                        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(q);
                        { let __ix1206 = (self.rover).wrapping_add(1i32); self.mem[(__ix1206) as usize].set_hh_lh(q); }
                        { let __v1207 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v1207); }
                        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(p);
                        self.mem[(q) as usize].set_hh_rh(65535i32);
                        self.mem[(q) as usize].set_hh_lh(((0i32).wrapping_neg()).wrapping_sub(q));
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < (self.lo_mem_max).wrapping_add(1i32)) || (x > 29987i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.hi_mem_min = x;
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > 30000i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.avail = x;
                    }
                }
                self.mem_end = 30000i32;
                {
                    let __for_end_4 = self.mem_end;
                    k = self.hi_mem_min;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v1208 = self.fmt_file.buf; self.mem[(k) as usize] = __v1208; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.var_used = self.fmt_file.buf.int();
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.dyn_used = self.fmt_file.buf.int();
                }
                // §1317
                k = 1i32;
                loop {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1i32) || ((k).wrapping_add(x) > 6107i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1209 = self.fmt_file.buf; self.eqtb[((j) - 1) as usize] = __v1209; }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || ((k).wrapping_add(x) > 6107i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            { let __v1210 = self.eqtb[(((k).wrapping_sub(1i32)) - 1) as usize]; self.eqtb[((j) - 1) as usize] = __v1210; }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    if (k > 6106i32) { break; }
                }
                // §1314
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 514i32) || (x > 2614i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.par_loc = x;
                    }
                }
                self.par_token = (4095i32).wrapping_add(self.par_loc);
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 514i32) || (x > 2614i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.write_loc = x;
                    }
                }
                // §1319
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 514i32) || (x > 2614i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.hash_used = x;
                    }
                }
                p = 513i32;
                loop {
                    {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            x = self.fmt_file.buf.int();
                        }
                        if ((x < (p).wrapping_add(1i32)) || (x > self.hash_used)) {
                            break 'l_L6666_f;
                        } else {
                            p = x;
                        }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v1211 = self.fmt_file.buf.hh(); self.hash[((p) - 514) as usize] = __v1211; }
                    }
                    if (p == self.hash_used) { break; }
                }
                {
                    let __for_end_4 = 2880i32;
                    p = (self.hash_used).wrapping_add(1i32);
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v1212 = self.fmt_file.buf.hh(); self.hash[((p) - 514) as usize] = __v1212; }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.cs_count = self.fmt_file.buf.int();
                }
                // §1321
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 7i32) {
                        break 'l_L6666_f;
                    }
                    if (x > font_mem_size) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "font mem size");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        self.fmem_ptr = x;
                    }
                }
                {
                    let __for_end_4 = (self.fmem_ptr).wrapping_sub(1i32);
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v1213 = self.fmt_file.buf; self.font_info[(k) as usize] = __v1213; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 0i32) {
                        break 'l_L6666_f;
                    }
                    if (x > font_max) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "font max");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        self.font_ptr = x;
                    }
                }
                {
                    let __for_end_4 = self.font_ptr;
                    k = 0i32;
                    while k <= __for_end_4 {
                        // §1323
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1214 = self.fmt_file.buf.qqqq(); self.font_check[(k) as usize] = __v1214; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1215 = self.fmt_file.buf.int(); self.font_size[(k) as usize] = __v1215; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1216 = self.fmt_file.buf.int(); self.font_dsize[(k) as usize] = __v1216; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 65535i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_params[(k) as usize] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1217 = self.fmt_file.buf.int(); self.hyphen_char[(k) as usize] = __v1217; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1218 = self.fmt_file.buf.int(); self.skew_char[(k) as usize] = __v1218; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.str_ptr)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_name[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.str_ptr)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_area[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 255i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_bc[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 255i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_ec[(k) as usize] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1219 = self.fmt_file.buf.int(); self.char_base[(k) as usize] = __v1219; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1220 = self.fmt_file.buf.int(); self.width_base[(k) as usize] = __v1220; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1221 = self.fmt_file.buf.int(); self.height_base[(k) as usize] = __v1221; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1222 = self.fmt_file.buf.int(); self.depth_base[(k) as usize] = __v1222; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1223 = self.fmt_file.buf.int(); self.italic_base[(k) as usize] = __v1223; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1224 = self.fmt_file.buf.int(); self.lig_kern_base[(k) as usize] = __v1224; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1225 = self.fmt_file.buf.int(); self.kern_base[(k) as usize] = __v1225; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1226 = self.fmt_file.buf.int(); self.exten_base[(k) as usize] = __v1226; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v1227 = self.fmt_file.buf.int(); self.param_base[(k) as usize] = __v1227; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.lo_mem_max)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_glue[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > (self.fmem_ptr).wrapping_sub(1i32))) {
                                    break 'l_L6666_f;
                                } else {
                                    self.bchar_label[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 256i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_bchar[(k) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 256i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_false_bchar[(k) as usize] = x;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §1325
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > 307i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.hyph_count = x;
                    }
                }
                {
                    let __for_end_4 = self.hyph_count;
                    k = 1i32;
                    while k <= __for_end_4 {
                        {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 307i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    j = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.str_ptr)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyph_word[(j) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 65535i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyph_list[(j) as usize] = x;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 0i32) {
                        break 'l_L6666_f;
                    }
                    if (x > trie_size) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "trie size");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        j = x;
                    }
                }
                self.trie_max = j;
                {
                    let __for_end_4 = j;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v1228 = self.fmt_file.buf.hh(); self.trie[(k) as usize] = __v1228; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < 0i32) {
                        break 'l_L6666_f;
                    }
                    if (x > trie_op_size) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "---! Must increase the ");
                                crate::system::wr_str(&mut self.term_out, "trie op size");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        j = x;
                    }
                }
                self.trie_op_ptr = j;
                {
                    let __for_end_4 = j;
                    k = 1i32;
                    while k <= __for_end_4 {
                        {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 63i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyf_distance[((k) - 1) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 63i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyf_num[((k) - 1) as usize] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 255i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyf_next[((k) - 1) as usize] = x;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    let __for_end_4 = 255i32;
                    k = 0i32;
                    while k <= __for_end_4 {
                        self.trie_used[(k) as usize] = 0i32;
                        k = k.wrapping_add(1);
                    }
                }
                k = 256i32;
                while (j > 0i32) {
                    {
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                x = self.fmt_file.buf.int();
                            }
                            if ((x < 0i32) || (x > (k).wrapping_sub(1i32))) {
                                break 'l_L6666_f;
                            } else {
                                k = x;
                            }
                        }
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                x = self.fmt_file.buf.int();
                            }
                            if ((x < 1i32) || (x > j)) {
                                break 'l_L6666_f;
                            } else {
                                x = x;
                            }
                        }
                        self.trie_used[(k) as usize] = (x).wrapping_add(0i32);
                        j = (j).wrapping_sub(x);
                        self.op_start[(k) as usize] = (j).wrapping_sub(0i32);
                    }
                }
                self.trie_not_ready = false;
                // §1327
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > 3i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.interaction = x;
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > self.str_ptr)) {
                        break 'l_L6666_f;
                    } else {
                        self.format_ident = x;
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if ((x != 69069i32) || crate::system::eof(&self.fmt_file)) {
                    break 'l_L6666_f;
                }
                // §1303
                load_fmt_file = true;
                break 'l_exit_f;
            }
            {
                crate::system::wr_str(&mut self.term_out, "(Fatal format file error; I'm stymied)");
                crate::system::wr_ln(&mut self.term_out);
            }
            load_fmt_file = false;
        }
        load_fmt_file
    }

    /// Here we do whatever is needed to complete \TeX's job gracefully on the
    /// local operating system. The code here might come into play after a fatal
    /// error; it must therefore consist entirely of ``safe'' operations that
    /// cannot produce error messages. For example, it would be a mistake to call
    /// `str_room` or `make_string` at this time, because a call on `overflow`
    /// might lead to an infinite loop.
    /// (Actually there's one way to get error messages, via `prepare_mag`;
    /// but that can't cause infinite recursion.)
    /// If `final_cleanup` is bypassed, this program doesn't bother to close
    /// the input files that may still be open.
    /// @<Last-minute...
    // §1333
    pub fn close_files_and_terminate(&mut self) {
        let mut k: i32 = 0; // §1333
        // §1378
        {
            let __for_end_2 = 15i32;
            k = 0i32;
            while k <= __for_end_2 {
                if self.write_open[(k) as usize] {
                    { let mut __f = ::core::mem::take(&mut self.write_file[(k) as usize]); let __r = self.a_close(&mut __f); self.write_file[(k) as usize] = __f; __r };
                }
                k = k.wrapping_add(1);
            }
        }
        // §1333
        self.eqtb[((5312i32) - 1) as usize].set_int((1i32).wrapping_neg());
        if (self.eqtb[((5294i32) - 1) as usize].int() > 0i32) {
            // §1334
            if self.log_opened {
                {
                    {
                        let __w0 = b' ';
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, "Here is how much of TeX's memory");
                        crate::system::wr_str(&mut self.log_file, " you used:");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = (self.str_ptr).wrapping_sub(self.init_str_ptr);
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " string");
                    }
                    if (self.str_ptr != (self.init_str_ptr).wrapping_add(1i32)) {
                        {
                            let __w0 = b's';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                    }
                    {
                        let __w1 = (max_strings).wrapping_sub(self.init_str_ptr);
                        crate::system::wr_str(&mut self.log_file, " out of ");
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = (self.pool_ptr).wrapping_sub(self.init_pool_ptr);
                        let __w3 = (pool_size).wrapping_sub(self.init_pool_ptr);
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " string characters out of ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = ((((self.lo_mem_max).wrapping_sub(mem_min)).wrapping_add(self.mem_end)).wrapping_sub(self.hi_mem_min)).wrapping_add(2i32);
                        let __w3 = ((self.mem_end).wrapping_add(1i32)).wrapping_sub(mem_min);
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " words of memory out of ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = self.cs_count;
                        let __w3 = 2100i32;
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " multiletter control sequences out of ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = self.fmem_ptr;
                        let __w3 = (self.font_ptr).wrapping_sub(0i32);
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " words of font info for ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_str(&mut self.log_file, " font");
                    }
                    if (self.font_ptr != 1i32) {
                        {
                            let __w0 = b's';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                    }
                    {
                        let __w1 = font_mem_size;
                        let __w3 = (font_max).wrapping_sub(0i32);
                        crate::system::wr_str(&mut self.log_file, ", out of ");
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " for ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = self.hyph_count;
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " hyphenation exception");
                    }
                    if (self.hyph_count != 1i32) {
                        {
                            let __w0 = b's';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                    }
                    {
                        let __w1 = 307i32;
                        crate::system::wr_str(&mut self.log_file, " out of ");
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        let __w1 = self.max_in_stack;
                        let __w3 = self.max_nest_stack;
                        let __w5 = self.max_param_stack;
                        let __w7 = (self.max_buf_stack).wrapping_add(1i32);
                        let __w9 = (self.max_save_stack).wrapping_add(6i32);
                        let __w11 = stack_size;
                        let __w13 = nest_size;
                        let __w15 = param_size;
                        let __w17 = buf_size;
                        let __w19 = save_size;
                        let __w20 = b's';
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, "i,");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_str(&mut self.log_file, "n,");
                        crate::system::wr_int(&mut self.log_file, __w5, 1i32);
                        crate::system::wr_str(&mut self.log_file, "p,");
                        crate::system::wr_int(&mut self.log_file, __w7, 1i32);
                        crate::system::wr_str(&mut self.log_file, "b,");
                        crate::system::wr_int(&mut self.log_file, __w9, 1i32);
                        crate::system::wr_str(&mut self.log_file, "s stack positions out of ");
                        crate::system::wr_int(&mut self.log_file, __w11, 1i32);
                        crate::system::wr_str(&mut self.log_file, "i,");
                        crate::system::wr_int(&mut self.log_file, __w13, 1i32);
                        crate::system::wr_str(&mut self.log_file, "n,");
                        crate::system::wr_int(&mut self.log_file, __w15, 1i32);
                        crate::system::wr_str(&mut self.log_file, "p,");
                        crate::system::wr_int(&mut self.log_file, __w17, 1i32);
                        crate::system::wr_str(&mut self.log_file, "b,");
                        crate::system::wr_int(&mut self.log_file, __w19, 1i32);
                        crate::system::wr_char(&mut self.log_file, __w20);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                }
            }
        }
        // §642
        while (self.cur_s > (1i32).wrapping_neg()) {
            {
                if (self.cur_s > 0i32) {
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = 142i32;
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                } else {
                    {
                        {
                            self.dvi_buf[(self.dvi_ptr) as usize] = 140i32;
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        self.total_pages = (self.total_pages).wrapping_add(1i32);
                    }
                }
                self.cur_s = (self.cur_s).wrapping_sub(1i32);
            }
        }
        if (self.total_pages == 0i32) {
            self.print_nl(837i32);
        } else {
            {
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = 248i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                self.dvi_four(self.last_bop);
                self.last_bop = ((self.dvi_offset).wrapping_add(self.dvi_ptr)).wrapping_sub(5i32);
                self.dvi_four(25400000i32);
                self.dvi_four(473628672i32);
                self.prepare_mag();
                self.dvi_four(self.eqtb[((5280i32) - 1) as usize].int());
                self.dvi_four(self.max_v);
                self.dvi_four(self.max_h);
                {
                    { let __ix1229 = self.dvi_ptr; let __v1230 = (self.max_push / 256i32); self.dvi_buf[(__ix1229) as usize] = __v1230; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix1231 = self.dvi_ptr; let __v1232 = (self.max_push % 256i32); self.dvi_buf[(__ix1231) as usize] = __v1232; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix1233 = self.dvi_ptr; let __v1234 = ((self.total_pages / 256i32) % 256i32); self.dvi_buf[(__ix1233) as usize] = __v1234; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix1235 = self.dvi_ptr; let __v1236 = (self.total_pages % 256i32); self.dvi_buf[(__ix1235) as usize] = __v1236; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                // §643
                while (self.font_ptr > 0i32) {
                    {
                        if self.font_used[(self.font_ptr) as usize] {
                            self.dvi_font_def(self.font_ptr);
                        }
                        self.font_ptr = (self.font_ptr).wrapping_sub(1i32);
                    }
                }
                // §642
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = 249i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                self.dvi_four(self.last_bop);
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = 2i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = (4i32).wrapping_add(((dvi_buf_size).wrapping_sub(self.dvi_ptr) % 4i32));
                while (k > 0i32) {
                    {
                        {
                            self.dvi_buf[(self.dvi_ptr) as usize] = 223i32;
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        k = (k).wrapping_sub(1i32);
                    }
                }
                // §599
                if (self.dvi_limit == self.half_buf) {
                    self.write_dvi(self.half_buf, (dvi_buf_size).wrapping_sub(1i32));
                }
                if (self.dvi_ptr > 0i32) {
                    self.write_dvi(0i32, (self.dvi_ptr).wrapping_sub(1i32));
                }
                // §642
                self.print_nl(838i32);
                self.slow_print(self.output_file_name);
                self.print(286i32);
                self.print_int(self.total_pages);
                self.print(839i32);
                if (self.total_pages != 1i32) {
                    self.print_char(115i32);
                }
                self.print(840i32);
                self.print_int((self.dvi_offset).wrapping_add(self.dvi_ptr));
                self.print(841i32);
                { let mut __f = ::core::mem::take(&mut self.dvi_file); let __r = self.b_close(&mut __f); self.dvi_file = __f; __r };
            }
        }
        // §1333
        if self.log_opened {
            {
                {
                    crate::system::wr_ln(&mut self.log_file);
                }
                { let mut __f = ::core::mem::take(&mut self.log_file); let __r = self.a_close(&mut __f); self.log_file = __f; __r };
                self.selector = (self.selector).wrapping_sub(2i32);
                if (self.selector == 17i32) {
                    {
                        self.print_nl(1275i32);
                        self.slow_print(self.log_name);
                        self.print_char(46i32);
                    }
                }
            }
        }
    }

    /// We get to the `final_cleanup` routine when \.{\\end} or \.{\\dump} has
    /// been scanned and `its_all_over`\kern-2pt.
    /// @<Last-minute...
    // §1335
    pub fn final_cleanup(&mut self) {
        let mut c: small_number = 0; // §1335
        'l_exit_f: {
            c = self.cur_chr;
            if (c != 1i32) {
                self.eqtb[((5312i32) - 1) as usize].set_int((1i32).wrapping_neg());
            }
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            while (self.input_ptr > 0i32) {
                if (self.cur_input.state_field == 0i32) {
                    self.end_token_list();
                } else {
                    self.end_file_reading();
                }
            }
            while (self.open_parens > 0i32) {
                {
                    self.print(1276i32);
                    self.open_parens = (self.open_parens).wrapping_sub(1i32);
                }
            }
            if (self.cur_level > 1i32) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1277i32);
                    self.print(1278i32);
                    self.print_int((self.cur_level).wrapping_sub(1i32));
                    self.print_char(41i32);
                }
            }
            while (self.cond_ptr != 0i32) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1277i32);
                    self.print(1279i32);
                    self.print_cmd_chr(105i32, self.cur_if);
                    if (self.if_line != 0i32) {
                        {
                            self.print(1280i32);
                            self.print_int(self.if_line);
                        }
                    }
                    self.print(1281i32);
                    self.if_line = self.mem[((self.cond_ptr).wrapping_add(1i32)) as usize].int();
                    self.cur_if = self.mem[(self.cond_ptr) as usize].hh().b1();
                    self.temp_ptr = self.cond_ptr;
                    self.cond_ptr = self.mem[(self.cond_ptr) as usize].hh().rh();
                    self.free_node(self.temp_ptr, 2i32);
                }
            }
            if (self.history != 0i32) {
                if ((self.history == 1i32) || (self.interaction < 3i32)) {
                    if (self.selector == 19i32) {
                        {
                            self.selector = 17i32;
                            self.print_nl(1282i32);
                            self.selector = 19i32;
                        }
                    }
                }
            }
            if (c == 1i32) {
                {
                    {
                        let __for_end_5 = 4i32;
                        c = 0i32;
                        while c <= __for_end_5 {
                            if (self.cur_mark[(c) as usize] != 0i32) {
                                self.delete_token_ref(self.cur_mark[(c) as usize]);
                            }
                            c = c.wrapping_add(1);
                        }
                    }
                    if (self.last_glue != 65535i32) {
                        self.delete_glue_ref(self.last_glue);
                    }
                    self.store_fmt_file();
                    break 'l_exit_f;
                    self.print_nl(1283i32);
                    break 'l_exit_f;
                }
            }
        }
    }

    /// @<Last-minute...
    // §1336
    pub fn init_prim(&mut self) {
        self.no_new_control_sequence = false;
        // §226
        self.primitive(376i32, 75i32, 2882i32);
        self.primitive(377i32, 75i32, 2883i32);
        self.primitive(378i32, 75i32, 2884i32);
        self.primitive(379i32, 75i32, 2885i32);
        self.primitive(380i32, 75i32, 2886i32);
        self.primitive(381i32, 75i32, 2887i32);
        self.primitive(382i32, 75i32, 2888i32);
        self.primitive(383i32, 75i32, 2889i32);
        self.primitive(384i32, 75i32, 2890i32);
        self.primitive(385i32, 75i32, 2891i32);
        self.primitive(386i32, 75i32, 2892i32);
        self.primitive(387i32, 75i32, 2893i32);
        self.primitive(388i32, 75i32, 2894i32);
        self.primitive(389i32, 75i32, 2895i32);
        self.primitive(390i32, 75i32, 2896i32);
        self.primitive(391i32, 76i32, 2897i32);
        self.primitive(392i32, 76i32, 2898i32);
        self.primitive(393i32, 76i32, 2899i32);
        // §230
        self.primitive(398i32, 72i32, 3413i32);
        self.primitive(399i32, 72i32, 3414i32);
        self.primitive(400i32, 72i32, 3415i32);
        self.primitive(401i32, 72i32, 3416i32);
        self.primitive(402i32, 72i32, 3417i32);
        self.primitive(403i32, 72i32, 3418i32);
        self.primitive(404i32, 72i32, 3419i32);
        self.primitive(405i32, 72i32, 3420i32);
        self.primitive(406i32, 72i32, 3421i32);
        // §238
        self.primitive(420i32, 73i32, 5263i32);
        self.primitive(421i32, 73i32, 5264i32);
        self.primitive(422i32, 73i32, 5265i32);
        self.primitive(423i32, 73i32, 5266i32);
        self.primitive(424i32, 73i32, 5267i32);
        self.primitive(425i32, 73i32, 5268i32);
        self.primitive(426i32, 73i32, 5269i32);
        self.primitive(427i32, 73i32, 5270i32);
        self.primitive(428i32, 73i32, 5271i32);
        self.primitive(429i32, 73i32, 5272i32);
        self.primitive(430i32, 73i32, 5273i32);
        self.primitive(431i32, 73i32, 5274i32);
        self.primitive(432i32, 73i32, 5275i32);
        self.primitive(433i32, 73i32, 5276i32);
        self.primitive(434i32, 73i32, 5277i32);
        self.primitive(435i32, 73i32, 5278i32);
        self.primitive(436i32, 73i32, 5279i32);
        self.primitive(437i32, 73i32, 5280i32);
        self.primitive(438i32, 73i32, 5281i32);
        self.primitive(439i32, 73i32, 5282i32);
        self.primitive(440i32, 73i32, 5283i32);
        self.primitive(441i32, 73i32, 5284i32);
        self.primitive(442i32, 73i32, 5285i32);
        self.primitive(443i32, 73i32, 5286i32);
        self.primitive(444i32, 73i32, 5287i32);
        self.primitive(445i32, 73i32, 5288i32);
        self.primitive(446i32, 73i32, 5289i32);
        self.primitive(447i32, 73i32, 5290i32);
        self.primitive(448i32, 73i32, 5291i32);
        self.primitive(449i32, 73i32, 5292i32);
        self.primitive(450i32, 73i32, 5293i32);
        self.primitive(451i32, 73i32, 5294i32);
        self.primitive(452i32, 73i32, 5295i32);
        self.primitive(453i32, 73i32, 5296i32);
        self.primitive(454i32, 73i32, 5297i32);
        self.primitive(455i32, 73i32, 5298i32);
        self.primitive(456i32, 73i32, 5299i32);
        self.primitive(457i32, 73i32, 5300i32);
        self.primitive(458i32, 73i32, 5301i32);
        self.primitive(459i32, 73i32, 5302i32);
        self.primitive(460i32, 73i32, 5303i32);
        self.primitive(461i32, 73i32, 5304i32);
        self.primitive(462i32, 73i32, 5305i32);
        self.primitive(463i32, 73i32, 5306i32);
        self.primitive(464i32, 73i32, 5307i32);
        self.primitive(465i32, 73i32, 5308i32);
        self.primitive(466i32, 73i32, 5309i32);
        self.primitive(467i32, 73i32, 5310i32);
        self.primitive(468i32, 73i32, 5311i32);
        self.primitive(469i32, 73i32, 5312i32);
        self.primitive(470i32, 73i32, 5313i32);
        self.primitive(471i32, 73i32, 5314i32);
        self.primitive(472i32, 73i32, 5315i32);
        self.primitive(473i32, 73i32, 5316i32);
        self.primitive(474i32, 73i32, 5317i32);
        // §248
        self.primitive(478i32, 74i32, 5830i32);
        self.primitive(479i32, 74i32, 5831i32);
        self.primitive(480i32, 74i32, 5832i32);
        self.primitive(481i32, 74i32, 5833i32);
        self.primitive(482i32, 74i32, 5834i32);
        self.primitive(483i32, 74i32, 5835i32);
        self.primitive(484i32, 74i32, 5836i32);
        self.primitive(485i32, 74i32, 5837i32);
        self.primitive(486i32, 74i32, 5838i32);
        self.primitive(487i32, 74i32, 5839i32);
        self.primitive(488i32, 74i32, 5840i32);
        self.primitive(489i32, 74i32, 5841i32);
        self.primitive(490i32, 74i32, 5842i32);
        self.primitive(491i32, 74i32, 5843i32);
        self.primitive(492i32, 74i32, 5844i32);
        self.primitive(493i32, 74i32, 5845i32);
        self.primitive(494i32, 74i32, 5846i32);
        self.primitive(495i32, 74i32, 5847i32);
        self.primitive(496i32, 74i32, 5848i32);
        self.primitive(497i32, 74i32, 5849i32);
        self.primitive(498i32, 74i32, 5850i32);
        // §265
        self.primitive(32i32, 64i32, 0i32);
        self.primitive(47i32, 44i32, 0i32);
        self.primitive(508i32, 45i32, 0i32);
        self.primitive(509i32, 90i32, 0i32);
        self.primitive(510i32, 40i32, 0i32);
        self.primitive(511i32, 41i32, 0i32);
        self.primitive(512i32, 61i32, 0i32);
        self.primitive(513i32, 16i32, 0i32);
        self.primitive(504i32, 107i32, 0i32);
        self.primitive(514i32, 15i32, 0i32);
        self.primitive(515i32, 92i32, 0i32);
        self.primitive(505i32, 67i32, 0i32);
        self.primitive(516i32, 62i32, 0i32);
        self.hash[((2616i32) - 514) as usize].set_rh(516i32);
        { let __v1237 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2616i32) - 1) as usize] = __v1237; }
        self.primitive(517i32, 102i32, 0i32);
        self.primitive(518i32, 88i32, 0i32);
        self.primitive(519i32, 77i32, 0i32);
        self.primitive(520i32, 32i32, 0i32);
        self.primitive(521i32, 36i32, 0i32);
        self.primitive(522i32, 39i32, 0i32);
        self.primitive(330i32, 37i32, 0i32);
        self.primitive(351i32, 18i32, 0i32);
        self.primitive(523i32, 46i32, 0i32);
        self.primitive(524i32, 17i32, 0i32);
        self.primitive(525i32, 54i32, 0i32);
        self.primitive(526i32, 91i32, 0i32);
        self.primitive(527i32, 34i32, 0i32);
        self.primitive(528i32, 65i32, 0i32);
        self.primitive(529i32, 103i32, 0i32);
        self.primitive(335i32, 55i32, 0i32);
        self.primitive(530i32, 63i32, 0i32);
        self.primitive(408i32, 84i32, 0i32);
        self.primitive(531i32, 42i32, 0i32);
        self.primitive(532i32, 80i32, 0i32);
        self.primitive(533i32, 66i32, 0i32);
        self.primitive(534i32, 96i32, 0i32);
        self.primitive(535i32, 0i32, 256i32);
        self.hash[((2621i32) - 514) as usize].set_rh(535i32);
        { let __v1238 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2621i32) - 1) as usize] = __v1238; }
        self.primitive(536i32, 98i32, 0i32);
        self.primitive(537i32, 109i32, 0i32);
        self.primitive(407i32, 71i32, 0i32);
        self.primitive(352i32, 38i32, 0i32);
        self.primitive(538i32, 33i32, 0i32);
        self.primitive(539i32, 56i32, 0i32);
        self.primitive(540i32, 35i32, 0i32);
        // §334
        self.primitive(597i32, 13i32, 256i32);
        self.par_loc = self.cur_val;
        self.par_token = (4095i32).wrapping_add(self.par_loc);
        // §376
        self.primitive(629i32, 104i32, 0i32);
        self.primitive(630i32, 104i32, 1i32);
        // §384
        self.primitive(631i32, 110i32, 0i32);
        self.primitive(632i32, 110i32, 1i32);
        self.primitive(633i32, 110i32, 2i32);
        self.primitive(634i32, 110i32, 3i32);
        self.primitive(635i32, 110i32, 4i32);
        // §411
        self.primitive(476i32, 89i32, 0i32);
        self.primitive(500i32, 89i32, 1i32);
        self.primitive(395i32, 89i32, 2i32);
        self.primitive(396i32, 89i32, 3i32);
        // §416
        self.primitive(668i32, 79i32, 102i32);
        self.primitive(669i32, 79i32, 1i32);
        self.primitive(670i32, 82i32, 0i32);
        self.primitive(671i32, 82i32, 1i32);
        self.primitive(672i32, 83i32, 1i32);
        self.primitive(673i32, 83i32, 3i32);
        self.primitive(674i32, 83i32, 2i32);
        self.primitive(675i32, 70i32, 0i32);
        self.primitive(676i32, 70i32, 1i32);
        self.primitive(677i32, 70i32, 2i32);
        self.primitive(678i32, 70i32, 3i32);
        self.primitive(679i32, 70i32, 4i32);
        // §468
        self.primitive(735i32, 108i32, 0i32);
        self.primitive(736i32, 108i32, 1i32);
        self.primitive(737i32, 108i32, 2i32);
        self.primitive(738i32, 108i32, 3i32);
        self.primitive(739i32, 108i32, 4i32);
        self.primitive(740i32, 108i32, 5i32);
        // §487
        self.primitive(757i32, 105i32, 0i32);
        self.primitive(758i32, 105i32, 1i32);
        self.primitive(759i32, 105i32, 2i32);
        self.primitive(760i32, 105i32, 3i32);
        self.primitive(761i32, 105i32, 4i32);
        self.primitive(762i32, 105i32, 5i32);
        self.primitive(763i32, 105i32, 6i32);
        self.primitive(764i32, 105i32, 7i32);
        self.primitive(765i32, 105i32, 8i32);
        self.primitive(766i32, 105i32, 9i32);
        self.primitive(767i32, 105i32, 10i32);
        self.primitive(768i32, 105i32, 11i32);
        self.primitive(769i32, 105i32, 12i32);
        self.primitive(770i32, 105i32, 13i32);
        self.primitive(771i32, 105i32, 14i32);
        self.primitive(772i32, 105i32, 15i32);
        self.primitive(773i32, 105i32, 16i32);
        // §491
        self.primitive(774i32, 106i32, 2i32);
        self.hash[((2618i32) - 514) as usize].set_rh(774i32);
        { let __v1239 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2618i32) - 1) as usize] = __v1239; }
        self.primitive(775i32, 106i32, 4i32);
        self.primitive(776i32, 106i32, 3i32);
        // §553
        self.primitive(801i32, 87i32, 0i32);
        self.hash[((2624i32) - 514) as usize].set_rh(801i32);
        { let __v1240 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2624i32) - 1) as usize] = __v1240; }
        // §780
        self.primitive(898i32, 4i32, 256i32);
        self.primitive(899i32, 5i32, 257i32);
        self.hash[((2615i32) - 514) as usize].set_rh(899i32);
        { let __v1241 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2615i32) - 1) as usize] = __v1241; }
        self.primitive(900i32, 5i32, 258i32);
        self.hash[((2619i32) - 514) as usize].set_rh(901i32);
        self.hash[((2620i32) - 514) as usize].set_rh(901i32);
        self.eqtb[((2620i32) - 1) as usize].set_hh_b0(9i32);
        self.eqtb[((2620i32) - 1) as usize].set_hh_rh(29989i32);
        self.eqtb[((2620i32) - 1) as usize].set_hh_b1(1i32);
        { let __v1242 = self.eqtb[((2620i32) - 1) as usize]; self.eqtb[((2619i32) - 1) as usize] = __v1242; }
        self.eqtb[((2619i32) - 1) as usize].set_hh_b0(115i32);
        // §983
        self.primitive(970i32, 81i32, 0i32);
        self.primitive(971i32, 81i32, 1i32);
        self.primitive(972i32, 81i32, 2i32);
        self.primitive(973i32, 81i32, 3i32);
        self.primitive(974i32, 81i32, 4i32);
        self.primitive(975i32, 81i32, 5i32);
        self.primitive(976i32, 81i32, 6i32);
        self.primitive(977i32, 81i32, 7i32);
        // §1052
        self.primitive(1025i32, 14i32, 0i32);
        self.primitive(1026i32, 14i32, 1i32);
        // §1058
        self.primitive(1027i32, 26i32, 4i32);
        self.primitive(1028i32, 26i32, 0i32);
        self.primitive(1029i32, 26i32, 1i32);
        self.primitive(1030i32, 26i32, 2i32);
        self.primitive(1031i32, 26i32, 3i32);
        self.primitive(1032i32, 27i32, 4i32);
        self.primitive(1033i32, 27i32, 0i32);
        self.primitive(1034i32, 27i32, 1i32);
        self.primitive(1035i32, 27i32, 2i32);
        self.primitive(1036i32, 27i32, 3i32);
        self.primitive(336i32, 28i32, 5i32);
        self.primitive(340i32, 29i32, 1i32);
        self.primitive(342i32, 30i32, 99i32);
        // §1071
        self.primitive(1054i32, 21i32, 1i32);
        self.primitive(1055i32, 21i32, 0i32);
        self.primitive(1056i32, 22i32, 1i32);
        self.primitive(1057i32, 22i32, 0i32);
        self.primitive(409i32, 20i32, 0i32);
        self.primitive(1058i32, 20i32, 1i32);
        self.primitive(1059i32, 20i32, 2i32);
        self.primitive(965i32, 20i32, 3i32);
        self.primitive(1060i32, 20i32, 4i32);
        self.primitive(967i32, 20i32, 5i32);
        self.primitive(1061i32, 20i32, 106i32);
        self.primitive(1062i32, 31i32, 99i32);
        self.primitive(1063i32, 31i32, 100i32);
        self.primitive(1064i32, 31i32, 101i32);
        self.primitive(1065i32, 31i32, 102i32);
        // §1088
        self.primitive(1080i32, 43i32, 1i32);
        self.primitive(1081i32, 43i32, 0i32);
        // §1107
        self.primitive(1090i32, 25i32, 12i32);
        self.primitive(1091i32, 25i32, 11i32);
        self.primitive(1092i32, 25i32, 10i32);
        self.primitive(1093i32, 23i32, 0i32);
        self.primitive(1094i32, 23i32, 1i32);
        self.primitive(1095i32, 24i32, 0i32);
        self.primitive(1096i32, 24i32, 1i32);
        // §1114
        self.primitive(45i32, 47i32, 1i32);
        self.primitive(349i32, 47i32, 0i32);
        // §1141
        self.primitive(1127i32, 48i32, 0i32);
        self.primitive(1128i32, 48i32, 1i32);
        // §1156
        self.primitive(866i32, 50i32, 16i32);
        self.primitive(867i32, 50i32, 17i32);
        self.primitive(868i32, 50i32, 18i32);
        self.primitive(869i32, 50i32, 19i32);
        self.primitive(870i32, 50i32, 20i32);
        self.primitive(871i32, 50i32, 21i32);
        self.primitive(872i32, 50i32, 22i32);
        self.primitive(873i32, 50i32, 23i32);
        self.primitive(875i32, 50i32, 26i32);
        self.primitive(874i32, 50i32, 27i32);
        self.primitive(1129i32, 51i32, 0i32);
        self.primitive(878i32, 51i32, 1i32);
        self.primitive(879i32, 51i32, 2i32);
        // §1169
        self.primitive(861i32, 53i32, 0i32);
        self.primitive(862i32, 53i32, 2i32);
        self.primitive(863i32, 53i32, 4i32);
        self.primitive(864i32, 53i32, 6i32);
        // §1178
        self.primitive(1147i32, 52i32, 0i32);
        self.primitive(1148i32, 52i32, 1i32);
        self.primitive(1149i32, 52i32, 2i32);
        self.primitive(1150i32, 52i32, 3i32);
        self.primitive(1151i32, 52i32, 4i32);
        self.primitive(1152i32, 52i32, 5i32);
        // §1188
        self.primitive(876i32, 49i32, 30i32);
        self.primitive(877i32, 49i32, 31i32);
        self.hash[((2617i32) - 514) as usize].set_rh(877i32);
        { let __v1243 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((2617i32) - 1) as usize] = __v1243; }
        // §1208
        self.primitive(1171i32, 93i32, 1i32);
        self.primitive(1172i32, 93i32, 2i32);
        self.primitive(1173i32, 93i32, 4i32);
        self.primitive(1174i32, 97i32, 0i32);
        self.primitive(1175i32, 97i32, 1i32);
        self.primitive(1176i32, 97i32, 2i32);
        self.primitive(1177i32, 97i32, 3i32);
        // §1219
        self.primitive(1191i32, 94i32, 0i32);
        self.primitive(1192i32, 94i32, 1i32);
        // §1222
        self.primitive(1193i32, 95i32, 0i32);
        self.primitive(1194i32, 95i32, 1i32);
        self.primitive(1195i32, 95i32, 2i32);
        self.primitive(1196i32, 95i32, 3i32);
        self.primitive(1197i32, 95i32, 4i32);
        self.primitive(1198i32, 95i32, 5i32);
        self.primitive(1199i32, 95i32, 6i32);
        // §1230
        self.primitive(415i32, 85i32, 3983i32);
        self.primitive(419i32, 85i32, 5007i32);
        self.primitive(416i32, 85i32, 4239i32);
        self.primitive(417i32, 85i32, 4495i32);
        self.primitive(418i32, 85i32, 4751i32);
        self.primitive(477i32, 85i32, 5574i32);
        self.primitive(412i32, 86i32, 3935i32);
        self.primitive(413i32, 86i32, 3951i32);
        self.primitive(414i32, 86i32, 3967i32);
        // §1250
        self.primitive(941i32, 99i32, 0i32);
        self.primitive(953i32, 99i32, 1i32);
        // §1254
        self.primitive(1217i32, 78i32, 0i32);
        self.primitive(1218i32, 78i32, 1i32);
        // §1262
        self.primitive(274i32, 100i32, 0i32);
        self.primitive(275i32, 100i32, 1i32);
        self.primitive(276i32, 100i32, 2i32);
        self.primitive(1227i32, 100i32, 3i32);
        // §1272
        self.primitive(1228i32, 60i32, 1i32);
        self.primitive(1229i32, 60i32, 0i32);
        // §1277
        self.primitive(1230i32, 58i32, 0i32);
        self.primitive(1231i32, 58i32, 1i32);
        // §1286
        self.primitive(1237i32, 57i32, 4239i32);
        self.primitive(1238i32, 57i32, 4495i32);
        // §1291
        self.primitive(1239i32, 19i32, 0i32);
        self.primitive(1240i32, 19i32, 1i32);
        self.primitive(1241i32, 19i32, 2i32);
        self.primitive(1242i32, 19i32, 3i32);
        // §1344
        self.primitive(1285i32, 59i32, 0i32);
        self.primitive(594i32, 59i32, 1i32);
        self.write_loc = self.cur_val;
        self.primitive(1286i32, 59i32, 2i32);
        self.primitive(1287i32, 59i32, 3i32);
        self.primitive(1288i32, 59i32, 4i32);
        self.primitive(1289i32, 59i32, 5i32);
        // §1336
        self.no_new_control_sequence = true;
    }

}
