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
    /// @<Declare subprocedures for `line_break`
    // §944
    pub fn hyphenate(&mut self) {
        let mut i: i32 = 0; // §954
        let mut j: i32 = 0; // §954
        let mut l: i32 = 0; // §954
        let mut q: halfword = 0; // §954
        let mut r: halfword = 0; // §954
        let mut s: halfword = 0; // §954
        let mut bchar: halfword = 0; // §954
        let mut major_tail: halfword = 0; // §966
        let mut minor_tail: halfword = 0; // §966
        let mut c: UnicodeScalar = 0; // §966
        let mut c_loc: i32 = 0; // §966
        let mut r_count: i32 = 0; // §966
        let mut hyf_node: halfword = 0; // §966
        let mut z: trie_pointer = 0; // §976
        let mut v: i32 = 0; // §976
        let mut h: hyph_pointer = 0; // §983
        let mut k: str_number = 0; // §983
        let mut u: pool_pointer = 0; // §983
        'l_exit_f: {
            'l_found1_f: {
                'l_found_f: {
                    'l_not_found_f: {
                        // §977
                        {
                            let __for_end_6 = self.hn;
                            j = 0i32;
                            while j <= __for_end_6 {
                                self.hyf[crate::ix::U((j) as usize)] = 0i32;
                                j = j.wrapping_add(1);
                            }
                        }
                        // §984
                        h = self.hc[crate::ix::U((1i32) as usize)];
                        self.hn = (self.hn).wrapping_add(1i32);
                        { let __ix1076 = self.hn; let __v1077 = self.cur_lang; self.hc[crate::ix::U((__ix1076) as usize)] = __v1077; }
                        {
                            let __for_end_6 = self.hn;
                            j = 2i32;
                            while j <= __for_end_6 {
                                h = (((h).wrapping_add(h)).wrapping_add(self.hc[crate::ix::U((j) as usize)]) % hyph_size);
                                j = j.wrapping_add(1);
                            }
                        }
                        while true {
                            {
                                'l_done_f: {
                                    // §985
                                    k = self.hyph_word[crate::ix::U((h) as usize)];
                                    if (k == 0i32) {
                                        break 'l_not_found_f;
                                    }
                                    if (self.length(k) < self.hn) {
                                        break 'l_not_found_f;
                                    }
                                    if (self.length(k) == self.hn) {
                                        {
                                            j = 1i32;
                                            u = self.str_start[crate::ix::U(((k).wrapping_sub(65536i32)) as usize)];
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
                                            // §986
                                            s = self.hyph_list[crate::ix::U((h) as usize)];
                                            while (s != (268435455i32).wrapping_neg()) {
                                                {
                                                    self.hyf[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().lh()) as usize)] = 1i32;
                                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                }
                                            }
                                            // §985
                                            self.hn = (self.hn).wrapping_sub(1i32);
                                            break 'l_found_f;
                                        }
                                    }
                                }
                                // §984
                                if (h > 0i32) {
                                    h = (h).wrapping_sub(1i32);
                                } else {
                                    h = hyph_size;
                                }
                            }
                        }
                    }
                    self.hn = (self.hn).wrapping_sub(1i32);
                    // §977
                    if (self.trie[crate::ix::U(((self.cur_lang).wrapping_add(1i32)) as usize)].b1() != self.cur_lang) {
                        break 'l_exit_f;
                    }
                    self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                    self.hc[crate::ix::U(((self.hn).wrapping_add(1i32)) as usize)] = 0i32;
                    { let __ix1078 = (self.hn).wrapping_add(2i32); let __v1079 = self.max_hyph_char; self.hc[crate::ix::U((__ix1078) as usize)] = __v1079; }
                    {
                        let __for_end_5 = ((self.hn).wrapping_sub(self.r_hyf)).wrapping_add(1i32);
                        j = 0i32;
                        while j <= __for_end_5 {
                            {
                                z = (self.trie[crate::ix::U(((self.cur_lang).wrapping_add(1i32)) as usize)].rh()).wrapping_add(self.hc[crate::ix::U((j) as usize)]);
                                l = j;
                                while (self.hc[crate::ix::U((l) as usize)] == self.trie[crate::ix::U((z) as usize)].b1()) {
                                    {
                                        if (self.trie[crate::ix::U((z) as usize)].b0() != min_quarterword) {
                                            // §978
                                            {
                                                v = self.trie[crate::ix::U((z) as usize)].b0();
                                                loop {
                                                    v = (v).wrapping_add(self.op_start[crate::ix::U((self.cur_lang) as usize)]);
                                                    i = (l).wrapping_sub(self.hyf_distance[crate::ix::U(((v) - 1) as usize)]);
                                                    if (self.hyf_num[crate::ix::U(((v) - 1) as usize)] > self.hyf[crate::ix::U((i) as usize)]) {
                                                        { let __v1080 = self.hyf_num[crate::ix::U(((v) - 1) as usize)]; self.hyf[crate::ix::U((i) as usize)] = __v1080; }
                                                    }
                                                    v = self.hyf_next[crate::ix::U(((v) - 1) as usize)];
                                                    if (v == min_quarterword) { break; }
                                                }
                                            }
                                        }
                                        // §977
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
                    let __for_end_4 = (self.l_hyf).wrapping_sub(1i32);
                    j = 0i32;
                    while j <= __for_end_4 {
                        self.hyf[crate::ix::U((j) as usize)] = 0i32;
                        j = j.wrapping_add(1);
                    }
                }
                {
                    let __for_end_4 = (self.r_hyf).wrapping_sub(1i32);
                    j = 0i32;
                    while j <= __for_end_4 {
                        self.hyf[crate::ix::U(((self.hn).wrapping_sub(j)) as usize)] = 0i32;
                        j = j.wrapping_add(1);
                    }
                }
                // §955
                {
                    let __for_end_4 = (self.hn).wrapping_sub(self.r_hyf);
                    j = self.l_hyf;
                    while j <= __for_end_4 {
                        if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                            break 'l_found1_f;
                        }
                        j = j.wrapping_add(1);
                    }
                }
                break 'l_exit_f;
            }
            // §956
            if ((((self.ha != (268435455i32).wrapping_neg()) && (!(self.ha >= self.hi_mem_min))) && (self.mem[crate::ix::U((self.ha) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((self.ha) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((self.ha) as usize)].hh().b1() <= native_word_node_AT))) {
                {
                    // §957
                    s = self.cur_p;
                    while (self.mem[crate::ix::U((s) as usize)].hh().rh() != self.ha) {
                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                    }
                    self.hyphen_passed = 0i32;
                    {
                        let __for_end_5 = (self.hn).wrapping_sub(self.r_hyf);
                        j = self.l_hyf;
                        while j <= __for_end_5 {
                            {
                                if (((self.hyf[crate::ix::U((j) as usize)]) % 2) != 0) {
                                    {
                                        q = self.new_native_word_node(self.hf, (j).wrapping_sub(self.hyphen_passed));
                                        { let __v1081 = self.mem[crate::ix::U((self.ha) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1081); }
                                        {
                                            let __for_end_10 = ((j).wrapping_sub(self.hyphen_passed)).wrapping_sub(1i32);
                                            i = 0i32;
                                            while i <= __for_end_10 {
                                                { let __a1082_0 = q; let __a1082_1 = i; let __a1082_2 = self.get_native_char(self.ha, (i).wrapping_add(self.hyphen_passed)); self.set_native_char(__a1082_0, __a1082_1, __a1082_2) };
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        self.set_native_metrics(q, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                                        s = q;
                                        q = self.new_disc();
                                        { let __v1083 = self.new_native_character(self.hf, self.hyf_char); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1083); }
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                                        s = q;
                                        self.hyphen_passed = j;
                                    }
                                }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    self.hn = self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2();
                    q = self.new_native_word_node(self.hf, (self.hn).wrapping_sub(self.hyphen_passed));
                    { let __v1084 = self.mem[crate::ix::U((self.ha) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1084); }
                    {
                        let __for_end_5 = ((self.hn).wrapping_sub(self.hyphen_passed)).wrapping_sub(1i32);
                        i = 0i32;
                        while i <= __for_end_5 {
                            { let __a1085_0 = q; let __a1085_1 = i; let __a1085_2 = self.get_native_char(self.ha, (i).wrapping_add(self.hyphen_passed)); self.set_native_char(__a1085_0, __a1085_1, __a1085_2) };
                            i = i.wrapping_add(1);
                        }
                    }
                    self.set_native_metrics(q, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                    self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                    s = q;
                    q = self.mem[crate::ix::U((self.ha) as usize)].hh().rh();
                    self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                    { let __ix1086 = self.ha; self.mem[crate::ix::U((__ix1086) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                    self.flush_node_list(self.ha);
                }
            } else {
                // §956
                {
                    'l_common_ending_f: {
                        'l_found2_f: {
                            q = self.mem[crate::ix::U((self.hb) as usize)].hh().rh();
                            { let __ix1087 = self.hb; self.mem[crate::ix::U((__ix1087) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                            r = self.mem[crate::ix::U((self.ha) as usize)].hh().rh();
                            { let __ix1088 = self.ha; self.mem[crate::ix::U((__ix1088) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                            bchar = self.hyf_bchar;
                            if (self.ha >= self.hi_mem_min) {
                                if (self.mem[crate::ix::U((self.ha) as usize)].hh().b0() != self.hf) {
                                    break 'l_found2_f;
                                } else {
                                    {
                                        self.init_list = self.ha;
                                        self.init_lig = false;
                                        { let __v1089 = self.mem[crate::ix::U((self.ha) as usize)].hh().b1(); self.hu[crate::ix::U((0i32) as usize)] = __v1089; }
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
                                            { let __v1090 = self.mem[crate::ix::U(((self.ha).wrapping_add(1i32)) as usize)].hh().b1(); self.hu[crate::ix::U((0i32) as usize)] = __v1090; }
                                            if (self.init_list == (268435455i32).wrapping_neg()) {
                                                if self.init_lft {
                                                    {
                                                        { let __v1091 = self.max_hyph_char; self.hu[crate::ix::U((0i32) as usize)] = __v1091; }
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
                                        self.init_list = (268435455i32).wrapping_neg();
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
                        { let __v1092 = self.max_hyph_char; self.hu[crate::ix::U((0i32) as usize)] = __v1092; }
                        self.init_lig = false;
                        self.init_list = (268435455i32).wrapping_neg();
                    }
                    self.flush_node_list(r);
                    // §967
                    loop {
                        l = j;
                        j = (self.reconstitute(j, self.hn, bchar, self.hyf_char)).wrapping_add(1i32);
                        if (self.hyphen_passed == 0i32) {
                            {
                                { let __v1093 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1093); }
                                while (self.mem[crate::ix::U((s) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                }
                                if (((self.hyf[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]) % 2) != 0) {
                                    {
                                        l = j;
                                        self.hyphen_passed = (j).wrapping_sub(1i32);
                                        self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                    }
                                }
                            }
                        }
                        if (self.hyphen_passed > 0i32) {
                            // §968
                            loop {
                                r = self.get_node(small_node_size);
                                { let __v1094 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1094); }
                                self.mem[crate::ix::U((r) as usize)].set_hh_b0(disc_node);
                                major_tail = r;
                                r_count = 0i32;
                                while (self.mem[crate::ix::U((major_tail) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                    {
                                        major_tail = self.mem[crate::ix::U((major_tail) as usize)].hh().rh();
                                        r_count = (r_count).wrapping_add(1i32);
                                    }
                                }
                                i = self.hyphen_passed;
                                self.hyf[crate::ix::U((i) as usize)] = 0i32;
                                // §969
                                minor_tail = (268435455i32).wrapping_neg();
                                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                hyf_node = self.new_character(self.hf, self.hyf_char);
                                if (hyf_node != (268435455i32).wrapping_neg()) {
                                    {
                                        i = (i).wrapping_add(1i32);
                                        c = self.hu[crate::ix::U((i) as usize)];
                                        { let __v1095 = self.hyf_char; self.hu[crate::ix::U((i) as usize)] = __v1095; }
                                        {
                                            { let __v1096 = self.avail; self.mem[crate::ix::U((hyf_node) as usize)].set_hh_rh(__v1096); }
                                            self.avail = hyf_node;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                }
                                while (l <= i) {
                                    {
                                        l = (self.reconstitute(l, i, self.font_bchar[crate::ix::U((self.hf) as usize)], non_char)).wrapping_add(1i32);
                                        if (self.mem[crate::ix::U((hold_head) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                            {
                                                if (minor_tail == (268435455i32).wrapping_neg()) {
                                                    { let __v1097 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v1097); }
                                                } else {
                                                    { let __v1098 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((minor_tail) as usize)].set_hh_rh(__v1098); }
                                                }
                                                minor_tail = self.mem[crate::ix::U((hold_head) as usize)].hh().rh();
                                                while (self.mem[crate::ix::U((minor_tail) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                                    minor_tail = self.mem[crate::ix::U((minor_tail) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    }
                                }
                                if (hyf_node != (268435455i32).wrapping_neg()) {
                                    {
                                        self.hu[crate::ix::U((i) as usize)] = c;
                                        l = i;
                                        i = (i).wrapping_sub(1i32);
                                    }
                                }
                                // §970
                                minor_tail = (268435455i32).wrapping_neg();
                                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                c_loc = 0i32;
                                if (self.bchar_label[crate::ix::U((self.hf) as usize)] != non_address) {
                                    {
                                        l = (l).wrapping_sub(1i32);
                                        c = self.hu[crate::ix::U((l) as usize)];
                                        c_loc = l;
                                        { let __v1099 = self.max_hyph_char; self.hu[crate::ix::U((l) as usize)] = __v1099; }
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
                                            if (self.mem[crate::ix::U((hold_head) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                                {
                                                    if (minor_tail == (268435455i32).wrapping_neg()) {
                                                        { let __v1100 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v1100); }
                                                    } else {
                                                        { let __v1101 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((minor_tail) as usize)].set_hh_rh(__v1101); }
                                                    }
                                                    minor_tail = self.mem[crate::ix::U((hold_head) as usize)].hh().rh();
                                                    while (self.mem[crate::ix::U((minor_tail) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                                        minor_tail = self.mem[crate::ix::U((minor_tail) as usize)].hh().rh();
                                                    }
                                                }
                                            }
                                            if (l >= j) { break; }
                                        }
                                        while (l > j) {
                                            // §971
                                            {
                                                j = (self.reconstitute(j, self.hn, bchar, non_char)).wrapping_add(1i32);
                                                { let __v1102 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((major_tail) as usize)].set_hh_rh(__v1102); }
                                                while (self.mem[crate::ix::U((major_tail) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                                    {
                                                        major_tail = self.mem[crate::ix::U((major_tail) as usize)].hh().rh();
                                                        r_count = (r_count).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // §972
                                if (r_count > 127i32) {
                                    {
                                        { let __v1103 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1103); }
                                        self.mem[crate::ix::U((r) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                        self.flush_node_list(r);
                                    }
                                } else {
                                    {
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b1(r_count);
                                    }
                                }
                                s = major_tail;
                                // §968
                                self.hyphen_passed = (j).wrapping_sub(1i32);
                                self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                if (!(((self.hyf[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]) % 2) != 0)) { break; }
                            }
                        }
                        if (j > self.hn) { break; }
                    }
                    // §967
                    self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                    // §956
                    self.flush_list(self.init_list);
                }
            }
        }
        // §944
    }

    /// @<Declare subprocedures for `line_break`
    // §944
    pub fn max_hyphenatable_length(&mut self) -> i32 {
        let mut max_hyphenatable_length: i32 = 0;
        if (self.eqtb[crate::ix::U(((7892350i32) - 1) as usize)].int() > hyphenatable_length_limit) {
            max_hyphenatable_length = hyphenatable_length_limit;
        } else {
            max_hyphenatable_length = self.eqtb[crate::ix::U(((7892350i32) - 1) as usize)].int();
        }
        max_hyphenatable_length
    }

    /// It's tempting to remove the `overflow` stops in the following procedure;
    /// `new_trie_op` could return `min_quarterword` (thereby simply ignoring
    /// part of a hyphenation pattern) instead of aborting the job. However, that would
    /// lead to different hyphenation results on different installations of \TeX\
    /// using the same patterns. The `overflow` stops are necessary for portability
    /// of patterns.
    /// @<Declare procedures for preprocessing hyph...
    // §998
    pub fn new_trie_op(&mut self, mut d: small_number, mut n: small_number, mut v: quarterword) -> quarterword {
        let mut new_trie_op: quarterword = 0;
        let mut h: i32 = 0; // §998
        let mut u: quarterword = 0; // §998
        let mut l: i32 = 0; // §998
        'l_exit_f: {
            h = ((((((n).wrapping_add((313i32).wrapping_mul(d))).wrapping_add((361i32).wrapping_mul(v))).wrapping_add((1009i32).wrapping_mul(self.cur_lang))).wrapping_abs() % (trie_op_size).wrapping_add(trie_op_size))).wrapping_sub(trie_op_size);
            while true {
                {
                    l = self.trie_op_hash[crate::ix::U(((h) + 35111) as usize)];
                    if (l == 0i32) {
                        {
                            if (self.trie_op_ptr == trie_op_size) {
                                self.overflow(66378i32, trie_op_size);
                            }
                            u = self.trie_used[crate::ix::U((self.cur_lang) as usize)];
                            if (u == max_trie_op) {
                                self.overflow(66379i32, 65535i32);
                            }
                            self.trie_op_ptr = (self.trie_op_ptr).wrapping_add(1i32);
                            u = (u).wrapping_add(1i32);
                            self.trie_used[crate::ix::U((self.cur_lang) as usize)] = u;
                            self.hyf_distance[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = d;
                            self.hyf_num[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = n;
                            self.hyf_next[crate::ix::U(((self.trie_op_ptr) - 1) as usize)] = v;
                            { let __ix1104 = self.trie_op_ptr; let __v1105 = self.cur_lang; self.trie_op_lang[crate::ix::U(((__ix1104) - 1) as usize)] = __v1105; }
                            { let __v1106 = self.trie_op_ptr; self.trie_op_hash[crate::ix::U(((h) + 35111) as usize)] = __v1106; }
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
    // §1002
    pub fn trie_node(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut trie_node: trie_pointer = 0;
        let mut h: trie_pointer = 0; // §1002
        let mut q: trie_pointer = 0; // §1002
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
    // §1003
    pub fn compress_trie(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut compress_trie: trie_pointer = 0;
        if (p == 0i32) {
            compress_trie = 0i32;
        } else {
            {
                { let __v1107 = self.compress_trie(self.trie_l[crate::ix::U((p) as usize)]); self.trie_l[crate::ix::U((p) as usize)] = __v1107; }
                { let __v1108 = self.compress_trie(self.trie_r[crate::ix::U((p) as usize)]); self.trie_r[crate::ix::U((p) as usize)] = __v1108; }
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
    // §1007
    pub fn first_fit(&mut self, mut p: trie_pointer) {
        let mut h: trie_pointer = 0; // §1007
        let mut z: trie_pointer = 0; // §1007
        let mut q: trie_pointer = 0; // §1007
        let mut c: UTF16_code = 0; // §1007
        let mut l: trie_pointer = 0; // §1007
        let mut r: trie_pointer = 0; // §1007
        let mut ll: i32 = 0; // §1007
        'l_found_f: {
            c = self.trie_c[crate::ix::U((p) as usize)];
            z = self.trie_min[crate::ix::U((c) as usize)];
            while true {
                {
                    'l_not_found_f: {
                        h = (z).wrapping_sub(c);
                        // §1008
                        if (self.trie_max < (h).wrapping_add(self.max_hyph_char)) {
                            {
                                if (trie_size <= (h).wrapping_add(self.max_hyph_char)) {
                                    self.overflow(66380i32, trie_size);
                                }
                                loop {
                                    self.trie_max = (self.trie_max).wrapping_add(1i32);
                                    { let __ix1109 = self.trie_max; let __v1110 = false; self.trie_taken[crate::ix::U(((__ix1109) - 1) as usize)] = __v1110; }
                                    { let __ix1111 = self.trie_max; let __v1112 = (self.trie_max).wrapping_add(1i32); self.trie[crate::ix::U((__ix1111) as usize)].set_rh(__v1112); }
                                    { let __ix1113 = self.trie_max; let __v1114 = (self.trie_max).wrapping_sub(1i32); self.trie[crate::ix::U((__ix1113) as usize)].set_lh(__v1114); }
                                    if (self.trie_max == (h).wrapping_add(self.max_hyph_char)) { break; }
                                }
                            }
                        }
                        // §1007
                        if self.trie_taken[crate::ix::U(((h) - 1) as usize)] {
                            break 'l_not_found_f;
                        }
                        // §1009
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
                    // §1007
                    z = self.trie[crate::ix::U((z) as usize)].rh();
                }
            }
        }
        { let __v1115 = true; self.trie_taken[crate::ix::U(((h) - 1) as usize)] = __v1115; }
        // §1010
        self.trie_hash[crate::ix::U((p) as usize)] = h;
        q = p;
        loop {
            z = (h).wrapping_add(self.trie_c[crate::ix::U((q) as usize)]);
            l = self.trie[crate::ix::U((z) as usize)].lh();
            r = self.trie[crate::ix::U((z) as usize)].rh();
            self.trie[crate::ix::U((r) as usize)].set_lh(l);
            self.trie[crate::ix::U((l) as usize)].set_rh(r);
            self.trie[crate::ix::U((z) as usize)].set_rh(0i32);
            if (l < self.max_hyph_char) {
                {
                    if (z < self.max_hyph_char) {
                        ll = z;
                    } else {
                        ll = self.max_hyph_char;
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
    // §1011
    pub fn trie_pack(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §1011
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
    // §1013
    pub fn trie_fix(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §1013
        let mut c: UTF16_code = 0; // §1013
        let mut z: trie_pointer = 0; // §1013
        z = self.trie_hash[crate::ix::U((p) as usize)];
        loop {
            q = self.trie_l[crate::ix::U((p) as usize)];
            c = self.trie_c[crate::ix::U((p) as usize)];
            { let __v1116 = self.trie_hash[crate::ix::U((q) as usize)]; self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_rh(__v1116); }
            self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_b1(c);
            { let __v1117 = self.trie_o[crate::ix::U((p) as usize)]; self.trie[crate::ix::U(((z).wrapping_add(c)) as usize)].set_b0(__v1117); }
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
    // §1014
    pub fn new_patterns(&mut self) {
        let mut k: i32 = 0; // §1014
        let mut l: i32 = 0; // §1014
        let mut digit_sensed: bool = false; // §1014
        let mut v: quarterword = 0; // §1014
        let mut p: trie_pointer = 0; // §1014
        let mut q: trie_pointer = 0; // §1014
        let mut first_child: bool = false; // §1014
        let mut c: UTF16_code = 0; // §1014
        if self.trie_not_ready {
            {
                'l_done_f: {
                    if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() <= 0i32) {
                        self.cur_lang = 0i32;
                    } else {
                        if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() > biggest_lang) {
                            self.cur_lang = 0i32;
                        } else {
                            self.cur_lang = self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int();
                        }
                    }
                    self.scan_left_brace();
                    // §1015
                    k = 0i32;
                    self.hyf[crate::ix::U((0i32) as usize)] = 0i32;
                    digit_sensed = false;
                    while true {
                        {
                            self.get_x_token();
                            match self.cur_cmd {
                                letter | other_char => {
                                    // §1016
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
                                                                if (self.interaction == error_stop_mode) {
                                                                }
                                                                if self.file_line_error_style_p {
                                                                    self.print_file_line();
                                                                } else {
                                                                    self.print_nl(65544i32);
                                                                }
                                                                self.print(66386i32);
                                                            }
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[crate::ix::U((0i32) as usize)] = 66385i32;
                                                            }
                                                            self.error();
                                                        }
                                                    }
                                                }
                                            }
                                            if (self.cur_chr > self.max_hyph_char) {
                                                self.max_hyph_char = self.cur_chr;
                                            }
                                            if (k < self.max_hyphenatable_length()) {
                                                {
                                                    k = (k).wrapping_add(1i32);
                                                    { let __v1118 = self.cur_chr; self.hc[crate::ix::U((k) as usize)] = __v1118; }
                                                    self.hyf[crate::ix::U((k) as usize)] = 0i32;
                                                    digit_sensed = false;
                                                }
                                            }
                                        }
                                    } else {
                                        if (k < self.max_hyphenatable_length()) {
                                            {
                                                { let __v1119 = (self.cur_chr).wrapping_sub(48i32); self.hyf[crate::ix::U((k) as usize)] = __v1119; }
                                                digit_sensed = true;
                                            }
                                        }
                                    }
                                }
                                spacer | right_brace => {
                                    // §1015
                                    {
                                        if (k > 0i32) {
                                            // §1017
                                            {
                                                'l_done1_f: {
                                                    // §1019
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
                                                // §1017
                                                q = 0i32;
                                                { let __v1120 = self.cur_lang; self.hc[crate::ix::U((0i32) as usize)] = __v1120; }
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
                                                            // §1018
                                                            {
                                                                if (self.trie_ptr == trie_size) {
                                                                    self.overflow(66380i32, trie_size);
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
                                                        // §1017
                                                        q = p;
                                                    }
                                                }
                                                if (self.trie_o[crate::ix::U((q) as usize)] != min_quarterword) {
                                                    {
                                                        {
                                                            if (self.interaction == error_stop_mode) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(65544i32);
                                                            }
                                                            self.print(66387i32);
                                                        }
                                                        {
                                                            self.help_ptr = 1i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 66385i32;
                                                        }
                                                        self.error();
                                                    }
                                                }
                                                self.trie_o[crate::ix::U((q) as usize)] = v;
                                            }
                                        }
                                        // §1015
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
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(65544i32);
                                            }
                                            self.print(66384i32);
                                        }
                                        self.print_esc(66382i32);
                                        {
                                            self.help_ptr = 1i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 66385i32;
                                        }
                                        self.error();
                                    }
                                }
                            }
                        }
                    }
                }
                // §1014
                if (self.eqtb[crate::ix::U(((7892333i32) - 1) as usize)].int() > 0i32) {
                    // §1666
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
                            // §1018
                            {
                                if (self.trie_ptr == trie_size) {
                                    self.overflow(66380i32, trie_size);
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
                        // §1666
                        q = p;
                        // §1667
                        p = self.trie_l[crate::ix::U((q) as usize)];
                        first_child = true;
                        {
                            let __for_end_6 = 255i32;
                            c = 0i32;
                            while c <= __for_end_6 {
                                if ((self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh() > 0i32) || ((c == 255i32) && first_child)) {
                                    {
                                        if (p == 0i32) {
                                            // §1018
                                            {
                                                if (self.trie_ptr == trie_size) {
                                                    self.overflow(66380i32, trie_size);
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
                                            // §1667
                                            self.trie_c[crate::ix::U((p) as usize)] = c;
                                        }
                                        { let __v1121 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.trie_o[crate::ix::U((p) as usize)] = __v1121; }
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
            // §1014
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66381i32);
                }
                self.print_esc(66382i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66383i32;
                }
                self.error();
                { let __v1122 = self.scan_toks(false, false); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v1122); }
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
    // §1020
    pub fn init_trie(&mut self) {
        let mut p: trie_pointer = 0; // §1020
        let mut j: i32 = 0; // §1020
        let mut k: i32 = 0; // §1020
        let mut t: i32 = 0; // §1020
        let mut r: trie_pointer = 0; // §1020
        let mut s: trie_pointer = 0; // §1020
        let mut h: two_halves = two_halves::default(); // §1020
        self.max_hyph_char = (self.max_hyph_char).wrapping_add(1i32);
        // §999
        self.op_start[crate::ix::U((0i32) as usize)] = (0i32).wrapping_neg();
        {
            let __for_end_2 = biggest_lang;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v1123 = (self.op_start[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]).wrapping_add(self.trie_used[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]); self.op_start[crate::ix::U((j) as usize)] = __v1123; }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v1124 = (self.op_start[crate::ix::U((self.trie_op_lang[crate::ix::U(((j) - 1) as usize)]) as usize)]).wrapping_add(self.trie_op_val[crate::ix::U(((j) - 1) as usize)]); self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)] = __v1124; }
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
                        { let __v1125 = self.hyf_distance[crate::ix::U(((j) - 1) as usize)]; self.hyf_distance[crate::ix::U(((k) - 1) as usize)] = __v1125; }
                        self.hyf_distance[crate::ix::U(((j) - 1) as usize)] = t;
                        t = self.hyf_num[crate::ix::U(((k) - 1) as usize)];
                        { let __v1126 = self.hyf_num[crate::ix::U(((j) - 1) as usize)]; self.hyf_num[crate::ix::U(((k) - 1) as usize)] = __v1126; }
                        self.hyf_num[crate::ix::U(((j) - 1) as usize)] = t;
                        t = self.hyf_next[crate::ix::U(((k) - 1) as usize)];
                        { let __v1127 = self.hyf_next[crate::ix::U(((j) - 1) as usize)]; self.hyf_next[crate::ix::U(((k) - 1) as usize)] = __v1127; }
                        self.hyf_next[crate::ix::U(((j) - 1) as usize)] = t;
                        { let __v1128 = self.trie_op_hash[crate::ix::U(((k) + 35111) as usize)]; self.trie_op_hash[crate::ix::U(((j) + 35111) as usize)] = __v1128; }
                        self.trie_op_hash[crate::ix::U(((k) + 35111) as usize)] = k;
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        // §1006
        {
            let __for_end_2 = trie_size;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[crate::ix::U((p) as usize)] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        { let __v1129 = self.compress_trie(self.trie_r[crate::ix::U((0i32) as usize)]); self.trie_r[crate::ix::U((0i32) as usize)] = __v1129; }
        { let __v1130 = self.compress_trie(self.trie_l[crate::ix::U((0i32) as usize)]); self.trie_l[crate::ix::U((0i32) as usize)] = __v1130; }
        {
            let __for_end_2 = self.trie_ptr;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[crate::ix::U((p) as usize)] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = biggest_char;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_min[crate::ix::U((p) as usize)] = (p).wrapping_add(1i32);
                p = p.wrapping_add(1);
            }
        }
        self.trie[crate::ix::U((0i32) as usize)].set_rh(1i32);
        self.trie_max = 0i32;
        // §1020
        if (self.trie_l[crate::ix::U((0i32) as usize)] != 0i32) {
            {
                self.first_fit(self.trie_l[crate::ix::U((0i32) as usize)]);
                self.trie_pack(self.trie_l[crate::ix::U((0i32) as usize)]);
            }
        }
        if (self.trie_r[crate::ix::U((0i32) as usize)] != 0i32) {
            // §1668
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
        // §1012
        h.set_rh(0i32);
        h.set_b0(min_quarterword);
        h.set_b1(min_quarterword);
        if (self.trie_max == 0i32) {
            {
                {
                    let __for_end_4 = self.max_hyph_char;
                    r = 0i32;
                    while r <= __for_end_4 {
                        self.trie[crate::ix::U((r) as usize)] = h;
                        r = r.wrapping_add(1);
                    }
                }
                self.trie_max = self.max_hyph_char;
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
        // §1020
        self.trie_not_ready = false;
    }

    /// Since `line_break` is a rather lengthy procedure---sort of a small world unto
    /// itself---we must build it up little by little, somewhat more cautiously
    /// than we have done with the simpler procedures of \TeX. Here is the
    /// general outline.
    // §863
    pub fn line_break(&mut self, mut d: bool) {
        let mut auto_breaking: bool = false; // §910
        let mut prev_p: halfword = 0; // §910
        let mut q: halfword = 0; // §910
        let mut r: halfword = 0; // §910
        let mut s: halfword = 0; // §910
        let mut prev_s: halfword = 0; // §910
        let mut f: internal_font_number = 0; // §910
        let mut j: small_number = 0; // §942
        let mut c: UnicodeScalar = 0; // §942
        let mut l: i32 = 0; // §948
        let mut i: i32 = 0; // §948
        'l_done_f: {
            self.pack_begin_line = self.cur_list.ml_field;
            // §864
            { let __v1131 = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(); self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(__v1131); }
            if (self.cur_list.tail_field >= self.hi_mem_min) {
                {
                    { let __ix1132 = self.cur_list.tail_field; let __v1133 = self.new_penalty(inf_penalty); self.mem[crate::ix::U((__ix1132) as usize)].set_hh_rh(__v1133); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            } else {
                if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() != glue_node) {
                    {
                        { let __ix1134 = self.cur_list.tail_field; let __v1135 = self.new_penalty(inf_penalty); self.mem[crate::ix::U((__ix1134) as usize)].set_hh_rh(__v1135); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                } else {
                    {
                        { let __ix1136 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1136) as usize)].set_hh_b0(penalty_node); }
                        self.delete_glue_ref(self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().lh());
                        self.flush_node_list(self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().rh());
                        { let __ix1137 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1137) as usize)].set_int(inf_penalty); }
                    }
                }
            }
            { let __ix1138 = self.cur_list.tail_field; let __v1139 = self.new_param_glue(par_fill_skip_code); self.mem[crate::ix::U((__ix1138) as usize)].set_hh_rh(__v1139); }
            self.last_line_fill = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
            self.init_cur_lang = (self.cur_list.pg_field % 65536i32);
            self.init_l_hyf = (self.cur_list.pg_field / 4194304i32);
            self.init_r_hyf = ((self.cur_list.pg_field / 65536i32) % 64i32);
            self.pop_nest();
            // §875
            self.no_shrink_error_yet = true;
            if ((self.mem[crate::ix::U((self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].hh().rh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                {
                    { let __v1140 = self.finite_shrink(self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].hh().rh()); self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].set_hh_rh(__v1140); }
                }
            }
            if ((self.mem[crate::ix::U((self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                {
                    { let __v1141 = self.finite_shrink(self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh()); self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].set_hh_rh(__v1141); }
                }
            }
            q = self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].hh().rh();
            r = self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh();
            { let __v1142 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.background[crate::ix::U(((1i32) - 1) as usize)] = __v1142; }
            self.background[crate::ix::U(((2i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((3i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((4i32) - 1) as usize)] = 0i32;
            self.background[crate::ix::U(((5i32) - 1) as usize)] = 0i32;
            { let __ix1143 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1144 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.background[crate::ix::U(((__ix1143) - 1) as usize)] = __v1144; }
            { let __ix1145 = (2i32).wrapping_add(self.mem[crate::ix::U((r) as usize)].hh().b0()); let __v1146 = (self.background[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((r) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.background[crate::ix::U(((__ix1145) - 1) as usize)] = __v1146; }
            { let __v1147 = (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.background[crate::ix::U(((6i32) - 1) as usize)] = __v1147; }
            // §1654
            self.do_last_line_fit = false;
            self.active_node_size = active_node_size_normal;
            if (self.eqtb[crate::ix::U(((7892331i32) - 1) as usize)].int() > 0i32) {
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
                                { let __ix1148 = (self.mem[crate::ix::U((q) as usize)].hh().b0()).wrapping_sub(1i32); let __v1149 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.fill_width[crate::ix::U((__ix1148) as usize)] = __v1149; }
                            }
                        }
                    }
                }
            }
            // §882
            self.minimum_demerits = awful_bad;
            self.minimal_demerits[crate::ix::U((tight_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((decent_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((loose_fit) as usize)] = awful_bad;
            self.minimal_demerits[crate::ix::U((very_loose_fit) as usize)] = awful_bad;
            // §896
            if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                if (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() == 0i32) {
                    {
                        self.last_special_line = 0i32;
                        self.second_width = self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int();
                        self.second_indent = 0i32;
                    }
                } else {
                    // §897
                    {
                        self.last_special_line = (self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int()).wrapping_abs();
                        if (self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int() < 0i32) {
                            {
                                self.first_width = (self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int()).wrapping_sub((self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int()).wrapping_abs());
                                if (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() >= 0i32) {
                                    self.first_indent = self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int();
                                } else {
                                    self.first_indent = 0i32;
                                }
                                self.second_width = self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int();
                                self.second_indent = 0i32;
                            }
                        } else {
                            {
                                self.first_width = self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int();
                                self.first_indent = 0i32;
                                self.second_width = (self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int()).wrapping_sub((self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int()).wrapping_abs());
                                if (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() >= 0i32) {
                                    self.second_indent = self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int();
                                } else {
                                    self.second_indent = 0i32;
                                }
                            }
                        }
                    }
                }
            } else {
                // §896
                {
                    self.last_special_line = (self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_sub(1i32);
                    self.second_width = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul((self.last_special_line).wrapping_add(1i32)))) as usize)].int();
                    self.second_indent = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(self.last_special_line))).wrapping_add(1i32)) as usize)].int();
                }
            }
            if (self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int() == 0i32) {
                self.easy_line = self.last_special_line;
            } else {
                self.easy_line = max_halfword;
            }
            // §911
            self.threshold = self.eqtb[crate::ix::U(((7892264i32) - 1) as usize)].int();
            if (self.threshold >= 0i32) {
                {
                    if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(66360i32);
                        }
                    }
                    self.second_pass = false;
                    self.final_pass = false;
                }
            } else {
                {
                    self.threshold = self.eqtb[crate::ix::U(((7892265i32) - 1) as usize)].int();
                    self.second_pass = true;
                    self.final_pass = (self.eqtb[crate::ix::U(((9006740i32) - 1) as usize)].int() <= 0i32);
                    if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
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
                        // §939
                        {
                            if self.trie_not_ready {
                                self.init_trie();
                            }
                            self.cur_lang = self.init_cur_lang;
                            self.l_hyf = self.init_l_hyf;
                            self.r_hyf = self.init_r_hyf;
                            if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != self.cur_lang) {
                                self.hyph_index = 0i32;
                            } else {
                                self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                            }
                        }
                    }
                    // §912
                    q = self.get_node(self.active_node_size);
                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(unhyphenated);
                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(decent_fit);
                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(last_active);
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                    { let __v1150 = (self.cur_list.pg_field).wrapping_add(1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1150); }
                    self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
                    self.mem[crate::ix::U((active) as usize)].set_hh_rh(q);
                    if self.do_last_line_fit {
                        // §1656
                        {
                            self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
                            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(0i32);
                        }
                    }
                    // §912
                    { let __v1151 = self.background[crate::ix::U(((1i32) - 1) as usize)]; self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1151; }
                    { let __v1152 = self.background[crate::ix::U(((2i32) - 1) as usize)]; self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1152; }
                    { let __v1153 = self.background[crate::ix::U(((3i32) - 1) as usize)]; self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1153; }
                    { let __v1154 = self.background[crate::ix::U(((4i32) - 1) as usize)]; self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1154; }
                    { let __v1155 = self.background[crate::ix::U(((5i32) - 1) as usize)]; self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1155; }
                    { let __v1156 = self.background[crate::ix::U(((6i32) - 1) as usize)]; self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1156; }
                    self.passive = (268435455i32).wrapping_neg();
                    self.printed_node = temp_head;
                    self.pass_number = 0i32;
                    self.font_in_short_display = null_font;
                    // §911
                    self.cur_p = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                    auto_breaking = true;
                    {
                        prev_p = self.cur_p;
                        self.global_prev_p = self.cur_p;
                    }
                    self.first_p = self.cur_p;
                    while ((self.cur_p != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((active) as usize)].hh().rh() != last_active)) {
                        // §914
                        {
                            'l_done5_f: {
                                if (self.cur_p >= self.hi_mem_min) {
                                    // §915
                                    {
                                        {
                                            prev_p = self.cur_p;
                                            self.global_prev_p = self.cur_p;
                                        }
                                        loop {
                                            f = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0();
                                            { let __v1157 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s1159 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1158 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1158)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1159)] }.int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1157; }
                                            self.cur_p = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                            if (!(self.cur_p >= self.hi_mem_min)) { break; }
                                        }
                                    }
                                }
                                // §914
                                match self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() {
                                    hlist_node | vlist_node | rule_node => {
                                        { let __v1160 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1160; }
                                    }
                                    whatsit_node => {
                                        // §1422
                                        if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == language_node) {
                                            {
                                                self.cur_lang = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                                                self.l_hyf = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b0();
                                                self.r_hyf = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1();
                                                if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != self.cur_lang) {
                                                    self.hyph_index = 0i32;
                                                } else {
                                                    self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                                                }
                                            }
                                        } else {
                                            if (((((self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == pdf_node)) {
                                                {
                                                    { let __v1161 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1161; }
                                                }
                                            }
                                        }
                                    }
                                    glue_node => {
                                        // §914
                                        {
                                            // §916
                                            if auto_breaking {
                                                {
                                                    if (prev_p >= self.hi_mem_min) {
                                                        self.try_break(0i32, unhyphenated);
                                                    } else {
                                                        if (self.mem[crate::ix::U((prev_p) as usize)].hh().b0() < math_node) {
                                                            self.try_break(0i32, unhyphenated);
                                                        } else {
                                                            if ((self.mem[crate::ix::U((prev_p) as usize)].hh().b0() == kern_node) && (self.mem[crate::ix::U((prev_p) as usize)].hh().b1() != explicit)) {
                                                                self.try_break(0i32, unhyphenated);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if ((self.mem[crate::ix::U((self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                {
                                                    { let __ix1162 = (self.cur_p).wrapping_add(1i32); let __v1163 = self.finite_shrink(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix1162) as usize)].set_hh_lh(__v1163); }
                                                }
                                            }
                                            q = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
                                            { let __v1164 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1164; }
                                            { let __ix1165 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1166 = (self.active_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((__ix1165) - 1) as usize)] = __v1166; }
                                            { let __v1167 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1167; }
                                            // §914
                                            if (self.second_pass && auto_breaking) {
                                                // §943
                                                {
                                                    'l_done1_f: {
                                                        prev_s = self.cur_p;
                                                        s = self.mem[crate::ix::U((prev_s) as usize)].hh().rh();
                                                        if (s != (268435455i32).wrapping_neg()) {
                                                            {
                                                                'l_done4_f: {
                                                                    'l_done2_f: {
                                                                        // §949
                                                                        while true {
                                                                            {
                                                                                'l_continue_f: {
                                                                                    if (s >= self.hi_mem_min) {
                                                                                        {
                                                                                            c = self.mem[crate::ix::U((s) as usize)].hh().b1();
                                                                                            self.hf = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                                        }
                                                                                    } else {
                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b0() == ligature_node) {
                                                                                            if (self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                                                                break 'l_continue_f;
                                                                                            } else {
                                                                                                {
                                                                                                    q = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                                    c = self.mem[crate::ix::U((q) as usize)].hh().b1();
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
                                                                                                            if ((self.mem[crate::ix::U((s) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() <= native_word_node_AT)) {
                                                                                                                {
                                                                                                                    {
                                                                                                                        let __for_end_29 = (self.mem[crate::ix::U(((s).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                                                        l = 0i32;
                                                                                                                        while l <= __for_end_29 {
                                                                                                                            {
                                                                                                                                c = self.get_native_usv(s, l);
                                                                                                                                if (self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh() != 0i32) {
                                                                                                                                    {
                                                                                                                                        self.hf = self.mem[crate::ix::U(((s).wrapping_add(4i32)) as usize)].qqqq().b1();
                                                                                                                                        prev_s = s;
                                                                                                                                        if ((self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh() == c) || (self.eqtb[crate::ix::U(((7892302i32) - 1) as usize)].int() > 0i32)) {
                                                                                                                                            break 'l_done2_f;
                                                                                                                                        } else {
                                                                                                                                            break 'l_done1_f;
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                                if (c >= 65536i32) {
                                                                                                                                    l = (l).wrapping_add(1i32);
                                                                                                                                }
                                                                                                                            }
                                                                                                                            l = l.wrapping_add(1);
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                            // §1423
                                                                                                            if (self.mem[crate::ix::U((s) as usize)].hh().b1() == language_node) {
                                                                                                                {
                                                                                                                    self.cur_lang = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                                                    self.l_hyf = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                                                    self.r_hyf = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1();
                                                                                                                    if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != self.cur_lang) {
                                                                                                                        self.hyph_index = 0i32;
                                                                                                                    } else {
                                                                                                                        self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                            // §949
                                                                                                            break 'l_continue_f;
                                                                                                        }
                                                                                                    } else {
                                                                                                        break 'l_done1_f;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    if ((self.hyph_index == 0i32) || (c > 255i32)) {
                                                                                        { let __v1168 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1168; }
                                                                                    } else {
                                                                                        if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != c) {
                                                                                            self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                        } else {
                                                                                            { let __v1169 = self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0(); self.hc[crate::ix::U((0i32) as usize)] = __v1169; }
                                                                                        }
                                                                                    }
                                                                                    if (self.hc[crate::ix::U((0i32) as usize)] != 0i32) {
                                                                                        if ((self.hc[crate::ix::U((0i32) as usize)] == c) || (self.eqtb[crate::ix::U(((7892302i32) - 1) as usize)].int() > 0i32)) {
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
                                                                    if (self.hyf_char > biggest_char) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    self.ha = prev_s;
                                                                    // §943
                                                                    if ((self.l_hyf).wrapping_add(self.r_hyf) > self.max_hyphenatable_length()) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    if ((((self.ha != (268435455i32).wrapping_neg()) && (!(self.ha >= self.hi_mem_min))) && (self.mem[crate::ix::U((self.ha) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((self.ha) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((self.ha) as usize)].hh().b1() <= native_word_node_AT))) {
                                                                        {
                                                                            // goto labels: done6, restart, L37
                                                                            let mut __goto_1: i32 = 0;
                                                                            'l_dispatch_1: loop {
                                                                                if __goto_1 <= 0 {
                                                                                    // §945
                                                                                    s = self.mem[crate::ix::U((self.ha) as usize)].hh().rh();
                                                                                    while true {
                                                                                        {
                                                                                            if (!(s >= self.hi_mem_min)) {
                                                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                                                    ligature_node => {
                                                                                                    }
                                                                                                    kern_node => {
                                                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b1() != normal) {
                                                                                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                                                        }
                                                                                                    }
                                                                                                    whatsit_node | glue_node | penalty_node | ins_node | adjust_node | mark_node => {
                                                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
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
                                                                                if __goto_1 <= 1 { // done6
                                                                                    // §946
                                                                                    self.hn = 0i32;
                                                                                }
                                                                                if __goto_1 <= 2 { // restart
                                                                                    {
                                                                                        let __for_end_21 = (self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                        l = 0i32;
                                                                                        while l <= __for_end_21 {
                                                                                            {
                                                                                                c = self.get_native_usv(self.ha, l);
                                                                                                if ((self.hyph_index == 0i32) || (c > 255i32)) {
                                                                                                    { let __v1170 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1170; }
                                                                                                } else {
                                                                                                    if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != c) {
                                                                                                        self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                                    } else {
                                                                                                        { let __v1171 = self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0(); self.hc[crate::ix::U((0i32) as usize)] = __v1171; }
                                                                                                    }
                                                                                                }
                                                                                                if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                                                                                    {
                                                                                                        if (self.hn > 0i32) {
                                                                                                            {
                                                                                                                // §947
                                                                                                                q = self.new_native_word_node(self.hf, (self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(l));
                                                                                                                { let __v1172 = self.mem[crate::ix::U((self.ha) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1172); }
                                                                                                                {
                                                                                                                    let __for_end_28 = (self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                                                    i = l;
                                                                                                                    while i <= __for_end_28 {
                                                                                                                        { let __a1173_0 = q; let __a1173_1 = (i).wrapping_sub(l); let __a1173_2 = self.get_native_char(self.ha, i); self.set_native_char(__a1173_0, __a1173_1, __a1173_2) };
                                                                                                                        i = i.wrapping_add(1);
                                                                                                                    }
                                                                                                                }
                                                                                                                self.set_native_metrics(q, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                                                                                                { let __v1174 = self.mem[crate::ix::U((self.ha) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1174); }
                                                                                                                { let __ix1175 = self.ha; self.mem[crate::ix::U((__ix1175) as usize)].set_hh_rh(q); }
                                                                                                                { let __ix1176 = (self.ha).wrapping_add(4i32); self.mem[crate::ix::U((__ix1176) as usize)].set_qqqq_b2(l); }
                                                                                                                self.set_native_metrics(self.ha, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                                                                                                // §946
                                                                                                                { __goto_1 = 3; continue 'l_dispatch_1; }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                } else {
                                                                                                    if ((self.hn == 0i32) && (l > 0i32)) {
                                                                                                        {
                                                                                                            // §947
                                                                                                            q = self.new_native_word_node(self.hf, (self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(l));
                                                                                                            { let __v1177 = self.mem[crate::ix::U((self.ha) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1177); }
                                                                                                            {
                                                                                                                let __for_end_27 = (self.mem[crate::ix::U(((self.ha).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                                                i = l;
                                                                                                                while i <= __for_end_27 {
                                                                                                                    { let __a1178_0 = q; let __a1178_1 = (i).wrapping_sub(l); let __a1178_2 = self.get_native_char(self.ha, i); self.set_native_char(__a1178_0, __a1178_1, __a1178_2) };
                                                                                                                    i = i.wrapping_add(1);
                                                                                                                }
                                                                                                            }
                                                                                                            self.set_native_metrics(q, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                                                                                            { let __v1179 = self.mem[crate::ix::U((self.ha) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1179); }
                                                                                                            { let __ix1180 = self.ha; self.mem[crate::ix::U((__ix1180) as usize)].set_hh_rh(q); }
                                                                                                            { let __ix1181 = (self.ha).wrapping_add(4i32); self.mem[crate::ix::U((__ix1181) as usize)].set_qqqq_b2(l); }
                                                                                                            self.set_native_metrics(self.ha, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                                                                                            // §946
                                                                                                            self.ha = self.mem[crate::ix::U((self.ha) as usize)].hh().rh();
                                                                                                            { __goto_1 = 2; continue 'l_dispatch_1; }
                                                                                                        }
                                                                                                    } else {
                                                                                                        if (self.hn == self.max_hyphenatable_length()) {
                                                                                                            { __goto_1 = 3; continue 'l_dispatch_1; }
                                                                                                        } else {
                                                                                                            {
                                                                                                                self.hn = (self.hn).wrapping_add(1i32);
                                                                                                                if (c < 65536i32) {
                                                                                                                    {
                                                                                                                        self.hu[crate::ix::U((self.hn) as usize)] = c;
                                                                                                                        { let __ix1182 = self.hn; let __v1183 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((__ix1182) as usize)] = __v1183; }
                                                                                                                    }
                                                                                                                } else {
                                                                                                                    {
                                                                                                                        self.hu[crate::ix::U((self.hn) as usize)] = (((c).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32);
                                                                                                                        { let __ix1184 = self.hn; let __v1185 = (((self.hc[crate::ix::U((0i32) as usize)]).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.hc[crate::ix::U((__ix1184) as usize)] = __v1185; }
                                                                                                                        self.hn = (self.hn).wrapping_add(1i32);
                                                                                                                        self.hu[crate::ix::U((self.hn) as usize)] = ((c % 1024i32)).wrapping_add(56320i32);
                                                                                                                        { let __ix1186 = self.hn; let __v1187 = ((self.hc[crate::ix::U((0i32) as usize)] % 1024i32)).wrapping_add(56320i32); self.hc[crate::ix::U((__ix1186) as usize)] = __v1187; }
                                                                                                                        l = (l).wrapping_add(1i32);
                                                                                                                    }
                                                                                                                }
                                                                                                                self.hyf_bchar = non_char;
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                            l = l.wrapping_add(1);
                                                                                        }
                                                                                    }
                                                                                }
                                                                                if __goto_1 <= 3 { // L37
                                                                                }
                                                                                break 'l_dispatch_1;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        // §943
                                                                        {
                                                                            'l_done3_f: {
                                                                                // §950
                                                                                self.hn = 0i32;
                                                                                while true {
                                                                                    {
                                                                                        if (s >= self.hi_mem_min) {
                                                                                            {
                                                                                                if (self.mem[crate::ix::U((s) as usize)].hh().b0() != self.hf) {
                                                                                                    break 'l_done3_f;
                                                                                                }
                                                                                                self.hyf_bchar = self.mem[crate::ix::U((s) as usize)].hh().b1();
                                                                                                c = self.hyf_bchar;
                                                                                                if ((self.hyph_index == 0i32) || (c > 255i32)) {
                                                                                                    { let __v1188 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1188; }
                                                                                                } else {
                                                                                                    if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != c) {
                                                                                                        self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                                    } else {
                                                                                                        { let __v1189 = self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0(); self.hc[crate::ix::U((0i32) as usize)] = __v1189; }
                                                                                                    }
                                                                                                }
                                                                                                if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                                                                                    break 'l_done3_f;
                                                                                                }
                                                                                                if (self.hc[crate::ix::U((0i32) as usize)] > self.max_hyph_char) {
                                                                                                    break 'l_done3_f;
                                                                                                }
                                                                                                if (self.hn == self.max_hyphenatable_length()) {
                                                                                                    break 'l_done3_f;
                                                                                                }
                                                                                                self.hb = s;
                                                                                                self.hn = (self.hn).wrapping_add(1i32);
                                                                                                self.hu[crate::ix::U((self.hn) as usize)] = c;
                                                                                                { let __ix1190 = self.hn; let __v1191 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((__ix1190) as usize)] = __v1191; }
                                                                                                self.hyf_bchar = non_char;
                                                                                            }
                                                                                        } else {
                                                                                            if (self.mem[crate::ix::U((s) as usize)].hh().b0() == ligature_node) {
                                                                                                // §951
                                                                                                {
                                                                                                    if (self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0() != self.hf) {
                                                                                                        break 'l_done3_f;
                                                                                                    }
                                                                                                    j = self.hn;
                                                                                                    q = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().rh();
                                                                                                    if (q > (268435455i32).wrapping_neg()) {
                                                                                                        self.hyf_bchar = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                                                                    }
                                                                                                    while (q > (268435455i32).wrapping_neg()) {
                                                                                                        {
                                                                                                            c = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                                                                            if ((self.hyph_index == 0i32) || (c > 255i32)) {
                                                                                                                { let __v1192 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(c)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1192; }
                                                                                                            } else {
                                                                                                                if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b1() != c) {
                                                                                                                    self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                                                                                                } else {
                                                                                                                    { let __v1193 = self.trie[crate::ix::U(((self.hyph_index).wrapping_add(c)) as usize)].b0(); self.hc[crate::ix::U((0i32) as usize)] = __v1193; }
                                                                                                                }
                                                                                                            }
                                                                                                            if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                                                                                                break 'l_done3_f;
                                                                                                            }
                                                                                                            if (self.hc[crate::ix::U((0i32) as usize)] > self.max_hyph_char) {
                                                                                                                break 'l_done3_f;
                                                                                                            }
                                                                                                            if (j == self.max_hyphenatable_length()) {
                                                                                                                break 'l_done3_f;
                                                                                                            }
                                                                                                            j = (j).wrapping_add(1i32);
                                                                                                            self.hu[crate::ix::U((j) as usize)] = c;
                                                                                                            { let __v1194 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((j) as usize)] = __v1194; }
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
                                                                                                // §950
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
                                                                        }
                                                                    }
                                                                    // §952
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
                                                                // §943
                                                                self.hyphenate();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    kern_node => {
                                        // §914
                                        if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() == explicit) {
                                            {
                                                if ((!(self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh() >= self.hi_mem_min)) && auto_breaking) {
                                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh()) as usize)].hh().b0() == glue_node) {
                                                        self.try_break(0i32, unhyphenated);
                                                    }
                                                }
                                                { let __v1195 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1195; }
                                            }
                                        } else {
                                            { let __v1196 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1196; }
                                        }
                                    }
                                    ligature_node => {
                                        {
                                            f = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b0();
                                            self.xtx_ligature_present = true;
                                            { let __v1197 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s1199 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1198 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1198)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1199)] }.int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1197; }
                                        }
                                    }
                                    disc_node => {
                                        // §917
                                        {
                                            s = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
                                            self.disc_width = 0i32;
                                            if (s == (268435455i32).wrapping_neg()) {
                                                self.try_break(self.eqtb[crate::ix::U(((7892268i32) - 1) as usize)].int(), hyphenated);
                                            } else {
                                                {
                                                    loop {
                                                        // §918
                                                        if (s >= self.hi_mem_min) {
                                                            {
                                                                f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                self.disc_width = (self.disc_width).wrapping_add({ let __s1201 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1200 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((s) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1200)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1201)] }.int());
                                                            }
                                                        } else {
                                                            match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                ligature_node => {
                                                                    {
                                                                        f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                        self.xtx_ligature_present = true;
                                                                        self.disc_width = (self.disc_width).wrapping_add({ let __s1203 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1202 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1202)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1203)] }.int());
                                                                    }
                                                                }
                                                                hlist_node | vlist_node | rule_node | kern_node => {
                                                                    self.disc_width = (self.disc_width).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int());
                                                                }
                                                                whatsit_node => {
                                                                    if (((((self.mem[crate::ix::U((s) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pdf_node)) {
                                                                        self.disc_width = (self.disc_width).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int());
                                                                    } else {
                                                                        self.confusion(66364i32);
                                                                    }
                                                                }
                                                                _ => {
                                                                    self.confusion(66365i32);
                                                                }
                                                            }
                                                        }
                                                        // §917
                                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                        if (s == (268435455i32).wrapping_neg()) { break; }
                                                    }
                                                    { let __v1204 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.disc_width); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1204; }
                                                    self.try_break(self.eqtb[crate::ix::U(((7892267i32) - 1) as usize)].int(), hyphenated);
                                                    { let __v1205 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.disc_width); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1205; }
                                                }
                                            }
                                            r = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1();
                                            s = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                            while (r > 0i32) {
                                                {
                                                    // §919
                                                    if (s >= self.hi_mem_min) {
                                                        {
                                                            f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                            { let __v1206 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s1208 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1207 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((s) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1207)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1208)] }.int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1206; }
                                                        }
                                                    } else {
                                                        match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                            ligature_node => {
                                                                {
                                                                    f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                    self.xtx_ligature_present = true;
                                                                    { let __v1209 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s1211 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1210 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1210)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1211)] }.int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1209; }
                                                                }
                                                            }
                                                            hlist_node | vlist_node | rule_node | kern_node => {
                                                                { let __v1212 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1212; }
                                                            }
                                                            whatsit_node => {
                                                                if (((((self.mem[crate::ix::U((s) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pdf_node)) {
                                                                    { let __v1213 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1213; }
                                                                } else {
                                                                    self.confusion(66366i32);
                                                                }
                                                            }
                                                            _ => {
                                                                self.confusion(66367i32);
                                                            }
                                                        }
                                                    }
                                                    // §917
                                                    r = (r).wrapping_sub(1i32);
                                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                }
                                            }
                                            {
                                                prev_p = self.cur_p;
                                                self.global_prev_p = self.cur_p;
                                            }
                                            self.cur_p = s;
                                            break 'l_done5_f;
                                        }
                                    }
                                    math_node => {
                                        // §914
                                        {
                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1() < L_code) {
                                                auto_breaking = (((self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1()) % 2) != 0);
                                            }
                                            {
                                                if ((!(self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh() >= self.hi_mem_min)) && auto_breaking) {
                                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh()) as usize)].hh().b0() == glue_node) {
                                                        self.try_break(0i32, unhyphenated);
                                                    }
                                                }
                                                { let __v1214 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1214; }
                                            }
                                        }
                                    }
                                    penalty_node => {
                                        self.try_break(self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].int(), unhyphenated);
                                    }
                                    mark_node | ins_node | adjust_node => {
                                    }
                                    _ => {
                                        self.confusion(66363i32);
                                    }
                                }
                                {
                                    prev_p = self.cur_p;
                                    self.global_prev_p = self.cur_p;
                                }
                                self.cur_p = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                            }
                        }
                    }
                    // §911
                    if (self.cur_p == (268435455i32).wrapping_neg()) {
                        // §921
                        {
                            self.try_break((10000i32).wrapping_neg(), hyphenated);
                            if (self.mem[crate::ix::U((active) as usize)].hh().rh() != last_active) {
                                {
                                    // §922
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
                                    // §921
                                    if (self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int() == 0i32) {
                                        break 'l_done_f;
                                    }
                                    // §923
                                    {
                                        r = self.mem[crate::ix::U((active) as usize)].hh().rh();
                                        self.actual_looseness = 0i32;
                                        loop {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() != delta_node) {
                                                {
                                                    self.line_diff = (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(self.best_line);
                                                    if (((self.line_diff < self.actual_looseness) && (self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int() <= self.line_diff)) || ((self.line_diff > self.actual_looseness) && (self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int() >= self.line_diff))) {
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
                                    // §921
                                    if ((self.actual_looseness == self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int()) || self.final_pass) {
                                        break 'l_done_f;
                                    }
                                }
                            }
                        }
                    }
                    // §913
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
                    while (q != (268435455i32).wrapping_neg()) {
                        {
                            self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            self.free_node(q, passive_node_size);
                            q = self.cur_p;
                        }
                    }
                    // §911
                    if (!self.second_pass) {
                        {
                            if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                                self.print_nl(66361i32);
                            }
                            self.threshold = self.eqtb[crate::ix::U(((7892265i32) - 1) as usize)].int();
                            self.second_pass = true;
                            self.final_pass = (self.eqtb[crate::ix::U(((9006740i32) - 1) as usize)].int() <= 0i32);
                        }
                    } else {
                        {
                            if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                                self.print_nl(66362i32);
                            }
                            { let __v1215 = (self.background[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.eqtb[crate::ix::U(((9006740i32) - 1) as usize)].int()); self.background[crate::ix::U(((2i32) - 1) as usize)] = __v1215; }
                            self.final_pass = true;
                        }
                    }
                }
            }
        }
        if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
            {
                self.end_diagnostic(true);
                self.normalize_selector();
            }
        }
        if self.do_last_line_fit {
            // §1664
            if (self.mem[crate::ix::U(((self.best_bet).wrapping_add(3i32)) as usize)].int() == 0i32) {
                self.do_last_line_fit = false;
            } else {
                {
                    q = self.new_spec(self.mem[crate::ix::U(((self.last_line_fill).wrapping_add(1i32)) as usize)].hh().lh());
                    self.delete_glue_ref(self.mem[crate::ix::U(((self.last_line_fill).wrapping_add(1i32)) as usize)].hh().lh());
                    { let __v1216 = ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.best_bet).wrapping_add(3i32)) as usize)].int())).wrapping_sub(self.mem[crate::ix::U(((self.best_bet).wrapping_add(4i32)) as usize)].int()); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1216); }
                    self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
                    { let __ix1217 = (self.last_line_fill).wrapping_add(1i32); self.mem[crate::ix::U((__ix1217) as usize)].set_hh_lh(q); }
                }
            }
        }
        // §924
        self.post_line_break(d);
        // §913
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
        while (q != (268435455i32).wrapping_neg()) {
            {
                self.cur_p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                self.free_node(q, passive_node_size);
                q = self.cur_p;
            }
        }
        // §863
        self.pack_begin_line = 0i32;
    }

    /// The `eTeX_enabled` function simply returns its first argument as
    /// result.  This argument is `true` if an optional \eTeX\ feature is
    /// currently enabled; otherwise, if the argument is `false`, the function
    /// gives an error message.
    /// @<Declare \eTeX\ procedures for use...
    // §1466
    pub fn eTeX_enabled(&mut self, mut b: bool, mut j: quarterword, mut k: halfword) -> bool {
        let mut eTeX_enabled: bool = false;
        if (!b) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66020i32);
                }
                self.print_cmd_chr(j, k);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66848i32;
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
    // §1489
    pub fn show_save_groups(&mut self) {
        let mut p: i32 = 0; // §1489
        let mut m: i32 = 0; // §1489
        let mut v: save_pointer = 0; // §1489
        let mut l: quarterword = 0; // §1489
        let mut c: group_code = 0; // §1489
        let mut a: i32 = 0; // §1489
        let mut i: i32 = 0; // §1489
        let mut j: quarterword = 0; // §1489
        let mut s: str_number = 0; // §1489
        'l_done_f: {
            p = self.nest_ptr;
            { let __v1218 = self.cur_list; self.nest[crate::ix::U((p) as usize)] = __v1218; }
            v = self.save_ptr;
            l = self.cur_level;
            c = self.cur_group;
            self.save_ptr = self.cur_boundary;
            self.cur_level = (self.cur_level).wrapping_sub(1i32);
            a = 1i32;
            self.print_nl(65626i32);
            self.print_ln();
            while true {
                {
                    'l_found_f: {
                        'l_found2_f: {
                            'l_found1_f: {
                                self.print_nl(65654i32);
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
                                self.print(65566i32);
                                match self.cur_group {
                                    simple_group => {
                                        {
                                            p = (p).wrapping_add(1i32);
                                            break 'l_found2_f;
                                        }
                                    }
                                    hbox_group | adjusted_hbox_group => {
                                        s = 66488i32;
                                    }
                                    vbox_group => {
                                        s = 66396i32;
                                    }
                                    vtop_group => {
                                        s = 66487i32;
                                    }
                                    align_group => {
                                        if (a == 0i32) {
                                            {
                                                if (m == (1i32).wrapping_neg()) {
                                                    s = 65827i32;
                                                } else {
                                                    s = 65854i32;
                                                }
                                                a = 1i32;
                                                break 'l_found1_f;
                                            }
                                        } else {
                                            {
                                                if (a == 1i32) {
                                                    self.print(66886i32);
                                                } else {
                                                    self.print_esc(66320i32);
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
                                            self.print_esc(65840i32);
                                            break 'l_found2_f;
                                        }
                                    }
                                    output_group => {
                                        {
                                            self.print_esc(65690i32);
                                            break 'l_found_f;
                                        }
                                    }
                                    math_group => {
                                        break 'l_found2_f;
                                    }
                                    disc_group | math_choice_group => {
                                        {
                                            if (self.cur_group == disc_group) {
                                                self.print_esc(65639i32);
                                            } else {
                                                self.print_esc(65838i32);
                                            }
                                            {
                                                let __for_end_11 = 3i32;
                                                i = 1i32;
                                                while i <= __for_end_11 {
                                                    if (i <= self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int()) {
                                                        self.print(66263i32);
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
                                                self.print_esc(65642i32);
                                            } else {
                                                {
                                                    self.print_esc(65618i32);
                                                    self.print_int(self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int());
                                                }
                                            }
                                            break 'l_found2_f;
                                        }
                                    }
                                    vcenter_group => {
                                        {
                                            s = 65855i32;
                                            break 'l_found1_f;
                                        }
                                    }
                                    semi_simple_group => {
                                        {
                                            p = (p).wrapping_add(1i32);
                                            self.print_esc(65817i32);
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
                                                self.print_esc(66279i32);
                                            } else {
                                                self.print_esc(66281i32);
                                            }
                                            break 'l_found_f;
                                        }
                                    }
                                    _ => {}
                                }
                                // §1491
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
                                            self.print(65689i32);
                                        }
                                    } else {
                                        if (i < ship_out_flag) {
                                            {
                                                if (i >= global_box_flag) {
                                                    {
                                                        self.print_esc(66605i32);
                                                        i = (i).wrapping_sub(32768i32);
                                                    }
                                                }
                                                self.print_esc(65852i32);
                                                self.print_int((i).wrapping_sub(1073741824i32));
                                                self.print_char(61i32);
                                            }
                                        } else {
                                            self.print_cmd_chr(leader_ship, (i).wrapping_sub(1073807261i32));
                                        }
                                    }
                                }
                            }
                            // §1489
                            self.print_esc(s);
                            // §1490
                            if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int() != 0i32) {
                                {
                                    self.print_char(32i32);
                                    if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(3i32)) as usize)].int() == exactly) {
                                        self.print(66244i32);
                                    } else {
                                        self.print(66245i32);
                                    }
                                    self.print_scaled(self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(2i32)) as usize)].int());
                                    self.print(65689i32);
                                }
                            }
                        }
                        // §1489
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
    // §988
    pub fn new_hyph_exceptions(&mut self) {
        let mut n: i32 = 0; // §988
        let mut j: i32 = 0; // §988
        let mut h: hyph_pointer = 0; // §988
        let mut k: str_number = 0; // §988
        let mut p: halfword = 0; // §988
        let mut q: halfword = 0; // §988
        let mut s: str_number = 0; // §988
        let mut t: str_number = 0; // §988
        let mut u: pool_pointer = 0; // §988
        let mut v: pool_pointer = 0; // §988
        'l_exit_f: {
            'l_not_found1_f: {
                self.scan_left_brace();
                if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() <= 0i32) {
                    self.cur_lang = 0i32;
                } else {
                    if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() > biggest_lang) {
                        self.cur_lang = 0i32;
                    } else {
                        self.cur_lang = self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int();
                    }
                }
                if self.trie_not_ready {
                    {
                        self.hyph_index = 0i32;
                        break 'l_not_found1_f;
                    }
                }
                if (self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].b1() != self.cur_lang) {
                    self.hyph_index = 0i32;
                } else {
                    self.hyph_index = self.trie[crate::ix::U(((self.hyph_start).wrapping_add(self.cur_lang)) as usize)].rh();
                }
            }
            n = 0i32;
            // §989
            p = (268435455i32).wrapping_neg();
            while true {
                {
                    self.get_x_token();
                    'l_reswitch_b: loop {
                        match self.cur_cmd {
                            letter | other_char | char_given => {
                                // §991
                                if (self.cur_chr == 45i32) {
                                    // §992
                                    {
                                        if (n < self.max_hyphenatable_length()) {
                                            {
                                                q = self.get_avail();
                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                self.mem[crate::ix::U((q) as usize)].set_hh_lh(n);
                                                p = q;
                                            }
                                        }
                                    }
                                } else {
                                    // §991
                                    {
                                        if ((self.hyph_index == 0i32) || (self.cur_chr > 255i32)) {
                                            { let __v1219 = self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh(); self.hc[crate::ix::U((0i32) as usize)] = __v1219; }
                                        } else {
                                            if (self.trie[crate::ix::U(((self.hyph_index).wrapping_add(self.cur_chr)) as usize)].b1() != self.cur_chr) {
                                                self.hc[crate::ix::U((0i32) as usize)] = 0i32;
                                            } else {
                                                { let __v1220 = self.trie[crate::ix::U(((self.hyph_index).wrapping_add(self.cur_chr)) as usize)].b0(); self.hc[crate::ix::U((0i32) as usize)] = __v1220; }
                                            }
                                        }
                                        if (self.hc[crate::ix::U((0i32) as usize)] == 0i32) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66374i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 66375i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66376i32;
                                                }
                                                self.error();
                                            }
                                        } else {
                                            if (n < self.max_hyphenatable_length()) {
                                                {
                                                    n = (n).wrapping_add(1i32);
                                                    if (self.hc[crate::ix::U((0i32) as usize)] < 65536i32) {
                                                        { let __v1221 = self.hc[crate::ix::U((0i32) as usize)]; self.hc[crate::ix::U((n) as usize)] = __v1221; }
                                                    } else {
                                                        {
                                                            { let __v1222 = (((self.hc[crate::ix::U((0i32) as usize)]).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.hc[crate::ix::U((n) as usize)] = __v1222; }
                                                            n = (n).wrapping_add(1i32);
                                                            { let __v1223 = ((self.hc[crate::ix::U((0i32) as usize)] % 1024i32)).wrapping_add(56320i32); self.hc[crate::ix::U((n) as usize)] = __v1223; }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            char_num => {
                                // §989
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
                                        // §993
                                        {
                                            'l_found1_f: {
                                                n = (n).wrapping_add(1i32);
                                                { let __v1224 = self.cur_lang; self.hc[crate::ix::U((n) as usize)] = __v1224; }
                                                {
                                                    if ((self.pool_ptr).wrapping_add(n) > pool_size) {
                                                        self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
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
                                                                if (self.hc[crate::ix::U((j) as usize)] > 65535i32) {
                                                                    {
                                                                        { let __ix1225 = self.pool_ptr; let __v1226 = (((self.hc[crate::ix::U((j) as usize)]).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.str_pool[crate::ix::U((__ix1225) as usize)] = __v1226; }
                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                        { let __ix1227 = self.pool_ptr; let __v1228 = ((self.hc[crate::ix::U((j) as usize)] % 1024i32)).wrapping_add(56320i32); self.str_pool[crate::ix::U((__ix1227) as usize)] = __v1228; }
                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                    }
                                                                } else {
                                                                    {
                                                                        { let __ix1229 = self.pool_ptr; let __v1230 = self.hc[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U((__ix1229) as usize)] = __v1230; }
                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        j = j.wrapping_add(1);
                                                    }
                                                }
                                                s = self.make_string();
                                                // §994
                                                if (self.hyph_count == hyph_size) {
                                                    self.overflow(66377i32, hyph_size);
                                                }
                                                self.hyph_count = (self.hyph_count).wrapping_add(1i32);
                                                while (self.hyph_word[crate::ix::U((h) as usize)] != 0i32) {
                                                    {
                                                        'l_not_found_f: {
                                                            'l_found_f: {
                                                                // §995
                                                                k = self.hyph_word[crate::ix::U((h) as usize)];
                                                                if (self.length(k) < self.length(s)) {
                                                                    break 'l_found_f;
                                                                }
                                                                if (self.length(k) > self.length(s)) {
                                                                    break 'l_not_found_f;
                                                                }
                                                                u = self.str_start[crate::ix::U(((k).wrapping_sub(65536i32)) as usize)];
                                                                v = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
                                                                loop {
                                                                    if (self.str_pool[crate::ix::U((u) as usize)] < self.str_pool[crate::ix::U((v) as usize)]) {
                                                                        break 'l_found_f;
                                                                    }
                                                                    if (self.str_pool[crate::ix::U((u) as usize)] > self.str_pool[crate::ix::U((v) as usize)]) {
                                                                        break 'l_not_found_f;
                                                                    }
                                                                    u = (u).wrapping_add(1i32);
                                                                    v = (v).wrapping_add(1i32);
                                                                    if (u == self.str_start[crate::ix::U((((k).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]) { break; }
                                                                }
                                                                {
                                                                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                                                    self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
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
                                                        // §994
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
                                    // §989
                                    if (self.cur_cmd == right_brace) {
                                        break 'l_exit_f;
                                    }
                                    n = 0i32;
                                    p = (268435455i32).wrapping_neg();
                                }
                            }
                            _ => {
                                // §990
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66020i32);
                                    }
                                    self.print_esc(66370i32);
                                    self.print(66371i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66372i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66373i32;
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
        // §988
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
    // §1022
    pub fn prune_page_top(&mut self, mut p: halfword, mut s: bool) -> halfword {
        let mut prune_page_top: halfword = 0;
        let mut prev_p: halfword = 0; // §1022
        let mut q: halfword = 0; // §1022
        let mut r: halfword = 0; // §1022
        prev_p = temp_head;
        self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
        while (p != (268435455i32).wrapping_neg()) {
            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                hlist_node | vlist_node | rule_node => {
                    // §1023
                    {
                        q = self.new_skip_param(split_top_skip_code);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(q);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        if (self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int() > 0i32) {
                            {
                                if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()) {
                                    { let __ix1231 = (self.temp_ptr).wrapping_add(1i32); let __v1232 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U((__ix1231) as usize)].set_int(__v1232); }
                                } else {
                                    { let __ix1233 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1233) as usize)].set_int(0i32); }
                                }
                            }
                        } else {
                            {
                                if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()) {
                                    { let __ix1234 = (self.temp_ptr).wrapping_add(1i32); let __v1235 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1234) as usize)].set_int(__v1235); }
                                } else {
                                    { let __ix1236 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1236) as usize)].set_int(0i32); }
                                }
                            }
                        }
                        p = (268435455i32).wrapping_neg();
                    }
                }
                whatsit_node | mark_node | ins_node => {
                    // §1022
                    {
                        prev_p = p;
                        p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                    }
                }
                glue_node | kern_node | penalty_node => {
                    {
                        q = p;
                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        if s {
                            {
                                if (self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] == (268435455i32).wrapping_neg()) {
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
                    self.confusion(66388i32);
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
    // §1024
    pub fn vert_break(&mut self, mut p: halfword, mut h: scaled, mut d: scaled) -> halfword {
        let mut vert_break: halfword = 0;
        let mut prev_p: halfword = 0; // §1024
        let mut q: halfword = 0; // §1024
        let mut r: halfword = 0; // §1024
        let mut pi: i32 = 0; // §1024
        let mut b: i32 = 0; // §1024
        let mut least_cost: i32 = 0; // §1024
        let mut best_place: halfword = 0; // §1024
        let mut prev_dp: scaled = 0; // §1024
        let mut t: small_number = 0; // §1024
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
                            // §1026
                            if (p == (268435455i32).wrapping_neg()) {
                                pi = (10000i32).wrapping_neg();
                            } else {
                                // §1027
                                match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                    hlist_node | vlist_node | rule_node => {
                                        {
                                            { let __v1237 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1237; }
                                            prev_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                            break 'l_not_found_f;
                                        }
                                    }
                                    whatsit_node => {
                                        // §1425
                                        {
                                            if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pic_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_node)) {
                                                {
                                                    { let __v1238 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1238; }
                                                    prev_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                }
                                            }
                                            break 'l_not_found_f;
                                        }
                                    }
                                    glue_node => {
                                        // §1027
                                        if (self.mem[crate::ix::U((prev_p) as usize)].hh().b0() < math_node) {
                                            pi = 0i32;
                                        } else {
                                            break 'l_L90_f;
                                        }
                                    }
                                    kern_node => {
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
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
                                        self.confusion(66389i32);
                                    }
                                }
                            }
                            // §1028
                            if (pi < inf_penalty) {
                                {
                                    // §1029
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
                                    // §1028
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
                            // §1026
                            if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > kern_node)) {
                                break 'l_not_found_f;
                            }
                        }
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                            // §1030
                            q = p;
                        } else {
                            {
                                q = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1239 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1240 = (self.active_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((__ix1239) - 1) as usize)] = __v1240; }
                                { let __v1241 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1241; }
                                if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                    {
                                        if self.is_bit_set(self.eqtb[crate::ix::U(((7892335i32) - 1) as usize)].int(), 1i32) {
                                            {
                                                self.old_selector_ignored_err = self.selector;
                                                self.selector = log_only;
                                                {
                                                    crate::system::wr_ln(&mut self.log_file);
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "ignored: ");
                                                }
                                                self.print(66390i32);
                                                self.selector = self.old_selector_ignored_err;
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
                                                    self.print(66390i32);
                                                }
                                                {
                                                    self.help_ptr = 4i32;
                                                    self.help_line[crate::ix::U((3i32) as usize)] = 66391i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 66392i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 66393i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66343i32;
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
                        { let __v1242 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1242; }
                        prev_dp = 0i32;
                    }
                    // §1026
                    if (prev_dp > d) {
                        {
                            { let __v1243 = ((self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(prev_dp)).wrapping_sub(d); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1243; }
                            prev_dp = d;
                        }
                    }
                    // §1024
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
    // §1636
    pub fn do_marks(&mut self, mut a: small_number, mut l: small_number, mut q: halfword) -> bool {
        let mut do_marks: bool = false;
        let mut i: small_number = 0; // §1636
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
                            if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                if self.do_marks(a, (l).wrapping_add(1i32), self.cur_ptr) {
                                    {
                                        if (((i) % 2) != 0) {
                                            self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                        } else {
                                            self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                        }
                                        { let __v1244 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v1244); }
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
                        q = (268435455i32).wrapping_neg();
                    }
                }
            }
        } else {
            {
                match a {
                    vsplit_init => {
                        // §1637
                        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                            {
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh());
                                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().lh());
                                self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                            }
                        }
                    }
                    fire_up_init => {
                        // §1639
                        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                            {
                                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                                    self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh());
                                }
                                self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh());
                                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                if (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                    {
                                        self.delete_token_ref(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh());
                                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                    }
                                } else {
                                    { let __ix1245 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh(); let __v1246 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1245) as usize)].set_hh_lh(__v1246); }
                                }
                                { let __v1247 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1247); }
                            }
                        }
                    }
                    fire_up_done => {
                        // §1640
                        if ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) {
                            {
                                { let __v1248 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v1248); }
                                { let __ix1249 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh(); let __v1250 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1249) as usize)].set_hh_lh(__v1250); }
                            }
                        }
                    }
                    destroy_marks => {
                        // §1642
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
                                    if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                        {
                                            self.delete_token_ref(self.cur_ptr);
                                            if (((i) % 2) != 0) {
                                                self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                            } else {
                                                self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
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
                // §1636
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
                    if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
                        {
                            self.free_node(q, mark_class_node_size);
                            q = (268435455i32).wrapping_neg();
                        }
                    }
                }
            }
        }
        do_marks = (q == (268435455i32).wrapping_neg());
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
    // §1031
    pub fn vsplit(&mut self, mut n: halfword, mut h: scaled) -> halfword {
        let mut vsplit: halfword = 0;
        let mut v: halfword = 0; // §1031
        let mut p: halfword = 0; // §1031
        let mut q: halfword = 0; // §1031
        'l_exit_f: {
            'l_done_f: {
                self.cur_val = n;
                if (self.cur_val < 256i32) {
                    v = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                } else {
                    {
                        self.find_sa_element(box_val, self.cur_val, false);
                        if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                            v = (268435455i32).wrapping_neg();
                        } else {
                            v = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                        }
                    }
                }
                self.flush_node_list(self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)]);
                self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] = (268435455i32).wrapping_neg();
                if (self.sa_root[crate::ix::U((mark_val) as usize)] != (268435455i32).wrapping_neg()) {
                    if self.do_marks(vsplit_init, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                        self.sa_root[crate::ix::U((mark_val) as usize)] = (268435455i32).wrapping_neg();
                    }
                }
                if (self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] != (268435455i32).wrapping_neg()) {
                    {
                        self.delete_token_ref(self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]);
                        self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] = (268435455i32).wrapping_neg();
                        self.delete_token_ref(self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]);
                        self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = (268435455i32).wrapping_neg();
                    }
                }
                // §1032
                if (v == (268435455i32).wrapping_neg()) {
                    {
                        vsplit = (268435455i32).wrapping_neg();
                        break 'l_exit_f;
                    }
                }
                if (self.mem[crate::ix::U((v) as usize)].hh().b0() != vlist_node) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(65626i32);
                        }
                        self.print_esc(66394i32);
                        self.print(66395i32);
                        self.print_esc(66396i32);
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66397i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66398i32;
                        }
                        self.error();
                        vsplit = (268435455i32).wrapping_neg();
                        break 'l_exit_f;
                    }
                }
                // §1031
                q = self.vert_break(self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh(), h, self.eqtb[crate::ix::U(((9006726i32) - 1) as usize)].int());
                // §1033
                p = self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh();
                if (p == q) {
                    self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                } else {
                    while true {
                        {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node) {
                                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                    // §1638
                                    {
                                        self.find_sa_element(mark_val, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), true);
                                        if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                            {
                                                { let __ix1251 = (self.cur_ptr).wrapping_add(2i32); let __v1252 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1251) as usize)].set_hh_rh(__v1252); }
                                                { let __ix1253 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1254 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1253) as usize)].set_hh_lh(__v1254); }
                                            }
                                        } else {
                                            self.delete_token_ref(self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(3i32)) as usize)].hh().lh());
                                        }
                                        { let __ix1255 = (self.cur_ptr).wrapping_add(3i32); let __v1256 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1255) as usize)].set_hh_lh(__v1256); }
                                        { let __ix1257 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1258 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1257) as usize)].set_hh_lh(__v1258); }
                                    }
                                } else {
                                    // §1033
                                    if (self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] == (268435455i32).wrapping_neg()) {
                                        {
                                            { let __v1259 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] = __v1259; }
                                            { let __v1260 = self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]; self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = __v1260; }
                                            { let __ix1261 = self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]; let __v1262 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((split_first_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(2i32); self.mem[crate::ix::U((__ix1261) as usize)].set_hh_lh(__v1262); }
                                        }
                                    } else {
                                        {
                                            self.delete_token_ref(self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]);
                                            { let __v1263 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = __v1263; }
                                            { let __ix1264 = self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]; let __v1265 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1264) as usize)].set_hh_lh(__v1265); }
                                        }
                                    }
                                }
                            }
                            if (self.mem[crate::ix::U((p) as usize)].hh().rh() == q) {
                                {
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                    break 'l_done_f;
                                }
                            }
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §1031
            q = self.prune_page_top(q, (self.eqtb[crate::ix::U(((7892332i32) - 1) as usize)].int() > 0i32));
            p = self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].hh().rh();
            self.free_node(v, box_node_size);
            if (q != (268435455i32).wrapping_neg()) {
                q = self.vpackage(q, 0i32, additional, max_dimen);
            }
            if (self.cur_val < 256i32) {
                { let __ix1266 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1266) - 1) as usize)].set_hh_rh(q); }
            } else {
                {
                    self.find_sa_element(box_val, self.cur_val, false);
                    if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                        {
                            { let __ix1267 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1267) as usize)].set_hh_rh(q); }
                            { let __ix1268 = (self.cur_ptr).wrapping_add(1i32); let __v1269 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1268) as usize)].set_hh_lh(__v1269); }
                            self.delete_sa_ref(self.cur_ptr);
                        }
                    }
                }
            }
            vsplit = self.vpackage(p, h, exactly, self.eqtb[crate::ix::U(((9006726i32) - 1) as usize)].int());
        }
        vsplit
    }

    // §1039
    pub fn print_totals(&mut self) {
        self.print_scaled(self.page_so_far[crate::ix::U((1i32) as usize)]);
        if (self.page_so_far[crate::ix::U((2i32) as usize)] != 0i32) {
            {
                self.print(65598i32);
                self.print_scaled(self.page_so_far[crate::ix::U((2i32) as usize)]);
                self.print(65626i32);
            }
        }
        if (self.page_so_far[crate::ix::U((3i32) as usize)] != 0i32) {
            {
                self.print(65598i32);
                self.print_scaled(self.page_so_far[crate::ix::U((3i32) as usize)]);
                self.print(65597i32);
            }
        }
        if (self.page_so_far[crate::ix::U((4i32) as usize)] != 0i32) {
            {
                self.print(65598i32);
                self.print_scaled(self.page_so_far[crate::ix::U((4i32) as usize)]);
                self.print(66407i32);
            }
        }
        if (self.page_so_far[crate::ix::U((5i32) as usize)] != 0i32) {
            {
                self.print(65598i32);
                self.print_scaled(self.page_so_far[crate::ix::U((5i32) as usize)]);
                self.print(66408i32);
            }
        }
        if (self.page_so_far[crate::ix::U((6i32) as usize)] != 0i32) {
            {
                self.print(65599i32);
                self.print_scaled(self.page_so_far[crate::ix::U((6i32) as usize)]);
            }
        }
    }

    /// Here is a procedure that is called when the `page_contents` is changing
    /// from `empty` to `inserts_only` or `box_there`.
    // §1041
    pub fn freeze_page_specs(&mut self, mut s: small_number) {
        self.page_contents = s;
        { let __v1270 = self.eqtb[crate::ix::U(((9006724i32) - 1) as usize)].int(); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1270; }
        self.page_max_depth = self.eqtb[crate::ix::U(((9006725i32) - 1) as usize)].int();
        self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((1i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((2i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((3i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((4i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((5i32) as usize)] = 0i32;
        self.page_so_far[crate::ix::U((6i32) as usize)] = 0i32;
        self.least_page_cost = awful_bad;
        if (self.eqtb[crate::ix::U(((7892297i32) - 1) as usize)].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_nl(66416i32);
                self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                self.print(66417i32);
                self.print_scaled(self.page_max_depth);
                self.end_diagnostic(false);
            }
        }
    }

    /// At certain times box 255 is supposed to be void (i.e., `null`),
    /// or an insertion box is supposed to be ready to accept a vertical list.
    /// If not, an error message is printed, and the following subroutine
    /// flushes the unwanted contents, reporting them to the user.
    // §1046
    pub fn box_error(&mut self, mut n: eight_bits) {
        self.error();
        self.begin_diagnostic();
        self.print_nl(66231i32);
        self.show_box(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        self.end_diagnostic(true);
        self.flush_node_list(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
    }

    /// The following procedure guarantees that a given box register
    /// does not contain an \.{\\hbox}.
    // §1047
    pub fn ensure_vbox(&mut self, mut n: eight_bits) {
        let mut p: halfword = 0; // §1047
        p = self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh();
        if (p != (268435455i32).wrapping_neg()) {
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) {
                {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66418i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66419i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66420i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66421i32;
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
    // §1066
    pub fn fire_up(&mut self, mut c: halfword) {
        let mut p: halfword = 0; // §1066
        let mut q: halfword = 0; // §1066
        let mut r: halfword = 0; // §1066
        let mut s: halfword = 0; // §1066
        let mut prev_p: halfword = 0; // §1066
        let mut n: i32 = 0; // §1066
        let mut wait: bool = false; // §1066
        let mut save_vbadness: i32 = 0; // §1066
        let mut save_vfuzz: scaled = 0; // §1066
        let mut save_split_top_skip: halfword = 0; // §1066
        'l_exit_f: {
            // §1067
            if (self.mem[crate::ix::U((self.best_page_break) as usize)].hh().b0() == penalty_node) {
                {
                    self.geq_word_define(7892303i32, self.mem[crate::ix::U(((self.best_page_break).wrapping_add(1i32)) as usize)].int());
                    { let __ix1271 = (self.best_page_break).wrapping_add(1i32); self.mem[crate::ix::U((__ix1271) as usize)].set_int(inf_penalty); }
                }
            } else {
                self.geq_word_define(7892303i32, inf_penalty);
            }
            // §1066
            if (self.sa_root[crate::ix::U((mark_val) as usize)] != (268435455i32).wrapping_neg()) {
                if self.do_marks(fire_up_init, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                    self.sa_root[crate::ix::U((mark_val) as usize)] = (268435455i32).wrapping_neg();
                }
            }
            if (self.cur_mark[crate::ix::U((bot_mark_code) as usize)] != (268435455i32).wrapping_neg()) {
                {
                    if (self.cur_mark[crate::ix::U((top_mark_code) as usize)] != (268435455i32).wrapping_neg()) {
                        self.delete_token_ref(self.cur_mark[crate::ix::U((top_mark_code) as usize)]);
                    }
                    { let __v1272 = self.cur_mark[crate::ix::U((bot_mark_code) as usize)]; self.cur_mark[crate::ix::U((top_mark_code) as usize)] = __v1272; }
                    { let __ix1273 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; let __v1274 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((top_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1273) as usize)].set_hh_lh(__v1274); }
                    self.delete_token_ref(self.cur_mark[crate::ix::U((first_mark_code) as usize)]);
                    self.cur_mark[crate::ix::U((first_mark_code) as usize)] = (268435455i32).wrapping_neg();
                }
            }
            // §1068
            if (c == self.best_page_break) {
                self.best_page_break = (268435455i32).wrapping_neg();
            }
            // §1069
            if (self.eqtb[crate::ix::U(((1206822i32) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(65626i32);
                    }
                    self.print_esc(65701i32);
                    self.print(66432i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66433i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66421i32;
                    }
                    self.box_error(255i32);
                }
            }
            // §1068
            self.insert_penalties = 0i32;
            save_split_top_skip = self.eqtb[crate::ix::U(((1205774i32) - 1) as usize)].hh().rh();
            if (self.eqtb[crate::ix::U(((7892317i32) - 1) as usize)].int() <= 0i32) {
                // §1072
                {
                    r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                    while (r != page_ins_head) {
                        {
                            if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                                {
                                    n = self.mem[crate::ix::U((r) as usize)].hh().b1();
                                    self.ensure_vbox(n);
                                    if (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                        { let __v1275 = self.new_null_box(); self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh(__v1275); }
                                    }
                                    p = (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(5i32);
                                    while (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
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
            // §1068
            q = hold_head;
            self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
            prev_p = page_head;
            p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
            while (p != self.best_page_break) {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) {
                        {
                            if (self.eqtb[crate::ix::U(((7892317i32) - 1) as usize)].int() <= 0i32) {
                                // §1074
                                {
                                    r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                    while (self.mem[crate::ix::U((r) as usize)].hh().b1() != self.mem[crate::ix::U((p) as usize)].hh().b1()) {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                    if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
                                        wait = true;
                                    } else {
                                        {
                                            wait = false;
                                            s = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().rh();
                                            { let __v1276 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh(); self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1276); }
                                            if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() == p) {
                                                // §1075
                                                {
                                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == split_up) {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == p) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
                                                            {
                                                                while (self.mem[crate::ix::U((s) as usize)].hh().rh() != self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh()) {
                                                                    s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                }
                                                                self.mem[crate::ix::U((s) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                                { let __v1277 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(); self.eqtb[crate::ix::U(((1205774i32) - 1) as usize)].set_hh_rh(__v1277); }
                                                                { let __v1278 = self.prune_page_top(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh(), false); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_lh(__v1278); }
                                                                if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                                                                    {
                                                                        self.temp_ptr = self.vpackage(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh(), 0i32, additional, max_dimen);
                                                                        { let __v1279 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v1279); }
                                                                        self.free_node(self.temp_ptr, box_node_size);
                                                                        wait = true;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                                    n = self.mem[crate::ix::U((r) as usize)].hh().b1();
                                                    self.temp_ptr = self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(5i32)) as usize)].hh().rh();
                                                    self.free_node(self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh(), box_node_size);
                                                    { let __v1280 = self.vpackage(self.temp_ptr, 0i32, additional, max_dimen); self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].set_hh_rh(__v1280); }
                                                }
                                            } else {
                                                // §1074
                                                {
                                                    while (self.mem[crate::ix::U((s) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                    }
                                                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_rh(s);
                                                }
                                            }
                                        }
                                    }
                                    // §1076
                                    { let __v1281 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(__v1281); }
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
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
                        // §1068
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node) {
                            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                // §1641
                                {
                                    self.find_sa_element(mark_val, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), true);
                                    if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                        {
                                            { let __ix1282 = (self.cur_ptr).wrapping_add(1i32); let __v1283 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1282) as usize)].set_hh_rh(__v1283); }
                                            { let __ix1284 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1285 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1284) as usize)].set_hh_lh(__v1285); }
                                        }
                                    }
                                    if (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                                        self.delete_token_ref(self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].hh().lh());
                                    }
                                    { let __ix1286 = (self.cur_ptr).wrapping_add(2i32); let __v1287 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1286) as usize)].set_hh_lh(__v1287); }
                                    { let __ix1288 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1289 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1288) as usize)].set_hh_lh(__v1289); }
                                }
                            } else {
                                // §1070
                                {
                                    if (self.cur_mark[crate::ix::U((first_mark_code) as usize)] == (268435455i32).wrapping_neg()) {
                                        {
                                            { let __v1290 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((first_mark_code) as usize)] = __v1290; }
                                            { let __ix1291 = self.cur_mark[crate::ix::U((first_mark_code) as usize)]; let __v1292 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((first_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1291) as usize)].set_hh_lh(__v1292); }
                                        }
                                    }
                                    if (self.cur_mark[crate::ix::U((bot_mark_code) as usize)] != (268435455i32).wrapping_neg()) {
                                        self.delete_token_ref(self.cur_mark[crate::ix::U((bot_mark_code) as usize)]);
                                    }
                                    { let __v1293 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.cur_mark[crate::ix::U((bot_mark_code) as usize)] = __v1293; }
                                    { let __ix1294 = self.cur_mark[crate::ix::U((bot_mark_code) as usize)]; let __v1295 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((bot_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1294) as usize)].set_hh_lh(__v1295); }
                                }
                            }
                        }
                    }
                    // §1068
                    prev_p = p;
                    p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                }
            }
            self.eqtb[crate::ix::U(((1205774i32) - 1) as usize)].set_hh_rh(save_split_top_skip);
            // §1071
            if (p != (268435455i32).wrapping_neg()) {
                {
                    if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                        if (self.nest_ptr == 0i32) {
                            self.cur_list.tail_field = self.page_tail;
                        } else {
                            self.nest[crate::ix::U((0i32) as usize)].tail_field = self.page_tail;
                        }
                    }
                    { let __ix1296 = self.page_tail; let __v1297 = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1296) as usize)].set_hh_rh(__v1297); }
                    self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(p);
                    self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                }
            }
            save_vbadness = self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].int();
            self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].set_int(inf_bad);
            save_vfuzz = self.eqtb[crate::ix::U(((9006729i32) - 1) as usize)].int();
            self.eqtb[crate::ix::U(((9006729i32) - 1) as usize)].set_int(max_dimen);
            { let __v1298 = self.vpackage(self.mem[crate::ix::U((page_head) as usize)].hh().rh(), self.best_size, exactly, self.page_max_depth); self.eqtb[crate::ix::U(((1206822i32) - 1) as usize)].set_hh_rh(__v1298); }
            self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].set_int(save_vbadness);
            self.eqtb[crate::ix::U(((9006729i32) - 1) as usize)].set_int(save_vfuzz);
            if (self.last_glue != max_halfword) {
                self.delete_glue_ref(self.last_glue);
            }
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
            // §1071
            if (q != hold_head) {
                {
                    { let __v1299 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((page_head) as usize)].set_hh_rh(__v1299); }
                    self.page_tail = q;
                }
            }
            // §1073
            r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
            while (r != page_ins_head) {
                {
                    q = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    self.free_node(r, page_ins_node_size);
                    r = q;
                }
            }
            self.mem[crate::ix::U((page_ins_head) as usize)].set_hh_rh(page_ins_head);
            // §1066
            if (self.sa_root[crate::ix::U((mark_val) as usize)] != (268435455i32).wrapping_neg()) {
                if self.do_marks(fire_up_done, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                    self.sa_root[crate::ix::U((mark_val) as usize)] = (268435455i32).wrapping_neg();
                }
            }
            if ((self.cur_mark[crate::ix::U((top_mark_code) as usize)] != (268435455i32).wrapping_neg()) && (self.cur_mark[crate::ix::U((first_mark_code) as usize)] == (268435455i32).wrapping_neg())) {
                {
                    { let __v1300 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; self.cur_mark[crate::ix::U((first_mark_code) as usize)] = __v1300; }
                    { let __ix1301 = self.cur_mark[crate::ix::U((top_mark_code) as usize)]; let __v1302 = (self.mem[crate::ix::U((self.cur_mark[crate::ix::U((top_mark_code) as usize)]) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1301) as usize)].set_hh_lh(__v1302); }
                }
            }
            if (self.eqtb[crate::ix::U(((output_routine_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                if (self.dead_cycles >= self.eqtb[crate::ix::U(((7892304i32) - 1) as usize)].int()) {
                    // §1078
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66434i32);
                        }
                        self.print_int(self.dead_cycles);
                        self.print(66435i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66436i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66437i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66438i32;
                        }
                        self.error();
                    }
                } else {
                    // §1079
                    {
                        self.output_active = true;
                        self.dead_cycles = (self.dead_cycles).wrapping_add(1i32);
                        self.push_nest();
                        self.cur_list.mode_field = (1i32).wrapping_neg();
                        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                        self.cur_list.ml_field = (self.line).wrapping_neg();
                        self.begin_token_list(self.eqtb[crate::ix::U(((output_routine_loc) - 1) as usize)].hh().rh(), output_text);
                        self.new_save_level(output_group);
                        self.normal_paragraph();
                        self.scan_left_brace();
                        break 'l_exit_f;
                    }
                }
            }
            // §1077
            {
                if (self.mem[crate::ix::U((page_head) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                    {
                        if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                            if (self.nest_ptr == 0i32) {
                                self.cur_list.tail_field = self.page_tail;
                            } else {
                                self.nest[crate::ix::U((0i32) as usize)].tail_field = self.page_tail;
                            }
                        } else {
                            { let __ix1303 = self.page_tail; let __v1304 = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1303) as usize)].set_hh_rh(__v1304); }
                        }
                        { let __v1305 = self.mem[crate::ix::U((page_head) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1305); }
                        self.mem[crate::ix::U((page_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        self.page_tail = page_head;
                    }
                }
                self.flush_node_list(self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)]);
                self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = (268435455i32).wrapping_neg();
                self.ship_out(self.eqtb[crate::ix::U(((1206822i32) - 1) as usize)].hh().rh());
                self.eqtb[crate::ix::U(((1206822i32) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
            }
        }
        // §1066
    }

    /// \TeX\ is not always in vertical mode at the time `build_page`
    /// is called; the current mode reflects what \TeX\ should return to, after
    /// the contribution list has been emptied. A call on `build_page` should
    /// be immediately followed by ``goto big_switch`', which is \TeX's central
    /// control point.
    // §1048
    pub fn build_page(&mut self) {
        let mut p: halfword = 0; // §1048
        let mut q: halfword = 0; // §1048
        let mut r: halfword = 0; // §1048
        let mut b: i32 = 0; // §1048
        let mut c: i32 = 0; // §1048
        let mut pi: i32 = 0; // §1048
        let mut n: i32 = 0; // §1048
        let mut delta: scaled = 0; // §1048
        let mut h: scaled = 0; // §1048
        let mut w: scaled = 0; // §1048
        'l_exit_f: {
            if ((self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == (268435455i32).wrapping_neg()) || self.output_active) {
                break 'l_exit_f;
            }
            loop {
                // goto labels: continue, L90, L80, done1, done
                let mut __goto_1: i32 = 0;
                'l_dispatch_1: loop {
                    if __goto_1 <= 0 {
                        p = self.mem[crate::ix::U((contrib_head) as usize)].hh().rh();
                        // §1050
                        if (self.last_glue != max_halfword) {
                            self.delete_glue_ref(self.last_glue);
                        }
                        self.last_penalty = 0i32;
                        self.last_kern = 0i32;
                        self.last_node_type = (self.mem[crate::ix::U((p) as usize)].hh().b0()).wrapping_add(1i32);
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) {
                            {
                                self.last_glue = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1306 = self.last_glue; let __v1307 = (self.mem[crate::ix::U((self.last_glue) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1306) as usize)].set_hh_rh(__v1307); }
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
                        // §1054
                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                            hlist_node | vlist_node | rule_node => {
                                if (self.page_contents < box_there) {
                                    // §1055
                                    {
                                        if (self.page_contents == empty) {
                                            self.freeze_page_specs(box_there);
                                        } else {
                                            self.page_contents = box_there;
                                        }
                                        q = self.new_skip_param(top_skip_code);
                                        if (self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int() > 0i32) {
                                            {
                                                if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()) {
                                                    { let __ix1308 = (self.temp_ptr).wrapping_add(1i32); let __v1309 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U((__ix1308) as usize)].set_int(__v1309); }
                                                } else {
                                                    { let __ix1310 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1310) as usize)].set_int(0i32); }
                                                }
                                            }
                                        } else {
                                            {
                                                if (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()) {
                                                    { let __ix1311 = (self.temp_ptr).wrapping_add(1i32); let __v1312 = (self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1311) as usize)].set_int(__v1312); }
                                                } else {
                                                    { let __ix1313 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1313) as usize)].set_int(0i32); }
                                                }
                                            }
                                        }
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                        self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(q);
                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    // §1056
                                    {
                                        { let __v1314 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1314; }
                                        { let __v1315 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.page_so_far[crate::ix::U((7i32) as usize)] = __v1315; }
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            whatsit_node => {
                                // §1424
                                {
                                    if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pic_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_node)) {
                                        {
                                            { let __v1316 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1316; }
                                            { let __v1317 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.page_so_far[crate::ix::U((7i32) as usize)] = __v1317; }
                                        }
                                    }
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            glue_node => {
                                // §1054
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
                                    if (self.mem[crate::ix::U((p) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
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
                                // §1062
                                {
                                    if (self.page_contents == empty) {
                                        self.freeze_page_specs(inserts_only);
                                    }
                                    n = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    r = page_ins_head;
                                    while (n >= self.mem[crate::ix::U((self.mem[crate::ix::U((r) as usize)].hh().rh()) as usize)].hh().b1()) {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                    n = n;
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b1() != n) {
                                        // §1063
                                        {
                                            q = self.get_node(page_ins_node_size);
                                            { let __v1318 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1318); }
                                            self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                                            r = q;
                                            self.mem[crate::ix::U((r) as usize)].set_hh_b1(n);
                                            self.mem[crate::ix::U((r) as usize)].set_hh_b0(inserting);
                                            self.ensure_vbox(n);
                                            if (self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(0i32);
                                            } else {
                                                { let __v1319 = (self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.eqtb[crate::ix::U((((box_base).wrapping_add(n)) - 1) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1319); }
                                            }
                                            self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                            q = self.eqtb[crate::ix::U((((skip_base).wrapping_add(n)) - 1) as usize)].hh().rh();
                                            if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() == 1000i32) {
                                                h = self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int();
                                            } else {
                                                h = (self.x_over_n(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int(), 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int());
                                            }
                                            { let __v1320 = ((self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(h)).wrapping_sub(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1320; }
                                            { let __ix1321 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1322 = (self.page_so_far[crate::ix::U(((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.page_so_far[crate::ix::U((__ix1321) as usize)] = __v1322; }
                                            { let __v1323 = (self.page_so_far[crate::ix::U((6i32) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((6i32) as usize)] = __v1323; }
                                            if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                {
                                                    {
                                                        if (self.interaction == error_stop_mode) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(65544i32);
                                                        }
                                                        self.print(66427i32);
                                                    }
                                                    self.print_esc(65687i32);
                                                    self.print_int(n);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[crate::ix::U((2i32) as usize)] = 66428i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 66429i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 66343i32;
                                                    }
                                                    self.error();
                                                }
                                            }
                                        }
                                    }
                                    // §1062
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
                                                    { let __v1324 = (self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(h); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1324; }
                                                    { let __v1325 = (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1325); }
                                                }
                                            } else {
                                                // §1064
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
                                                    { let __v1326 = (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.best_height_plus_depth); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1326); }
                                                    if (self.eqtb[crate::ix::U(((7892297i32) - 1) as usize)].int() > 0i32) {
                                                        // §1065
                                                        {
                                                            self.begin_diagnostic();
                                                            self.print_nl(66430i32);
                                                            self.print_int(n);
                                                            self.print(66431i32);
                                                            self.print_scaled(w);
                                                            self.print_char(44i32);
                                                            self.print_scaled(self.best_height_plus_depth);
                                                            self.print(66358i32);
                                                            if (q == (268435455i32).wrapping_neg()) {
                                                                self.print_int((10000i32).wrapping_neg());
                                                            } else {
                                                                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) {
                                                                    self.print_int(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                                                                } else {
                                                                    self.print_char(48i32);
                                                                }
                                                            }
                                                            self.end_diagnostic(false);
                                                        }
                                                    }
                                                    // §1064
                                                    if (self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int() != 1000i32) {
                                                        self.best_height_plus_depth = (self.x_over_n(self.best_height_plus_depth, 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(n)) - 1) as usize)].int());
                                                    }
                                                    { let __v1327 = (self.page_so_far[crate::ix::U((0i32) as usize)]).wrapping_sub(self.best_height_plus_depth); self.page_so_far[crate::ix::U((0i32) as usize)] = __v1327; }
                                                    self.mem[crate::ix::U((r) as usize)].set_hh_b0(split_up);
                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(q);
                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                                                    if (q == (268435455i32).wrapping_neg()) {
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
                                    // §1062
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            _ => {
                                // §1054
                                self.confusion(66422i32);
                            }
                        }
                        // §1059
                        if (pi < inf_penalty) {
                            {
                                // §1061
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
                                // §1059
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
                                if (self.eqtb[crate::ix::U(((7892297i32) - 1) as usize)].int() > 0i32) {
                                    // §1060
                                    {
                                        self.begin_diagnostic();
                                        self.print_nl(37i32);
                                        self.print(66354i32);
                                        self.print_totals();
                                        self.print(66425i32);
                                        self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                                        self.print(66357i32);
                                        if (b == awful_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(b);
                                        }
                                        self.print(66358i32);
                                        self.print_int(pi);
                                        self.print(66426i32);
                                        if (c == awful_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(c);
                                        }
                                        if (c <= self.least_page_cost) {
                                            self.print_char(35i32);
                                        }
                                        self.end_diagnostic(false);
                                    }
                                }
                                // §1059
                                if (c <= self.least_page_cost) {
                                    {
                                        self.best_page_break = p;
                                        self.best_size = self.page_so_far[crate::ix::U((0i32) as usize)];
                                        self.least_page_cost = c;
                                        r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                        while (r != page_ins_head) {
                                            {
                                                { let __v1328 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(__v1328); }
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
                        // §1051
                        if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > kern_node)) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    if __goto_1 <= 1 { // L90
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                            // §1058
                            q = p;
                        } else {
                            {
                                q = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix1329 = (2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0()); let __v1330 = (self.page_so_far[crate::ix::U(((2i32).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().b0())) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()); self.page_so_far[crate::ix::U((__ix1329) as usize)] = __v1330; }
                                { let __v1331 = (self.page_so_far[crate::ix::U((6i32) as usize)]).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()); self.page_so_far[crate::ix::U((6i32) as usize)] = __v1331; }
                                if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != normal) && (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                    {
                                        {
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(65544i32);
                                            }
                                            self.print(66423i32);
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[crate::ix::U((3i32) as usize)] = 66424i32;
                                            self.help_line[crate::ix::U((2i32) as usize)] = 66392i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 66393i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 66343i32;
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
                        { let __v1332 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1332; }
                        self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
                    }
                    if __goto_1 <= 2 { // L80
                        // §1051
                        if (self.page_so_far[crate::ix::U((7i32) as usize)] > self.page_max_depth) {
                            // §1057
                            {
                                { let __v1333 = ((self.page_so_far[crate::ix::U((1i32) as usize)]).wrapping_add(self.page_so_far[crate::ix::U((7i32) as usize)])).wrapping_sub(self.page_max_depth); self.page_so_far[crate::ix::U((1i32) as usize)] = __v1333; }
                                { let __v1334 = self.page_max_depth; self.page_so_far[crate::ix::U((7i32) as usize)] = __v1334; }
                            }
                        }
                        // §1052
                        { let __ix1335 = self.page_tail; self.mem[crate::ix::U((__ix1335) as usize)].set_hh_rh(p); }
                        self.page_tail = p;
                        { let __v1336 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1336); }
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        { __goto_1 = 4; continue 'l_dispatch_1; }
                    }
                    if __goto_1 <= 3 { // done1
                        // §1051
                        { let __v1337 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v1337); }
                        // §1053
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        if (self.eqtb[crate::ix::U(((7892332i32) - 1) as usize)].int() > 0i32) {
                            {
                                if (self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] == (268435455i32).wrapping_neg()) {
                                    self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = p;
                                } else {
                                    { let __ix1338 = self.disc_ptr[crate::ix::U(((copy_code) - 1) as usize)]; self.mem[crate::ix::U((__ix1338) as usize)].set_hh_rh(p); }
                                }
                                self.disc_ptr[crate::ix::U(((copy_code) - 1) as usize)] = p;
                            }
                        } else {
                            self.flush_node_list(p);
                        }
                    }
                    if __goto_1 <= 4 { // done
                        // §1051
                    }
                    break 'l_dispatch_1;
                }
                if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() == (268435455i32).wrapping_neg()) { break; }
            }
            // §1049
            if (self.nest_ptr == 0i32) {
                self.cur_list.tail_field = contrib_head;
            } else {
                self.nest[crate::ix::U((0i32) as usize)].tail_field = contrib_head;
            }
        }
        // §1048
    }

    /// @<Declare act...
    // §1097
    pub fn app_space(&mut self) {
        let mut q: halfword = 0; // §1097
        if ((self.cur_list.aux_field.hh().lh() >= 2000i32) && (self.eqtb[crate::ix::U(((1205777i32) - 1) as usize)].hh().rh() != zero_glue)) {
            q = self.new_param_glue(xspace_skip_code);
        } else {
            {
                if (self.eqtb[crate::ix::U(((1205776i32) - 1) as usize)].hh().rh() != zero_glue) {
                    self.main_p = self.eqtb[crate::ix::U(((1205776i32) - 1) as usize)].hh().rh();
                } else {
                    // §1096
                    {
                        self.main_p = self.font_glue[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)];
                        if (self.main_p == (268435455i32).wrapping_neg()) {
                            {
                                self.main_p = self.new_spec(zero_glue);
                                self.main_k = (self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)]).wrapping_add(2i32);
                                { let __ix1339 = (self.main_p).wrapping_add(1i32); let __v1340 = self.font_info[crate::ix::U((self.main_k) as usize)].int(); self.mem[crate::ix::U((__ix1339) as usize)].set_int(__v1340); }
                                { let __ix1341 = (self.main_p).wrapping_add(2i32); let __v1342 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix1341) as usize)].set_int(__v1342); }
                                { let __ix1343 = (self.main_p).wrapping_add(3i32); let __v1344 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U((__ix1343) as usize)].set_int(__v1344); }
                                { let __ix1345 = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(); let __v1346 = self.main_p; self.font_glue[crate::ix::U((__ix1345) as usize)] = __v1346; }
                            }
                        }
                    }
                }
                // §1097
                self.main_p = self.new_spec(self.main_p);
                // §1098
                if (self.cur_list.aux_field.hh().lh() >= 2000i32) {
                    { let __ix1347 = (self.main_p).wrapping_add(1i32); let __v1348 = (self.mem[crate::ix::U(((self.main_p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((extra_space_code).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); self.mem[crate::ix::U((__ix1347) as usize)].set_int(__v1348); }
                }
                { let __ix1349 = (self.main_p).wrapping_add(2i32); let __v1350 = self.xn_over_d(self.mem[crate::ix::U(((self.main_p).wrapping_add(2i32)) as usize)].int(), self.cur_list.aux_field.hh().lh(), 1000i32); self.mem[crate::ix::U((__ix1349) as usize)].set_int(__v1350); }
                { let __ix1351 = (self.main_p).wrapping_add(3i32); let __v1352 = self.xn_over_d(self.mem[crate::ix::U(((self.main_p).wrapping_add(3i32)) as usize)].int(), 1000i32, self.cur_list.aux_field.hh().lh()); self.mem[crate::ix::U((__ix1351) as usize)].set_int(__v1352); }
                // §1097
                q = self.new_glue(self.main_p);
                { let __ix1353 = self.main_p; self.mem[crate::ix::U((__ix1353) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
            }
        }
        { let __ix1354 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1354) as usize)].set_hh_rh(q); }
        self.cur_list.tail_field = q;
    }

    /// @<Declare action...
    // §1101
    pub fn insert_dollar_sign(&mut self) {
        self.back_input();
        self.cur_tok = 6291492i32;
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66445i32);
        }
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 66446i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 66447i32;
        }
        self.ins_error();
    }

    /// The ``you_cant`' procedure prints a line saying that the current command
    /// is illegal in the current mode; it identifies these things symbolically.
    /// @<Declare action...
    // §1103
    pub fn you_cant(&mut self) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66025i32);
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        self.print(66448i32);
        self.print_mode(self.cur_list.mode_field);
    }

    /// @<Declare act...
    // §1104
    pub fn report_illegal_case(&mut self) {
        self.you_cant();
        {
            self.help_ptr = 4i32;
            self.help_line[crate::ix::U((3i32) as usize)] = 66449i32;
            self.help_line[crate::ix::U((2i32) as usize)] = 66450i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 66451i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 66452i32;
        }
        self.error();
    }

    /// Some operations are allowed only in privileged modes, i.e., in cases
    /// that `mode>0`. The `privileged` function is used to detect violations
    /// of this rule; it issues an error message and returns `false` if the
    /// current `mode` is negative.
    /// @<Declare act...
    // §1105
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
    // §1108
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
                        { let __ix1355 = self.cur_list.tail_field; let __v1356 = self.new_null_box(); self.mem[crate::ix::U((__ix1355) as usize)].set_hh_rh(__v1356); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix1357 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1358 = self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1357) as usize)].set_int(__v1358); }
                    {
                        { let __ix1359 = self.cur_list.tail_field; let __v1360 = self.new_glue(fill_glue); self.mem[crate::ix::U((__ix1359) as usize)].set_hh_rh(__v1360); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    {
                        { let __ix1361 = self.cur_list.tail_field; let __v1362 = self.new_penalty((1073741824i32).wrapping_neg()); self.mem[crate::ix::U((__ix1361) as usize)].set_hh_rh(__v1362); }
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
    // §1114
    pub fn append_glue(&mut self) {
        let mut s: small_number = 0; // §1114
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
            { let __ix1363 = self.cur_list.tail_field; let __v1364 = self.new_glue(self.cur_val); self.mem[crate::ix::U((__ix1363) as usize)].set_hh_rh(__v1364); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (s >= skip_code) {
            {
                { let __ix1365 = self.cur_val; let __v1366 = (self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix1365) as usize)].set_hh_rh(__v1366); }
                if (s > skip_code) {
                    { let __ix1367 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1367) as usize)].set_hh_b1(mu_glue); }
                }
            }
        }
    }

    /// @<Declare act...
    // §1115
    pub fn append_kern(&mut self) {
        let mut s: quarterword = 0; // §1115
        s = self.cur_chr;
        self.scan_dimen((s == mu_glue), false, false);
        {
            { let __ix1368 = self.cur_list.tail_field; let __v1369 = self.new_kern(self.cur_val); self.mem[crate::ix::U((__ix1368) as usize)].set_hh_rh(__v1369); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix1370 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1370) as usize)].set_hh_b1(s); }
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
    // §1118
    pub fn off_save(&mut self) {
        let mut p: halfword = 0; // §1118
        if (self.cur_group == bottom_level) {
            // §1120
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66159i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66470i32;
                }
                self.error();
            }
        } else {
            // §1118
            {
                self.back_input();
                p = self.get_avail();
                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65949i32);
                }
                // §1119
                match self.cur_group {
                    semi_simple_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(34749083i32);
                            self.print_esc(65823i32);
                        }
                    }
                    math_shift_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(6291492i32);
                            self.print_char(36i32);
                        }
                    }
                    math_left_group => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(34749084i32);
                            { let __v1371 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v1371); }
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(25165870i32);
                            self.print_esc(66469i32);
                        }
                    }
                    _ => {
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(4194429i32);
                            self.print_char(125i32);
                        }
                    }
                }
                // §1118
                self.print(65950i32);
                self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                {
                    self.help_ptr = 5i32;
                    self.help_line[crate::ix::U((4i32) as usize)] = 66464i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 66465i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66466i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66467i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66468i32;
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1123
    pub fn extra_right_brace(&mut self) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66475i32);
        }
        match self.cur_group {
            semi_simple_group => {
                self.print_esc(65823i32);
            }
            math_shift_group => {
                self.print_char(36i32);
            }
            math_left_group => {
                self.print_esc(66280i32);
            }
            _ => {}
        }
        {
            self.help_ptr = 5i32;
            self.help_line[crate::ix::U((4i32) as usize)] = 66476i32;
            self.help_line[crate::ix::U((3i32) as usize)] = 66477i32;
            self.help_line[crate::ix::U((2i32) as usize)] = 66478i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 66479i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 66480i32;
        }
        self.error();
        self.align_state = (self.align_state).wrapping_add(1i32);
    }

    /// Here is where we clear the parameters that are supposed to revert to their
    /// default values after every paragraph and when internal vertical mode is entered.
    /// @<Declare act...
    // §1124
    pub fn normal_paragraph(&mut self) {
        if (self.eqtb[crate::ix::U(((7892283i32) - 1) as usize)].int() != 0i32) {
            self.eq_word_define(7892283i32, 0i32);
        }
        if (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() != 0i32) {
            self.eq_word_define(9006737i32, 0i32);
        }
        if (self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int() != 1i32) {
            self.eq_word_define(7892305i32, 1i32);
        }
        if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
            self.eq_define(par_shape_loc, shape_ref, (268435455i32).wrapping_neg());
        }
        if (self.eqtb[crate::ix::U(((inter_line_penalties_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
            self.eq_define(inter_line_penalties_loc, shape_ref, (268435455i32).wrapping_neg());
        }
    }

    /// The `box_end` procedure does the right thing with `cur_box`, if
    /// `box_context` represents the context as explained above.
    /// @<Declare act...
    // §1129
    pub fn box_end(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1129
        let mut a: small_number = 0; // §1129
        if (box_context < box_flag) {
            // §1130
            {
                if (self.cur_box != (268435455i32).wrapping_neg()) {
                    {
                        { let __ix1372 = (self.cur_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1372) as usize)].set_int(box_context); }
                        if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                            {
                                if (self.pre_adjust_tail != (268435455i32).wrapping_neg()) {
                                    {
                                        if (pre_adjust_head != self.pre_adjust_tail) {
                                            {
                                                { let __ix1373 = self.cur_list.tail_field; let __v1374 = self.mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1373) as usize)].set_hh_rh(__v1374); }
                                                self.cur_list.tail_field = self.pre_adjust_tail;
                                            }
                                        }
                                        self.pre_adjust_tail = (268435455i32).wrapping_neg();
                                    }
                                }
                                self.append_to_vlist(self.cur_box);
                                if (self.adjust_tail != (268435455i32).wrapping_neg()) {
                                    {
                                        if (adjust_head != self.adjust_tail) {
                                            {
                                                { let __ix1375 = self.cur_list.tail_field; let __v1376 = self.mem[crate::ix::U((adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1375) as usize)].set_hh_rh(__v1376); }
                                                self.cur_list.tail_field = self.adjust_tail;
                                            }
                                        }
                                        self.adjust_tail = (268435455i32).wrapping_neg();
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
                                        { let __v1377 = self.cur_box; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v1377); }
                                        self.cur_box = p;
                                    }
                                }
                                { let __ix1378 = self.cur_list.tail_field; let __v1379 = self.cur_box; self.mem[crate::ix::U((__ix1378) as usize)].set_hh_rh(__v1379); }
                                self.cur_list.tail_field = self.cur_box;
                            }
                        }
                    }
                }
            }
        } else {
            // §1129
            if (box_context < ship_out_flag) {
                // §1131
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
                // §1129
                if (self.cur_box != (268435455i32).wrapping_neg()) {
                    if (box_context > ship_out_flag) {
                        // §1132
                        {
                            // §438
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                            }
                            // §1132
                            if (((self.cur_cmd == hskip) && ((self.cur_list.mode_field).wrapping_abs() != vmode)) || ((self.cur_cmd == vskip) && ((self.cur_list.mode_field).wrapping_abs() == vmode))) {
                                {
                                    self.append_glue();
                                    { let __ix1380 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1380) as usize)].set_hh_b1((box_context).wrapping_sub(1073807261i32)); }
                                    { let __ix1381 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1382 = self.cur_box; self.mem[crate::ix::U((__ix1381) as usize)].set_hh_rh(__v1382); }
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
                                        self.print(66493i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 66494i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66495i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66496i32;
                                    }
                                    self.back_error();
                                    self.flush_node_list(self.cur_box);
                                }
                            }
                        }
                    } else {
                        // §1129
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
    // §1133
    pub fn begin_box(&mut self, mut box_context: i32) {
        let mut p: halfword = 0; // §1133
        let mut q: halfword = 0; // §1133
        let mut r: halfword = 0; // §1133
        let mut fm: bool = false; // §1133
        let mut tx: halfword = 0; // §1133
        let mut m: quarterword = 0; // §1133
        let mut k: halfword = 0; // §1133
        let mut n: halfword = 0; // §1133
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
                                if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                    self.cur_box = (268435455i32).wrapping_neg();
                                } else {
                                    self.cur_box = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                        if (self.cur_val < 256i32) {
                            { let __ix1383 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1383) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                    {
                                        { let __ix1384 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1384) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                                        { let __ix1385 = (self.cur_ptr).wrapping_add(1i32); let __v1386 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1385) as usize)].set_hh_lh(__v1386); }
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
                                if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                    q = (268435455i32).wrapping_neg();
                                } else {
                                    q = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                        self.cur_box = self.copy_node_list(q);
                    }
                }
                last_box_code => {
                    // §1134
                    {
                        self.cur_box = (268435455i32).wrapping_neg();
                        if ((self.cur_list.mode_field).wrapping_abs() == mmode) {
                            {
                                self.you_cant();
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66498i32;
                                }
                                self.error();
                            }
                        } else {
                            if ((self.cur_list.mode_field == vmode) && (self.cur_list.head_field == self.cur_list.tail_field)) {
                                {
                                    self.you_cant();
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66499i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66500i32;
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
                                                // §1135
                                                {
                                                    q = self.cur_list.head_field;
                                                    p = (268435455i32).wrapping_neg();
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
                                                    self.mem[crate::ix::U((tx) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                    if (q == (268435455i32).wrapping_neg()) {
                                                        if fm {
                                                            self.confusion(66497i32);
                                                        } else {
                                                            self.cur_list.tail_field = p;
                                                        }
                                                    } else {
                                                        if fm {
                                                            {
                                                                self.cur_list.tail_field = r;
                                                                self.mem[crate::ix::U((r) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                                self.flush_node_list(p);
                                                            }
                                                        }
                                                    }
                                                    self.cur_box = tx;
                                                    { let __ix1387 = (self.cur_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1387) as usize)].set_int(0i32); }
                                                }
                                            }
                                        }
                                    }
                                    // §1134
                                }
                            }
                        }
                    }
                }
                vsplit_code => {
                    // §1136
                    {
                        self.scan_register_num();
                        n = self.cur_val;
                        if (!self.scan_keyword(66244i32)) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66501i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66502i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66503i32;
                                }
                                self.error();
                            }
                        }
                        self.scan_dimen(false, false, false);
                        self.cur_box = self.vsplit(n, self.cur_val);
                    }
                }
                _ => {
                    // §1137
                    {
                        k = (self.cur_chr).wrapping_sub(4i32);
                        { let __ix1388 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix1388) as usize)].set_int(box_context); }
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
                                self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                                if (self.eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                    self.begin_token_list(self.eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh(), every_vbox_text);
                                }
                            }
                        } else {
                            {
                                self.cur_list.aux_field.set_hh_lh(1000i32);
                                if (self.eqtb[crate::ix::U(((every_hbox_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                    self.begin_token_list(self.eqtb[crate::ix::U(((every_hbox_loc) - 1) as usize)].hh().rh(), every_hbox_text);
                                }
                            }
                        }
                        break 'l_exit_f;
                    }
                }
            }
            // §1133
            self.box_end(box_context);
        }
    }

    /// @<Declare act...
    // §1138
    pub fn scan_box(&mut self, mut box_context: i32) {
        // §438
        loop {
            self.get_x_token();
            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
        }
        // §1138
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
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66504i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66505i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66506i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66507i32;
                    }
                    self.back_error();
                }
            }
        }
    }

    /// @<Declare action...
    // §1140
    pub fn package(&mut self, mut c: small_number) {
        let mut h: scaled = 0; // §1140
        let mut p: halfword = 0; // §1140
        let mut d: scaled = 0; // §1140
        let mut u: i32 = 0; // §1140
        let mut v: i32 = 0; // §1140
        d = self.eqtb[crate::ix::U(((9006727i32) - 1) as usize)].int();
        u = self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int();
        self.unsave();
        self.save_ptr = (self.save_ptr).wrapping_sub(3i32);
        v = self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int();
        self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].set_int(u);
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            self.cur_box = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(2i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int());
        } else {
            {
                self.cur_box = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(2i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), d);
                if (c == vtop_code) {
                    // §1141
                    {
                        h = 0i32;
                        p = self.mem[crate::ix::U(((self.cur_box).wrapping_add(5i32)) as usize)].hh().rh();
                        if (p != (268435455i32).wrapping_neg()) {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() <= rule_node) {
                                h = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                            }
                        }
                        { let __ix1389 = (self.cur_box).wrapping_add(2i32); let __v1390 = ((self.mem[crate::ix::U(((self.cur_box).wrapping_add(2i32)) as usize)].int()).wrapping_sub(h)).wrapping_add(self.mem[crate::ix::U(((self.cur_box).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((__ix1389) as usize)].set_int(__v1390); }
                        { let __ix1391 = (self.cur_box).wrapping_add(3i32); self.mem[crate::ix::U((__ix1391) as usize)].set_int(h); }
                    }
                }
            }
        }
        // §1140
        self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].set_int(v);
        self.pop_nest();
        self.box_end(self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int());
    }

    /// @<Declare act...
    // §1145
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
    // §1145
    pub fn new_graf(&mut self, mut indented: bool) {
        self.cur_list.pg_field = 0i32;
        if ((self.cur_list.mode_field == vmode) || (self.cur_list.head_field != self.cur_list.tail_field)) {
            {
                { let __ix1392 = self.cur_list.tail_field; let __v1393 = self.new_param_glue(par_skip_code); self.mem[crate::ix::U((__ix1392) as usize)].set_hh_rh(__v1393); }
                self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
            }
        }
        self.push_nest();
        self.cur_list.mode_field = hmode;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() > biggest_lang) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int();
            }
        }
        { let __v1394 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v1394); }
        self.cur_list.pg_field = ((((self.norm_min(self.eqtb[crate::ix::U(((7892315i32) - 1) as usize)].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(self.eqtb[crate::ix::U(((7892316i32) - 1) as usize)].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        if indented {
            {
                self.cur_list.tail_field = self.new_null_box();
                { let __ix1395 = self.cur_list.head_field; let __v1396 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1395) as usize)].set_hh_rh(__v1396); }
                { let __ix1397 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1398 = self.eqtb[crate::ix::U(((9006720i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix1397) as usize)].set_int(__v1398); }
            }
        }
        if (self.eqtb[crate::ix::U(((every_par_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_par_loc) - 1) as usize)].hh().rh(), every_par_text);
        }
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// @<Declare act...
    // §1147
    pub fn indent_in_hmode(&mut self) {
        let mut p: halfword = 0; // §1147
        let mut q: halfword = 0; // §1147
        if (self.cur_chr > 0i32) {
            {
                p = self.new_null_box();
                { let __v1399 = self.eqtb[crate::ix::U(((9006720i32) - 1) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v1399); }
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
                    { let __ix1400 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1400) as usize)].set_hh_rh(p); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            }
        }
    }

    /// @<Declare act...
    // §1149
    pub fn head_for_vmode(&mut self) {
        if (self.cur_list.mode_field < 0i32) {
            if (self.cur_cmd != hrule) {
                self.off_save();
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
                        self.print(66025i32);
                    }
                    self.print_esc(65828i32);
                    self.print(66510i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66511i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66512i32;
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
    // §1150
    pub fn end_graf(&mut self) {
        if (self.cur_list.mode_field == hmode) {
            {
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    self.pop_nest();
                } else {
                    self.line_break(false);
                }
                if (self.cur_list.eTeX_aux_field != (268435455i32).wrapping_neg()) {
                    {
                        self.flush_list(self.cur_list.eTeX_aux_field);
                        self.cur_list.eTeX_aux_field = (268435455i32).wrapping_neg();
                    }
                }
                self.normal_paragraph();
                self.error_count = 0i32;
            }
        }
    }

    /// @<Declare act...
    // §1153
    pub fn begin_insert_or_adjust(&mut self) {
        if (self.cur_cmd == vadjust) {
            self.cur_val = 255i32;
        } else {
            {
                self.scan_eight_bit_int();
                if (self.cur_val == 255i32) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66513i32);
                        }
                        self.print_esc(65618i32);
                        self.print_int(255i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66514i32;
                        }
                        self.error();
                        self.cur_val = 0i32;
                    }
                }
            }
        }
        { let __ix1401 = (self.save_ptr).wrapping_add(0i32); let __v1402 = self.cur_val; self.save_stack[crate::ix::U((__ix1401) as usize)].set_int(__v1402); }
        if ((self.cur_cmd == vadjust) && self.scan_keyword(66515i32)) {
            { let __ix1403 = (self.save_ptr).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1403) as usize)].set_int(1i32); }
        } else {
            { let __ix1404 = (self.save_ptr).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1404) as usize)].set_int(0i32); }
        }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(insert_group);
        self.scan_left_brace();
        self.normal_paragraph();
        self.push_nest();
        self.cur_list.mode_field = (1i32).wrapping_neg();
        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
    }

    /// @<Declare act...
    // §1155
    pub fn make_mark(&mut self) {
        let mut p: halfword = 0; // §1155
        let mut c: halfword = 0; // §1155
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
        { let __v1405 = self.def_ref; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v1405); }
        { let __ix1406 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1406) as usize)].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// @<Declare action...
    // §1157
    pub fn append_penalty(&mut self) {
        self.scan_int();
        {
            { let __ix1407 = self.cur_list.tail_field; let __v1408 = self.new_penalty(self.cur_val); self.mem[crate::ix::U((__ix1407) as usize)].set_hh_rh(__v1408); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (self.cur_list.mode_field == vmode) {
            self.build_page();
        }
    }

    /// When `delete_last` is called, `cur_chr` is the `type` of node that
    /// will be deleted, if present.
    /// @<Declare action...
    // §1159
    pub fn delete_last(&mut self) {
        let mut p: halfword = 0; // §1159
        let mut q: halfword = 0; // §1159
        let mut r: halfword = 0; // §1159
        let mut fm: bool = false; // §1159
        let mut tx: halfword = 0; // §1159
        let mut m: quarterword = 0; // §1159
        'l_exit_f: {
            if ((self.cur_list.mode_field == vmode) && (self.cur_list.tail_field == self.cur_list.head_field)) {
                // §1160
                {
                    if ((self.cur_chr != glue_node) || (self.last_glue != max_halfword)) {
                        {
                            self.you_cant();
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66499i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66516i32;
                            }
                            if (self.cur_chr == kern_node) {
                                self.help_line[crate::ix::U((0i32) as usize)] = 66517i32;
                            } else {
                                if (self.cur_chr != glue_node) {
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66518i32;
                                }
                            }
                            self.error();
                        }
                    }
                }
            } else {
                // §1159
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
                                p = (268435455i32).wrapping_neg();
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
                                self.mem[crate::ix::U((tx) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                if (q == (268435455i32).wrapping_neg()) {
                                    if fm {
                                        self.confusion(66497i32);
                                    } else {
                                        self.cur_list.tail_field = p;
                                    }
                                } else {
                                    if fm {
                                        {
                                            self.cur_list.tail_field = r;
                                            self.mem[crate::ix::U((r) as usize)].set_hh_rh((268435455i32).wrapping_neg());
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
    // §1164
    pub fn unpackage(&mut self) {
        let mut p: halfword = 0; // §1164
        let mut r: halfword = 0; // §1164
        let mut c: i32 = 0; // §1164
        'l_exit_f: {
            'l_done_f: {
                if (self.cur_chr > copy_code) {
                    // §1674
                    {
                        { let __ix1409 = self.cur_list.tail_field; let __v1410 = self.disc_ptr[crate::ix::U(((self.cur_chr) - 1) as usize)]; self.mem[crate::ix::U((__ix1409) as usize)].set_hh_rh(__v1410); }
                        self.disc_ptr[crate::ix::U(((self.cur_chr) - 1) as usize)] = (268435455i32).wrapping_neg();
                        break 'l_done_f;
                    }
                }
                // §1164
                c = self.cur_chr;
                self.scan_register_num();
                if (self.cur_val < 256i32) {
                    p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                } else {
                    {
                        self.find_sa_element(box_val, self.cur_val, false);
                        if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                            p = (268435455i32).wrapping_neg();
                        } else {
                            p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                        }
                    }
                }
                if (p == (268435455i32).wrapping_neg()) {
                    break 'l_exit_f;
                }
                if ((((self.cur_list.mode_field).wrapping_abs() == mmode) || (((self.cur_list.mode_field).wrapping_abs() == vmode) && (self.mem[crate::ix::U((p) as usize)].hh().b0() != vlist_node))) || (((self.cur_list.mode_field).wrapping_abs() == hmode) && (self.mem[crate::ix::U((p) as usize)].hh().b0() != hlist_node))) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66526i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66527i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66528i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66529i32;
                        }
                        self.error();
                        break 'l_exit_f;
                    }
                }
                if (c == copy_code) {
                    { let __ix1411 = self.cur_list.tail_field; let __v1412 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()); self.mem[crate::ix::U((__ix1411) as usize)].set_hh_rh(__v1412); }
                } else {
                    {
                        { let __ix1413 = self.cur_list.tail_field; let __v1414 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1413) as usize)].set_hh_rh(__v1414); }
                        if (self.cur_val < 256i32) {
                            { let __ix1415 = (box_base).wrapping_add(self.cur_val); self.eqtb[crate::ix::U(((__ix1415) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                    {
                                        { let __ix1416 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix1416) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                                        { let __ix1417 = (self.cur_ptr).wrapping_add(1i32); let __v1418 = (self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1417) as usize)].set_hh_lh(__v1418); }
                                        self.delete_sa_ref(self.cur_ptr);
                                    }
                                }
                            }
                        }
                        self.free_node(p, box_node_size);
                    }
                }
            }
            while (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                {
                    r = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    if ((!(r >= self.hi_mem_min)) && (self.mem[crate::ix::U((r) as usize)].hh().b0() == margin_kern_node)) {
                        {
                            { let __ix1419 = self.cur_list.tail_field; let __v1420 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1419) as usize)].set_hh_rh(__v1420); }
                            self.free_node(r, margin_kern_node_size);
                        }
                    }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
            }
        }
    }

    /// @<Declare act...
    // §1167
    pub fn append_italic_correction(&mut self) {
        let mut p: halfword = 0; // §1167
        let mut f: internal_font_number = 0; // §1167
        'l_exit_f: {
            if (self.cur_list.tail_field != self.cur_list.head_field) {
                {
                    if (self.cur_list.tail_field >= self.hi_mem_min) {
                        p = self.cur_list.tail_field;
                    } else {
                        if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == ligature_node) {
                            p = (self.cur_list.tail_field).wrapping_add(1i32);
                        } else {
                            if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == whatsit_node) {
                                {
                                    if ((self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() <= native_word_node_AT)) {
                                        {
                                            {
                                                { let __ix1421 = self.cur_list.tail_field; let __v1422 = { let __a1423_0 = self.get_native_italic_correction(self.cur_list.tail_field); self.new_kern(__a1423_0) }; self.mem[crate::ix::U((__ix1421) as usize)].set_hh_rh(__v1422); }
                                                self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                            }
                                            { let __ix1424 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1424) as usize)].set_hh_b1(explicit); }
                                        }
                                    } else {
                                        if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == glyph_node) {
                                            {
                                                {
                                                    { let __ix1425 = self.cur_list.tail_field; let __v1426 = { let __a1427_0 = self.get_native_glyph_italic_correction(self.cur_list.tail_field); self.new_kern(__a1427_0) }; self.mem[crate::ix::U((__ix1425) as usize)].set_hh_rh(__v1426); }
                                                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                                }
                                                { let __ix1428 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1428) as usize)].set_hh_b1(explicit); }
                                            }
                                        }
                                    }
                                    break 'l_exit_f;
                                }
                            } else {
                                break 'l_exit_f;
                            }
                        }
                    }
                    f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                    {
                        { let __ix1429 = self.cur_list.tail_field; let __v1430 = { let __a1431_0 = { let __s1433 = ((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(({ let __s1432 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1432)] }.qqqq().b2() / 4i32))) as usize; self.font_info[crate::ix::U(__s1433)] }.int(); self.new_kern(__a1431_0) }; self.mem[crate::ix::U((__ix1429) as usize)].set_hh_rh(__v1430); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix1434 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1434) as usize)].set_hh_b1(explicit); }
                }
            }
        }
    }

    /// The space factor does not change when we append a discretionary node,
    /// but it starts out as 1000 in the subsidiary lists.
    /// @<Declare act...
    // §1171
    pub fn append_discretionary(&mut self) {
        let mut c: i32 = 0; // §1171
        {
            { let __ix1435 = self.cur_list.tail_field; let __v1436 = self.new_disc(); self.mem[crate::ix::U((__ix1435) as usize)].set_hh_rh(__v1436); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        if (self.cur_chr == 1i32) {
            {
                c = self.hyphen_char[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)];
                if (c >= 0i32) {
                    if (c <= biggest_char) {
                        { let __ix1437 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1438 = self.new_character(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), c); self.mem[crate::ix::U((__ix1437) as usize)].set_hh_lh(__v1438); }
                    }
                }
            }
        } else {
            {
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix1439 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[crate::ix::U((__ix1439) as usize)].set_int(0i32); }
                self.new_save_level(disc_group);
                self.scan_left_brace();
                self.push_nest();
                self.cur_list.mode_field = (105i32).wrapping_neg();
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1173
    pub fn build_discretionary(&mut self) {
        let mut p: halfword = 0; // §1173
        let mut q: halfword = 0; // §1173
        let mut n: i32 = 0; // §1173
        'l_exit_f: {
            'l_done_f: {
                self.unsave();
                // §1175
                q = self.cur_list.head_field;
                p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                n = 0i32;
                while (p != (268435455i32).wrapping_neg()) {
                    {
                        if (!(p >= self.hi_mem_min)) {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() > rule_node) {
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() != kern_node) {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() != ligature_node) {
                                        if ((self.mem[crate::ix::U((p) as usize)].hh().b0() != whatsit_node) || ((!((self.mem[crate::ix::U((p) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U((p) as usize)].hh().b1() != glyph_node))) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66536i32);
                                                }
                                                {
                                                    self.help_ptr = 1i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66537i32;
                                                }
                                                self.error();
                                                self.begin_diagnostic();
                                                self.print_nl(66538i32);
                                                self.show_box(p);
                                                self.end_diagnostic(true);
                                                self.flush_node_list(p);
                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                break 'l_done_f;
                                            }
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
            // §1173
            p = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
            self.pop_nest();
            match self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int() {
                0 => {
                    { let __ix1440 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1440) as usize)].set_hh_lh(p); }
                }
                1 => {
                    { let __ix1441 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[crate::ix::U((__ix1441) as usize)].set_hh_rh(p); }
                }
                2 => {
                    // §1174
                    {
                        if ((n > 0i32) && ((self.cur_list.mode_field).wrapping_abs() == mmode)) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66530i32);
                                }
                                self.print_esc(65639i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66531i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66532i32;
                                }
                                self.flush_node_list(p);
                                n = 0i32;
                                self.error();
                            }
                        } else {
                            { let __ix1442 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1442) as usize)].set_hh_rh(p); }
                        }
                        if (n <= max_quarterword) {
                            { let __ix1443 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1443) as usize)].set_hh_b1(n); }
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
                                    self.print(66533i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66534i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66535i32;
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
            // §1173
            { let __ix1444 = (self.save_ptr).wrapping_sub(1i32); let __v1445 = (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1444) as usize)].set_int(__v1445); }
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
    // §1177
    pub fn make_accent(&mut self) {
        let mut s: f64 = 0.0; // §1177
        let mut t: f64 = 0.0; // §1177
        let mut p: halfword = 0; // §1177
        let mut q: halfword = 0; // §1177
        let mut r: halfword = 0; // §1177
        let mut f: internal_font_number = 0; // §1177
        let mut a: scaled = 0; // §1177
        let mut h: scaled = 0; // §1177
        let mut x: scaled = 0; // §1177
        let mut w: scaled = 0; // §1177
        let mut delta: scaled = 0; // §1177
        let mut lsb: scaled = 0; // §1177
        let mut rsb: scaled = 0; // §1177
        let mut i: four_quarters = four_quarters::default(); // §1177
        self.scan_char_num();
        f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
        p = self.new_character(f, self.cur_val);
        if (p != (268435455i32).wrapping_neg()) {
            {
                x = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
                s = (((self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) as f64) / 65536.0f64);
                if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
                    {
                        a = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                        if (a == 0i32) {
                            { let mut __f2 = ::core::mem::take(&mut lsb); let mut __f3 = ::core::mem::take(&mut rsb); let __r = self.get_native_char_sidebearings(f, self.cur_val, &mut __f2, &mut __f3); lsb = __f2; rsb = __f3; __r };
                        }
                    }
                } else {
                    a = { let __s1447 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1446 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1446)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1447)] }.int();
                }
                self.do_assignments();
                // §1178
                q = (268435455i32).wrapping_neg();
                f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
                if (((self.cur_cmd == letter) || (self.cur_cmd == other_char)) || (self.cur_cmd == char_given)) {
                    {
                        q = self.new_character(f, self.cur_chr);
                        self.cur_val = self.cur_chr;
                    }
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
                // §1177
                if (q != (268435455i32).wrapping_neg()) {
                    // §1179
                    {
                        t = (((self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) as f64) / 65536.0f64);
                        if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
                            {
                                w = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                                { let mut __f2 = ::core::mem::take(&mut h); let mut __f3 = ::core::mem::take(&mut delta); let __r = self.get_native_char_height_depth(f, self.cur_val, &mut __f2, &mut __f3); h = __f2; delta = __f3; __r };
                            }
                        } else {
                            {
                                i = { let __s1448 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((q) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1448)] }.qqqq();
                                w = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int();
                                h = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((i.b1() / 16i32))) as usize)].int();
                            }
                        }
                        if (h != x) {
                            {
                                p = self.hpack(p, 0i32, additional);
                                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int((x).wrapping_sub(h));
                            }
                        }
                        if (((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) && (a == 0i32)) {
                            delta = crate::system::pas_round((((((((w).wrapping_sub(lsb)).wrapping_add(rsb)) as f64) / 2.0f64) + (((h) as f64) * t)) - (((x) as f64) * s)));
                        } else {
                            delta = crate::system::pas_round(((((((w).wrapping_sub(a)) as f64) / 2.0f64) + (((h) as f64) * t)) - (((x) as f64) * s)));
                        }
                        r = self.new_kern(delta);
                        self.mem[crate::ix::U((r) as usize)].set_hh_b1(acc_kern);
                        { let __ix1449 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1449) as usize)].set_hh_rh(r); }
                        self.mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                        self.cur_list.tail_field = self.new_kern(((a).wrapping_neg()).wrapping_sub(delta));
                        { let __ix1450 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1450) as usize)].set_hh_b1(acc_kern); }
                        { let __v1451 = self.cur_list.tail_field; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v1451); }
                        p = q;
                    }
                }
                // §1177
                { let __ix1452 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1452) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
    }

    /// @<Declare act...
    // §1181
    pub fn align_error(&mut self) {
        if ((self.align_state).wrapping_abs() > 2i32) {
            // §1182
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66543i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                if (self.cur_tok == 8388646i32) {
                    {
                        {
                            self.help_ptr = 6i32;
                            self.help_line[crate::ix::U((5i32) as usize)] = 66544i32;
                            self.help_line[crate::ix::U((4i32) as usize)] = 66545i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 66546i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66547i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66548i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66549i32;
                        }
                    }
                } else {
                    {
                        {
                            self.help_ptr = 5i32;
                            self.help_line[crate::ix::U((4i32) as usize)] = 66544i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 66550i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66547i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66548i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66549i32;
                        }
                    }
                }
                self.error();
            }
        } else {
            // §1181
            {
                self.back_input();
                if (self.align_state < 0i32) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(65981i32);
                        }
                        self.align_state = (self.align_state).wrapping_add(1i32);
                        self.cur_tok = 2097275i32;
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
                            self.print(66539i32);
                        }
                        self.align_state = (self.align_state).wrapping_sub(1i32);
                        self.cur_tok = 4194429i32;
                    }
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66540i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66541i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66542i32;
                }
                self.ins_error();
            }
        }
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1183
    pub fn no_align_error(&mut self) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66543i32);
        }
        self.print_esc(65840i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 66551i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 66552i32;
        }
        self.error();
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1183
    pub fn omit_error(&mut self) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66543i32);
        }
        self.print_esc(65843i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 66553i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 66552i32;
        }
        self.error();
    }

    /// An `align_group` code is supposed to remain on the `save_stack`
    /// during an entire alignment, until `fin_align` removes it.
    /// A devious user might force an `endv` command to occur just about anywhere;
    /// we must defeat such hacks.
    /// @<Declare act...
    // §1185
    pub fn do_endv(&mut self) {
        self.base_ptr = self.input_ptr;
        { let __ix1453 = self.base_ptr; let __v1454 = self.cur_input; self.input_stack[crate::ix::U((__ix1453) as usize)] = __v1454; }
        while (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field == (268435455i32).wrapping_neg())) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == token_list)) {
            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
        }
        if (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field != (268435455i32).wrapping_neg())) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field != token_list)) {
            self.fatal_error(65917i32);
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
