// GENERATED FILE -- DO NOT EDIT.
// Translated WEB procedures and functions.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments, unused_imports)]
#![allow(dead_code, unreachable_code, unused_labels, while_true, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// The `make_left_right` function constructs a left or right delimiter of
    /// the required size and returns the value `open_noad` or `close_noad`. The
    /// `right_noad` and `left_noad` will both be based on the original `style`,
    /// so they will have consistent sizes.
    /// We use the fact that `right_noad-left_noad=close_noad-open_noad`.
    /// @<Declare math...
    // §762
    pub fn make_left_right(&mut self, mut q: halfword, mut style: small_number, mut max_d: scaled, mut max_h: scaled) -> small_number {
        let mut make_left_right: small_number = 0;
        let mut delta: scaled = 0; // §762
        let mut delta1: scaled = 0; // §762
        let mut delta2: scaled = 0; // §762
        if (style < 4i32) {
            self.cur_size = 0i32;
        } else {
            self.cur_size = (16i32).wrapping_mul(((style).wrapping_sub(2i32) / 2i32));
        }
        delta2 = (max_d).wrapping_add(self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
        delta1 = ((max_h).wrapping_add(max_d)).wrapping_sub(delta2);
        if (delta2 > delta1) {
            delta1 = delta2;
        }
        delta = ((delta1 / 500i32)).wrapping_mul(self.eqtb[((618181i32) - 1) as usize].int());
        delta2 = ((delta1).wrapping_add(delta1)).wrapping_sub(self.eqtb[((618740i32) - 1) as usize].int());
        if (delta < delta2) {
            delta = delta2;
        }
        { let __v381 = self.var_delimiter((q).wrapping_add(1i32), self.cur_size, delta); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v381); }
        make_left_right = (self.mem[(q) as usize].hh().b0()).wrapping_sub(10i32);
        make_left_right
    }

    /// Here is the overall plan of `mlist_to_hlist`, and the list of its
    /// local variables.
    // §726
    pub fn mlist_to_hlist(&mut self) {
        let mut mlist: halfword = 0; // §726
        let mut penalties: bool = false; // §726
        let mut style: small_number = 0; // §726
        let mut save_style: small_number = 0; // §726
        let mut q: halfword = 0; // §726
        let mut r: halfword = 0; // §726
        let mut r_type: small_number = 0; // §726
        let mut t: small_number = 0; // §726
        let mut p: halfword = 0; // §726
        let mut x: halfword = 0; // §726
        let mut y: halfword = 0; // §726
        let mut z: halfword = 0; // §726
        let mut pen: i32 = 0; // §726
        let mut s: small_number = 0; // §726
        let mut max_h: scaled = 0; // §726
        let mut max_d: scaled = 0; // §726
        let mut delta: scaled = 0; // §726
        mlist = self.cur_mlist;
        penalties = self.mlist_penalties;
        style = self.cur_style;
        q = mlist;
        r = 0i32;
        r_type = 17i32;
        max_h = 0i32;
        max_d = 0i32;
        // §703
        {
            if (self.cur_style < 4i32) {
                self.cur_size = 0i32;
            } else {
                self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
            }
            self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
        }
        // §726
        while (q != 0i32) {
            // §727
            {
                // goto labels: reswitch, L82, L80, L81
                let mut __goto_1: i32 = 0;
                'l_dispatch_1: loop {
                    if __goto_1 <= 0 {
                        // §728
                        delta = 0i32;
                        match self.mem[(q) as usize].hh().b0() {
                            18 => {
                                match r_type {
                                    18 | 17 | 19 | 20 | 22 | 30 => {
                                        {
                                            self.mem[(q) as usize].set_hh_b0(16i32);
                                            { __goto_1 = 0; continue 'l_dispatch_1; }
                                        }
                                    }
                                    _ => {
                                    }
                                }
                            }
                            19 | 21 | 22 | 31 => {
                                {
                                    // §729
                                    if (r_type == 18i32) {
                                        self.mem[(r) as usize].set_hh_b0(16i32);
                                    }
                                    // §728
                                    if (self.mem[(q) as usize].hh().b0() == 31i32) {
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            30 => {
                                // §733
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            }
                            25 => {
                                {
                                    self.make_fraction(q);
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                            17 => {
                                {
                                    delta = self.make_op(q);
                                    if (self.mem[(q) as usize].hh().b1() == 1i32) {
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            16 => {
                                self.make_ord(q);
                            }
                            20 | 23 => {
                            }
                            24 => {
                                self.make_radical(q);
                            }
                            27 => {
                                self.make_over(q);
                            }
                            26 => {
                                self.make_under(q);
                            }
                            28 => {
                                self.make_math_accent(q);
                            }
                            29 => {
                                self.make_vcenter(q);
                            }
                            14 => {
                                // §730
                                {
                                    self.cur_style = self.mem[(q) as usize].hh().b1();
                                    // §703
                                    {
                                        if (self.cur_style < 4i32) {
                                            self.cur_size = 0i32;
                                        } else {
                                            self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
                                    }
                                    // §730
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            15 => {
                                // §731
                                {
                                    match (self.cur_style / 2i32) {
                                        0 => {
                                            {
                                                p = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
                                                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
                                            }
                                        }
                                        1 => {
                                            {
                                                p = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                                                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
                                            }
                                        }
                                        2 => {
                                            {
                                                p = self.mem[((q).wrapping_add(2i32)) as usize].hh().lh();
                                                self.mem[((q).wrapping_add(2i32)) as usize].set_hh_lh(0i32);
                                            }
                                        }
                                        3 => {
                                            {
                                                p = self.mem[((q).wrapping_add(2i32)) as usize].hh().rh();
                                                self.mem[((q).wrapping_add(2i32)) as usize].set_hh_rh(0i32);
                                            }
                                        }
                                        _ => {}
                                    }
                                    self.flush_node_list(self.mem[((q).wrapping_add(1i32)) as usize].hh().lh());
                                    self.flush_node_list(self.mem[((q).wrapping_add(1i32)) as usize].hh().rh());
                                    self.flush_node_list(self.mem[((q).wrapping_add(2i32)) as usize].hh().lh());
                                    self.flush_node_list(self.mem[((q).wrapping_add(2i32)) as usize].hh().rh());
                                    self.mem[(q) as usize].set_hh_b0(14i32);
                                    { let __v382 = self.cur_style; self.mem[(q) as usize].set_hh_b1(__v382); }
                                    self.mem[((q).wrapping_add(1i32)) as usize].set_int(0i32);
                                    self.mem[((q).wrapping_add(2i32)) as usize].set_int(0i32);
                                    if (p != 0i32) {
                                        {
                                            z = self.mem[(q) as usize].hh().rh();
                                            self.mem[(q) as usize].set_hh_rh(p);
                                            while (self.mem[(p) as usize].hh().rh() != 0i32) {
                                                p = self.mem[(p) as usize].hh().rh();
                                            }
                                            self.mem[(p) as usize].set_hh_rh(z);
                                        }
                                    }
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            3 | 4 | 5 | 8 | 12 | 7 => {
                                // §730
                                { __goto_1 = 3; continue 'l_dispatch_1; }
                            }
                            2 => {
                                {
                                    if (self.mem[((q).wrapping_add(3i32)) as usize].int() > max_h) {
                                        max_h = self.mem[((q).wrapping_add(3i32)) as usize].int();
                                    }
                                    if (self.mem[((q).wrapping_add(2i32)) as usize].int() > max_d) {
                                        max_d = self.mem[((q).wrapping_add(2i32)) as usize].int();
                                    }
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            10 => {
                                {
                                    // §732
                                    if (self.mem[(q) as usize].hh().b1() == 99i32) {
                                        {
                                            x = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
                                            y = self.math_glue(x, self.cur_mu);
                                            self.delete_glue_ref(x);
                                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(y);
                                            self.mem[(q) as usize].set_hh_b1(0i32);
                                        }
                                    } else {
                                        if ((self.cur_size != 0i32) && (self.mem[(q) as usize].hh().b1() == 98i32)) {
                                            {
                                                p = self.mem[(q) as usize].hh().rh();
                                                if (p != 0i32) {
                                                    if ((self.mem[(p) as usize].hh().b0() == 10i32) || (self.mem[(p) as usize].hh().b0() == 11i32)) {
                                                        {
                                                            { let __v383 = self.mem[(p) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v383); }
                                                            self.mem[(p) as usize].set_hh_rh(0i32);
                                                            self.flush_node_list(p);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // §730
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            11 => {
                                {
                                    self.math_kern(q, self.cur_mu);
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            _ => {
                                // §728
                                self.confusion(889i32);
                            }
                        }
                        // §754
                        match self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() {
                            1 | 4 => {
                                // §755
                                {
                                    self.fetch((q).wrapping_add(1i32));
                                    if (self.cur_i.b0() > 0i32) {
                                        {
                                            delta = self.font_info[((self.italic_base[(self.cur_f) as usize]).wrapping_add(((self.cur_i.b2()).wrapping_sub(0i32) / 4i32))) as usize].int();
                                            p = self.new_character(self.cur_f, (self.cur_c).wrapping_sub(0i32));
                                            if ((self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() == 4i32) && (self.font_info[((2i32).wrapping_add(self.param_base[(self.cur_f) as usize])) as usize].int() != 0i32)) {
                                                delta = 0i32;
                                            }
                                            if ((self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() == 0i32) && (delta != 0i32)) {
                                                {
                                                    { let __v384 = self.new_kern(delta); self.mem[(p) as usize].set_hh_rh(__v384); }
                                                    delta = 0i32;
                                                }
                                            }
                                        }
                                    } else {
                                        p = 0i32;
                                    }
                                }
                            }
                            0 => {
                                // §754
                                p = 0i32;
                            }
                            2 => {
                                p = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
                            }
                            3 => {
                                {
                                    self.cur_mlist = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
                                    save_style = self.cur_style;
                                    self.mlist_penalties = false;
                                    self.mlist_to_hlist();
                                    self.cur_style = save_style;
                                    // §703
                                    {
                                        if (self.cur_style < 4i32) {
                                            self.cur_size = 0i32;
                                        } else {
                                            self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
                                    }
                                    // §754
                                    p = self.hpack(self.mem[(4999996i32) as usize].hh().rh(), 0i32, 1i32);
                                }
                            }
                            _ => {
                                self.confusion(890i32);
                            }
                        }
                        self.mem[((q).wrapping_add(1i32)) as usize].set_int(p);
                        if ((self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() == 0i32) && (self.mem[((q).wrapping_add(2i32)) as usize].hh().rh() == 0i32)) {
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                        self.make_scripts(q, delta);
                    }
                    if __goto_1 <= 1 { // L82
                        // §727
                        z = self.hpack(self.mem[((q).wrapping_add(1i32)) as usize].int(), 0i32, 1i32);
                        if (self.mem[((z).wrapping_add(3i32)) as usize].int() > max_h) {
                            max_h = self.mem[((z).wrapping_add(3i32)) as usize].int();
                        }
                        if (self.mem[((z).wrapping_add(2i32)) as usize].int() > max_d) {
                            max_d = self.mem[((z).wrapping_add(2i32)) as usize].int();
                        }
                        self.free_node(z, 7i32);
                    }
                    if __goto_1 <= 2 { // L80
                        r = q;
                        r_type = self.mem[(r) as usize].hh().b0();
                    }
                    if __goto_1 <= 3 { // L81
                        q = self.mem[(q) as usize].hh().rh();
                    }
                    break 'l_dispatch_1;
                }
            }
        }
        // §729
        if (r_type == 18i32) {
            self.mem[(r) as usize].set_hh_b0(16i32);
        }
        // §760
        p = 4999996i32;
        self.mem[(p) as usize].set_hh_rh(0i32);
        q = mlist;
        r_type = 0i32;
        self.cur_style = style;
        // §703
        {
            if (self.cur_style < 4i32) {
                self.cur_size = 0i32;
            } else {
                self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
            }
            self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
        }
        // §760
        while (q != 0i32) {
            {
                'l_done_f: {
                    'l_L83_f: {
                        // §761
                        t = 16i32;
                        s = 4i32;
                        pen = 10000i32;
                        match self.mem[(q) as usize].hh().b0() {
                            17 | 20 | 21 | 22 | 23 => {
                                t = self.mem[(q) as usize].hh().b0();
                            }
                            18 => {
                                {
                                    t = 18i32;
                                    pen = self.eqtb[((618172i32) - 1) as usize].int();
                                }
                            }
                            19 => {
                                {
                                    t = 19i32;
                                    pen = self.eqtb[((618173i32) - 1) as usize].int();
                                }
                            }
                            16 | 29 | 27 | 26 => {
                            }
                            24 => {
                                s = 5i32;
                            }
                            28 => {
                                s = 5i32;
                            }
                            25 => {
                                s = 6i32;
                            }
                            30 | 31 => {
                                t = self.make_left_right(q, style, max_d, max_h);
                            }
                            14 => {
                                // §763
                                {
                                    self.cur_style = self.mem[(q) as usize].hh().b1();
                                    s = 3i32;
                                    // §703
                                    {
                                        if (self.cur_style < 4i32) {
                                            self.cur_size = 0i32;
                                        } else {
                                            self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((616837i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
                                    }
                                    // §763
                                    break 'l_L83_f;
                                }
                            }
                            8 | 12 | 2 | 7 | 5 | 3 | 4 | 10 | 11 => {
                                // §761
                                {
                                    self.mem[(p) as usize].set_hh_rh(q);
                                    p = q;
                                    q = self.mem[(q) as usize].hh().rh();
                                    self.mem[(p) as usize].set_hh_rh(0i32);
                                    break 'l_done_f;
                                }
                            }
                            _ => {
                                self.confusion(891i32);
                            }
                        }
                        // §766
                        if (r_type > 0i32) {
                            {
                                match self.str_pool[((((r_type).wrapping_mul(8i32)).wrapping_add(t)).wrapping_add(self.magic_offset)) as usize] {
                                    48 => {
                                        x = 0i32;
                                    }
                                    49 => {
                                        if (self.cur_style < 4i32) {
                                            x = 15i32;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    50 => {
                                        x = 15i32;
                                    }
                                    51 => {
                                        if (self.cur_style < 4i32) {
                                            x = 16i32;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    52 => {
                                        if (self.cur_style < 4i32) {
                                            x = 17i32;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    _ => {
                                        self.confusion(893i32);
                                    }
                                }
                                if (x != 0i32) {
                                    {
                                        y = self.math_glue(self.eqtb[(((615782i32).wrapping_add(x)) - 1) as usize].hh().rh(), self.cur_mu);
                                        z = self.new_glue(y);
                                        self.mem[(y) as usize].set_hh_rh(0i32);
                                        self.mem[(p) as usize].set_hh_rh(z);
                                        p = z;
                                        self.mem[(z) as usize].set_hh_b1((x).wrapping_add(1i32));
                                    }
                                }
                            }
                        }
                        // §767
                        if (self.mem[((q).wrapping_add(1i32)) as usize].int() != 0i32) {
                            {
                                { let __v385 = self.mem[((q).wrapping_add(1i32)) as usize].int(); self.mem[(p) as usize].set_hh_rh(__v385); }
                                loop {
                                    p = self.mem[(p) as usize].hh().rh();
                                    if (self.mem[(p) as usize].hh().rh() == 0i32) { break; }
                                }
                            }
                        }
                        if penalties {
                            if (self.mem[(q) as usize].hh().rh() != 0i32) {
                                if (pen < 10000i32) {
                                    {
                                        r_type = self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().b0();
                                        if (r_type != 12i32) {
                                            if (r_type != 19i32) {
                                                {
                                                    z = self.new_penalty(pen);
                                                    self.mem[(p) as usize].set_hh_rh(z);
                                                    p = z;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §760
                        r_type = t;
                    }
                    r = q;
                    q = self.mem[(q) as usize].hh().rh();
                    self.free_node(r, s);
                }
            }
        }
    }

    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §772
    pub fn push_alignment(&mut self) {
        let mut p: halfword = 0; // §772
        p = self.get_node(5i32);
        { let __v386 = self.align_ptr; self.mem[(p) as usize].set_hh_rh(__v386); }
        { let __v387 = self.cur_align; self.mem[(p) as usize].set_hh_lh(__v387); }
        { let __v388 = self.mem[(4999991i32) as usize].hh().rh(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(__v388); }
        { let __v389 = self.cur_span; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v389); }
        { let __v390 = self.cur_loop; self.mem[((p).wrapping_add(2i32)) as usize].set_int(__v390); }
        { let __v391 = self.align_state; self.mem[((p).wrapping_add(3i32)) as usize].set_int(__v391); }
        { let __v392 = self.cur_head; self.mem[((p).wrapping_add(4i32)) as usize].set_hh_lh(__v392); }
        { let __v393 = self.cur_tail; self.mem[((p).wrapping_add(4i32)) as usize].set_hh_rh(__v393); }
        self.align_ptr = p;
        self.cur_head = self.get_avail();
    }

    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §772
    pub fn pop_alignment(&mut self) {
        let mut p: halfword = 0; // §772
        {
            { let __ix394 = self.cur_head; let __v395 = self.avail; self.mem[(__ix394) as usize].set_hh_rh(__v395); }
            self.avail = self.cur_head;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        p = self.align_ptr;
        self.cur_tail = self.mem[((p).wrapping_add(4i32)) as usize].hh().rh();
        self.cur_head = self.mem[((p).wrapping_add(4i32)) as usize].hh().lh();
        self.align_state = self.mem[((p).wrapping_add(3i32)) as usize].int();
        self.cur_loop = self.mem[((p).wrapping_add(2i32)) as usize].int();
        self.cur_span = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
        { let __v396 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); self.mem[(4999991i32) as usize].set_hh_rh(__v396); }
        self.cur_align = self.mem[(p) as usize].hh().lh();
        self.align_ptr = self.mem[(p) as usize].hh().rh();
        self.free_node(p, 5i32);
    }

    /// The preamble is copied directly, except that \.{\\tabskip} causes a change
    /// to the tabskip glue, thereby possibly expanding macros that immediately
    /// follow it. An appearance of \.{\\span} also causes such an expansion.
    /// Note that if the preamble contains `\.{\\global\\tabskip}', the `\.{\\global}'
    /// token survives in the preamble and the `\.{\\tabskip}' defines new
    /// tabskip glue (locally).
    /// @<Declare the procedure called `get_preamble_token`
    // §782
    pub fn get_preamble_token(&mut self) {
        'l_restart_b: loop {
            self.get_token();
            while ((self.cur_chr == 256i32) && (self.cur_cmd == 4i32)) {
                {
                    self.get_token();
                    if (self.cur_cmd > 100i32) {
                        {
                            self.expand();
                            self.get_token();
                        }
                    }
                }
            }
            if (self.cur_cmd == 9i32) {
                self.fatal_error(595i32);
            }
            if ((self.cur_cmd == 75i32) && (self.cur_chr == 615793i32)) {
                {
                    self.scan_optional_equals();
                    self.scan_glue(2i32);
                    if (self.eqtb[((618206i32) - 1) as usize].int() > 0i32) {
                        self.geq_define(615793i32, 117i32, self.cur_val);
                    } else {
                        self.eq_define(615793i32, 117i32, self.cur_val);
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
    // §774
    pub fn init_align(&mut self) {
        let mut save_cs_ptr: halfword = 0; // §774
        let mut p: halfword = 0; // §774
        'l_done_f: {
            save_cs_ptr = self.cur_cs;
            self.push_alignment();
            self.align_state = (1000000i32).wrapping_neg();
            // §776
            if ((self.cur_list.mode_field == 203i32) && ((self.cur_list.tail_field != self.cur_list.head_field) || (self.cur_list.aux_field.int() != 0i32))) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(680i32);
                    }
                    self.print_esc(520i32);
                    self.print(894i32);
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 895i32;
                        self.help_line[(1i32) as usize] = 896i32;
                        self.help_line[(0i32) as usize] = 897i32;
                    }
                    self.error();
                    self.flush_math();
                }
            }
            // §774
            self.push_nest();
            // §775
            if (self.cur_list.mode_field == 203i32) {
                {
                    self.cur_list.mode_field = (1i32).wrapping_neg();
                    { let __v397 = self.nest[((self.nest_ptr).wrapping_sub(2i32)) as usize].aux_field.int(); self.cur_list.aux_field.set_int(__v397); }
                }
            } else {
                if (self.cur_list.mode_field > 0i32) {
                    self.cur_list.mode_field = (self.cur_list.mode_field).wrapping_neg();
                }
            }
            // §774
            self.scan_spec(6i32, false);
            // §777
            self.mem[(4999991i32) as usize].set_hh_rh(0i32);
            self.cur_align = 4999991i32;
            self.cur_loop = 0i32;
            self.scanner_status = 4i32;
            self.warning_index = save_cs_ptr;
            self.align_state = (1000000i32).wrapping_neg();
            while true {
                {
                    'l_done2_f: {
                        'l_done1_f: {
                            // §778
                            { let __ix398 = self.cur_align; let __v399 = self.new_param_glue(11i32); self.mem[(__ix398) as usize].set_hh_rh(__v399); }
                            self.cur_align = self.mem[(self.cur_align) as usize].hh().rh();
                            // §777
                            if (self.cur_cmd == 5i32) {
                                break 'l_done_f;
                            }
                            // §783
                            p = 4999995i32;
                            self.mem[(p) as usize].set_hh_rh(0i32);
                            while true {
                                {
                                    self.get_preamble_token();
                                    if (self.cur_cmd == 6i32) {
                                        break 'l_done1_f;
                                    }
                                    if (((self.cur_cmd <= 5i32) && (self.cur_cmd >= 4i32)) && (self.align_state == (1000000i32).wrapping_neg())) {
                                        if (((p == 4999995i32) && (self.cur_loop == 0i32)) && (self.cur_cmd == 4i32)) {
                                            self.cur_loop = self.cur_align;
                                        } else {
                                            {
                                                {
                                                    if (self.interaction == 3i32) {
                                                    }
                                                    self.print_nl(262i32);
                                                    self.print(903i32);
                                                }
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[(2i32) as usize] = 904i32;
                                                    self.help_line[(1i32) as usize] = 905i32;
                                                    self.help_line[(0i32) as usize] = 906i32;
                                                }
                                                self.back_error();
                                                break 'l_done1_f;
                                            }
                                        }
                                    } else {
                                        if ((self.cur_cmd != 10i32) || (p != 4999995i32)) {
                                            {
                                                { let __v400 = self.get_avail(); self.mem[(p) as usize].set_hh_rh(__v400); }
                                                p = self.mem[(p) as usize].hh().rh();
                                                { let __v401 = self.cur_tok; self.mem[(p) as usize].set_hh_lh(__v401); }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §779
                        { let __ix402 = self.cur_align; let __v403 = self.new_null_box(); self.mem[(__ix402) as usize].set_hh_rh(__v403); }
                        self.cur_align = self.mem[(self.cur_align) as usize].hh().rh();
                        { let __ix404 = self.cur_align; self.mem[(__ix404) as usize].set_hh_lh(4999990i32); }
                        { let __ix405 = (self.cur_align).wrapping_add(1i32); self.mem[(__ix405) as usize].set_int((1073741824i32).wrapping_neg()); }
                        { let __ix406 = (self.cur_align).wrapping_add(3i32); let __v407 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(__ix406) as usize].set_int(__v407); }
                        // §784
                        p = 4999995i32;
                        self.mem[(p) as usize].set_hh_rh(0i32);
                        while true {
                            {
                                'l_continue_b: loop {
                                    self.get_preamble_token();
                                    if (((self.cur_cmd <= 5i32) && (self.cur_cmd >= 4i32)) && (self.align_state == (1000000i32).wrapping_neg())) {
                                        break 'l_done2_f;
                                    }
                                    if (self.cur_cmd == 6i32) {
                                        {
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                self.print_nl(262i32);
                                                self.print(907i32);
                                            }
                                            {
                                                self.help_ptr = 3i32;
                                                self.help_line[(2i32) as usize] = 904i32;
                                                self.help_line[(1i32) as usize] = 905i32;
                                                self.help_line[(0i32) as usize] = 908i32;
                                            }
                                            self.error();
                                            continue 'l_continue_b;
                                        }
                                    }
                                    { let __v408 = self.get_avail(); self.mem[(p) as usize].set_hh_rh(__v408); }
                                    p = self.mem[(p) as usize].hh().rh();
                                    { let __v409 = self.cur_tok; self.mem[(p) as usize].set_hh_lh(__v409); }
                                    break 'l_continue_b;
                                }
                            }
                        }
                    }
                    { let __v410 = self.get_avail(); self.mem[(p) as usize].set_hh_rh(__v410); }
                    p = self.mem[(p) as usize].hh().rh();
                    self.mem[(p) as usize].set_hh_lh(619614i32);
                    // §779
                    { let __ix411 = (self.cur_align).wrapping_add(2i32); let __v412 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(__ix411) as usize].set_int(__v412); }
                }
            }
        }
        // §777
        self.scanner_status = 0i32;
        // §774
        self.new_save_level(6i32);
        if (self.eqtb[((616320i32) - 1) as usize].hh().rh() != 0i32) {
            self.begin_token_list(self.eqtb[((616320i32) - 1) as usize].hh().rh(), 13i32);
        }
        self.align_peek();
    }

    /// The parameter to `init_span` is a pointer to the alignrecord where the
    /// next column or group of columns will begin. A new semantic level is
    /// entered, so that the columns will generate a list for subsequent packaging.
    /// @<Declare the procedure called `init_span`
    // §787
    pub fn init_span(&mut self, mut p: halfword) {
        self.push_nest();
        if (self.cur_list.mode_field == (102i32).wrapping_neg()) {
            self.cur_list.aux_field.set_hh_lh(1000i32);
        } else {
            {
                self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
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
    // §786
    pub fn init_row(&mut self) {
        self.push_nest();
        self.cur_list.mode_field = ((103i32).wrapping_neg()).wrapping_sub(self.cur_list.mode_field);
        if (self.cur_list.mode_field == (102i32).wrapping_neg()) {
            self.cur_list.aux_field.set_hh_lh(0i32);
        } else {
            self.cur_list.aux_field.set_int(0i32);
        }
        {
            { let __ix413 = self.cur_list.tail_field; let __v414 = self.new_glue(self.mem[((self.mem[(4999991i32) as usize].hh().rh()).wrapping_add(1i32)) as usize].hh().lh()); self.mem[(__ix413) as usize].set_hh_rh(__v414); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix415 = self.cur_list.tail_field; self.mem[(__ix415) as usize].set_hh_b1(12i32); }
        self.cur_align = self.mem[(self.mem[(4999991i32) as usize].hh().rh()) as usize].hh().rh();
        self.cur_tail = self.cur_head;
        self.init_span(self.cur_align);
    }

    /// When a column begins, we assume that `cur_cmd` is either `omit` or else
    /// the current token should be put back into the input until the \<u_j>
    /// template has been scanned.  (Note that `cur_cmd` might be `tab_mark` or
    /// `car_ret`.)  We also assume that `align_state` is approximately 1000000 at
    /// this time.  We remain in the same mode, and start the template if it is
    /// called for.
    // §788
    pub fn init_col(&mut self) {
        { let __ix416 = (self.cur_align).wrapping_add(5i32); let __v417 = self.cur_cmd; self.mem[(__ix416) as usize].set_hh_lh(__v417); }
        if (self.cur_cmd == 63i32) {
            self.align_state = 0i32;
        } else {
            {
                self.back_input();
                self.begin_token_list(self.mem[((self.cur_align).wrapping_add(3i32)) as usize].int(), 1i32);
            }
        }
    }

    /// When the `endv` command at the end of a \<v_j> template comes through the
    /// scanner, things really start to happen; and it is the `fin_col` routine
    /// that makes them happen. This routine returns `true` if a row as well as a
    /// column has been finished.
    // §791
    pub fn fin_col(&mut self) -> bool {
        let mut fin_col: bool = false;
        let mut p: halfword = 0; // §791
        let mut q: halfword = 0; // §791
        let mut r: halfword = 0; // §791
        let mut s: halfword = 0; // §791
        let mut u: halfword = 0; // §791
        let mut w: scaled = 0; // §791
        let mut o: glue_ord = 0; // §791
        let mut n: halfword = 0; // §791
        'l_exit_f: {
            if (self.cur_align == 0i32) {
                self.confusion(909i32);
            }
            q = self.mem[(self.cur_align) as usize].hh().rh();
            if (q == 0i32) {
                self.confusion(909i32);
            }
            if (self.align_state < 500000i32) {
                self.fatal_error(595i32);
            }
            p = self.mem[(q) as usize].hh().rh();
            // §792
            if ((p == 0i32) && (self.mem[((self.cur_align).wrapping_add(5i32)) as usize].hh().lh() < 257i32)) {
                if (self.cur_loop != 0i32) {
                    // §793
                    {
                        { let __v418 = self.new_null_box(); self.mem[(q) as usize].set_hh_rh(__v418); }
                        p = self.mem[(q) as usize].hh().rh();
                        self.mem[(p) as usize].set_hh_lh(4999990i32);
                        self.mem[((p).wrapping_add(1i32)) as usize].set_int((1073741824i32).wrapping_neg());
                        self.cur_loop = self.mem[(self.cur_loop) as usize].hh().rh();
                        // §794
                        q = 4999995i32;
                        r = self.mem[((self.cur_loop).wrapping_add(3i32)) as usize].int();
                        while (r != 0i32) {
                            {
                                { let __v419 = self.get_avail(); self.mem[(q) as usize].set_hh_rh(__v419); }
                                q = self.mem[(q) as usize].hh().rh();
                                { let __v420 = self.mem[(r) as usize].hh().lh(); self.mem[(q) as usize].set_hh_lh(__v420); }
                                r = self.mem[(r) as usize].hh().rh();
                            }
                        }
                        self.mem[(q) as usize].set_hh_rh(0i32);
                        { let __v421 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[((p).wrapping_add(3i32)) as usize].set_int(__v421); }
                        q = 4999995i32;
                        r = self.mem[((self.cur_loop).wrapping_add(2i32)) as usize].int();
                        while (r != 0i32) {
                            {
                                { let __v422 = self.get_avail(); self.mem[(q) as usize].set_hh_rh(__v422); }
                                q = self.mem[(q) as usize].hh().rh();
                                { let __v423 = self.mem[(r) as usize].hh().lh(); self.mem[(q) as usize].set_hh_lh(__v423); }
                                r = self.mem[(r) as usize].hh().rh();
                            }
                        }
                        self.mem[(q) as usize].set_hh_rh(0i32);
                        { let __v424 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[((p).wrapping_add(2i32)) as usize].set_int(__v424); }
                        // §793
                        self.cur_loop = self.mem[(self.cur_loop) as usize].hh().rh();
                        { let __v425 = self.new_glue(self.mem[((self.cur_loop).wrapping_add(1i32)) as usize].hh().lh()); self.mem[(p) as usize].set_hh_rh(__v425); }
                        { let __ix426 = self.mem[(p) as usize].hh().rh(); self.mem[(__ix426) as usize].set_hh_b1(12i32); }
                    }
                } else {
                    // §792
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(910i32);
                        }
                        self.print_esc(899i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 911i32;
                            self.help_line[(1i32) as usize] = 912i32;
                            self.help_line[(0i32) as usize] = 913i32;
                        }
                        { let __ix427 = (self.cur_align).wrapping_add(5i32); self.mem[(__ix427) as usize].set_hh_lh(257i32); }
                        self.error();
                    }
                }
            }
            // §791
            if (self.mem[((self.cur_align).wrapping_add(5i32)) as usize].hh().lh() != 256i32) {
                {
                    self.unsave();
                    self.new_save_level(6i32);
                    // §796
                    {
                        if (self.cur_list.mode_field == (102i32).wrapping_neg()) {
                            {
                                self.adjust_tail = self.cur_tail;
                                u = self.hpack(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32);
                                w = self.mem[((u).wrapping_add(1i32)) as usize].int();
                                self.cur_tail = self.adjust_tail;
                                self.adjust_tail = 0i32;
                            }
                        } else {
                            {
                                u = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32, 0i32);
                                w = self.mem[((u).wrapping_add(3i32)) as usize].int();
                            }
                        }
                        n = 0i32;
                        if (self.cur_span != self.cur_align) {
                            // §798
                            {
                                q = self.cur_span;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    q = self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().rh();
                                    if (q == self.cur_align) { break; }
                                }
                                if (n > 255i32) {
                                    self.confusion(914i32);
                                }
                                q = self.cur_span;
                                while (self.mem[(self.mem[(q) as usize].hh().lh()) as usize].hh().rh() < n) {
                                    q = self.mem[(q) as usize].hh().lh();
                                }
                                if (self.mem[(self.mem[(q) as usize].hh().lh()) as usize].hh().rh() > n) {
                                    {
                                        s = self.get_node(2i32);
                                        { let __v428 = self.mem[(q) as usize].hh().lh(); self.mem[(s) as usize].set_hh_lh(__v428); }
                                        self.mem[(s) as usize].set_hh_rh(n);
                                        self.mem[(q) as usize].set_hh_lh(s);
                                        self.mem[((s).wrapping_add(1i32)) as usize].set_int(w);
                                    }
                                } else {
                                    if (self.mem[((self.mem[(q) as usize].hh().lh()).wrapping_add(1i32)) as usize].int() < w) {
                                        { let __ix429 = (self.mem[(q) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix429) as usize].set_int(w); }
                                    }
                                }
                            }
                        } else {
                            // §796
                            if (w > self.mem[((self.cur_align).wrapping_add(1i32)) as usize].int()) {
                                { let __ix430 = (self.cur_align).wrapping_add(1i32); self.mem[(__ix430) as usize].set_int(w); }
                            }
                        }
                        self.mem[(u) as usize].set_hh_b0(13i32);
                        self.mem[(u) as usize].set_hh_b1(n);
                        // §659
                        if (self.total_stretch[(3i32) as usize] != 0i32) {
                            o = 3i32;
                        } else {
                            if (self.total_stretch[(2i32) as usize] != 0i32) {
                                o = 2i32;
                            } else {
                                if (self.total_stretch[(1i32) as usize] != 0i32) {
                                    o = 1i32;
                                } else {
                                    o = 0i32;
                                }
                            }
                        }
                        // §796
                        self.mem[((u).wrapping_add(5i32)) as usize].set_hh_b1(o);
                        { let __v431 = self.total_stretch[(o) as usize]; self.mem[((u).wrapping_add(6i32)) as usize].set_int(__v431); }
                        // §665
                        if (self.total_shrink[(3i32) as usize] != 0i32) {
                            o = 3i32;
                        } else {
                            if (self.total_shrink[(2i32) as usize] != 0i32) {
                                o = 2i32;
                            } else {
                                if (self.total_shrink[(1i32) as usize] != 0i32) {
                                    o = 1i32;
                                } else {
                                    o = 0i32;
                                }
                            }
                        }
                        // §796
                        self.mem[((u).wrapping_add(5i32)) as usize].set_hh_b0(o);
                        { let __v432 = self.total_shrink[(o) as usize]; self.mem[((u).wrapping_add(4i32)) as usize].set_int(__v432); }
                        self.pop_nest();
                        { let __ix433 = self.cur_list.tail_field; self.mem[(__ix433) as usize].set_hh_rh(u); }
                        self.cur_list.tail_field = u;
                    }
                    // §795
                    {
                        { let __ix434 = self.cur_list.tail_field; let __v435 = self.new_glue(self.mem[((self.mem[(self.cur_align) as usize].hh().rh()).wrapping_add(1i32)) as usize].hh().lh()); self.mem[(__ix434) as usize].set_hh_rh(__v435); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                    { let __ix436 = self.cur_list.tail_field; self.mem[(__ix436) as usize].set_hh_b1(12i32); }
                    // §791
                    if (self.mem[((self.cur_align).wrapping_add(5i32)) as usize].hh().lh() >= 257i32) {
                        {
                            fin_col = true;
                            break 'l_exit_f;
                        }
                    }
                    self.init_span(p);
                }
            }
            self.align_state = 1000000i32;
            // §406
            loop {
                self.get_x_token();
                if (self.cur_cmd != 10i32) { break; }
            }
            // §791
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
    // §799
    pub fn fin_row(&mut self) {
        let mut p: halfword = 0; // §799
        if (self.cur_list.mode_field == (102i32).wrapping_neg()) {
            {
                p = self.hpack(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32);
                self.pop_nest();
                self.append_to_vlist(p);
                if (self.cur_head != self.cur_tail) {
                    {
                        { let __ix437 = self.cur_list.tail_field; let __v438 = self.mem[(self.cur_head) as usize].hh().rh(); self.mem[(__ix437) as usize].set_hh_rh(__v438); }
                        self.cur_list.tail_field = self.cur_tail;
                    }
                }
            }
        } else {
            {
                p = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32, 1073741823i32);
                self.pop_nest();
                { let __ix439 = self.cur_list.tail_field; self.mem[(__ix439) as usize].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
        self.mem[(p) as usize].set_hh_b0(13i32);
        self.mem[((p).wrapping_add(6i32)) as usize].set_int(0i32);
        if (self.eqtb[((616320i32) - 1) as usize].hh().rh() != 0i32) {
            self.begin_token_list(self.eqtb[((616320i32) - 1) as usize].hh().rh(), 13i32);
        }
        self.align_peek();
    }

    /// Finally, we will reach the end of the alignment, and we can breathe a
    /// sigh of relief that memory hasn't overflowed. All the unset boxes will now be
    /// set so that the columns line up, taking due account of spanned columns.
    // §800
    pub fn fin_align(&mut self) {
        let mut p: halfword = 0; // §800
        let mut q: halfword = 0; // §800
        let mut r: halfword = 0; // §800
        let mut s: halfword = 0; // §800
        let mut u: halfword = 0; // §800
        let mut v: halfword = 0; // §800
        let mut t: scaled = 0; // §800
        let mut w: scaled = 0; // §800
        let mut o: scaled = 0; // §800
        let mut n: halfword = 0; // §800
        let mut rule_save: scaled = 0; // §800
        let mut aux_save: memory_word = memory_word::default(); // §800
        if (self.cur_group != 6i32) {
            self.confusion(915i32);
        }
        self.unsave();
        if (self.cur_group != 6i32) {
            self.confusion(916i32);
        }
        self.unsave();
        if (self.nest[((self.nest_ptr).wrapping_sub(1i32)) as usize].mode_field == 203i32) {
            o = self.eqtb[((618745i32) - 1) as usize].int();
        } else {
            o = 0i32;
        }
        // §801
        q = self.mem[(self.mem[(4999991i32) as usize].hh().rh()) as usize].hh().rh();
        loop {
            self.flush_list(self.mem[((q).wrapping_add(3i32)) as usize].int());
            self.flush_list(self.mem[((q).wrapping_add(2i32)) as usize].int());
            p = self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().rh();
            if (self.mem[((q).wrapping_add(1i32)) as usize].int() == (1073741824i32).wrapping_neg()) {
                // §802
                {
                    self.mem[((q).wrapping_add(1i32)) as usize].set_int(0i32);
                    r = self.mem[(q) as usize].hh().rh();
                    s = self.mem[((r).wrapping_add(1i32)) as usize].hh().lh();
                    if (s != 0i32) {
                        {
                            { let __v440 = (self.mem[(0i32) as usize].hh().rh()).wrapping_add(1i32); self.mem[(0i32) as usize].set_hh_rh(__v440); }
                            self.delete_glue_ref(s);
                            self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
                        }
                    }
                }
            }
            // §801
            if (self.mem[(q) as usize].hh().lh() != 4999990i32) {
                // §803
                {
                    t = (self.mem[((q).wrapping_add(1i32)) as usize].int()).wrapping_add(self.mem[((self.mem[((self.mem[(q) as usize].hh().rh()).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32)) as usize].int());
                    r = self.mem[(q) as usize].hh().lh();
                    s = 4999990i32;
                    self.mem[(s) as usize].set_hh_lh(p);
                    n = 1i32;
                    loop {
                        { let __v441 = (self.mem[((r).wrapping_add(1i32)) as usize].int()).wrapping_sub(t); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v441); }
                        u = self.mem[(r) as usize].hh().lh();
                        while (self.mem[(r) as usize].hh().rh() > n) {
                            {
                                s = self.mem[(s) as usize].hh().lh();
                                n = (self.mem[(self.mem[(s) as usize].hh().lh()) as usize].hh().rh()).wrapping_add(1i32);
                            }
                        }
                        if (self.mem[(r) as usize].hh().rh() < n) {
                            {
                                { let __v442 = self.mem[(s) as usize].hh().lh(); self.mem[(r) as usize].set_hh_lh(__v442); }
                                self.mem[(s) as usize].set_hh_lh(r);
                                { let __v443 = (self.mem[(r) as usize].hh().rh()).wrapping_sub(1i32); self.mem[(r) as usize].set_hh_rh(__v443); }
                                s = r;
                            }
                        } else {
                            {
                                if (self.mem[((r).wrapping_add(1i32)) as usize].int() > self.mem[((self.mem[(s) as usize].hh().lh()).wrapping_add(1i32)) as usize].int()) {
                                    { let __ix444 = (self.mem[(s) as usize].hh().lh()).wrapping_add(1i32); let __v445 = self.mem[((r).wrapping_add(1i32)) as usize].int(); self.mem[(__ix444) as usize].set_int(__v445); }
                                }
                                self.free_node(r, 2i32);
                            }
                        }
                        r = u;
                        if (r == 4999990i32) { break; }
                    }
                }
            }
            // §801
            self.mem[(q) as usize].set_hh_b0(13i32);
            self.mem[(q) as usize].set_hh_b1(0i32);
            self.mem[((q).wrapping_add(3i32)) as usize].set_int(0i32);
            self.mem[((q).wrapping_add(2i32)) as usize].set_int(0i32);
            self.mem[((q).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
            self.mem[((q).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
            self.mem[((q).wrapping_add(6i32)) as usize].set_int(0i32);
            self.mem[((q).wrapping_add(4i32)) as usize].set_int(0i32);
            q = p;
            if (q == 0i32) { break; }
        }
        // §804
        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
        self.pack_begin_line = (self.cur_list.ml_field).wrapping_neg();
        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
            {
                rule_save = self.eqtb[((618746i32) - 1) as usize].int();
                self.eqtb[((618746i32) - 1) as usize].set_int(0i32);
                p = self.hpack(self.mem[(4999991i32) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int());
                self.eqtb[((618746i32) - 1) as usize].set_int(rule_save);
            }
        } else {
            {
                q = self.mem[(self.mem[(4999991i32) as usize].hh().rh()) as usize].hh().rh();
                loop {
                    { let __v446 = self.mem[((q).wrapping_add(1i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v446); }
                    self.mem[((q).wrapping_add(1i32)) as usize].set_int(0i32);
                    q = self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().rh();
                    if (q == 0i32) { break; }
                }
                p = self.vpackage(self.mem[(4999991i32) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(), 1073741823i32);
                q = self.mem[(self.mem[(4999991i32) as usize].hh().rh()) as usize].hh().rh();
                loop {
                    { let __v447 = self.mem[((q).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v447); }
                    self.mem[((q).wrapping_add(3i32)) as usize].set_int(0i32);
                    q = self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().rh();
                    if (q == 0i32) { break; }
                }
            }
        }
        self.pack_begin_line = 0i32;
        // §805
        q = self.mem[(self.cur_list.head_field) as usize].hh().rh();
        s = self.cur_list.head_field;
        while (q != 0i32) {
            {
                if (!(q >= self.hi_mem_min)) {
                    if (self.mem[(q) as usize].hh().b0() == 13i32) {
                        // §807
                        {
                            if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                {
                                    self.mem[(q) as usize].set_hh_b0(0i32);
                                    { let __v448 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v448); }
                                }
                            } else {
                                {
                                    self.mem[(q) as usize].set_hh_b0(1i32);
                                    { let __v449 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v449); }
                                }
                            }
                            { let __v450 = self.mem[((p).wrapping_add(5i32)) as usize].hh().b1(); self.mem[((q).wrapping_add(5i32)) as usize].set_hh_b1(__v450); }
                            { let __v451 = self.mem[((p).wrapping_add(5i32)) as usize].hh().b0(); self.mem[((q).wrapping_add(5i32)) as usize].set_hh_b0(__v451); }
                            { let __v452 = self.mem[((p).wrapping_add(6i32)) as usize].gr(); self.mem[((q).wrapping_add(6i32)) as usize].set_gr(__v452); }
                            self.mem[((q).wrapping_add(4i32)) as usize].set_int(o);
                            r = self.mem[(self.mem[((q).wrapping_add(5i32)) as usize].hh().rh()) as usize].hh().rh();
                            s = self.mem[(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh()) as usize].hh().rh();
                            loop {
                                // §808
                                n = self.mem[(r) as usize].hh().b1();
                                t = self.mem[((s).wrapping_add(1i32)) as usize].int();
                                w = t;
                                u = 4999995i32;
                                while (n > 0i32) {
                                    {
                                        n = (n).wrapping_sub(1i32);
                                        // §809
                                        s = self.mem[(s) as usize].hh().rh();
                                        v = self.mem[((s).wrapping_add(1i32)) as usize].hh().lh();
                                        { let __v453 = self.new_glue(v); self.mem[(u) as usize].set_hh_rh(__v453); }
                                        u = self.mem[(u) as usize].hh().rh();
                                        self.mem[(u) as usize].set_hh_b1(12i32);
                                        t = (t).wrapping_add(self.mem[((v).wrapping_add(1i32)) as usize].int());
                                        if (self.mem[((p).wrapping_add(5i32)) as usize].hh().b0() == 1i32) {
                                            {
                                                if (self.mem[(v) as usize].hh().b0() == self.mem[((p).wrapping_add(5i32)) as usize].hh().b1()) {
                                                    t = (t).wrapping_add(crate::system::pas_round((((self.mem[((p).wrapping_add(6i32)) as usize].gr()) as f64) * ((self.mem[((v).wrapping_add(2i32)) as usize].int()) as f64))));
                                                }
                                            }
                                        } else {
                                            if (self.mem[((p).wrapping_add(5i32)) as usize].hh().b0() == 2i32) {
                                                {
                                                    if (self.mem[(v) as usize].hh().b1() == self.mem[((p).wrapping_add(5i32)) as usize].hh().b1()) {
                                                        t = (t).wrapping_sub(crate::system::pas_round((((self.mem[((p).wrapping_add(6i32)) as usize].gr()) as f64) * ((self.mem[((v).wrapping_add(3i32)) as usize].int()) as f64))));
                                                    }
                                                }
                                            }
                                        }
                                        s = self.mem[(s) as usize].hh().rh();
                                        { let __v454 = self.new_null_box(); self.mem[(u) as usize].set_hh_rh(__v454); }
                                        u = self.mem[(u) as usize].hh().rh();
                                        t = (t).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].int());
                                        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                            { let __v455 = self.mem[((s).wrapping_add(1i32)) as usize].int(); self.mem[((u).wrapping_add(1i32)) as usize].set_int(__v455); }
                                        } else {
                                            {
                                                self.mem[(u) as usize].set_hh_b0(1i32);
                                                { let __v456 = self.mem[((s).wrapping_add(1i32)) as usize].int(); self.mem[((u).wrapping_add(3i32)) as usize].set_int(__v456); }
                                            }
                                        }
                                    }
                                }
                                // §808
                                if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                    // §810
                                    {
                                        { let __v457 = self.mem[((q).wrapping_add(3i32)) as usize].int(); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v457); }
                                        { let __v458 = self.mem[((q).wrapping_add(2i32)) as usize].int(); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v458); }
                                        if (t == self.mem[((r).wrapping_add(1i32)) as usize].int()) {
                                            {
                                                self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                                self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
                                                self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                            }
                                        } else {
                                            if (t > self.mem[((r).wrapping_add(1i32)) as usize].int()) {
                                                {
                                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(1i32);
                                                    if (self.mem[((r).wrapping_add(6i32)) as usize].int() == 0i32) {
                                                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                                    } else {
                                                        { let __v459 = ((((((t).wrapping_sub(self.mem[((r).wrapping_add(1i32)) as usize].int())) as f64) / ((self.mem[((r).wrapping_add(6i32)) as usize].int()) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v459); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v460 = self.mem[((r).wrapping_add(5i32)) as usize].hh().b0(); self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(__v460); }
                                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(2i32);
                                                    if (self.mem[((r).wrapping_add(4i32)) as usize].int() == 0i32) {
                                                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                                    } else {
                                                        if ((self.mem[((r).wrapping_add(5i32)) as usize].hh().b1() == 0i32) && ((self.mem[((r).wrapping_add(1i32)) as usize].int()).wrapping_sub(t) > self.mem[((r).wrapping_add(4i32)) as usize].int())) {
                                                            self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((1.0f64) as f32));
                                                        } else {
                                                            { let __v461 = ((((((self.mem[((r).wrapping_add(1i32)) as usize].int()).wrapping_sub(t)) as f64) / ((self.mem[((r).wrapping_add(4i32)) as usize].int()) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v461); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[((r).wrapping_add(1i32)) as usize].set_int(w);
                                        self.mem[(r) as usize].set_hh_b0(0i32);
                                    }
                                } else {
                                    // §811
                                    {
                                        { let __v462 = self.mem[((q).wrapping_add(1i32)) as usize].int(); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v462); }
                                        if (t == self.mem[((r).wrapping_add(3i32)) as usize].int()) {
                                            {
                                                self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                                self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
                                                self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                            }
                                        } else {
                                            if (t > self.mem[((r).wrapping_add(3i32)) as usize].int()) {
                                                {
                                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(1i32);
                                                    if (self.mem[((r).wrapping_add(6i32)) as usize].int() == 0i32) {
                                                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                                    } else {
                                                        { let __v463 = ((((((t).wrapping_sub(self.mem[((r).wrapping_add(3i32)) as usize].int())) as f64) / ((self.mem[((r).wrapping_add(6i32)) as usize].int()) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v463); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v464 = self.mem[((r).wrapping_add(5i32)) as usize].hh().b0(); self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(__v464); }
                                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(2i32);
                                                    if (self.mem[((r).wrapping_add(4i32)) as usize].int() == 0i32) {
                                                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                                    } else {
                                                        if ((self.mem[((r).wrapping_add(5i32)) as usize].hh().b1() == 0i32) && ((self.mem[((r).wrapping_add(3i32)) as usize].int()).wrapping_sub(t) > self.mem[((r).wrapping_add(4i32)) as usize].int())) {
                                                            self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((1.0f64) as f32));
                                                        } else {
                                                            { let __v465 = ((((((self.mem[((r).wrapping_add(3i32)) as usize].int()).wrapping_sub(t)) as f64) / ((self.mem[((r).wrapping_add(4i32)) as usize].int()) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v465); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[((r).wrapping_add(3i32)) as usize].set_int(w);
                                        self.mem[(r) as usize].set_hh_b0(1i32);
                                    }
                                }
                                // §808
                                self.mem[((r).wrapping_add(4i32)) as usize].set_int(0i32);
                                if (u != 4999995i32) {
                                    {
                                        { let __v466 = self.mem[(r) as usize].hh().rh(); self.mem[(u) as usize].set_hh_rh(__v466); }
                                        { let __v467 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(r) as usize].set_hh_rh(__v467); }
                                        r = u;
                                    }
                                }
                                // §807
                                r = self.mem[(self.mem[(r) as usize].hh().rh()) as usize].hh().rh();
                                s = self.mem[(self.mem[(s) as usize].hh().rh()) as usize].hh().rh();
                                if (r == 0i32) { break; }
                            }
                        }
                    } else {
                        // §805
                        if (self.mem[(q) as usize].hh().b0() == 2i32) {
                            // §806
                            {
                                if (self.mem[((q).wrapping_add(1i32)) as usize].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v468 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v468); }
                                }
                                if (self.mem[((q).wrapping_add(3i32)) as usize].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v469 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v469); }
                                }
                                if (self.mem[((q).wrapping_add(2i32)) as usize].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v470 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v470); }
                                }
                                if (o != 0i32) {
                                    {
                                        r = self.mem[(q) as usize].hh().rh();
                                        self.mem[(q) as usize].set_hh_rh(0i32);
                                        q = self.hpack(q, 0i32, 1i32);
                                        self.mem[((q).wrapping_add(4i32)) as usize].set_int(o);
                                        self.mem[(q) as usize].set_hh_rh(r);
                                        self.mem[(s) as usize].set_hh_rh(q);
                                    }
                                }
                            }
                        }
                    }
                }
                // §805
                s = q;
                q = self.mem[(q) as usize].hh().rh();
            }
        }
        // §800
        self.flush_node_list(p);
        self.pop_alignment();
        // §812
        aux_save = self.cur_list.aux_field;
        p = self.mem[(self.cur_list.head_field) as usize].hh().rh();
        q = self.cur_list.tail_field;
        self.pop_nest();
        if (self.cur_list.mode_field == 203i32) {
            // §1206
            {
                self.do_assignments();
                if (self.cur_cmd != 3i32) {
                    // §1207
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(1170i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 895i32;
                            self.help_line[(0i32) as usize] = 896i32;
                        }
                        self.back_error();
                    }
                } else {
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
                // §1206
                self.pop_nest();
                {
                    { let __ix471 = self.cur_list.tail_field; let __v472 = self.new_penalty(self.eqtb[((618174i32) - 1) as usize].int()); self.mem[(__ix471) as usize].set_hh_rh(__v472); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                {
                    { let __ix473 = self.cur_list.tail_field; let __v474 = self.new_param_glue(3i32); self.mem[(__ix473) as usize].set_hh_rh(__v474); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                { let __ix475 = self.cur_list.tail_field; self.mem[(__ix475) as usize].set_hh_rh(p); }
                if (p != 0i32) {
                    self.cur_list.tail_field = q;
                }
                {
                    { let __ix476 = self.cur_list.tail_field; let __v477 = self.new_penalty(self.eqtb[((618175i32) - 1) as usize].int()); self.mem[(__ix476) as usize].set_hh_rh(__v477); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                {
                    { let __ix478 = self.cur_list.tail_field; let __v479 = self.new_param_glue(4i32); self.mem[(__ix478) as usize].set_hh_rh(__v479); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                self.cur_list.aux_field.set_int(aux_save.int());
                self.resume_after_display();
            }
        } else {
            // §812
            {
                self.cur_list.aux_field = aux_save;
                { let __ix480 = self.cur_list.tail_field; self.mem[(__ix480) as usize].set_hh_rh(p); }
                if (p != 0i32) {
                    self.cur_list.tail_field = q;
                }
                if (self.cur_list.mode_field == 1i32) {
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
    // §785
    pub fn align_peek(&mut self) {
        'l_restart_b: loop {
            self.align_state = 1000000i32;
            // §406
            loop {
                self.get_x_token();
                if (self.cur_cmd != 10i32) { break; }
            }
            // §785
            if (self.cur_cmd == 34i32) {
                {
                    self.scan_left_brace();
                    self.new_save_level(7i32);
                    if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                        self.normal_paragraph();
                    }
                }
            } else {
                if (self.cur_cmd == 2i32) {
                    self.fin_align();
                } else {
                    if ((self.cur_cmd == 5i32) && (self.cur_chr == 258i32)) {
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
    // §826
    pub fn finite_shrink(&mut self, mut p: halfword) -> halfword {
        let mut finite_shrink: halfword = 0;
        let mut q: halfword = 0; // §826
        if self.no_shrink_error_yet {
            {
                self.no_shrink_error_yet = false;
                if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                    self.end_diagnostic(true);
                }
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(917i32);
                }
                {
                    self.help_ptr = 5i32;
                    self.help_line[(4i32) as usize] = 918i32;
                    self.help_line[(3i32) as usize] = 919i32;
                    self.help_line[(2i32) as usize] = 920i32;
                    self.help_line[(1i32) as usize] = 921i32;
                    self.help_line[(0i32) as usize] = 922i32;
                }
                self.error();
                if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                    self.begin_diagnostic();
                }
            }
        }
        q = self.new_spec(p);
        self.mem[(q) as usize].set_hh_b1(0i32);
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
    // §829
    pub fn try_break(&mut self, mut pi: i32, mut break_type: small_number) {
        let mut r: halfword = 0; // §829
        let mut prev_r: halfword = 0; // §829
        let mut old_l: halfword = 0; // §829
        let mut no_break_yet: bool = false; // §829
        let mut prev_prev_r: halfword = 0; // §830
        let mut s: halfword = 0; // §830
        let mut q: halfword = 0; // §830
        let mut v: halfword = 0; // §830
        let mut t: i32 = 0; // §830
        let mut f: internal_font_number = 0; // §830
        let mut l: halfword = 0; // §830
        let mut node_r_stays_active: bool = false; // §830
        let mut line_width: scaled = 0; // §830
        let mut fit_class: i32 = 0; // §830
        let mut b: halfword = 0; // §830
        let mut d: i32 = 0; // §830
        let mut artificial_demerits: bool = false; // §830
        let mut save_link: halfword = 0; // §830
        let mut shortfall: scaled = 0; // §830
        'l_exit_f: {
            // §831
            if ((pi).wrapping_abs() >= 10000i32) {
                if (pi > 0i32) {
                    break 'l_exit_f;
                } else {
                    pi = (10000i32).wrapping_neg();
                }
            }
            // §829
            no_break_yet = true;
            prev_r = 4999992i32;
            old_l = 0i32;
            { let __v481 = self.active_width[((1i32) - 1) as usize]; self.cur_active_width[((1i32) - 1) as usize] = __v481; }
            { let __v482 = self.active_width[((2i32) - 1) as usize]; self.cur_active_width[((2i32) - 1) as usize] = __v482; }
            { let __v483 = self.active_width[((3i32) - 1) as usize]; self.cur_active_width[((3i32) - 1) as usize] = __v483; }
            { let __v484 = self.active_width[((4i32) - 1) as usize]; self.cur_active_width[((4i32) - 1) as usize] = __v484; }
            { let __v485 = self.active_width[((5i32) - 1) as usize]; self.cur_active_width[((5i32) - 1) as usize] = __v485; }
            { let __v486 = self.active_width[((6i32) - 1) as usize]; self.cur_active_width[((6i32) - 1) as usize] = __v486; }
            while true {
                {
                    'l_continue_b: loop {
                        r = self.mem[(prev_r) as usize].hh().rh();
                        // §832
                        if (self.mem[(r) as usize].hh().b0() == 2i32) {
                            {
                                { let __v487 = (self.cur_active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.cur_active_width[((1i32) - 1) as usize] = __v487; }
                                { let __v488 = (self.cur_active_width[((2i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.cur_active_width[((2i32) - 1) as usize] = __v488; }
                                { let __v489 = (self.cur_active_width[((3i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.cur_active_width[((3i32) - 1) as usize] = __v489; }
                                { let __v490 = (self.cur_active_width[((4i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(4i32)) as usize].int()); self.cur_active_width[((4i32) - 1) as usize] = __v490; }
                                { let __v491 = (self.cur_active_width[((5i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(5i32)) as usize].int()); self.cur_active_width[((5i32) - 1) as usize] = __v491; }
                                { let __v492 = (self.cur_active_width[((6i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(6i32)) as usize].int()); self.cur_active_width[((6i32) - 1) as usize] = __v492; }
                                prev_prev_r = prev_r;
                                prev_r = r;
                                continue 'l_continue_b;
                            }
                        }
                        // §835
                        {
                            l = self.mem[((r).wrapping_add(1i32)) as usize].hh().lh();
                            if (l > old_l) {
                                {
                                    if ((self.minimum_demerits < 1073741823i32) && ((old_l != self.easy_line) || (r == 4999992i32))) {
                                        // §836
                                        {
                                            if no_break_yet {
                                                // §837
                                                {
                                                    'l_done_f: {
                                                        no_break_yet = false;
                                                        { let __v493 = self.background[((1i32) - 1) as usize]; self.break_width[((1i32) - 1) as usize] = __v493; }
                                                        { let __v494 = self.background[((2i32) - 1) as usize]; self.break_width[((2i32) - 1) as usize] = __v494; }
                                                        { let __v495 = self.background[((3i32) - 1) as usize]; self.break_width[((3i32) - 1) as usize] = __v495; }
                                                        { let __v496 = self.background[((4i32) - 1) as usize]; self.break_width[((4i32) - 1) as usize] = __v496; }
                                                        { let __v497 = self.background[((5i32) - 1) as usize]; self.break_width[((5i32) - 1) as usize] = __v497; }
                                                        { let __v498 = self.background[((6i32) - 1) as usize]; self.break_width[((6i32) - 1) as usize] = __v498; }
                                                        s = self.cur_p;
                                                        if (break_type > 0i32) {
                                                            if (self.cur_p != 0i32) {
                                                                // §840
                                                                {
                                                                    t = self.mem[(self.cur_p) as usize].hh().b1();
                                                                    v = self.cur_p;
                                                                    s = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().rh();
                                                                    while (t > 0i32) {
                                                                        {
                                                                            t = (t).wrapping_sub(1i32);
                                                                            v = self.mem[(v) as usize].hh().rh();
                                                                            // §841
                                                                            if (v >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[(v) as usize].hh().b0();
                                                                                    { let __v499 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(v) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v499; }
                                                                                }
                                                                            } else {
                                                                                match self.mem[(v) as usize].hh().b0() {
                                                                                    6 => {
                                                                                        {
                                                                                            f = self.mem[((v).wrapping_add(1i32)) as usize].hh().b0();
                                                                                            { let __v500 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[((v).wrapping_add(1i32)) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v500; }
                                                                                        }
                                                                                    }
                                                                                    0 | 1 | 2 | 11 => {
                                                                                        { let __v501 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.mem[((v).wrapping_add(1i32)) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v501; }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(923i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §840
                                                                    while (s != 0i32) {
                                                                        {
                                                                            // §842
                                                                            if (s >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[(s) as usize].hh().b0();
                                                                                    { let __v502 = (self.break_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(s) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v502; }
                                                                                }
                                                                            } else {
                                                                                match self.mem[(s) as usize].hh().b0() {
                                                                                    6 => {
                                                                                        {
                                                                                            f = self.mem[((s).wrapping_add(1i32)) as usize].hh().b0();
                                                                                            { let __v503 = (self.break_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v503; }
                                                                                        }
                                                                                    }
                                                                                    0 | 1 | 2 | 11 => {
                                                                                        { let __v504 = (self.break_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v504; }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(924i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                            // §840
                                                                            s = self.mem[(s) as usize].hh().rh();
                                                                        }
                                                                    }
                                                                    { let __v505 = (self.break_width[((1i32) - 1) as usize]).wrapping_add(self.disc_width); self.break_width[((1i32) - 1) as usize] = __v505; }
                                                                    if (self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().rh() == 0i32) {
                                                                        s = self.mem[(v) as usize].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §837
                                                        while (s != 0i32) {
                                                            {
                                                                if (s >= self.hi_mem_min) {
                                                                    break 'l_done_f;
                                                                }
                                                                match self.mem[(s) as usize].hh().b0() {
                                                                    10 => {
                                                                        // §838
                                                                        {
                                                                            v = self.mem[((s).wrapping_add(1i32)) as usize].hh().lh();
                                                                            { let __v506 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.mem[((v).wrapping_add(1i32)) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v506; }
                                                                            { let __ix507 = (2i32).wrapping_add(self.mem[(v) as usize].hh().b0()); let __v508 = (self.break_width[(((2i32).wrapping_add(self.mem[(v) as usize].hh().b0())) - 1) as usize]).wrapping_sub(self.mem[((v).wrapping_add(2i32)) as usize].int()); self.break_width[((__ix507) - 1) as usize] = __v508; }
                                                                            { let __v509 = (self.break_width[((6i32) - 1) as usize]).wrapping_sub(self.mem[((v).wrapping_add(3i32)) as usize].int()); self.break_width[((6i32) - 1) as usize] = __v509; }
                                                                        }
                                                                    }
                                                                    12 => {
                                                                        // §837
                                                                    }
                                                                    9 => {
                                                                        { let __v510 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.mem[((s).wrapping_add(1i32)) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v510; }
                                                                    }
                                                                    11 => {
                                                                        if (self.mem[(s) as usize].hh().b1() != 1i32) {
                                                                            break 'l_done_f;
                                                                        } else {
                                                                            { let __v511 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.mem[((s).wrapping_add(1i32)) as usize].int()); self.break_width[((1i32) - 1) as usize] = __v511; }
                                                                        }
                                                                    }
                                                                    _ => {
                                                                        break 'l_done_f;
                                                                    }
                                                                }
                                                                s = self.mem[(s) as usize].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // §843
                                            if (self.mem[(prev_r) as usize].hh().b0() == 2i32) {
                                                {
                                                    { let __v512 = ((self.mem[((prev_r).wrapping_add(1i32)) as usize].int()).wrapping_sub(self.cur_active_width[((1i32) - 1) as usize])).wrapping_add(self.break_width[((1i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(1i32)) as usize].set_int(__v512); }
                                                    { let __v513 = ((self.mem[((prev_r).wrapping_add(2i32)) as usize].int()).wrapping_sub(self.cur_active_width[((2i32) - 1) as usize])).wrapping_add(self.break_width[((2i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(2i32)) as usize].set_int(__v513); }
                                                    { let __v514 = ((self.mem[((prev_r).wrapping_add(3i32)) as usize].int()).wrapping_sub(self.cur_active_width[((3i32) - 1) as usize])).wrapping_add(self.break_width[((3i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(3i32)) as usize].set_int(__v514); }
                                                    { let __v515 = ((self.mem[((prev_r).wrapping_add(4i32)) as usize].int()).wrapping_sub(self.cur_active_width[((4i32) - 1) as usize])).wrapping_add(self.break_width[((4i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(4i32)) as usize].set_int(__v515); }
                                                    { let __v516 = ((self.mem[((prev_r).wrapping_add(5i32)) as usize].int()).wrapping_sub(self.cur_active_width[((5i32) - 1) as usize])).wrapping_add(self.break_width[((5i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(5i32)) as usize].set_int(__v516); }
                                                    { let __v517 = ((self.mem[((prev_r).wrapping_add(6i32)) as usize].int()).wrapping_sub(self.cur_active_width[((6i32) - 1) as usize])).wrapping_add(self.break_width[((6i32) - 1) as usize]); self.mem[((prev_r).wrapping_add(6i32)) as usize].set_int(__v517); }
                                                }
                                            } else {
                                                if (prev_r == 4999992i32) {
                                                    {
                                                        { let __v518 = self.break_width[((1i32) - 1) as usize]; self.active_width[((1i32) - 1) as usize] = __v518; }
                                                        { let __v519 = self.break_width[((2i32) - 1) as usize]; self.active_width[((2i32) - 1) as usize] = __v519; }
                                                        { let __v520 = self.break_width[((3i32) - 1) as usize]; self.active_width[((3i32) - 1) as usize] = __v520; }
                                                        { let __v521 = self.break_width[((4i32) - 1) as usize]; self.active_width[((4i32) - 1) as usize] = __v521; }
                                                        { let __v522 = self.break_width[((5i32) - 1) as usize]; self.active_width[((5i32) - 1) as usize] = __v522; }
                                                        { let __v523 = self.break_width[((6i32) - 1) as usize]; self.active_width[((6i32) - 1) as usize] = __v523; }
                                                    }
                                                } else {
                                                    {
                                                        q = self.get_node(7i32);
                                                        self.mem[(q) as usize].set_hh_rh(r);
                                                        self.mem[(q) as usize].set_hh_b0(2i32);
                                                        self.mem[(q) as usize].set_hh_b1(0i32);
                                                        { let __v524 = (self.break_width[((1i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((1i32) - 1) as usize]); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v524); }
                                                        { let __v525 = (self.break_width[((2i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((2i32) - 1) as usize]); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v525); }
                                                        { let __v526 = (self.break_width[((3i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((3i32) - 1) as usize]); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v526); }
                                                        { let __v527 = (self.break_width[((4i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((4i32) - 1) as usize]); self.mem[((q).wrapping_add(4i32)) as usize].set_int(__v527); }
                                                        { let __v528 = (self.break_width[((5i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((5i32) - 1) as usize]); self.mem[((q).wrapping_add(5i32)) as usize].set_int(__v528); }
                                                        { let __v529 = (self.break_width[((6i32) - 1) as usize]).wrapping_sub(self.cur_active_width[((6i32) - 1) as usize]); self.mem[((q).wrapping_add(6i32)) as usize].set_int(__v529); }
                                                        self.mem[(prev_r) as usize].set_hh_rh(q);
                                                        prev_prev_r = prev_r;
                                                        prev_r = q;
                                                    }
                                                }
                                            }
                                            // §836
                                            if ((self.eqtb[((618179i32) - 1) as usize].int()).wrapping_abs() >= (1073741823i32).wrapping_sub(self.minimum_demerits)) {
                                                self.minimum_demerits = 1073741822i32;
                                            } else {
                                                self.minimum_demerits = (self.minimum_demerits).wrapping_add((self.eqtb[((618179i32) - 1) as usize].int()).wrapping_abs());
                                            }
                                            {
                                                let __for_end_11 = 3i32;
                                                fit_class = 0i32;
                                                while fit_class <= __for_end_11 {
                                                    {
                                                        if (self.minimal_demerits[(fit_class) as usize] <= self.minimum_demerits) {
                                                            // §845
                                                            {
                                                                q = self.get_node(2i32);
                                                                { let __v530 = self.passive; self.mem[(q) as usize].set_hh_rh(__v530); }
                                                                self.passive = q;
                                                                { let __v531 = self.cur_p; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v531); }
                                                                self.pass_number = (self.pass_number).wrapping_add(1i32);
                                                                { let __v532 = self.pass_number; self.mem[(q) as usize].set_hh_lh(__v532); }
                                                                { let __v533 = self.best_place[(fit_class) as usize]; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v533); }
                                                                q = self.get_node(3i32);
                                                                { let __v534 = self.passive; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v534); }
                                                                { let __v535 = (self.best_pl_line[(fit_class) as usize]).wrapping_add(1i32); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v535); }
                                                                self.mem[(q) as usize].set_hh_b1(fit_class);
                                                                self.mem[(q) as usize].set_hh_b0(break_type);
                                                                { let __v536 = self.minimal_demerits[(fit_class) as usize]; self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v536); }
                                                                self.mem[(q) as usize].set_hh_rh(r);
                                                                self.mem[(prev_r) as usize].set_hh_rh(q);
                                                                prev_r = q;
                                                                if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                                                                    // §846
                                                                    {
                                                                        self.print_nl(925i32);
                                                                        self.print_int(self.mem[(self.passive) as usize].hh().lh());
                                                                        self.print(926i32);
                                                                        self.print_int((self.mem[((q).wrapping_add(1i32)) as usize].hh().lh()).wrapping_sub(1i32));
                                                                        self.print_char(46i32);
                                                                        self.print_int(fit_class);
                                                                        if (break_type == 1i32) {
                                                                            self.print_char(45i32);
                                                                        }
                                                                        self.print(927i32);
                                                                        self.print_int(self.mem[((q).wrapping_add(2i32)) as usize].int());
                                                                        self.print(928i32);
                                                                        if (self.mem[((self.passive).wrapping_add(1i32)) as usize].hh().lh() == 0i32) {
                                                                            self.print_char(48i32);
                                                                        } else {
                                                                            self.print_int(self.mem[(self.mem[((self.passive).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().lh());
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §836
                                                        self.minimal_demerits[(fit_class) as usize] = 1073741823i32;
                                                    }
                                                    fit_class = fit_class.wrapping_add(1);
                                                }
                                            }
                                            self.minimum_demerits = 1073741823i32;
                                            // §844
                                            if (r != 4999992i32) {
                                                {
                                                    q = self.get_node(7i32);
                                                    self.mem[(q) as usize].set_hh_rh(r);
                                                    self.mem[(q) as usize].set_hh_b0(2i32);
                                                    self.mem[(q) as usize].set_hh_b1(0i32);
                                                    { let __v537 = (self.cur_active_width[((1i32) - 1) as usize]).wrapping_sub(self.break_width[((1i32) - 1) as usize]); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v537); }
                                                    { let __v538 = (self.cur_active_width[((2i32) - 1) as usize]).wrapping_sub(self.break_width[((2i32) - 1) as usize]); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v538); }
                                                    { let __v539 = (self.cur_active_width[((3i32) - 1) as usize]).wrapping_sub(self.break_width[((3i32) - 1) as usize]); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v539); }
                                                    { let __v540 = (self.cur_active_width[((4i32) - 1) as usize]).wrapping_sub(self.break_width[((4i32) - 1) as usize]); self.mem[((q).wrapping_add(4i32)) as usize].set_int(__v540); }
                                                    { let __v541 = (self.cur_active_width[((5i32) - 1) as usize]).wrapping_sub(self.break_width[((5i32) - 1) as usize]); self.mem[((q).wrapping_add(5i32)) as usize].set_int(__v541); }
                                                    { let __v542 = (self.cur_active_width[((6i32) - 1) as usize]).wrapping_sub(self.break_width[((6i32) - 1) as usize]); self.mem[((q).wrapping_add(6i32)) as usize].set_int(__v542); }
                                                    self.mem[(prev_r) as usize].set_hh_rh(q);
                                                    prev_prev_r = prev_r;
                                                    prev_r = q;
                                                }
                                            }
                                        }
                                    }
                                    // §835
                                    if (r == 4999992i32) {
                                        break 'l_exit_f;
                                    }
                                    // §850
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
                                                if (self.eqtb[((616312i32) - 1) as usize].hh().rh() == 0i32) {
                                                    line_width = self.first_width;
                                                } else {
                                                    line_width = self.mem[((self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(l))) as usize].int();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §851
                        {
                            'l_L60_f: {
                                artificial_demerits = false;
                                shortfall = (line_width).wrapping_sub(self.cur_active_width[((1i32) - 1) as usize]);
                                if (shortfall > 0i32) {
                                    // §852
                                    if (((self.cur_active_width[((3i32) - 1) as usize] != 0i32) || (self.cur_active_width[((4i32) - 1) as usize] != 0i32)) || (self.cur_active_width[((5i32) - 1) as usize] != 0i32)) {
                                        {
                                            b = 0i32;
                                            fit_class = 2i32;
                                        }
                                    } else {
                                        {
                                            'l_done1_f: {
                                                if (shortfall > 7230584i32) {
                                                    if (self.cur_active_width[((2i32) - 1) as usize] < 1663497i32) {
                                                        {
                                                            b = 10000i32;
                                                            fit_class = 0i32;
                                                            break 'l_done1_f;
                                                        }
                                                    }
                                                }
                                                b = self.badness(shortfall, self.cur_active_width[((2i32) - 1) as usize]);
                                                if (b > 12i32) {
                                                    if (b > 99i32) {
                                                        fit_class = 0i32;
                                                    } else {
                                                        fit_class = 1i32;
                                                    }
                                                } else {
                                                    fit_class = 2i32;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // §853
                                    {
                                        if ((shortfall).wrapping_neg() > self.cur_active_width[((6i32) - 1) as usize]) {
                                            b = 10001i32;
                                        } else {
                                            b = self.badness((shortfall).wrapping_neg(), self.cur_active_width[((6i32) - 1) as usize]);
                                        }
                                        if (b > 12i32) {
                                            fit_class = 3i32;
                                        } else {
                                            fit_class = 2i32;
                                        }
                                    }
                                }
                                // §851
                                if ((b > 10000i32) || (pi == (10000i32).wrapping_neg())) {
                                    // §854
                                    {
                                        if (((self.final_pass && (self.minimum_demerits == 1073741823i32)) && (self.mem[(r) as usize].hh().rh() == 4999992i32)) && (prev_r == 4999992i32)) {
                                            artificial_demerits = true;
                                        } else {
                                            if (b > self.threshold) {
                                                break 'l_L60_f;
                                            }
                                        }
                                        node_r_stays_active = false;
                                    }
                                } else {
                                    // §851
                                    {
                                        prev_r = r;
                                        if (b > self.threshold) {
                                            continue 'l_continue_b;
                                        }
                                        node_r_stays_active = true;
                                    }
                                }
                                // §855
                                if artificial_demerits {
                                    d = 0i32;
                                } else {
                                    // §859
                                    {
                                        d = (self.eqtb[((618165i32) - 1) as usize].int()).wrapping_add(b);
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
                                        if ((break_type == 1i32) && (self.mem[(r) as usize].hh().b0() == 1i32)) {
                                            if (self.cur_p != 0i32) {
                                                d = (d).wrapping_add(self.eqtb[((618177i32) - 1) as usize].int());
                                            } else {
                                                d = (d).wrapping_add(self.eqtb[((618178i32) - 1) as usize].int());
                                            }
                                        }
                                        if (((fit_class).wrapping_sub(self.mem[(r) as usize].hh().b1())).wrapping_abs() > 1i32) {
                                            d = (d).wrapping_add(self.eqtb[((618179i32) - 1) as usize].int());
                                        }
                                    }
                                }
                                // §855
                                if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                                    // §856
                                    {
                                        if (self.printed_node != self.cur_p) {
                                            // §857
                                            {
                                                self.print_nl(338i32);
                                                if (self.cur_p == 0i32) {
                                                    self.short_display(self.mem[(self.printed_node) as usize].hh().rh());
                                                } else {
                                                    {
                                                        save_link = self.mem[(self.cur_p) as usize].hh().rh();
                                                        { let __ix543 = self.cur_p; self.mem[(__ix543) as usize].set_hh_rh(0i32); }
                                                        self.print_nl(338i32);
                                                        self.short_display(self.mem[(self.printed_node) as usize].hh().rh());
                                                        { let __ix544 = self.cur_p; self.mem[(__ix544) as usize].set_hh_rh(save_link); }
                                                    }
                                                }
                                                self.printed_node = self.cur_p;
                                            }
                                        }
                                        // §856
                                        self.print_nl(64i32);
                                        if (self.cur_p == 0i32) {
                                            self.print_esc(597i32);
                                        } else {
                                            if (self.mem[(self.cur_p) as usize].hh().b0() != 10i32) {
                                                {
                                                    if (self.mem[(self.cur_p) as usize].hh().b0() == 12i32) {
                                                        self.print_esc(531i32);
                                                    } else {
                                                        if (self.mem[(self.cur_p) as usize].hh().b0() == 7i32) {
                                                            self.print_esc(349i32);
                                                        } else {
                                                            if (self.mem[(self.cur_p) as usize].hh().b0() == 11i32) {
                                                                self.print_esc(340i32);
                                                            } else {
                                                                self.print_esc(343i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.print(929i32);
                                        if (self.mem[((r).wrapping_add(1i32)) as usize].hh().rh() == 0i32) {
                                            self.print_char(48i32);
                                        } else {
                                            self.print_int(self.mem[(self.mem[((r).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().lh());
                                        }
                                        self.print(930i32);
                                        if (b > 10000i32) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(b);
                                        }
                                        self.print(931i32);
                                        self.print_int(pi);
                                        self.print(932i32);
                                        if artificial_demerits {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(d);
                                        }
                                    }
                                }
                                // §855
                                d = (d).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int());
                                if (d <= self.minimal_demerits[(fit_class) as usize]) {
                                    {
                                        self.minimal_demerits[(fit_class) as usize] = d;
                                        { let __v545 = self.mem[((r).wrapping_add(1i32)) as usize].hh().rh(); self.best_place[(fit_class) as usize] = __v545; }
                                        self.best_pl_line[(fit_class) as usize] = l;
                                        if (d < self.minimum_demerits) {
                                            self.minimum_demerits = d;
                                        }
                                    }
                                }
                                // §851
                                if node_r_stays_active {
                                    continue 'l_continue_b;
                                }
                            }
                            { let __v546 = self.mem[(r) as usize].hh().rh(); self.mem[(prev_r) as usize].set_hh_rh(__v546); }
                            // §860
                            self.free_node(r, 3i32);
                            if (prev_r == 4999992i32) {
                                // §861
                                {
                                    r = self.mem[(4999992i32) as usize].hh().rh();
                                    if (self.mem[(r) as usize].hh().b0() == 2i32) {
                                        {
                                            { let __v547 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v547; }
                                            { let __v548 = (self.active_width[((2i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.active_width[((2i32) - 1) as usize] = __v548; }
                                            { let __v549 = (self.active_width[((3i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.active_width[((3i32) - 1) as usize] = __v549; }
                                            { let __v550 = (self.active_width[((4i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(4i32)) as usize].int()); self.active_width[((4i32) - 1) as usize] = __v550; }
                                            { let __v551 = (self.active_width[((5i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(5i32)) as usize].int()); self.active_width[((5i32) - 1) as usize] = __v551; }
                                            { let __v552 = (self.active_width[((6i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(6i32)) as usize].int()); self.active_width[((6i32) - 1) as usize] = __v552; }
                                            { let __v553 = self.active_width[((1i32) - 1) as usize]; self.cur_active_width[((1i32) - 1) as usize] = __v553; }
                                            { let __v554 = self.active_width[((2i32) - 1) as usize]; self.cur_active_width[((2i32) - 1) as usize] = __v554; }
                                            { let __v555 = self.active_width[((3i32) - 1) as usize]; self.cur_active_width[((3i32) - 1) as usize] = __v555; }
                                            { let __v556 = self.active_width[((4i32) - 1) as usize]; self.cur_active_width[((4i32) - 1) as usize] = __v556; }
                                            { let __v557 = self.active_width[((5i32) - 1) as usize]; self.cur_active_width[((5i32) - 1) as usize] = __v557; }
                                            { let __v558 = self.active_width[((6i32) - 1) as usize]; self.cur_active_width[((6i32) - 1) as usize] = __v558; }
                                            { let __v559 = self.mem[(r) as usize].hh().rh(); self.mem[(4999992i32) as usize].set_hh_rh(__v559); }
                                            self.free_node(r, 7i32);
                                        }
                                    }
                                }
                            } else {
                                // §860
                                if (self.mem[(prev_r) as usize].hh().b0() == 2i32) {
                                    {
                                        r = self.mem[(prev_r) as usize].hh().rh();
                                        if (r == 4999992i32) {
                                            {
                                                { let __v560 = (self.cur_active_width[((1i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(1i32)) as usize].int()); self.cur_active_width[((1i32) - 1) as usize] = __v560; }
                                                { let __v561 = (self.cur_active_width[((2i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(2i32)) as usize].int()); self.cur_active_width[((2i32) - 1) as usize] = __v561; }
                                                { let __v562 = (self.cur_active_width[((3i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(3i32)) as usize].int()); self.cur_active_width[((3i32) - 1) as usize] = __v562; }
                                                { let __v563 = (self.cur_active_width[((4i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(4i32)) as usize].int()); self.cur_active_width[((4i32) - 1) as usize] = __v563; }
                                                { let __v564 = (self.cur_active_width[((5i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(5i32)) as usize].int()); self.cur_active_width[((5i32) - 1) as usize] = __v564; }
                                                { let __v565 = (self.cur_active_width[((6i32) - 1) as usize]).wrapping_sub(self.mem[((prev_r).wrapping_add(6i32)) as usize].int()); self.cur_active_width[((6i32) - 1) as usize] = __v565; }
                                                self.mem[(prev_prev_r) as usize].set_hh_rh(4999992i32);
                                                self.free_node(prev_r, 7i32);
                                                prev_r = prev_prev_r;
                                            }
                                        } else {
                                            if (self.mem[(r) as usize].hh().b0() == 2i32) {
                                                {
                                                    { let __v566 = (self.cur_active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.cur_active_width[((1i32) - 1) as usize] = __v566; }
                                                    { let __v567 = (self.cur_active_width[((2i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.cur_active_width[((2i32) - 1) as usize] = __v567; }
                                                    { let __v568 = (self.cur_active_width[((3i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.cur_active_width[((3i32) - 1) as usize] = __v568; }
                                                    { let __v569 = (self.cur_active_width[((4i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(4i32)) as usize].int()); self.cur_active_width[((4i32) - 1) as usize] = __v569; }
                                                    { let __v570 = (self.cur_active_width[((5i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(5i32)) as usize].int()); self.cur_active_width[((5i32) - 1) as usize] = __v570; }
                                                    { let __v571 = (self.cur_active_width[((6i32) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(6i32)) as usize].int()); self.cur_active_width[((6i32) - 1) as usize] = __v571; }
                                                    { let __v572 = (self.mem[((prev_r).wrapping_add(1i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.mem[((prev_r).wrapping_add(1i32)) as usize].set_int(__v572); }
                                                    { let __v573 = (self.mem[((prev_r).wrapping_add(2i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.mem[((prev_r).wrapping_add(2i32)) as usize].set_int(__v573); }
                                                    { let __v574 = (self.mem[((prev_r).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.mem[((prev_r).wrapping_add(3i32)) as usize].set_int(__v574); }
                                                    { let __v575 = (self.mem[((prev_r).wrapping_add(4i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(4i32)) as usize].int()); self.mem[((prev_r).wrapping_add(4i32)) as usize].set_int(__v575); }
                                                    { let __v576 = (self.mem[((prev_r).wrapping_add(5i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(5i32)) as usize].int()); self.mem[((prev_r).wrapping_add(5i32)) as usize].set_int(__v576); }
                                                    { let __v577 = (self.mem[((prev_r).wrapping_add(6i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(6i32)) as usize].int()); self.mem[((prev_r).wrapping_add(6i32)) as usize].set_int(__v577); }
                                                    { let __v578 = self.mem[(r) as usize].hh().rh(); self.mem[(prev_r) as usize].set_hh_rh(__v578); }
                                                    self.free_node(r, 7i32);
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
        // §829
        if (self.cur_p == self.printed_node) {
            // §858
            if (self.cur_p != 0i32) {
                if (self.mem[(self.cur_p) as usize].hh().b0() == 7i32) {
                    {
                        t = self.mem[(self.cur_p) as usize].hh().b1();
                        while (t > 0i32) {
                            {
                                t = (t).wrapping_sub(1i32);
                                self.printed_node = self.mem[(self.printed_node) as usize].hh().rh();
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
    // §877
    pub fn post_line_break(&mut self, mut final_widow_penalty: i32) {
        let mut q: halfword = 0; // §877
        let mut r: halfword = 0; // §877
        let mut s: halfword = 0; // §877
        let mut disc_break: bool = false; // §877
        let mut post_disc_break: bool = false; // §877
        let mut cur_width: scaled = 0; // §877
        let mut cur_indent: scaled = 0; // §877
        let mut t: quarterword = 0; // §877
        let mut pen: i32 = 0; // §877
        let mut cur_line: halfword = 0; // §877
        // §878
        q = self.mem[((self.best_bet).wrapping_add(1i32)) as usize].hh().rh();
        self.cur_p = 0i32;
        loop {
            r = q;
            q = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
            { let __v579 = self.cur_p; self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(__v579); }
            self.cur_p = r;
            if (q == 0i32) { break; }
        }
        // §877
        cur_line = (self.cur_list.pg_field).wrapping_add(1i32);
        loop {
            'l_done_f: {
                // §881
                q = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().rh();
                disc_break = false;
                post_disc_break = false;
                if (q != 0i32) {
                    if (self.mem[(q) as usize].hh().b0() == 10i32) {
                        {
                            self.delete_glue_ref(self.mem[((q).wrapping_add(1i32)) as usize].hh().lh());
                            { let __v580 = self.eqtb[((615790i32) - 1) as usize].hh().rh(); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v580); }
                            self.mem[(q) as usize].set_hh_b1(9i32);
                            { let __ix581 = self.eqtb[((615790i32) - 1) as usize].hh().rh(); let __v582 = (self.mem[(self.eqtb[((615790i32) - 1) as usize].hh().rh()) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix581) as usize].set_hh_rh(__v582); }
                            break 'l_done_f;
                        }
                    } else {
                        {
                            if (self.mem[(q) as usize].hh().b0() == 7i32) {
                                // §882
                                {
                                    t = self.mem[(q) as usize].hh().b1();
                                    // §883
                                    if (t == 0i32) {
                                        r = self.mem[(q) as usize].hh().rh();
                                    } else {
                                        {
                                            r = q;
                                            while (t > 1i32) {
                                                {
                                                    r = self.mem[(r) as usize].hh().rh();
                                                    t = (t).wrapping_sub(1i32);
                                                }
                                            }
                                            s = self.mem[(r) as usize].hh().rh();
                                            r = self.mem[(s) as usize].hh().rh();
                                            self.mem[(s) as usize].set_hh_rh(0i32);
                                            self.flush_node_list(self.mem[(q) as usize].hh().rh());
                                            self.mem[(q) as usize].set_hh_b1(0i32);
                                        }
                                    }
                                    // §882
                                    if (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() != 0i32) {
                                        // §884
                                        {
                                            s = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                                            while (self.mem[(s) as usize].hh().rh() != 0i32) {
                                                s = self.mem[(s) as usize].hh().rh();
                                            }
                                            self.mem[(s) as usize].set_hh_rh(r);
                                            r = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
                                            post_disc_break = true;
                                        }
                                    }
                                    // §882
                                    if (self.mem[((q).wrapping_add(1i32)) as usize].hh().lh() != 0i32) {
                                        // §885
                                        {
                                            s = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
                                            self.mem[(q) as usize].set_hh_rh(s);
                                            while (self.mem[(s) as usize].hh().rh() != 0i32) {
                                                s = self.mem[(s) as usize].hh().rh();
                                            }
                                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
                                            q = s;
                                        }
                                    }
                                    // §882
                                    self.mem[(q) as usize].set_hh_rh(r);
                                    disc_break = true;
                                }
                            } else {
                                // §881
                                if ((self.mem[(q) as usize].hh().b0() == 9i32) || (self.mem[(q) as usize].hh().b0() == 11i32)) {
                                    self.mem[((q).wrapping_add(1i32)) as usize].set_int(0i32);
                                }
                            }
                        }
                    }
                } else {
                    {
                        q = 4999996i32;
                        while (self.mem[(q) as usize].hh().rh() != 0i32) {
                            q = self.mem[(q) as usize].hh().rh();
                        }
                    }
                }
                // §886
                r = self.new_param_glue(8i32);
                { let __v583 = self.mem[(q) as usize].hh().rh(); self.mem[(r) as usize].set_hh_rh(__v583); }
                self.mem[(q) as usize].set_hh_rh(r);
                q = r;
            }
            // §881
            // §887
            r = self.mem[(q) as usize].hh().rh();
            self.mem[(q) as usize].set_hh_rh(0i32);
            q = self.mem[(4999996i32) as usize].hh().rh();
            self.mem[(4999996i32) as usize].set_hh_rh(r);
            if (self.eqtb[((615789i32) - 1) as usize].hh().rh() != 0i32) {
                {
                    r = self.new_param_glue(7i32);
                    self.mem[(r) as usize].set_hh_rh(q);
                    q = r;
                }
            }
            // §889
            if (cur_line > self.last_special_line) {
                {
                    cur_width = self.second_width;
                    cur_indent = self.second_indent;
                }
            } else {
                if (self.eqtb[((616312i32) - 1) as usize].hh().rh() == 0i32) {
                    {
                        cur_width = self.first_width;
                        cur_indent = self.first_indent;
                    }
                } else {
                    {
                        cur_width = self.mem[((self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(cur_line))) as usize].int();
                        cur_indent = self.mem[(((self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(cur_line))).wrapping_sub(1i32)) as usize].int();
                    }
                }
            }
            self.adjust_tail = 4999994i32;
            self.just_box = self.hpack(q, cur_width, 0i32);
            { let __ix584 = (self.just_box).wrapping_add(4i32); self.mem[(__ix584) as usize].set_int(cur_indent); }
            // §888
            self.append_to_vlist(self.just_box);
            if (4999994i32 != self.adjust_tail) {
                {
                    { let __ix585 = self.cur_list.tail_field; let __v586 = self.mem[(4999994i32) as usize].hh().rh(); self.mem[(__ix585) as usize].set_hh_rh(__v586); }
                    self.cur_list.tail_field = self.adjust_tail;
                }
            }
            self.adjust_tail = 0i32;
            // §890
            if ((cur_line).wrapping_add(1i32) != self.best_line) {
                {
                    pen = self.eqtb[((618176i32) - 1) as usize].int();
                    if (cur_line == (self.cur_list.pg_field).wrapping_add(1i32)) {
                        pen = (pen).wrapping_add(self.eqtb[((618168i32) - 1) as usize].int());
                    }
                    if ((cur_line).wrapping_add(2i32) == self.best_line) {
                        pen = (pen).wrapping_add(final_widow_penalty);
                    }
                    if disc_break {
                        pen = (pen).wrapping_add(self.eqtb[((618171i32) - 1) as usize].int());
                    }
                    if (pen != 0i32) {
                        {
                            r = self.new_penalty(pen);
                            { let __ix587 = self.cur_list.tail_field; self.mem[(__ix587) as usize].set_hh_rh(r); }
                            self.cur_list.tail_field = r;
                        }
                    }
                }
            }
            // §877
            cur_line = (cur_line).wrapping_add(1i32);
            self.cur_p = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh();
            if (self.cur_p != 0i32) {
                if (!post_disc_break) {
                    // §879
                    {
                        'l_done1_f: {
                            r = 4999996i32;
                            while true {
                                {
                                    q = self.mem[(r) as usize].hh().rh();
                                    if (q == self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().rh()) {
                                        break 'l_done1_f;
                                    }
                                    if (q >= self.hi_mem_min) {
                                        break 'l_done1_f;
                                    }
                                    if (self.mem[(q) as usize].hh().b0() < 9i32) {
                                        break 'l_done1_f;
                                    }
                                    if (self.mem[(q) as usize].hh().b0() == 11i32) {
                                        if (self.mem[(q) as usize].hh().b1() != 1i32) {
                                            break 'l_done1_f;
                                        }
                                    }
                                    r = q;
                                }
                            }
                        }
                        if (r != 4999996i32) {
                            {
                                self.mem[(r) as usize].set_hh_rh(0i32);
                                self.flush_node_list(self.mem[(4999996i32) as usize].hh().rh());
                                self.mem[(4999996i32) as usize].set_hh_rh(q);
                            }
                        }
                    }
                }
            }
            if (self.cur_p == 0i32) { break; }
        }
        // §877
        if ((cur_line != self.best_line) || (self.mem[(4999996i32) as usize].hh().rh() != 0i32)) {
            self.confusion(939i32);
        }
        self.cur_list.pg_field = (self.best_line).wrapping_sub(1i32);
    }

    /// @<Declare the function called `reconstitute`
    // §906
    pub fn reconstitute(&mut self, mut j: small_number, mut n: small_number, mut bchar: halfword, mut hchar: halfword) -> small_number {
        let mut reconstitute: small_number = 0;
        let mut p: halfword = 0; // §906
        let mut t: halfword = 0; // §906
        let mut q: four_quarters = four_quarters::default(); // §906
        let mut cur_rh: halfword = 0; // §906
        let mut test_char: halfword = 0; // §906
        let mut w: scaled = 0; // §906
        let mut k: font_index = 0; // §906
        // goto labels: continue, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.hyphen_passed = 0i32;
                t = 4999995i32;
                w = 0i32;
                self.mem[(4999995i32) as usize].set_hh_rh(0i32);
                // §908
                self.cur_l = (self.hu[(j) as usize]).wrapping_add(0i32);
                self.cur_q = t;
                if (j == 0i32) {
                    {
                        self.ligature_present = self.init_lig;
                        p = self.init_list;
                        if self.ligature_present {
                            self.lft_hit = self.init_lft;
                        }
                        while (p > 0i32) {
                            {
                                {
                                    { let __v588 = self.get_avail(); self.mem[(t) as usize].set_hh_rh(__v588); }
                                    t = self.mem[(t) as usize].hh().rh();
                                    { let __v589 = self.hf; self.mem[(t) as usize].set_hh_b0(__v589); }
                                    { let __v590 = self.mem[(p) as usize].hh().b1(); self.mem[(t) as usize].set_hh_b1(__v590); }
                                }
                                p = self.mem[(p) as usize].hh().rh();
                            }
                        }
                    }
                } else {
                    if (self.cur_l < 256i32) {
                        {
                            { let __v591 = self.get_avail(); self.mem[(t) as usize].set_hh_rh(__v591); }
                            t = self.mem[(t) as usize].hh().rh();
                            { let __v592 = self.hf; self.mem[(t) as usize].set_hh_b0(__v592); }
                            { let __v593 = self.cur_l; self.mem[(t) as usize].set_hh_b1(__v593); }
                        }
                    }
                }
                self.lig_stack = 0i32;
                {
                    if (j < n) {
                        self.cur_r = (self.hu[((j).wrapping_add(1i32)) as usize]).wrapping_add(0i32);
                    } else {
                        self.cur_r = bchar;
                    }
                    if (((self.hyf[(j) as usize]) % 2) != 0) {
                        cur_rh = hchar;
                    } else {
                        cur_rh = 256i32;
                    }
                }
            }
            if __goto_1 <= 1 { // continue
                // §906
                if (self.cur_l == 256i32) {
                    // §909
                    {
                        k = self.bchar_label[(self.hf) as usize];
                        if (k == 0i32) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        } else {
                            q = self.font_info[(k) as usize].qqqq();
                        }
                    }
                } else {
                    {
                        q = self.font_info[((self.char_base[(self.hf) as usize]).wrapping_add(self.cur_l)) as usize].qqqq();
                        if (((q.b2()).wrapping_sub(0i32) % 4i32) != 1i32) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                        k = (self.lig_kern_base[(self.hf) as usize]).wrapping_add(q.b3());
                        q = self.font_info[(k) as usize].qqqq();
                        if (q.b0() > 128i32) {
                            {
                                k = ((((self.lig_kern_base[(self.hf) as usize]).wrapping_add((256i32).wrapping_mul(q.b2()))).wrapping_add(q.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                q = self.font_info[(k) as usize].qqqq();
                            }
                        }
                    }
                }
                if (cur_rh < 256i32) {
                    test_char = cur_rh;
                } else {
                    test_char = self.cur_r;
                }
                while true {
                    {
                        if (q.b1() == test_char) {
                            if (q.b0() <= 128i32) {
                                if (cur_rh < 256i32) {
                                    {
                                        self.hyphen_passed = j;
                                        hchar = 256i32;
                                        cur_rh = 256i32;
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    {
                                        if (hchar < 256i32) {
                                            if (((self.hyf[(j) as usize]) % 2) != 0) {
                                                {
                                                    self.hyphen_passed = j;
                                                    hchar = 256i32;
                                                }
                                            }
                                        }
                                        if (q.b2() < 128i32) {
                                            // §911
                                            {
                                                if (self.cur_l == 256i32) {
                                                    self.lft_hit = true;
                                                }
                                                if (j == n) {
                                                    if (self.lig_stack == 0i32) {
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
                                                            if (self.lig_stack > 0i32) {
                                                                { let __ix594 = self.lig_stack; let __v595 = self.cur_r; self.mem[(__ix594) as usize].set_hh_b1(__v595); }
                                                            } else {
                                                                {
                                                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                                                    if (j == n) {
                                                                        bchar = 256i32;
                                                                    } else {
                                                                        {
                                                                            p = self.get_avail();
                                                                            { let __ix596 = (self.lig_stack).wrapping_add(1i32); self.mem[(__ix596) as usize].set_hh_rh(p); }
                                                                            { let __v597 = (self.hu[((j).wrapping_add(1i32)) as usize]).wrapping_add(0i32); self.mem[(p) as usize].set_hh_b1(__v597); }
                                                                            { let __v598 = self.hf; self.mem[(p) as usize].set_hh_b0(__v598); }
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
                                                            { let __ix599 = self.lig_stack; self.mem[(__ix599) as usize].set_hh_rh(p); }
                                                        }
                                                    }
                                                    7 | 11 => {
                                                        {
                                                            if self.ligature_present {
                                                                {
                                                                    p = self.new_ligature(self.hf, self.cur_l, self.mem[(self.cur_q) as usize].hh().rh());
                                                                    if self.lft_hit {
                                                                        {
                                                                            self.mem[(p) as usize].set_hh_b1(2i32);
                                                                            self.lft_hit = false;
                                                                        }
                                                                    }
                                                                    if false {
                                                                        if (self.lig_stack == 0i32) {
                                                                            {
                                                                                { let __v600 = (self.mem[(p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(p) as usize].set_hh_b1(__v600); }
                                                                                self.rt_hit = false;
                                                                            }
                                                                        }
                                                                    }
                                                                    { let __ix601 = self.cur_q; self.mem[(__ix601) as usize].set_hh_rh(p); }
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
                                                            if (self.lig_stack > 0i32) {
                                                                {
                                                                    if (self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh() > 0i32) {
                                                                        {
                                                                            { let __v602 = self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh(); self.mem[(t) as usize].set_hh_rh(__v602); }
                                                                            t = self.mem[(t) as usize].hh().rh();
                                                                            j = (j).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                    p = self.lig_stack;
                                                                    self.lig_stack = self.mem[(p) as usize].hh().rh();
                                                                    self.free_node(p, 2i32);
                                                                    if (self.lig_stack == 0i32) {
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = (self.hu[((j).wrapping_add(1i32)) as usize]).wrapping_add(0i32);
                                                                            } else {
                                                                                self.cur_r = bchar;
                                                                            }
                                                                            if (((self.hyf[(j) as usize]) % 2) != 0) {
                                                                                cur_rh = hchar;
                                                                            } else {
                                                                                cur_rh = 256i32;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        self.cur_r = self.mem[(self.lig_stack) as usize].hh().b1();
                                                                    }
                                                                }
                                                            } else {
                                                                if (j == n) {
                                                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                                                } else {
                                                                    {
                                                                        {
                                                                            { let __v603 = self.get_avail(); self.mem[(t) as usize].set_hh_rh(__v603); }
                                                                            t = self.mem[(t) as usize].hh().rh();
                                                                            { let __v604 = self.hf; self.mem[(t) as usize].set_hh_b0(__v604); }
                                                                            { let __v605 = self.cur_r; self.mem[(t) as usize].set_hh_b1(__v605); }
                                                                        }
                                                                        j = (j).wrapping_add(1i32);
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = (self.hu[((j).wrapping_add(1i32)) as usize]).wrapping_add(0i32);
                                                                            } else {
                                                                                self.cur_r = bchar;
                                                                            }
                                                                            if (((self.hyf[(j) as usize]) % 2) != 0) {
                                                                                cur_rh = hchar;
                                                                            } else {
                                                                                cur_rh = 256i32;
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
                                        // §909
                                        w = self.font_info[(((self.kern_base[(self.hf) as usize]).wrapping_add((256i32).wrapping_mul(q.b2()))).wrapping_add(q.b3())) as usize].int();
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                        }
                        if (q.b0() >= 128i32) {
                            if (cur_rh == 256i32) {
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            } else {
                                {
                                    cur_rh = 256i32;
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                        }
                        k = ((k).wrapping_add(q.b0())).wrapping_add(1i32);
                        q = self.font_info[(k) as usize].qqqq();
                    }
                }
            }
            if __goto_1 <= 2 { // done
                // §910
                if self.ligature_present {
                    {
                        p = self.new_ligature(self.hf, self.cur_l, self.mem[(self.cur_q) as usize].hh().rh());
                        if self.lft_hit {
                            {
                                self.mem[(p) as usize].set_hh_b1(2i32);
                                self.lft_hit = false;
                            }
                        }
                        if self.rt_hit {
                            if (self.lig_stack == 0i32) {
                                {
                                    { let __v606 = (self.mem[(p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(p) as usize].set_hh_b1(__v606); }
                                    self.rt_hit = false;
                                }
                            }
                        }
                        { let __ix607 = self.cur_q; self.mem[(__ix607) as usize].set_hh_rh(p); }
                        t = p;
                        self.ligature_present = false;
                    }
                }
                if (w != 0i32) {
                    {
                        { let __v608 = self.new_kern(w); self.mem[(t) as usize].set_hh_rh(__v608); }
                        t = self.mem[(t) as usize].hh().rh();
                        w = 0i32;
                    }
                }
                if (self.lig_stack > 0i32) {
                    {
                        self.cur_q = t;
                        self.cur_l = self.mem[(self.lig_stack) as usize].hh().b1();
                        self.ligature_present = true;
                        {
                            if (self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh() > 0i32) {
                                {
                                    { let __v609 = self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh(); self.mem[(t) as usize].set_hh_rh(__v609); }
                                    t = self.mem[(t) as usize].hh().rh();
                                    j = (j).wrapping_add(1i32);
                                }
                            }
                            p = self.lig_stack;
                            self.lig_stack = self.mem[(p) as usize].hh().rh();
                            self.free_node(p, 2i32);
                            if (self.lig_stack == 0i32) {
                                {
                                    if (j < n) {
                                        self.cur_r = (self.hu[((j).wrapping_add(1i32)) as usize]).wrapping_add(0i32);
                                    } else {
                                        self.cur_r = bchar;
                                    }
                                    if (((self.hyf[(j) as usize]) % 2) != 0) {
                                        cur_rh = hchar;
                                    } else {
                                        cur_rh = 256i32;
                                    }
                                }
                            } else {
                                self.cur_r = self.mem[(self.lig_stack) as usize].hh().b1();
                            }
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                // §906
                reconstitute = j;
            }
            break 'l_dispatch_1;
        }
        reconstitute
    }

    /// @<Declare subprocedures for `line_break`
    // §895
    pub fn hyphenate(&mut self) {
        let mut i: i32 = 0; // §901
        let mut j: i32 = 0; // §901
        let mut l: i32 = 0; // §901
        let mut q: halfword = 0; // §901
        let mut r: halfword = 0; // §901
        let mut s: halfword = 0; // §901
        let mut bchar: halfword = 0; // §901
        let mut major_tail: halfword = 0; // §912
        let mut minor_tail: halfword = 0; // §912
        let mut c: ASCII_code = 0; // §912
        let mut c_loc: i32 = 0; // §912
        let mut r_count: i32 = 0; // §912
        let mut hyf_node: halfword = 0; // §912
        let mut z: trie_pointer = 0; // §922
        let mut v: i32 = 0; // §922
        let mut h: hyph_pointer = 0; // §929
        let mut k: str_number = 0; // §929
        let mut u: pool_pointer = 0; // §929
        'l_exit_f: {
            'l_common_ending_f: {
                'l_found2_f: {
                    'l_found1_f: {
                        'l_found_f: {
                            'l_not_found_f: {
                                // §923
                                {
                                    let __for_end_8 = self.hn;
                                    j = 0i32;
                                    while j <= __for_end_8 {
                                        self.hyf[(j) as usize] = 0i32;
                                        j = j.wrapping_add(1);
                                    }
                                }
                                // §930
                                h = self.hc[(1i32) as usize];
                                self.hn = (self.hn).wrapping_add(1i32);
                                { let __ix610 = self.hn; let __v611 = self.cur_lang; self.hc[(__ix610) as usize] = __v611; }
                                {
                                    let __for_end_8 = self.hn;
                                    j = 2i32;
                                    while j <= __for_end_8 {
                                        h = (((h).wrapping_add(h)).wrapping_add(self.hc[(j) as usize]) % 8191i32);
                                        j = j.wrapping_add(1);
                                    }
                                }
                                while true {
                                    {
                                        'l_done_f: {
                                            // §931
                                            k = self.hyph_word[(h) as usize];
                                            if (k == 0i32) {
                                                break 'l_not_found_f;
                                            }
                                            if ((self.str_start[((k).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(k) as usize]) < self.hn) {
                                                break 'l_not_found_f;
                                            }
                                            if ((self.str_start[((k).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(k) as usize]) == self.hn) {
                                                {
                                                    j = 1i32;
                                                    u = self.str_start[(k) as usize];
                                                    loop {
                                                        if (self.str_pool[(u) as usize] < self.hc[(j) as usize]) {
                                                            break 'l_not_found_f;
                                                        }
                                                        if (self.str_pool[(u) as usize] > self.hc[(j) as usize]) {
                                                            break 'l_done_f;
                                                        }
                                                        j = (j).wrapping_add(1i32);
                                                        u = (u).wrapping_add(1i32);
                                                        if (j > self.hn) { break; }
                                                    }
                                                    // §932
                                                    s = self.hyph_list[(h) as usize];
                                                    while (s != 0i32) {
                                                        {
                                                            self.hyf[(self.mem[(s) as usize].hh().lh()) as usize] = 1i32;
                                                            s = self.mem[(s) as usize].hh().rh();
                                                        }
                                                    }
                                                    // §931
                                                    self.hn = (self.hn).wrapping_sub(1i32);
                                                    break 'l_found_f;
                                                }
                                            }
                                        }
                                        // §930
                                        if (h > 0i32) {
                                            h = (h).wrapping_sub(1i32);
                                        } else {
                                            h = 8191i32;
                                        }
                                    }
                                }
                            }
                            self.hn = (self.hn).wrapping_sub(1i32);
                            // §923
                            if (self.trie[((self.cur_lang).wrapping_add(1i32)) as usize].b1() != (self.cur_lang).wrapping_add(0i32)) {
                                break 'l_exit_f;
                            }
                            self.hc[(0i32) as usize] = 0i32;
                            self.hc[((self.hn).wrapping_add(1i32)) as usize] = 0i32;
                            self.hc[((self.hn).wrapping_add(2i32)) as usize] = 256i32;
                            {
                                let __for_end_7 = ((self.hn).wrapping_sub(self.r_hyf)).wrapping_add(1i32);
                                j = 0i32;
                                while j <= __for_end_7 {
                                    {
                                        z = (self.trie[((self.cur_lang).wrapping_add(1i32)) as usize].rh()).wrapping_add(self.hc[(j) as usize]);
                                        l = j;
                                        while (self.hc[(l) as usize] == (self.trie[(z) as usize].b1()).wrapping_sub(0i32)) {
                                            {
                                                if (self.trie[(z) as usize].b0() != 0i32) {
                                                    // §924
                                                    {
                                                        v = self.trie[(z) as usize].b0();
                                                        loop {
                                                            v = (v).wrapping_add(self.op_start[(self.cur_lang) as usize]);
                                                            i = (l).wrapping_sub(self.hyf_distance[((v) - 1) as usize]);
                                                            if (self.hyf_num[((v) - 1) as usize] > self.hyf[(i) as usize]) {
                                                                { let __v612 = self.hyf_num[((v) - 1) as usize]; self.hyf[(i) as usize] = __v612; }
                                                            }
                                                            v = self.hyf_next[((v) - 1) as usize];
                                                            if (v == 0i32) { break; }
                                                        }
                                                    }
                                                }
                                                // §923
                                                l = (l).wrapping_add(1i32);
                                                z = (self.trie[(z) as usize].rh()).wrapping_add(self.hc[(l) as usize]);
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
                                self.hyf[(j) as usize] = 0i32;
                                j = j.wrapping_add(1);
                            }
                        }
                        {
                            let __for_end_6 = (self.r_hyf).wrapping_sub(1i32);
                            j = 0i32;
                            while j <= __for_end_6 {
                                self.hyf[((self.hn).wrapping_sub(j)) as usize] = 0i32;
                                j = j.wrapping_add(1);
                            }
                        }
                        // §902
                        {
                            let __for_end_6 = (self.hn).wrapping_sub(self.r_hyf);
                            j = self.l_hyf;
                            while j <= __for_end_6 {
                                if (((self.hyf[(j) as usize]) % 2) != 0) {
                                    break 'l_found1_f;
                                }
                                j = j.wrapping_add(1);
                            }
                        }
                        break 'l_exit_f;
                    }
                    // §903
                    q = self.mem[(self.hb) as usize].hh().rh();
                    { let __ix613 = self.hb; self.mem[(__ix613) as usize].set_hh_rh(0i32); }
                    r = self.mem[(self.ha) as usize].hh().rh();
                    { let __ix614 = self.ha; self.mem[(__ix614) as usize].set_hh_rh(0i32); }
                    bchar = self.hyf_bchar;
                    if (self.ha >= self.hi_mem_min) {
                        if (self.mem[(self.ha) as usize].hh().b0() != self.hf) {
                            break 'l_found2_f;
                        } else {
                            {
                                self.init_list = self.ha;
                                self.init_lig = false;
                                { let __v615 = (self.mem[(self.ha) as usize].hh().b1()).wrapping_sub(0i32); self.hu[(0i32) as usize] = __v615; }
                            }
                        }
                    } else {
                        if (self.mem[(self.ha) as usize].hh().b0() == 6i32) {
                            if (self.mem[((self.ha).wrapping_add(1i32)) as usize].hh().b0() != self.hf) {
                                break 'l_found2_f;
                            } else {
                                {
                                    self.init_list = self.mem[((self.ha).wrapping_add(1i32)) as usize].hh().rh();
                                    self.init_lig = true;
                                    self.init_lft = (self.mem[(self.ha) as usize].hh().b1() > 1i32);
                                    { let __v616 = (self.mem[((self.ha).wrapping_add(1i32)) as usize].hh().b1()).wrapping_sub(0i32); self.hu[(0i32) as usize] = __v616; }
                                    if (self.init_list == 0i32) {
                                        if self.init_lft {
                                            {
                                                self.hu[(0i32) as usize] = 256i32;
                                                self.init_lig = false;
                                            }
                                        }
                                    }
                                    self.free_node(self.ha, 2i32);
                                }
                            }
                        } else {
                            {
                                if (!(r >= self.hi_mem_min)) {
                                    if (self.mem[(r) as usize].hh().b0() == 6i32) {
                                        if (self.mem[(r) as usize].hh().b1() > 1i32) {
                                            break 'l_found2_f;
                                        }
                                    }
                                }
                                j = 1i32;
                                s = self.ha;
                                self.init_list = 0i32;
                                break 'l_common_ending_f;
                            }
                        }
                    }
                    s = self.cur_p;
                    while (self.mem[(s) as usize].hh().rh() != self.ha) {
                        s = self.mem[(s) as usize].hh().rh();
                    }
                    j = 0i32;
                    break 'l_common_ending_f;
                }
                s = self.ha;
                j = 0i32;
                self.hu[(0i32) as usize] = 256i32;
                self.init_lig = false;
                self.init_list = 0i32;
            }
            self.flush_node_list(r);
            // §913
            loop {
                l = j;
                j = (self.reconstitute(j, self.hn, bchar, (self.hyf_char).wrapping_add(0i32))).wrapping_add(1i32);
                if (self.hyphen_passed == 0i32) {
                    {
                        { let __v617 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(s) as usize].set_hh_rh(__v617); }
                        while (self.mem[(s) as usize].hh().rh() > 0i32) {
                            s = self.mem[(s) as usize].hh().rh();
                        }
                        if (((self.hyf[((j).wrapping_sub(1i32)) as usize]) % 2) != 0) {
                            {
                                l = j;
                                self.hyphen_passed = (j).wrapping_sub(1i32);
                                self.mem[(4999995i32) as usize].set_hh_rh(0i32);
                            }
                        }
                    }
                }
                if (self.hyphen_passed > 0i32) {
                    // §914
                    loop {
                        r = self.get_node(2i32);
                        { let __v618 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(r) as usize].set_hh_rh(__v618); }
                        self.mem[(r) as usize].set_hh_b0(7i32);
                        major_tail = r;
                        r_count = 0i32;
                        while (self.mem[(major_tail) as usize].hh().rh() > 0i32) {
                            {
                                major_tail = self.mem[(major_tail) as usize].hh().rh();
                                r_count = (r_count).wrapping_add(1i32);
                            }
                        }
                        i = self.hyphen_passed;
                        self.hyf[(i) as usize] = 0i32;
                        // §915
                        minor_tail = 0i32;
                        self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
                        hyf_node = self.new_character(self.hf, self.hyf_char);
                        if (hyf_node != 0i32) {
                            {
                                i = (i).wrapping_add(1i32);
                                c = self.hu[(i) as usize];
                                { let __v619 = self.hyf_char; self.hu[(i) as usize] = __v619; }
                                {
                                    { let __v620 = self.avail; self.mem[(hyf_node) as usize].set_hh_rh(__v620); }
                                    self.avail = hyf_node;
                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                }
                            }
                        }
                        while (l <= i) {
                            {
                                l = (self.reconstitute(l, i, self.font_bchar[(self.hf) as usize], 256i32)).wrapping_add(1i32);
                                if (self.mem[(4999995i32) as usize].hh().rh() > 0i32) {
                                    {
                                        if (minor_tail == 0i32) {
                                            { let __v621 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(__v621); }
                                        } else {
                                            { let __v622 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(minor_tail) as usize].set_hh_rh(__v622); }
                                        }
                                        minor_tail = self.mem[(4999995i32) as usize].hh().rh();
                                        while (self.mem[(minor_tail) as usize].hh().rh() > 0i32) {
                                            minor_tail = self.mem[(minor_tail) as usize].hh().rh();
                                        }
                                    }
                                }
                            }
                        }
                        if (hyf_node != 0i32) {
                            {
                                self.hu[(i) as usize] = c;
                                l = i;
                                i = (i).wrapping_sub(1i32);
                            }
                        }
                        // §916
                        minor_tail = 0i32;
                        self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
                        c_loc = 0i32;
                        if (self.bchar_label[(self.hf) as usize] != 0i32) {
                            {
                                l = (l).wrapping_sub(1i32);
                                c = self.hu[(l) as usize];
                                c_loc = l;
                                self.hu[(l) as usize] = 256i32;
                            }
                        }
                        while (l < j) {
                            {
                                loop {
                                    l = (self.reconstitute(l, self.hn, bchar, 256i32)).wrapping_add(1i32);
                                    if (c_loc > 0i32) {
                                        {
                                            self.hu[(c_loc) as usize] = c;
                                            c_loc = 0i32;
                                        }
                                    }
                                    if (self.mem[(4999995i32) as usize].hh().rh() > 0i32) {
                                        {
                                            if (minor_tail == 0i32) {
                                                { let __v623 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(__v623); }
                                            } else {
                                                { let __v624 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(minor_tail) as usize].set_hh_rh(__v624); }
                                            }
                                            minor_tail = self.mem[(4999995i32) as usize].hh().rh();
                                            while (self.mem[(minor_tail) as usize].hh().rh() > 0i32) {
                                                minor_tail = self.mem[(minor_tail) as usize].hh().rh();
                                            }
                                        }
                                    }
                                    if (l >= j) { break; }
                                }
                                while (l > j) {
                                    // §917
                                    {
                                        j = (self.reconstitute(j, self.hn, bchar, 256i32)).wrapping_add(1i32);
                                        { let __v625 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(major_tail) as usize].set_hh_rh(__v625); }
                                        while (self.mem[(major_tail) as usize].hh().rh() > 0i32) {
                                            {
                                                major_tail = self.mem[(major_tail) as usize].hh().rh();
                                                r_count = (r_count).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §918
                        if (r_count > 127i32) {
                            {
                                { let __v626 = self.mem[(r) as usize].hh().rh(); self.mem[(s) as usize].set_hh_rh(__v626); }
                                self.mem[(r) as usize].set_hh_rh(0i32);
                                self.flush_node_list(r);
                            }
                        } else {
                            {
                                self.mem[(s) as usize].set_hh_rh(r);
                                self.mem[(r) as usize].set_hh_b1(r_count);
                            }
                        }
                        s = major_tail;
                        // §914
                        self.hyphen_passed = (j).wrapping_sub(1i32);
                        self.mem[(4999995i32) as usize].set_hh_rh(0i32);
                        if (!(((self.hyf[((j).wrapping_sub(1i32)) as usize]) % 2) != 0)) { break; }
                    }
                }
                if (j > self.hn) { break; }
            }
            // §913
            self.mem[(s) as usize].set_hh_rh(q);
            // §903
            self.flush_list(self.init_list);
        }
        // §895
    }

    /// It's tempting to remove the `overflow` stops in the following procedure;
    /// `new_trie_op` could return `min_quarterword` (thereby simply ignoring
    /// part of a hyphenation pattern) instead of aborting the job. However, that would
    /// lead to different hyphenation results on different installations of \TeX\
    /// using the same patterns. The `overflow` stops are necessary for portability
    /// of patterns.
    /// @<Declare procedures for preprocessing hyph...
    // §944
    pub fn new_trie_op(&mut self, mut d: small_number, mut n: small_number, mut v: quarterword) -> quarterword {
        let mut new_trie_op: quarterword = 0;
        let mut h: i32 = 0; // §944
        let mut u: quarterword = 0; // §944
        let mut l: i32 = 0; // §944
        'l_exit_f: {
            h = ((((((n).wrapping_add((313i32).wrapping_mul(d))).wrapping_add((361i32).wrapping_mul(v))).wrapping_add((1009i32).wrapping_mul(self.cur_lang))).wrapping_abs() % (trie_op_size).wrapping_add(trie_op_size))).wrapping_sub(trie_op_size);
            while true {
                {
                    l = self.trie_op_hash[((h) + 35111) as usize];
                    if (l == 0i32) {
                        {
                            if (self.trie_op_ptr == trie_op_size) {
                                self.overflow(949i32, trie_op_size);
                            }
                            u = self.trie_used[(self.cur_lang) as usize];
                            if (u == 255i32) {
                                self.overflow(950i32, 255i32);
                            }
                            self.trie_op_ptr = (self.trie_op_ptr).wrapping_add(1i32);
                            u = (u).wrapping_add(1i32);
                            self.trie_used[(self.cur_lang) as usize] = u;
                            self.hyf_distance[((self.trie_op_ptr) - 1) as usize] = d;
                            self.hyf_num[((self.trie_op_ptr) - 1) as usize] = n;
                            self.hyf_next[((self.trie_op_ptr) - 1) as usize] = v;
                            { let __ix627 = self.trie_op_ptr; let __v628 = self.cur_lang; self.trie_op_lang[((__ix627) - 1) as usize] = __v628; }
                            { let __v629 = self.trie_op_ptr; self.trie_op_hash[((h) + 35111) as usize] = __v629; }
                            self.trie_op_val[((self.trie_op_ptr) - 1) as usize] = u;
                            new_trie_op = u;
                            break 'l_exit_f;
                        }
                    }
                    if ((((self.hyf_distance[((l) - 1) as usize] == d) && (self.hyf_num[((l) - 1) as usize] == n)) && (self.hyf_next[((l) - 1) as usize] == v)) && (self.trie_op_lang[((l) - 1) as usize] == self.cur_lang)) {
                        {
                            new_trie_op = self.trie_op_val[((l) - 1) as usize];
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
    // §948
    pub fn trie_node(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut trie_node: trie_pointer = 0;
        let mut h: trie_pointer = 0; // §948
        let mut q: trie_pointer = 0; // §948
        'l_exit_f: {
            h = (((((self.trie_c[(p) as usize]).wrapping_add((1009i32).wrapping_mul(self.trie_o[(p) as usize]))).wrapping_add((2718i32).wrapping_mul(self.trie_l[(p) as usize]))).wrapping_add((3142i32).wrapping_mul(self.trie_r[(p) as usize]))).wrapping_abs() % trie_size);
            while true {
                {
                    q = self.trie_hash[(h) as usize];
                    if (q == 0i32) {
                        {
                            self.trie_hash[(h) as usize] = p;
                            trie_node = p;
                            break 'l_exit_f;
                        }
                    }
                    if ((((self.trie_c[(q) as usize] == self.trie_c[(p) as usize]) && (self.trie_o[(q) as usize] == self.trie_o[(p) as usize])) && (self.trie_l[(q) as usize] == self.trie_l[(p) as usize])) && (self.trie_r[(q) as usize] == self.trie_r[(p) as usize])) {
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
    // §949
    pub fn compress_trie(&mut self, mut p: trie_pointer) -> trie_pointer {
        let mut compress_trie: trie_pointer = 0;
        if (p == 0i32) {
            compress_trie = 0i32;
        } else {
            {
                { let __v630 = self.compress_trie(self.trie_l[(p) as usize]); self.trie_l[(p) as usize] = __v630; }
                { let __v631 = self.compress_trie(self.trie_r[(p) as usize]); self.trie_r[(p) as usize] = __v631; }
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
    // §953
    pub fn first_fit(&mut self, mut p: trie_pointer) {
        let mut h: trie_pointer = 0; // §953
        let mut z: trie_pointer = 0; // §953
        let mut q: trie_pointer = 0; // §953
        let mut c: ASCII_code = 0; // §953
        let mut l: trie_pointer = 0; // §953
        let mut r: trie_pointer = 0; // §953
        let mut ll: i32 = 0; // §953
        'l_found_f: {
            c = self.trie_c[(p) as usize];
            z = self.trie_min[(c) as usize];
            while true {
                {
                    'l_not_found_f: {
                        h = (z).wrapping_sub(c);
                        // §954
                        if (self.trie_max < (h).wrapping_add(256i32)) {
                            {
                                if (trie_size <= (h).wrapping_add(256i32)) {
                                    self.overflow(951i32, trie_size);
                                }
                                loop {
                                    self.trie_max = (self.trie_max).wrapping_add(1i32);
                                    { let __ix632 = self.trie_max; let __v633 = false; self.trie_taken[((__ix632) - 1) as usize] = __v633; }
                                    { let __ix634 = self.trie_max; let __v635 = (self.trie_max).wrapping_add(1i32); self.trie[(__ix634) as usize].set_rh(__v635); }
                                    { let __ix636 = self.trie_max; let __v637 = (self.trie_max).wrapping_sub(1i32); self.trie[(__ix636) as usize].set_lh(__v637); }
                                    if (self.trie_max == (h).wrapping_add(256i32)) { break; }
                                }
                            }
                        }
                        // §953
                        if self.trie_taken[((h) - 1) as usize] {
                            break 'l_not_found_f;
                        }
                        // §955
                        q = self.trie_r[(p) as usize];
                        while (q > 0i32) {
                            {
                                if (self.trie[((h).wrapping_add(self.trie_c[(q) as usize])) as usize].rh() == 0i32) {
                                    break 'l_not_found_f;
                                }
                                q = self.trie_r[(q) as usize];
                            }
                        }
                        break 'l_found_f;
                    }
                    // §953
                    z = self.trie[(z) as usize].rh();
                }
            }
        }
        { let __v638 = true; self.trie_taken[((h) - 1) as usize] = __v638; }
        // §956
        self.trie_hash[(p) as usize] = h;
        q = p;
        loop {
            z = (h).wrapping_add(self.trie_c[(q) as usize]);
            l = self.trie[(z) as usize].lh();
            r = self.trie[(z) as usize].rh();
            self.trie[(r) as usize].set_lh(l);
            self.trie[(l) as usize].set_rh(r);
            self.trie[(z) as usize].set_rh(0i32);
            if (l < 256i32) {
                {
                    if (z < 256i32) {
                        ll = z;
                    } else {
                        ll = 256i32;
                    }
                    loop {
                        self.trie_min[(l) as usize] = r;
                        l = (l).wrapping_add(1i32);
                        if (l == ll) { break; }
                    }
                }
            }
            q = self.trie_r[(q) as usize];
            if (q == 0i32) { break; }
        }
    }

    /// To pack the entire linked trie, we use the following recursive procedure.
    /// @<Declare procedures for preprocessing hyph...
    // §957
    pub fn trie_pack(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §957
        loop {
            q = self.trie_l[(p) as usize];
            if ((q > 0i32) && (self.trie_hash[(q) as usize] == 0i32)) {
                {
                    self.first_fit(q);
                    self.trie_pack(q);
                }
            }
            p = self.trie_r[(p) as usize];
            if (p == 0i32) { break; }
        }
    }

    /// The fixing-up procedure is, of course, recursive. Since the linked trie
    /// usually has overlapping subtries, the same data may be moved several
    /// times; but that causes no harm, and at most as much work is done as it
    /// took to build the uncompressed trie.
    /// @<Declare procedures for preprocessing hyph...
    // §959
    pub fn trie_fix(&mut self, mut p: trie_pointer) {
        let mut q: trie_pointer = 0; // §959
        let mut c: ASCII_code = 0; // §959
        let mut z: trie_pointer = 0; // §959
        z = self.trie_hash[(p) as usize];
        loop {
            q = self.trie_l[(p) as usize];
            c = self.trie_c[(p) as usize];
            { let __v639 = self.trie_hash[(q) as usize]; self.trie[((z).wrapping_add(c)) as usize].set_rh(__v639); }
            self.trie[((z).wrapping_add(c)) as usize].set_b1((c).wrapping_add(0i32));
            { let __v640 = self.trie_o[(p) as usize]; self.trie[((z).wrapping_add(c)) as usize].set_b0(__v640); }
            if (q > 0i32) {
                self.trie_fix(q);
            }
            p = self.trie_r[(p) as usize];
            if (p == 0i32) { break; }
        }
    }

    /// Now let's go back to the easier problem, of building the linked
    /// trie.  When \.{INITEX} has scanned the `\.{\\patterns}' control
    /// sequence, it calls on `new_patterns` to do the right thing.
    /// @<Declare procedures for preprocessing hyph...
    // §960
    pub fn new_patterns(&mut self) {
        let mut k: i32 = 0; // §960
        let mut l: i32 = 0; // §960
        let mut digit_sensed: bool = false; // §960
        let mut v: quarterword = 0; // §960
        let mut p: trie_pointer = 0; // §960
        let mut q: trie_pointer = 0; // §960
        let mut first_child: bool = false; // §960
        let mut c: ASCII_code = 0; // §960
        if self.trie_not_ready {
            {
                'l_done_f: {
                    if (self.eqtb[((618213i32) - 1) as usize].int() <= 0i32) {
                        self.cur_lang = 0i32;
                    } else {
                        if (self.eqtb[((618213i32) - 1) as usize].int() > 255i32) {
                            self.cur_lang = 0i32;
                        } else {
                            self.cur_lang = self.eqtb[((618213i32) - 1) as usize].int();
                        }
                    }
                    self.scan_left_brace();
                    // §961
                    k = 0i32;
                    self.hyf[(0i32) as usize] = 0i32;
                    digit_sensed = false;
                    while true {
                        {
                            self.get_x_token();
                            match self.cur_cmd {
                                11 | 12 => {
                                    // §962
                                    if ((digit_sensed || (self.cur_chr < 48i32)) || (self.cur_chr > 57i32)) {
                                        {
                                            if (self.cur_chr == 46i32) {
                                                self.cur_chr = 0i32;
                                            } else {
                                                {
                                                    self.cur_chr = self.eqtb[(((617139i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                                                    if (self.cur_chr == 0i32) {
                                                        {
                                                            {
                                                                if (self.interaction == 3i32) {
                                                                }
                                                                self.print_nl(262i32);
                                                                self.print(957i32);
                                                            }
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[(0i32) as usize] = 956i32;
                                                            }
                                                            self.error();
                                                        }
                                                    }
                                                }
                                            }
                                            if (k < 63i32) {
                                                {
                                                    k = (k).wrapping_add(1i32);
                                                    { let __v641 = self.cur_chr; self.hc[(k) as usize] = __v641; }
                                                    self.hyf[(k) as usize] = 0i32;
                                                    digit_sensed = false;
                                                }
                                            }
                                        }
                                    } else {
                                        if (k < 63i32) {
                                            {
                                                { let __v642 = (self.cur_chr).wrapping_sub(48i32); self.hyf[(k) as usize] = __v642; }
                                                digit_sensed = true;
                                            }
                                        }
                                    }
                                }
                                10 | 2 => {
                                    // §961
                                    {
                                        if (k > 0i32) {
                                            // §963
                                            {
                                                'l_done1_f: {
                                                    // §965
                                                    if (self.hc[(1i32) as usize] == 0i32) {
                                                        self.hyf[(0i32) as usize] = 0i32;
                                                    }
                                                    if (self.hc[(k) as usize] == 0i32) {
                                                        self.hyf[(k) as usize] = 0i32;
                                                    }
                                                    l = k;
                                                    v = 0i32;
                                                    while true {
                                                        {
                                                            if (self.hyf[(l) as usize] != 0i32) {
                                                                v = self.new_trie_op((k).wrapping_sub(l), self.hyf[(l) as usize], v);
                                                            }
                                                            if (l > 0i32) {
                                                                l = (l).wrapping_sub(1i32);
                                                            } else {
                                                                break 'l_done1_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §963
                                                q = 0i32;
                                                { let __v643 = self.cur_lang; self.hc[(0i32) as usize] = __v643; }
                                                while (l <= k) {
                                                    {
                                                        c = self.hc[(l) as usize];
                                                        l = (l).wrapping_add(1i32);
                                                        p = self.trie_l[(q) as usize];
                                                        first_child = true;
                                                        while ((p > 0i32) && (c > self.trie_c[(p) as usize])) {
                                                            {
                                                                q = p;
                                                                p = self.trie_r[(q) as usize];
                                                                first_child = false;
                                                            }
                                                        }
                                                        if ((p == 0i32) || (c < self.trie_c[(p) as usize])) {
                                                            // §964
                                                            {
                                                                if (self.trie_ptr == trie_size) {
                                                                    self.overflow(951i32, trie_size);
                                                                }
                                                                self.trie_ptr = (self.trie_ptr).wrapping_add(1i32);
                                                                self.trie_r[(self.trie_ptr) as usize] = p;
                                                                p = self.trie_ptr;
                                                                self.trie_l[(p) as usize] = 0i32;
                                                                if first_child {
                                                                    self.trie_l[(q) as usize] = p;
                                                                } else {
                                                                    self.trie_r[(q) as usize] = p;
                                                                }
                                                                self.trie_c[(p) as usize] = c;
                                                                self.trie_o[(p) as usize] = 0i32;
                                                            }
                                                        }
                                                        // §963
                                                        q = p;
                                                    }
                                                }
                                                if (self.trie_o[(q) as usize] != 0i32) {
                                                    {
                                                        {
                                                            if (self.interaction == 3i32) {
                                                            }
                                                            self.print_nl(262i32);
                                                            self.print(958i32);
                                                        }
                                                        {
                                                            self.help_ptr = 1i32;
                                                            self.help_line[(0i32) as usize] = 956i32;
                                                        }
                                                        self.error();
                                                    }
                                                }
                                                self.trie_o[(q) as usize] = v;
                                            }
                                        }
                                        // §961
                                        if (self.cur_cmd == 2i32) {
                                            break 'l_done_f;
                                        }
                                        k = 0i32;
                                        self.hyf[(0i32) as usize] = 0i32;
                                        digit_sensed = false;
                                    }
                                }
                                _ => {
                                    {
                                        {
                                            if (self.interaction == 3i32) {
                                            }
                                            self.print_nl(262i32);
                                            self.print(955i32);
                                        }
                                        self.print_esc(953i32);
                                        {
                                            self.help_ptr = 1i32;
                                            self.help_line[(0i32) as usize] = 956i32;
                                        }
                                        self.error();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // §960
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(952i32);
                }
                self.print_esc(953i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 954i32;
                }
                self.error();
                { let __v644 = self.scan_toks(false, false); self.mem[(4999987i32) as usize].set_hh_rh(__v644); }
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
    // §966
    pub fn init_trie(&mut self) {
        let mut p: trie_pointer = 0; // §966
        let mut j: i32 = 0; // §966
        let mut k: i32 = 0; // §966
        let mut t: i32 = 0; // §966
        let mut r: trie_pointer = 0; // §966
        let mut s: trie_pointer = 0; // §966
        let mut h: two_halves = two_halves::default(); // §966
        // §945
        self.op_start[(0i32) as usize] = (0i32).wrapping_neg();
        {
            let __for_end_2 = 255i32;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v645 = ((self.op_start[((j).wrapping_sub(1i32)) as usize]).wrapping_add(self.trie_used[((j).wrapping_sub(1i32)) as usize])).wrapping_sub(0i32); self.op_start[(j) as usize] = __v645; }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            j = 1i32;
            while j <= __for_end_2 {
                { let __v646 = (self.op_start[(self.trie_op_lang[((j) - 1) as usize]) as usize]).wrapping_add(self.trie_op_val[((j) - 1) as usize]); self.trie_op_hash[((j) + 35111) as usize] = __v646; }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            j = 1i32;
            while j <= __for_end_2 {
                while (self.trie_op_hash[((j) + 35111) as usize] > j) {
                    {
                        k = self.trie_op_hash[((j) + 35111) as usize];
                        t = self.hyf_distance[((k) - 1) as usize];
                        { let __v647 = self.hyf_distance[((j) - 1) as usize]; self.hyf_distance[((k) - 1) as usize] = __v647; }
                        self.hyf_distance[((j) - 1) as usize] = t;
                        t = self.hyf_num[((k) - 1) as usize];
                        { let __v648 = self.hyf_num[((j) - 1) as usize]; self.hyf_num[((k) - 1) as usize] = __v648; }
                        self.hyf_num[((j) - 1) as usize] = t;
                        t = self.hyf_next[((k) - 1) as usize];
                        { let __v649 = self.hyf_next[((j) - 1) as usize]; self.hyf_next[((k) - 1) as usize] = __v649; }
                        self.hyf_next[((j) - 1) as usize] = t;
                        { let __v650 = self.trie_op_hash[((k) + 35111) as usize]; self.trie_op_hash[((j) + 35111) as usize] = __v650; }
                        self.trie_op_hash[((k) + 35111) as usize] = k;
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        // §952
        {
            let __for_end_2 = trie_size;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[(p) as usize] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        { let __v651 = self.compress_trie(self.trie_l[(0i32) as usize]); self.trie_l[(0i32) as usize] = __v651; }
        {
            let __for_end_2 = self.trie_ptr;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_hash[(p) as usize] = 0i32;
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            p = 0i32;
            while p <= __for_end_2 {
                self.trie_min[(p) as usize] = (p).wrapping_add(1i32);
                p = p.wrapping_add(1);
            }
        }
        self.trie[(0i32) as usize].set_rh(1i32);
        self.trie_max = 0i32;
        // §966
        if (self.trie_l[(0i32) as usize] != 0i32) {
            {
                self.first_fit(self.trie_l[(0i32) as usize]);
                self.trie_pack(self.trie_l[(0i32) as usize]);
            }
        }
        // §958
        h.set_rh(0i32);
        h.set_b0(0i32);
        h.set_b1(0i32);
        if (self.trie_l[(0i32) as usize] == 0i32) {
            {
                {
                    let __for_end_4 = 256i32;
                    r = 0i32;
                    while r <= __for_end_4 {
                        self.trie[(r) as usize] = h;
                        r = r.wrapping_add(1);
                    }
                }
                self.trie_max = 256i32;
            }
        } else {
            {
                self.trie_fix(self.trie_l[(0i32) as usize]);
                r = 0i32;
                loop {
                    s = self.trie[(r) as usize].rh();
                    self.trie[(r) as usize] = h;
                    r = s;
                    if (r > self.trie_max) { break; }
                }
            }
        }
        self.trie[(0i32) as usize].set_b1(63i32);
        // §966
        self.trie_not_ready = false;
    }

    /// Since `line_break` is a rather lengthy procedure---sort of a small world unto
    /// itself---we must build it up little by little, somewhat more cautiously
    /// than we have done with the simpler procedures of \TeX. Here is the
    /// general outline.
    // §815
    pub fn line_break(&mut self, mut final_widow_penalty: i32) {
        let mut auto_breaking: bool = false; // §862
        let mut prev_p: halfword = 0; // §862
        let mut q: halfword = 0; // §862
        let mut r: halfword = 0; // §862
        let mut s: halfword = 0; // §862
        let mut prev_s: halfword = 0; // §862
        let mut f: internal_font_number = 0; // §862
        let mut j: small_number = 0; // §893
        let mut c: i32 = 0; // §893
        'l_done_f: {
            self.pack_begin_line = self.cur_list.ml_field;
            // §816
            { let __v652 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(4999996i32) as usize].set_hh_rh(__v652); }
            if (self.cur_list.tail_field >= self.hi_mem_min) {
                {
                    { let __ix653 = self.cur_list.tail_field; let __v654 = self.new_penalty(10000i32); self.mem[(__ix653) as usize].set_hh_rh(__v654); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
            } else {
                if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() != 10i32) {
                    {
                        { let __ix655 = self.cur_list.tail_field; let __v656 = self.new_penalty(10000i32); self.mem[(__ix655) as usize].set_hh_rh(__v656); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                } else {
                    {
                        { let __ix657 = self.cur_list.tail_field; self.mem[(__ix657) as usize].set_hh_b0(12i32); }
                        self.delete_glue_ref(self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().lh());
                        self.flush_node_list(self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().rh());
                        { let __ix658 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix658) as usize].set_int(10000i32); }
                    }
                }
            }
            { let __ix659 = self.cur_list.tail_field; let __v660 = self.new_param_glue(14i32); self.mem[(__ix659) as usize].set_hh_rh(__v660); }
            self.init_cur_lang = (self.cur_list.pg_field % 65536i32);
            self.init_l_hyf = (self.cur_list.pg_field / 4194304i32);
            self.init_r_hyf = ((self.cur_list.pg_field / 65536i32) % 64i32);
            self.pop_nest();
            // §827
            self.no_shrink_error_yet = true;
            if ((self.mem[(self.eqtb[((615789i32) - 1) as usize].hh().rh()) as usize].hh().b1() != 0i32) && (self.mem[((self.eqtb[((615789i32) - 1) as usize].hh().rh()).wrapping_add(3i32)) as usize].int() != 0i32)) {
                {
                    { let __v661 = self.finite_shrink(self.eqtb[((615789i32) - 1) as usize].hh().rh()); self.eqtb[((615789i32) - 1) as usize].set_hh_rh(__v661); }
                }
            }
            if ((self.mem[(self.eqtb[((615790i32) - 1) as usize].hh().rh()) as usize].hh().b1() != 0i32) && (self.mem[((self.eqtb[((615790i32) - 1) as usize].hh().rh()).wrapping_add(3i32)) as usize].int() != 0i32)) {
                {
                    { let __v662 = self.finite_shrink(self.eqtb[((615790i32) - 1) as usize].hh().rh()); self.eqtb[((615790i32) - 1) as usize].set_hh_rh(__v662); }
                }
            }
            q = self.eqtb[((615789i32) - 1) as usize].hh().rh();
            r = self.eqtb[((615790i32) - 1) as usize].hh().rh();
            { let __v663 = (self.mem[((q).wrapping_add(1i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.background[((1i32) - 1) as usize] = __v663; }
            self.background[((2i32) - 1) as usize] = 0i32;
            self.background[((3i32) - 1) as usize] = 0i32;
            self.background[((4i32) - 1) as usize] = 0i32;
            self.background[((5i32) - 1) as usize] = 0i32;
            { let __ix664 = (2i32).wrapping_add(self.mem[(q) as usize].hh().b0()); let __v665 = self.mem[((q).wrapping_add(2i32)) as usize].int(); self.background[((__ix664) - 1) as usize] = __v665; }
            { let __ix666 = (2i32).wrapping_add(self.mem[(r) as usize].hh().b0()); let __v667 = (self.background[(((2i32).wrapping_add(self.mem[(r) as usize].hh().b0())) - 1) as usize]).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.background[((__ix666) - 1) as usize] = __v667; }
            { let __v668 = (self.mem[((q).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.background[((6i32) - 1) as usize] = __v668; }
            // §834
            self.minimum_demerits = 1073741823i32;
            self.minimal_demerits[(3i32) as usize] = 1073741823i32;
            self.minimal_demerits[(2i32) as usize] = 1073741823i32;
            self.minimal_demerits[(1i32) as usize] = 1073741823i32;
            self.minimal_demerits[(0i32) as usize] = 1073741823i32;
            // §848
            if (self.eqtb[((616312i32) - 1) as usize].hh().rh() == 0i32) {
                if (self.eqtb[((618747i32) - 1) as usize].int() == 0i32) {
                    {
                        self.last_special_line = 0i32;
                        self.second_width = self.eqtb[((618733i32) - 1) as usize].int();
                        self.second_indent = 0i32;
                    }
                } else {
                    // §849
                    {
                        self.last_special_line = (self.eqtb[((618204i32) - 1) as usize].int()).wrapping_abs();
                        if (self.eqtb[((618204i32) - 1) as usize].int() < 0i32) {
                            {
                                self.first_width = (self.eqtb[((618733i32) - 1) as usize].int()).wrapping_sub((self.eqtb[((618747i32) - 1) as usize].int()).wrapping_abs());
                                if (self.eqtb[((618747i32) - 1) as usize].int() >= 0i32) {
                                    self.first_indent = self.eqtb[((618747i32) - 1) as usize].int();
                                } else {
                                    self.first_indent = 0i32;
                                }
                                self.second_width = self.eqtb[((618733i32) - 1) as usize].int();
                                self.second_indent = 0i32;
                            }
                        } else {
                            {
                                self.first_width = self.eqtb[((618733i32) - 1) as usize].int();
                                self.first_indent = 0i32;
                                self.second_width = (self.eqtb[((618733i32) - 1) as usize].int()).wrapping_sub((self.eqtb[((618747i32) - 1) as usize].int()).wrapping_abs());
                                if (self.eqtb[((618747i32) - 1) as usize].int() >= 0i32) {
                                    self.second_indent = self.eqtb[((618747i32) - 1) as usize].int();
                                } else {
                                    self.second_indent = 0i32;
                                }
                            }
                        }
                    }
                }
            } else {
                // §848
                {
                    self.last_special_line = (self.mem[(self.eqtb[((616312i32) - 1) as usize].hh().rh()) as usize].hh().lh()).wrapping_sub(1i32);
                    self.second_width = self.mem[((self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul((self.last_special_line).wrapping_add(1i32)))) as usize].int();
                    self.second_indent = self.mem[(((self.eqtb[((616312i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(self.last_special_line))).wrapping_add(1i32)) as usize].int();
                }
            }
            if (self.eqtb[((618182i32) - 1) as usize].int() == 0i32) {
                self.easy_line = self.last_special_line;
            } else {
                self.easy_line = 268435455i32;
            }
            // §863
            self.threshold = self.eqtb[((618163i32) - 1) as usize].int();
            if (self.threshold >= 0i32) {
                {
                    if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(933i32);
                        }
                    }
                    self.second_pass = false;
                    self.final_pass = false;
                }
            } else {
                {
                    self.threshold = self.eqtb[((618164i32) - 1) as usize].int();
                    self.second_pass = true;
                    self.final_pass = (self.eqtb[((618750i32) - 1) as usize].int() <= 0i32);
                    if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                        self.begin_diagnostic();
                    }
                }
            }
            while true {
                {
                    if (self.threshold > 10000i32) {
                        self.threshold = 10000i32;
                    }
                    if self.second_pass {
                        // §891
                        {
                            if self.trie_not_ready {
                                self.init_trie();
                            }
                            self.cur_lang = self.init_cur_lang;
                            self.l_hyf = self.init_l_hyf;
                            self.r_hyf = self.init_r_hyf;
                        }
                    }
                    // §864
                    q = self.get_node(3i32);
                    self.mem[(q) as usize].set_hh_b0(0i32);
                    self.mem[(q) as usize].set_hh_b1(2i32);
                    self.mem[(q) as usize].set_hh_rh(4999992i32);
                    self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
                    { let __v669 = (self.cur_list.pg_field).wrapping_add(1i32); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v669); }
                    self.mem[((q).wrapping_add(2i32)) as usize].set_int(0i32);
                    self.mem[(4999992i32) as usize].set_hh_rh(q);
                    { let __v670 = self.background[((1i32) - 1) as usize]; self.active_width[((1i32) - 1) as usize] = __v670; }
                    { let __v671 = self.background[((2i32) - 1) as usize]; self.active_width[((2i32) - 1) as usize] = __v671; }
                    { let __v672 = self.background[((3i32) - 1) as usize]; self.active_width[((3i32) - 1) as usize] = __v672; }
                    { let __v673 = self.background[((4i32) - 1) as usize]; self.active_width[((4i32) - 1) as usize] = __v673; }
                    { let __v674 = self.background[((5i32) - 1) as usize]; self.active_width[((5i32) - 1) as usize] = __v674; }
                    { let __v675 = self.background[((6i32) - 1) as usize]; self.active_width[((6i32) - 1) as usize] = __v675; }
                    self.passive = 0i32;
                    self.printed_node = 4999996i32;
                    self.pass_number = 0i32;
                    self.font_in_short_display = 0i32;
                    // §863
                    self.cur_p = self.mem[(4999996i32) as usize].hh().rh();
                    auto_breaking = true;
                    prev_p = self.cur_p;
                    while ((self.cur_p != 0i32) && (self.mem[(4999992i32) as usize].hh().rh() != 4999992i32)) {
                        // §866
                        {
                            'l_done5_f: {
                                if (self.cur_p >= self.hi_mem_min) {
                                    // §867
                                    {
                                        prev_p = self.cur_p;
                                        loop {
                                            f = self.mem[(self.cur_p) as usize].hh().b0();
                                            { let __v676 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(self.cur_p) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v676; }
                                            self.cur_p = self.mem[(self.cur_p) as usize].hh().rh();
                                            if (!(self.cur_p >= self.hi_mem_min)) { break; }
                                        }
                                    }
                                }
                                // §866
                                match self.mem[(self.cur_p) as usize].hh().b0() {
                                    0 | 1 | 2 => {
                                        { let __v677 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v677; }
                                    }
                                    8 => {
                                        // §1362
                                        if (self.mem[(self.cur_p) as usize].hh().b1() == 4i32) {
                                            {
                                                self.cur_lang = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().rh();
                                                self.l_hyf = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().b0();
                                                self.r_hyf = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().b1();
                                            }
                                        }
                                    }
                                    10 => {
                                        // §866
                                        {
                                            // §868
                                            if auto_breaking {
                                                {
                                                    if (prev_p >= self.hi_mem_min) {
                                                        self.try_break(0i32, 0i32);
                                                    } else {
                                                        if (self.mem[(prev_p) as usize].hh().b0() < 9i32) {
                                                            self.try_break(0i32, 0i32);
                                                        } else {
                                                            if ((self.mem[(prev_p) as usize].hh().b0() == 11i32) && (self.mem[(prev_p) as usize].hh().b1() != 1i32)) {
                                                                self.try_break(0i32, 0i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if ((self.mem[(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().b1() != 0i32) && (self.mem[((self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                                {
                                                    { let __ix678 = (self.cur_p).wrapping_add(1i32); let __v679 = self.finite_shrink(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh()); self.mem[(__ix678) as usize].set_hh_lh(__v679); }
                                                }
                                            }
                                            q = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh();
                                            { let __v680 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((q).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v680; }
                                            { let __ix681 = (2i32).wrapping_add(self.mem[(q) as usize].hh().b0()); let __v682 = (self.active_width[(((2i32).wrapping_add(self.mem[(q) as usize].hh().b0())) - 1) as usize]).wrapping_add(self.mem[((q).wrapping_add(2i32)) as usize].int()); self.active_width[((__ix681) - 1) as usize] = __v682; }
                                            { let __v683 = (self.active_width[((6i32) - 1) as usize]).wrapping_add(self.mem[((q).wrapping_add(3i32)) as usize].int()); self.active_width[((6i32) - 1) as usize] = __v683; }
                                            // §866
                                            if (self.second_pass && auto_breaking) {
                                                // §894
                                                {
                                                    'l_done1_f: {
                                                        prev_s = self.cur_p;
                                                        s = self.mem[(prev_s) as usize].hh().rh();
                                                        if (s != 0i32) {
                                                            {
                                                                'l_done4_f: {
                                                                    'l_done3_f: {
                                                                        'l_done2_f: {
                                                                            // §896
                                                                            while true {
                                                                                {
                                                                                    'l_continue_f: {
                                                                                        if (s >= self.hi_mem_min) {
                                                                                            {
                                                                                                c = (self.mem[(s) as usize].hh().b1()).wrapping_sub(0i32);
                                                                                                self.hf = self.mem[(s) as usize].hh().b0();
                                                                                            }
                                                                                        } else {
                                                                                            if (self.mem[(s) as usize].hh().b0() == 6i32) {
                                                                                                if (self.mem[((s).wrapping_add(1i32)) as usize].hh().rh() == 0i32) {
                                                                                                    break 'l_continue_f;
                                                                                                } else {
                                                                                                    {
                                                                                                        q = self.mem[((s).wrapping_add(1i32)) as usize].hh().rh();
                                                                                                        c = (self.mem[(q) as usize].hh().b1()).wrapping_sub(0i32);
                                                                                                        self.hf = self.mem[(q) as usize].hh().b0();
                                                                                                    }
                                                                                                }
                                                                                            } else {
                                                                                                if ((self.mem[(s) as usize].hh().b0() == 11i32) && (self.mem[(s) as usize].hh().b1() == 0i32)) {
                                                                                                    break 'l_continue_f;
                                                                                                } else {
                                                                                                    if (self.mem[(s) as usize].hh().b0() == 8i32) {
                                                                                                        {
                                                                                                            // §1363
                                                                                                            if (self.mem[(s) as usize].hh().b1() == 4i32) {
                                                                                                                {
                                                                                                                    self.cur_lang = self.mem[((s).wrapping_add(1i32)) as usize].hh().rh();
                                                                                                                    self.l_hyf = self.mem[((s).wrapping_add(1i32)) as usize].hh().b0();
                                                                                                                    self.r_hyf = self.mem[((s).wrapping_add(1i32)) as usize].hh().b1();
                                                                                                                }
                                                                                                            }
                                                                                                            // §896
                                                                                                            break 'l_continue_f;
                                                                                                        }
                                                                                                    } else {
                                                                                                        break 'l_done1_f;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        if (self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh() != 0i32) {
                                                                                            if ((self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh() == c) || (self.eqtb[((618201i32) - 1) as usize].int() > 0i32)) {
                                                                                                break 'l_done2_f;
                                                                                            } else {
                                                                                                break 'l_done1_f;
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    prev_s = s;
                                                                                    s = self.mem[(prev_s) as usize].hh().rh();
                                                                                }
                                                                            }
                                                                        }
                                                                        self.hyf_char = self.hyphen_char[(self.hf) as usize];
                                                                        if (self.hyf_char < 0i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        if (self.hyf_char > 255i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        self.ha = prev_s;
                                                                        // §894
                                                                        if ((self.l_hyf).wrapping_add(self.r_hyf) > 63i32) {
                                                                            break 'l_done1_f;
                                                                        }
                                                                        // §897
                                                                        self.hn = 0i32;
                                                                        while true {
                                                                            {
                                                                                if (s >= self.hi_mem_min) {
                                                                                    {
                                                                                        if (self.mem[(s) as usize].hh().b0() != self.hf) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        self.hyf_bchar = self.mem[(s) as usize].hh().b1();
                                                                                        c = (self.hyf_bchar).wrapping_sub(0i32);
                                                                                        if (self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh() == 0i32) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        if (self.hn == 63i32) {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                        self.hb = s;
                                                                                        self.hn = (self.hn).wrapping_add(1i32);
                                                                                        self.hu[(self.hn) as usize] = c;
                                                                                        { let __ix684 = self.hn; let __v685 = self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh(); self.hc[(__ix684) as usize] = __v685; }
                                                                                        self.hyf_bchar = 256i32;
                                                                                    }
                                                                                } else {
                                                                                    if (self.mem[(s) as usize].hh().b0() == 6i32) {
                                                                                        // §898
                                                                                        {
                                                                                            if (self.mem[((s).wrapping_add(1i32)) as usize].hh().b0() != self.hf) {
                                                                                                break 'l_done3_f;
                                                                                            }
                                                                                            j = self.hn;
                                                                                            q = self.mem[((s).wrapping_add(1i32)) as usize].hh().rh();
                                                                                            if (q > 0i32) {
                                                                                                self.hyf_bchar = self.mem[(q) as usize].hh().b1();
                                                                                            }
                                                                                            while (q > 0i32) {
                                                                                                {
                                                                                                    c = (self.mem[(q) as usize].hh().b1()).wrapping_sub(0i32);
                                                                                                    if (self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh() == 0i32) {
                                                                                                        break 'l_done3_f;
                                                                                                    }
                                                                                                    if (j == 63i32) {
                                                                                                        break 'l_done3_f;
                                                                                                    }
                                                                                                    j = (j).wrapping_add(1i32);
                                                                                                    self.hu[(j) as usize] = c;
                                                                                                    { let __v686 = self.eqtb[(((617139i32).wrapping_add(c)) - 1) as usize].hh().rh(); self.hc[(j) as usize] = __v686; }
                                                                                                    q = self.mem[(q) as usize].hh().rh();
                                                                                                }
                                                                                            }
                                                                                            self.hb = s;
                                                                                            self.hn = j;
                                                                                            if (((self.mem[(s) as usize].hh().b1()) % 2) != 0) {
                                                                                                self.hyf_bchar = self.font_bchar[(self.hf) as usize];
                                                                                            } else {
                                                                                                self.hyf_bchar = 256i32;
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        // §897
                                                                                        if ((self.mem[(s) as usize].hh().b0() == 11i32) && (self.mem[(s) as usize].hh().b1() == 0i32)) {
                                                                                            {
                                                                                                self.hb = s;
                                                                                                self.hyf_bchar = self.font_bchar[(self.hf) as usize];
                                                                                            }
                                                                                        } else {
                                                                                            break 'l_done3_f;
                                                                                        }
                                                                                    }
                                                                                }
                                                                                s = self.mem[(s) as usize].hh().rh();
                                                                            }
                                                                        }
                                                                    }
                                                                    // §899
                                                                    if (self.hn < (self.l_hyf).wrapping_add(self.r_hyf)) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    while true {
                                                                        {
                                                                            if (!(s >= self.hi_mem_min)) {
                                                                                match self.mem[(s) as usize].hh().b0() {
                                                                                    6 => {
                                                                                    }
                                                                                    11 => {
                                                                                        if (self.mem[(s) as usize].hh().b1() != 0i32) {
                                                                                            break 'l_done4_f;
                                                                                        }
                                                                                    }
                                                                                    8 | 10 | 12 | 3 | 5 | 4 => {
                                                                                        break 'l_done4_f;
                                                                                    }
                                                                                    _ => {
                                                                                        break 'l_done1_f;
                                                                                    }
                                                                                }
                                                                            }
                                                                            s = self.mem[(s) as usize].hh().rh();
                                                                        }
                                                                    }
                                                                }
                                                                // §894
                                                                self.hyphenate();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    11 => {
                                        // §866
                                        if (self.mem[(self.cur_p) as usize].hh().b1() == 1i32) {
                                            {
                                                if ((!(self.mem[(self.cur_p) as usize].hh().rh() >= self.hi_mem_min)) && auto_breaking) {
                                                    if (self.mem[(self.mem[(self.cur_p) as usize].hh().rh()) as usize].hh().b0() == 10i32) {
                                                        self.try_break(0i32, 0i32);
                                                    }
                                                }
                                                { let __v687 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v687; }
                                            }
                                        } else {
                                            { let __v688 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v688; }
                                        }
                                    }
                                    6 => {
                                        {
                                            f = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().b0();
                                            { let __v689 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v689; }
                                        }
                                    }
                                    7 => {
                                        // §869
                                        {
                                            s = self.mem[((self.cur_p).wrapping_add(1i32)) as usize].hh().lh();
                                            self.disc_width = 0i32;
                                            if (s == 0i32) {
                                                self.try_break(self.eqtb[((618167i32) - 1) as usize].int(), 1i32);
                                            } else {
                                                {
                                                    loop {
                                                        // §870
                                                        if (s >= self.hi_mem_min) {
                                                            {
                                                                f = self.mem[(s) as usize].hh().b0();
                                                                self.disc_width = (self.disc_width).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(s) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int());
                                                            }
                                                        } else {
                                                            match self.mem[(s) as usize].hh().b0() {
                                                                6 => {
                                                                    {
                                                                        f = self.mem[((s).wrapping_add(1i32)) as usize].hh().b0();
                                                                        self.disc_width = (self.disc_width).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int());
                                                                    }
                                                                }
                                                                0 | 1 | 2 | 11 => {
                                                                    self.disc_width = (self.disc_width).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].int());
                                                                }
                                                                _ => {
                                                                    self.confusion(937i32);
                                                                }
                                                            }
                                                        }
                                                        // §869
                                                        s = self.mem[(s) as usize].hh().rh();
                                                        if (s == 0i32) { break; }
                                                    }
                                                    { let __v690 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.disc_width); self.active_width[((1i32) - 1) as usize] = __v690; }
                                                    self.try_break(self.eqtb[((618166i32) - 1) as usize].int(), 1i32);
                                                    { let __v691 = (self.active_width[((1i32) - 1) as usize]).wrapping_sub(self.disc_width); self.active_width[((1i32) - 1) as usize] = __v691; }
                                                }
                                            }
                                            r = self.mem[(self.cur_p) as usize].hh().b1();
                                            s = self.mem[(self.cur_p) as usize].hh().rh();
                                            while (r > 0i32) {
                                                {
                                                    // §871
                                                    if (s >= self.hi_mem_min) {
                                                        {
                                                            f = self.mem[(s) as usize].hh().b0();
                                                            { let __v692 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(s) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v692; }
                                                        }
                                                    } else {
                                                        match self.mem[(s) as usize].hh().b0() {
                                                            6 => {
                                                                {
                                                                    f = self.mem[((s).wrapping_add(1i32)) as usize].hh().b0();
                                                                    { let __v693 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v693; }
                                                                }
                                                            }
                                                            0 | 1 | 2 | 11 => {
                                                                { let __v694 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((s).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v694; }
                                                            }
                                                            _ => {
                                                                self.confusion(938i32);
                                                            }
                                                        }
                                                    }
                                                    // §869
                                                    r = (r).wrapping_sub(1i32);
                                                    s = self.mem[(s) as usize].hh().rh();
                                                }
                                            }
                                            prev_p = self.cur_p;
                                            self.cur_p = s;
                                            break 'l_done5_f;
                                        }
                                    }
                                    9 => {
                                        // §866
                                        {
                                            auto_breaking = (self.mem[(self.cur_p) as usize].hh().b1() == 1i32);
                                            {
                                                if ((!(self.mem[(self.cur_p) as usize].hh().rh() >= self.hi_mem_min)) && auto_breaking) {
                                                    if (self.mem[(self.mem[(self.cur_p) as usize].hh().rh()) as usize].hh().b0() == 10i32) {
                                                        self.try_break(0i32, 0i32);
                                                    }
                                                }
                                                { let __v695 = (self.active_width[((1i32) - 1) as usize]).wrapping_add(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v695; }
                                            }
                                        }
                                    }
                                    12 => {
                                        self.try_break(self.mem[((self.cur_p).wrapping_add(1i32)) as usize].int(), 0i32);
                                    }
                                    4 | 3 | 5 => {
                                    }
                                    _ => {
                                        self.confusion(936i32);
                                    }
                                }
                                prev_p = self.cur_p;
                                self.cur_p = self.mem[(self.cur_p) as usize].hh().rh();
                            }
                        }
                    }
                    // §863
                    if (self.cur_p == 0i32) {
                        // §873
                        {
                            self.try_break((10000i32).wrapping_neg(), 1i32);
                            if (self.mem[(4999992i32) as usize].hh().rh() != 4999992i32) {
                                {
                                    // §874
                                    r = self.mem[(4999992i32) as usize].hh().rh();
                                    self.fewest_demerits = 1073741823i32;
                                    loop {
                                        if (self.mem[(r) as usize].hh().b0() != 2i32) {
                                            if (self.mem[((r).wrapping_add(2i32)) as usize].int() < self.fewest_demerits) {
                                                {
                                                    self.fewest_demerits = self.mem[((r).wrapping_add(2i32)) as usize].int();
                                                    self.best_bet = r;
                                                }
                                            }
                                        }
                                        r = self.mem[(r) as usize].hh().rh();
                                        if (r == 4999992i32) { break; }
                                    }
                                    self.best_line = self.mem[((self.best_bet).wrapping_add(1i32)) as usize].hh().lh();
                                    // §873
                                    if (self.eqtb[((618182i32) - 1) as usize].int() == 0i32) {
                                        break 'l_done_f;
                                    }
                                    // §875
                                    {
                                        r = self.mem[(4999992i32) as usize].hh().rh();
                                        self.actual_looseness = 0i32;
                                        loop {
                                            if (self.mem[(r) as usize].hh().b0() != 2i32) {
                                                {
                                                    self.line_diff = (self.mem[((r).wrapping_add(1i32)) as usize].hh().lh()).wrapping_sub(self.best_line);
                                                    if (((self.line_diff < self.actual_looseness) && (self.eqtb[((618182i32) - 1) as usize].int() <= self.line_diff)) || ((self.line_diff > self.actual_looseness) && (self.eqtb[((618182i32) - 1) as usize].int() >= self.line_diff))) {
                                                        {
                                                            self.best_bet = r;
                                                            self.actual_looseness = self.line_diff;
                                                            self.fewest_demerits = self.mem[((r).wrapping_add(2i32)) as usize].int();
                                                        }
                                                    } else {
                                                        if ((self.line_diff == self.actual_looseness) && (self.mem[((r).wrapping_add(2i32)) as usize].int() < self.fewest_demerits)) {
                                                            {
                                                                self.best_bet = r;
                                                                self.fewest_demerits = self.mem[((r).wrapping_add(2i32)) as usize].int();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            r = self.mem[(r) as usize].hh().rh();
                                            if (r == 4999992i32) { break; }
                                        }
                                        self.best_line = self.mem[((self.best_bet).wrapping_add(1i32)) as usize].hh().lh();
                                    }
                                    // §873
                                    if ((self.actual_looseness == self.eqtb[((618182i32) - 1) as usize].int()) || self.final_pass) {
                                        break 'l_done_f;
                                    }
                                }
                            }
                        }
                    }
                    // §865
                    q = self.mem[(4999992i32) as usize].hh().rh();
                    while (q != 4999992i32) {
                        {
                            self.cur_p = self.mem[(q) as usize].hh().rh();
                            if (self.mem[(q) as usize].hh().b0() == 2i32) {
                                self.free_node(q, 7i32);
                            } else {
                                self.free_node(q, 3i32);
                            }
                            q = self.cur_p;
                        }
                    }
                    q = self.passive;
                    while (q != 0i32) {
                        {
                            self.cur_p = self.mem[(q) as usize].hh().rh();
                            self.free_node(q, 2i32);
                            q = self.cur_p;
                        }
                    }
                    // §863
                    if (!self.second_pass) {
                        {
                            if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                                self.print_nl(934i32);
                            }
                            self.threshold = self.eqtb[((618164i32) - 1) as usize].int();
                            self.second_pass = true;
                            self.final_pass = (self.eqtb[((618750i32) - 1) as usize].int() <= 0i32);
                        }
                    } else {
                        {
                            if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
                                self.print_nl(935i32);
                            }
                            { let __v696 = (self.background[((2i32) - 1) as usize]).wrapping_add(self.eqtb[((618750i32) - 1) as usize].int()); self.background[((2i32) - 1) as usize] = __v696; }
                            self.final_pass = true;
                        }
                    }
                }
            }
        }
        if (self.eqtb[((618195i32) - 1) as usize].int() > 0i32) {
            {
                self.end_diagnostic(true);
                self.normalize_selector();
            }
        }
        // §876
        self.post_line_break(final_widow_penalty);
        // §865
        q = self.mem[(4999992i32) as usize].hh().rh();
        while (q != 4999992i32) {
            {
                self.cur_p = self.mem[(q) as usize].hh().rh();
                if (self.mem[(q) as usize].hh().b0() == 2i32) {
                    self.free_node(q, 7i32);
                } else {
                    self.free_node(q, 3i32);
                }
                q = self.cur_p;
            }
        }
        q = self.passive;
        while (q != 0i32) {
            {
                self.cur_p = self.mem[(q) as usize].hh().rh();
                self.free_node(q, 2i32);
                q = self.cur_p;
            }
        }
        // §815
        self.pack_begin_line = 0i32;
    }

    /// We have now completed the hyphenation routine, so the `line_break` procedure
    /// is finished at last. Since the hyphenation exception table is fresh in our
    /// minds, it's a good time to deal with the routine that adds new entries to it.
    /// When \TeX\ has scanned `\.{\\hyphenation}', it calls on a procedure named
    /// `new_hyph_exceptions` to do the right thing.
    // §934
    pub fn new_hyph_exceptions(&mut self) {
        let mut n: i32 = 0; // §934
        let mut j: i32 = 0; // §934
        let mut h: hyph_pointer = 0; // §934
        let mut k: str_number = 0; // §934
        let mut p: halfword = 0; // §934
        let mut q: halfword = 0; // §934
        let mut s: str_number = 0; // §934
        let mut t: str_number = 0; // §934
        let mut u: pool_pointer = 0; // §934
        let mut v: pool_pointer = 0; // §934
        'l_exit_f: {
            self.scan_left_brace();
            if (self.eqtb[((618213i32) - 1) as usize].int() <= 0i32) {
                self.cur_lang = 0i32;
            } else {
                if (self.eqtb[((618213i32) - 1) as usize].int() > 255i32) {
                    self.cur_lang = 0i32;
                } else {
                    self.cur_lang = self.eqtb[((618213i32) - 1) as usize].int();
                }
            }
            // §935
            n = 0i32;
            p = 0i32;
            while true {
                {
                    self.get_x_token();
                    'l_reswitch_b: loop {
                        match self.cur_cmd {
                            11 | 12 | 68 => {
                                // §937
                                if (self.cur_chr == 45i32) {
                                    // §938
                                    {
                                        if (n < 63i32) {
                                            {
                                                q = self.get_avail();
                                                self.mem[(q) as usize].set_hh_rh(p);
                                                self.mem[(q) as usize].set_hh_lh(n);
                                                p = q;
                                            }
                                        }
                                    }
                                } else {
                                    // §937
                                    {
                                        if (self.eqtb[(((617139i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh() == 0i32) {
                                            {
                                                {
                                                    if (self.interaction == 3i32) {
                                                    }
                                                    self.print_nl(262i32);
                                                    self.print(945i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[(1i32) as usize] = 946i32;
                                                    self.help_line[(0i32) as usize] = 947i32;
                                                }
                                                self.error();
                                            }
                                        } else {
                                            if (n < 63i32) {
                                                {
                                                    n = (n).wrapping_add(1i32);
                                                    { let __v697 = self.eqtb[(((617139i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh(); self.hc[(n) as usize] = __v697; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            16 => {
                                // §935
                                {
                                    self.scan_char_num();
                                    self.cur_chr = self.cur_val;
                                    self.cur_cmd = 68i32;
                                    continue 'l_reswitch_b;
                                }
                            }
                            10 | 2 => {
                                {
                                    if (n > 1i32) {
                                        // §939
                                        {
                                            n = (n).wrapping_add(1i32);
                                            { let __v698 = self.cur_lang; self.hc[(n) as usize] = __v698; }
                                            {
                                                if ((self.pool_ptr).wrapping_add(n) > pool_size) {
                                                    self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                                }
                                            }
                                            h = 0i32;
                                            {
                                                let __for_end_11 = n;
                                                j = 1i32;
                                                while j <= __for_end_11 {
                                                    {
                                                        h = (((h).wrapping_add(h)).wrapping_add(self.hc[(j) as usize]) % 8191i32);
                                                        {
                                                            { let __ix699 = self.pool_ptr; let __v700 = self.hc[(j) as usize]; self.str_pool[(__ix699) as usize] = __v700; }
                                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                    j = j.wrapping_add(1);
                                                }
                                            }
                                            s = self.make_string();
                                            // §940
                                            if (self.hyph_count == 8191i32) {
                                                self.overflow(948i32, 8191i32);
                                            }
                                            self.hyph_count = (self.hyph_count).wrapping_add(1i32);
                                            while (self.hyph_word[(h) as usize] != 0i32) {
                                                {
                                                    'l_not_found_f: {
                                                        'l_found_f: {
                                                            // §941
                                                            k = self.hyph_word[(h) as usize];
                                                            if ((self.str_start[((k).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(k) as usize]) < (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) {
                                                                break 'l_found_f;
                                                            }
                                                            if ((self.str_start[((k).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(k) as usize]) > (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) {
                                                                break 'l_not_found_f;
                                                            }
                                                            u = self.str_start[(k) as usize];
                                                            v = self.str_start[(s) as usize];
                                                            loop {
                                                                if (self.str_pool[(u) as usize] < self.str_pool[(v) as usize]) {
                                                                    break 'l_found_f;
                                                                }
                                                                if (self.str_pool[(u) as usize] > self.str_pool[(v) as usize]) {
                                                                    break 'l_not_found_f;
                                                                }
                                                                u = (u).wrapping_add(1i32);
                                                                v = (v).wrapping_add(1i32);
                                                                if (u == self.str_start[((k).wrapping_add(1i32)) as usize]) { break; }
                                                            }
                                                        }
                                                        q = self.hyph_list[(h) as usize];
                                                        self.hyph_list[(h) as usize] = p;
                                                        p = q;
                                                        t = self.hyph_word[(h) as usize];
                                                        self.hyph_word[(h) as usize] = s;
                                                        s = t;
                                                    }
                                                    // §940
                                                    if (h > 0i32) {
                                                        h = (h).wrapping_sub(1i32);
                                                    } else {
                                                        h = 8191i32;
                                                    }
                                                }
                                            }
                                            self.hyph_word[(h) as usize] = s;
                                            self.hyph_list[(h) as usize] = p;
                                        }
                                    }
                                    // §935
                                    if (self.cur_cmd == 2i32) {
                                        break 'l_exit_f;
                                    }
                                    n = 0i32;
                                    p = 0i32;
                                }
                            }
                            _ => {
                                // §936
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(680i32);
                                    }
                                    self.print_esc(941i32);
                                    self.print(942i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 943i32;
                                        self.help_line[(0i32) as usize] = 944i32;
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
        // §934
    }

    /// A subroutine called `prune_page_top` takes a pointer to a vlist and
    /// returns a pointer to a modified vlist in which all glue, kern, and penalty nodes
    /// have been deleted before the first box or rule node. However, the first
    /// box or rule is actually preceded by a newly created glue node designed so that
    /// the topmost baseline will be at distance `split_top_skip` from the top,
    /// whenever this is possible without backspacing.
    /// In this routine and those that follow, we make use of the fact that a
    /// vertical list contains no character nodes, hence the `type` field exists
    /// for each node in the list.
    // §968
    pub fn prune_page_top(&mut self, mut p: halfword) -> halfword {
        let mut prune_page_top: halfword = 0;
        let mut prev_p: halfword = 0; // §968
        let mut q: halfword = 0; // §968
        prev_p = 4999996i32;
        self.mem[(4999996i32) as usize].set_hh_rh(p);
        while (p != 0i32) {
            match self.mem[(p) as usize].hh().b0() {
                0 | 1 | 2 => {
                    // §969
                    {
                        q = self.new_skip_param(10i32);
                        self.mem[(prev_p) as usize].set_hh_rh(q);
                        self.mem[(q) as usize].set_hh_rh(p);
                        if (self.mem[((self.temp_ptr).wrapping_add(1i32)) as usize].int() > self.mem[((p).wrapping_add(3i32)) as usize].int()) {
                            { let __ix701 = (self.temp_ptr).wrapping_add(1i32); let __v702 = (self.mem[((self.temp_ptr).wrapping_add(1i32)) as usize].int()).wrapping_sub(self.mem[((p).wrapping_add(3i32)) as usize].int()); self.mem[(__ix701) as usize].set_int(__v702); }
                        } else {
                            { let __ix703 = (self.temp_ptr).wrapping_add(1i32); self.mem[(__ix703) as usize].set_int(0i32); }
                        }
                        p = 0i32;
                    }
                }
                8 | 4 | 3 => {
                    // §968
                    {
                        prev_p = p;
                        p = self.mem[(prev_p) as usize].hh().rh();
                    }
                }
                10 | 11 | 12 => {
                    {
                        q = p;
                        p = self.mem[(q) as usize].hh().rh();
                        self.mem[(q) as usize].set_hh_rh(0i32);
                        self.mem[(prev_p) as usize].set_hh_rh(p);
                        self.flush_node_list(q);
                    }
                }
                _ => {
                    self.confusion(959i32);
                }
            }
        }
        prune_page_top = self.mem[(4999996i32) as usize].hh().rh();
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
    // §970
    pub fn vert_break(&mut self, mut p: halfword, mut h: scaled, mut d: scaled) -> halfword {
        let mut vert_break: halfword = 0;
        let mut prev_p: halfword = 0; // §970
        let mut q: halfword = 0; // §970
        let mut r: halfword = 0; // §970
        let mut pi: i32 = 0; // §970
        let mut b: i32 = 0; // §970
        let mut least_cost: i32 = 0; // §970
        let mut best_place: halfword = 0; // §970
        let mut prev_dp: scaled = 0; // §970
        let mut t: small_number = 0; // §970
        'l_done_f: {
            prev_p = p;
            least_cost = 1073741823i32;
            self.active_width[((1i32) - 1) as usize] = 0i32;
            self.active_width[((2i32) - 1) as usize] = 0i32;
            self.active_width[((3i32) - 1) as usize] = 0i32;
            self.active_width[((4i32) - 1) as usize] = 0i32;
            self.active_width[((5i32) - 1) as usize] = 0i32;
            self.active_width[((6i32) - 1) as usize] = 0i32;
            prev_dp = 0i32;
            while true {
                {
                    'l_not_found_f: {
                        'l_L90_f: {
                            // §972
                            if (p == 0i32) {
                                pi = (10000i32).wrapping_neg();
                            } else {
                                // §973
                                match self.mem[(p) as usize].hh().b0() {
                                    0 | 1 | 2 => {
                                        {
                                            { let __v704 = ((self.active_width[((1i32) - 1) as usize]).wrapping_add(prev_dp)).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v704; }
                                            prev_dp = self.mem[((p).wrapping_add(2i32)) as usize].int();
                                            break 'l_not_found_f;
                                        }
                                    }
                                    8 => {
                                        // §1365
                                        break 'l_not_found_f;
                                    }
                                    10 => {
                                        // §973
                                        if (self.mem[(prev_p) as usize].hh().b0() < 9i32) {
                                            pi = 0i32;
                                        } else {
                                            break 'l_L90_f;
                                        }
                                    }
                                    11 => {
                                        {
                                            if (self.mem[(p) as usize].hh().rh() == 0i32) {
                                                t = 12i32;
                                            } else {
                                                t = self.mem[(self.mem[(p) as usize].hh().rh()) as usize].hh().b0();
                                            }
                                            if (t == 10i32) {
                                                pi = 0i32;
                                            } else {
                                                break 'l_L90_f;
                                            }
                                        }
                                    }
                                    12 => {
                                        pi = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                    }
                                    4 | 3 => {
                                        break 'l_not_found_f;
                                    }
                                    _ => {
                                        self.confusion(960i32);
                                    }
                                }
                            }
                            // §974
                            if (pi < 10000i32) {
                                {
                                    // §975
                                    if (self.active_width[((1i32) - 1) as usize] < h) {
                                        if (((self.active_width[((3i32) - 1) as usize] != 0i32) || (self.active_width[((4i32) - 1) as usize] != 0i32)) || (self.active_width[((5i32) - 1) as usize] != 0i32)) {
                                            b = 0i32;
                                        } else {
                                            b = self.badness((h).wrapping_sub(self.active_width[((1i32) - 1) as usize]), self.active_width[((2i32) - 1) as usize]);
                                        }
                                    } else {
                                        if ((self.active_width[((1i32) - 1) as usize]).wrapping_sub(h) > self.active_width[((6i32) - 1) as usize]) {
                                            b = 1073741823i32;
                                        } else {
                                            b = self.badness((self.active_width[((1i32) - 1) as usize]).wrapping_sub(h), self.active_width[((6i32) - 1) as usize]);
                                        }
                                    }
                                    // §974
                                    if (b < 1073741823i32) {
                                        if (pi <= (10000i32).wrapping_neg()) {
                                            b = pi;
                                        } else {
                                            if (b < 10000i32) {
                                                b = (b).wrapping_add(pi);
                                            } else {
                                                b = 100000i32;
                                            }
                                        }
                                    }
                                    if (b <= least_cost) {
                                        {
                                            best_place = p;
                                            least_cost = b;
                                            self.best_height_plus_depth = (self.active_width[((1i32) - 1) as usize]).wrapping_add(prev_dp);
                                        }
                                    }
                                    if ((b == 1073741823i32) || (pi <= (10000i32).wrapping_neg())) {
                                        break 'l_done_f;
                                    }
                                }
                            }
                            // §972
                            if ((self.mem[(p) as usize].hh().b0() < 10i32) || (self.mem[(p) as usize].hh().b0() > 11i32)) {
                                break 'l_not_found_f;
                            }
                        }
                        if (self.mem[(p) as usize].hh().b0() == 11i32) {
                            // §976
                            q = p;
                        } else {
                            {
                                q = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                { let __ix705 = (2i32).wrapping_add(self.mem[(q) as usize].hh().b0()); let __v706 = (self.active_width[(((2i32).wrapping_add(self.mem[(q) as usize].hh().b0())) - 1) as usize]).wrapping_add(self.mem[((q).wrapping_add(2i32)) as usize].int()); self.active_width[((__ix705) - 1) as usize] = __v706; }
                                { let __v707 = (self.active_width[((6i32) - 1) as usize]).wrapping_add(self.mem[((q).wrapping_add(3i32)) as usize].int()); self.active_width[((6i32) - 1) as usize] = __v707; }
                                if ((self.mem[(q) as usize].hh().b1() != 0i32) && (self.mem[((q).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                    {
                                        {
                                            if (self.interaction == 3i32) {
                                            }
                                            self.print_nl(262i32);
                                            self.print(961i32);
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[(3i32) as usize] = 962i32;
                                            self.help_line[(2i32) as usize] = 963i32;
                                            self.help_line[(1i32) as usize] = 964i32;
                                            self.help_line[(0i32) as usize] = 922i32;
                                        }
                                        self.error();
                                        r = self.new_spec(q);
                                        self.mem[(r) as usize].set_hh_b1(0i32);
                                        self.delete_glue_ref(q);
                                        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(r);
                                        q = r;
                                    }
                                }
                            }
                        }
                        { let __v708 = ((self.active_width[((1i32) - 1) as usize]).wrapping_add(prev_dp)).wrapping_add(self.mem[((q).wrapping_add(1i32)) as usize].int()); self.active_width[((1i32) - 1) as usize] = __v708; }
                        prev_dp = 0i32;
                    }
                    // §972
                    if (prev_dp > d) {
                        {
                            { let __v709 = ((self.active_width[((1i32) - 1) as usize]).wrapping_add(prev_dp)).wrapping_sub(d); self.active_width[((1i32) - 1) as usize] = __v709; }
                            prev_dp = d;
                        }
                    }
                    // §970
                    prev_p = p;
                    p = self.mem[(prev_p) as usize].hh().rh();
                }
            }
        }
        vert_break = best_place;
        vert_break
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
    // §977
    pub fn vsplit(&mut self, mut n: eight_bits, mut h: scaled) -> halfword {
        let mut vsplit: halfword = 0;
        let mut v: halfword = 0; // §977
        let mut p: halfword = 0; // §977
        let mut q: halfword = 0; // §977
        'l_exit_f: {
            'l_done_f: {
                v = self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh();
                if (self.cur_mark[(3i32) as usize] != 0i32) {
                    {
                        self.delete_token_ref(self.cur_mark[(3i32) as usize]);
                        self.cur_mark[(3i32) as usize] = 0i32;
                        self.delete_token_ref(self.cur_mark[(4i32) as usize]);
                        self.cur_mark[(4i32) as usize] = 0i32;
                    }
                }
                // §978
                if (v == 0i32) {
                    {
                        vsplit = 0i32;
                        break 'l_exit_f;
                    }
                }
                if (self.mem[(v) as usize].hh().b0() != 1i32) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(338i32);
                        }
                        self.print_esc(965i32);
                        self.print(966i32);
                        self.print_esc(967i32);
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 968i32;
                            self.help_line[(0i32) as usize] = 969i32;
                        }
                        self.error();
                        vsplit = 0i32;
                        break 'l_exit_f;
                    }
                }
                // §977
                q = self.vert_break(self.mem[((v).wrapping_add(5i32)) as usize].hh().rh(), h, self.eqtb[((618736i32) - 1) as usize].int());
                // §979
                p = self.mem[((v).wrapping_add(5i32)) as usize].hh().rh();
                if (p == q) {
                    self.mem[((v).wrapping_add(5i32)) as usize].set_hh_rh(0i32);
                } else {
                    while true {
                        {
                            if (self.mem[(p) as usize].hh().b0() == 4i32) {
                                if (self.cur_mark[(3i32) as usize] == 0i32) {
                                    {
                                        { let __v710 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.cur_mark[(3i32) as usize] = __v710; }
                                        { let __v711 = self.cur_mark[(3i32) as usize]; self.cur_mark[(4i32) as usize] = __v711; }
                                        { let __ix712 = self.cur_mark[(3i32) as usize]; let __v713 = (self.mem[(self.cur_mark[(3i32) as usize]) as usize].hh().lh()).wrapping_add(2i32); self.mem[(__ix712) as usize].set_hh_lh(__v713); }
                                    }
                                } else {
                                    {
                                        self.delete_token_ref(self.cur_mark[(4i32) as usize]);
                                        { let __v714 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.cur_mark[(4i32) as usize] = __v714; }
                                        { let __ix715 = self.cur_mark[(4i32) as usize]; let __v716 = (self.mem[(self.cur_mark[(4i32) as usize]) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix715) as usize].set_hh_lh(__v716); }
                                    }
                                }
                            }
                            if (self.mem[(p) as usize].hh().rh() == q) {
                                {
                                    self.mem[(p) as usize].set_hh_rh(0i32);
                                    break 'l_done_f;
                                }
                            }
                            p = self.mem[(p) as usize].hh().rh();
                        }
                    }
                }
            }
            // §977
            q = self.prune_page_top(q);
            p = self.mem[((v).wrapping_add(5i32)) as usize].hh().rh();
            self.free_node(v, 7i32);
            if (q == 0i32) {
                self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].set_hh_rh(0i32);
            } else {
                { let __v717 = self.vpackage(q, 0i32, 1i32, 1073741823i32); self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].set_hh_rh(__v717); }
            }
            vsplit = self.vpackage(p, h, 0i32, self.eqtb[((618736i32) - 1) as usize].int());
        }
        vsplit
    }

    // §985
    pub fn print_totals(&mut self) {
        self.print_scaled(self.page_so_far[(1i32) as usize]);
        if (self.page_so_far[(2i32) as usize] != 0i32) {
            {
                self.print(312i32);
                self.print_scaled(self.page_so_far[(2i32) as usize]);
                self.print(338i32);
            }
        }
        if (self.page_so_far[(3i32) as usize] != 0i32) {
            {
                self.print(312i32);
                self.print_scaled(self.page_so_far[(3i32) as usize]);
                self.print(311i32);
            }
        }
        if (self.page_so_far[(4i32) as usize] != 0i32) {
            {
                self.print(312i32);
                self.print_scaled(self.page_so_far[(4i32) as usize]);
                self.print(978i32);
            }
        }
        if (self.page_so_far[(5i32) as usize] != 0i32) {
            {
                self.print(312i32);
                self.print_scaled(self.page_so_far[(5i32) as usize]);
                self.print(979i32);
            }
        }
        if (self.page_so_far[(6i32) as usize] != 0i32) {
            {
                self.print(313i32);
                self.print_scaled(self.page_so_far[(6i32) as usize]);
            }
        }
    }

    /// Here is a procedure that is called when the `page_contents` is changing
    /// from `empty` to `inserts_only` or `box_there`.
    // §987
    pub fn freeze_page_specs(&mut self, mut s: small_number) {
        self.page_contents = s;
        { let __v718 = self.eqtb[((618734i32) - 1) as usize].int(); self.page_so_far[(0i32) as usize] = __v718; }
        self.page_max_depth = self.eqtb[((618735i32) - 1) as usize].int();
        self.page_so_far[(7i32) as usize] = 0i32;
        self.page_so_far[(1i32) as usize] = 0i32;
        self.page_so_far[(2i32) as usize] = 0i32;
        self.page_so_far[(3i32) as usize] = 0i32;
        self.page_so_far[(4i32) as usize] = 0i32;
        self.page_so_far[(5i32) as usize] = 0i32;
        self.page_so_far[(6i32) as usize] = 0i32;
        self.least_page_cost = 1073741823i32;
        if (self.eqtb[((618196i32) - 1) as usize].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_nl(987i32);
                self.print_scaled(self.page_so_far[(0i32) as usize]);
                self.print(988i32);
                self.print_scaled(self.page_max_depth);
                self.end_diagnostic(false);
            }
        }
    }

    /// At certain times box 255 is supposed to be void (i.e., `null`),
    /// or an insertion box is supposed to be ready to accept a vertical list.
    /// If not, an error message is printed, and the following subroutine
    /// flushes the unwanted contents, reporting them to the user.
    // §992
    pub fn box_error(&mut self, mut n: eight_bits) {
        self.error();
        self.begin_diagnostic();
        self.print_nl(836i32);
        self.show_box(self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh());
        self.end_diagnostic(true);
        self.flush_node_list(self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh());
        self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].set_hh_rh(0i32);
    }

    /// The following procedure guarantees that a given box register
    /// does not contain an \.{\\hbox}.
    // §993
    pub fn ensure_vbox(&mut self, mut n: eight_bits) {
        let mut p: halfword = 0; // §993
        p = self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh();
        if (p != 0i32) {
            if (self.mem[(p) as usize].hh().b0() == 0i32) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(989i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 990i32;
                        self.help_line[(1i32) as usize] = 991i32;
                        self.help_line[(0i32) as usize] = 992i32;
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
    // §1012
    pub fn fire_up(&mut self, mut c: halfword) {
        let mut p: halfword = 0; // §1012
        let mut q: halfword = 0; // §1012
        let mut r: halfword = 0; // §1012
        let mut s: halfword = 0; // §1012
        let mut prev_p: halfword = 0; // §1012
        let mut n: i32 = 0; // §1012
        let mut wait: bool = false; // §1012
        let mut save_vbadness: i32 = 0; // §1012
        let mut save_vfuzz: scaled = 0; // §1012
        let mut save_split_top_skip: halfword = 0; // §1012
        'l_exit_f: {
            // §1013
            if (self.mem[(self.best_page_break) as usize].hh().b0() == 12i32) {
                {
                    self.geq_word_define(618202i32, self.mem[((self.best_page_break).wrapping_add(1i32)) as usize].int());
                    { let __ix719 = (self.best_page_break).wrapping_add(1i32); self.mem[(__ix719) as usize].set_int(10000i32); }
                }
            } else {
                self.geq_word_define(618202i32, 10000i32);
            }
            // §1012
            if (self.cur_mark[(2i32) as usize] != 0i32) {
                {
                    if (self.cur_mark[(0i32) as usize] != 0i32) {
                        self.delete_token_ref(self.cur_mark[(0i32) as usize]);
                    }
                    { let __v720 = self.cur_mark[(2i32) as usize]; self.cur_mark[(0i32) as usize] = __v720; }
                    { let __ix721 = self.cur_mark[(0i32) as usize]; let __v722 = (self.mem[(self.cur_mark[(0i32) as usize]) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix721) as usize].set_hh_lh(__v722); }
                    self.delete_token_ref(self.cur_mark[(1i32) as usize]);
                    self.cur_mark[(1i32) as usize] = 0i32;
                }
            }
            // §1014
            if (c == self.best_page_break) {
                self.best_page_break = 0i32;
            }
            // §1015
            if (self.eqtb[((616833i32) - 1) as usize].hh().rh() != 0i32) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(338i32);
                    }
                    self.print_esc(409i32);
                    self.print(1003i32);
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 1004i32;
                        self.help_line[(0i32) as usize] = 992i32;
                    }
                    self.box_error(255i32);
                }
            }
            // §1014
            self.insert_penalties = 0i32;
            save_split_top_skip = self.eqtb[((615792i32) - 1) as usize].hh().rh();
            if (self.eqtb[((618216i32) - 1) as usize].int() <= 0i32) {
                // §1018
                {
                    r = self.mem[(4999999i32) as usize].hh().rh();
                    while (r != 4999999i32) {
                        {
                            if (self.mem[((r).wrapping_add(2i32)) as usize].hh().lh() != 0i32) {
                                {
                                    n = (self.mem[(r) as usize].hh().b1()).wrapping_sub(0i32);
                                    self.ensure_vbox(n);
                                    if (self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh() == 0i32) {
                                        { let __v723 = self.new_null_box(); self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].set_hh_rh(__v723); }
                                    }
                                    p = (self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh()).wrapping_add(5i32);
                                    while (self.mem[(p) as usize].hh().rh() != 0i32) {
                                        p = self.mem[(p) as usize].hh().rh();
                                    }
                                    self.mem[((r).wrapping_add(2i32)) as usize].set_hh_rh(p);
                                }
                            }
                            r = self.mem[(r) as usize].hh().rh();
                        }
                    }
                }
            }
            // §1014
            q = 4999995i32;
            self.mem[(q) as usize].set_hh_rh(0i32);
            prev_p = 4999997i32;
            p = self.mem[(prev_p) as usize].hh().rh();
            while (p != self.best_page_break) {
                {
                    if (self.mem[(p) as usize].hh().b0() == 3i32) {
                        {
                            if (self.eqtb[((618216i32) - 1) as usize].int() <= 0i32) {
                                // §1020
                                {
                                    r = self.mem[(4999999i32) as usize].hh().rh();
                                    while (self.mem[(r) as usize].hh().b1() != self.mem[(p) as usize].hh().b1()) {
                                        r = self.mem[(r) as usize].hh().rh();
                                    }
                                    if (self.mem[((r).wrapping_add(2i32)) as usize].hh().lh() == 0i32) {
                                        wait = true;
                                    } else {
                                        {
                                            wait = false;
                                            s = self.mem[((r).wrapping_add(2i32)) as usize].hh().rh();
                                            { let __v724 = self.mem[((p).wrapping_add(4i32)) as usize].hh().lh(); self.mem[(s) as usize].set_hh_rh(__v724); }
                                            if (self.mem[((r).wrapping_add(2i32)) as usize].hh().lh() == p) {
                                                // §1021
                                                {
                                                    if (self.mem[(r) as usize].hh().b0() == 1i32) {
                                                        if ((self.mem[((r).wrapping_add(1i32)) as usize].hh().lh() == p) && (self.mem[((r).wrapping_add(1i32)) as usize].hh().rh() != 0i32)) {
                                                            {
                                                                while (self.mem[(s) as usize].hh().rh() != self.mem[((r).wrapping_add(1i32)) as usize].hh().rh()) {
                                                                    s = self.mem[(s) as usize].hh().rh();
                                                                }
                                                                self.mem[(s) as usize].set_hh_rh(0i32);
                                                                { let __v725 = self.mem[((p).wrapping_add(4i32)) as usize].hh().rh(); self.eqtb[((615792i32) - 1) as usize].set_hh_rh(__v725); }
                                                                { let __v726 = self.prune_page_top(self.mem[((r).wrapping_add(1i32)) as usize].hh().rh()); self.mem[((p).wrapping_add(4i32)) as usize].set_hh_lh(__v726); }
                                                                if (self.mem[((p).wrapping_add(4i32)) as usize].hh().lh() != 0i32) {
                                                                    {
                                                                        self.temp_ptr = self.vpackage(self.mem[((p).wrapping_add(4i32)) as usize].hh().lh(), 0i32, 1i32, 1073741823i32);
                                                                        { let __v727 = (self.mem[((self.temp_ptr).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((self.temp_ptr).wrapping_add(2i32)) as usize].int()); self.mem[((p).wrapping_add(3i32)) as usize].set_int(__v727); }
                                                                        self.free_node(self.temp_ptr, 7i32);
                                                                        wait = true;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.mem[((r).wrapping_add(2i32)) as usize].set_hh_lh(0i32);
                                                    n = (self.mem[(r) as usize].hh().b1()).wrapping_sub(0i32);
                                                    self.temp_ptr = self.mem[((self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh()).wrapping_add(5i32)) as usize].hh().rh();
                                                    self.free_node(self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh(), 7i32);
                                                    { let __v728 = self.vpackage(self.temp_ptr, 0i32, 1i32, 1073741823i32); self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].set_hh_rh(__v728); }
                                                }
                                            } else {
                                                // §1020
                                                {
                                                    while (self.mem[(s) as usize].hh().rh() != 0i32) {
                                                        s = self.mem[(s) as usize].hh().rh();
                                                    }
                                                    self.mem[((r).wrapping_add(2i32)) as usize].set_hh_rh(s);
                                                }
                                            }
                                        }
                                    }
                                    // §1022
                                    { let __v729 = self.mem[(p) as usize].hh().rh(); self.mem[(prev_p) as usize].set_hh_rh(__v729); }
                                    self.mem[(p) as usize].set_hh_rh(0i32);
                                    if wait {
                                        {
                                            self.mem[(q) as usize].set_hh_rh(p);
                                            q = p;
                                            self.insert_penalties = (self.insert_penalties).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            self.delete_glue_ref(self.mem[((p).wrapping_add(4i32)) as usize].hh().rh());
                                            self.free_node(p, 5i32);
                                        }
                                    }
                                    p = prev_p;
                                }
                            }
                        }
                    } else {
                        // §1014
                        if (self.mem[(p) as usize].hh().b0() == 4i32) {
                            // §1016
                            {
                                if (self.cur_mark[(1i32) as usize] == 0i32) {
                                    {
                                        { let __v730 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.cur_mark[(1i32) as usize] = __v730; }
                                        { let __ix731 = self.cur_mark[(1i32) as usize]; let __v732 = (self.mem[(self.cur_mark[(1i32) as usize]) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix731) as usize].set_hh_lh(__v732); }
                                    }
                                }
                                if (self.cur_mark[(2i32) as usize] != 0i32) {
                                    self.delete_token_ref(self.cur_mark[(2i32) as usize]);
                                }
                                { let __v733 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.cur_mark[(2i32) as usize] = __v733; }
                                { let __ix734 = self.cur_mark[(2i32) as usize]; let __v735 = (self.mem[(self.cur_mark[(2i32) as usize]) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix734) as usize].set_hh_lh(__v735); }
                            }
                        }
                    }
                    // §1014
                    prev_p = p;
                    p = self.mem[(prev_p) as usize].hh().rh();
                }
            }
            self.eqtb[((615792i32) - 1) as usize].set_hh_rh(save_split_top_skip);
            // §1017
            if (p != 0i32) {
                {
                    if (self.mem[(4999998i32) as usize].hh().rh() == 0i32) {
                        if (self.nest_ptr == 0i32) {
                            self.cur_list.tail_field = self.page_tail;
                        } else {
                            self.nest[(0i32) as usize].tail_field = self.page_tail;
                        }
                    }
                    { let __ix736 = self.page_tail; let __v737 = self.mem[(4999998i32) as usize].hh().rh(); self.mem[(__ix736) as usize].set_hh_rh(__v737); }
                    self.mem[(4999998i32) as usize].set_hh_rh(p);
                    self.mem[(prev_p) as usize].set_hh_rh(0i32);
                }
            }
            save_vbadness = self.eqtb[((618190i32) - 1) as usize].int();
            self.eqtb[((618190i32) - 1) as usize].set_int(10000i32);
            save_vfuzz = self.eqtb[((618739i32) - 1) as usize].int();
            self.eqtb[((618739i32) - 1) as usize].set_int(1073741823i32);
            { let __v738 = self.vpackage(self.mem[(4999997i32) as usize].hh().rh(), self.best_size, 0i32, self.page_max_depth); self.eqtb[((616833i32) - 1) as usize].set_hh_rh(__v738); }
            self.eqtb[((618190i32) - 1) as usize].set_int(save_vbadness);
            self.eqtb[((618739i32) - 1) as usize].set_int(save_vfuzz);
            if (self.last_glue != 268435455i32) {
                self.delete_glue_ref(self.last_glue);
            }
            // §991
            self.page_contents = 0i32;
            self.page_tail = 4999997i32;
            self.mem[(4999997i32) as usize].set_hh_rh(0i32);
            self.last_glue = 268435455i32;
            self.last_penalty = 0i32;
            self.last_kern = 0i32;
            self.page_so_far[(7i32) as usize] = 0i32;
            self.page_max_depth = 0i32;
            // §1017
            if (q != 4999995i32) {
                {
                    { let __v739 = self.mem[(4999995i32) as usize].hh().rh(); self.mem[(4999997i32) as usize].set_hh_rh(__v739); }
                    self.page_tail = q;
                }
            }
            // §1019
            r = self.mem[(4999999i32) as usize].hh().rh();
            while (r != 4999999i32) {
                {
                    q = self.mem[(r) as usize].hh().rh();
                    self.free_node(r, 4i32);
                    r = q;
                }
            }
            self.mem[(4999999i32) as usize].set_hh_rh(4999999i32);
            // §1012
            if ((self.cur_mark[(0i32) as usize] != 0i32) && (self.cur_mark[(1i32) as usize] == 0i32)) {
                {
                    { let __v740 = self.cur_mark[(0i32) as usize]; self.cur_mark[(1i32) as usize] = __v740; }
                    { let __ix741 = self.cur_mark[(0i32) as usize]; let __v742 = (self.mem[(self.cur_mark[(0i32) as usize]) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix741) as usize].set_hh_lh(__v742); }
                }
            }
            if (self.eqtb[((616313i32) - 1) as usize].hh().rh() != 0i32) {
                if (self.dead_cycles >= self.eqtb[((618203i32) - 1) as usize].int()) {
                    // §1024
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(1005i32);
                        }
                        self.print_int(self.dead_cycles);
                        self.print(1006i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 1007i32;
                            self.help_line[(1i32) as usize] = 1008i32;
                            self.help_line[(0i32) as usize] = 1009i32;
                        }
                        self.error();
                    }
                } else {
                    // §1025
                    {
                        self.output_active = true;
                        self.dead_cycles = (self.dead_cycles).wrapping_add(1i32);
                        self.push_nest();
                        self.cur_list.mode_field = (1i32).wrapping_neg();
                        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
                        self.cur_list.ml_field = (self.line).wrapping_neg();
                        self.begin_token_list(self.eqtb[((616313i32) - 1) as usize].hh().rh(), 6i32);
                        self.new_save_level(8i32);
                        self.normal_paragraph();
                        self.scan_left_brace();
                        break 'l_exit_f;
                    }
                }
            }
            // §1023
            {
                if (self.mem[(4999997i32) as usize].hh().rh() != 0i32) {
                    {
                        if (self.mem[(4999998i32) as usize].hh().rh() == 0i32) {
                            if (self.nest_ptr == 0i32) {
                                self.cur_list.tail_field = self.page_tail;
                            } else {
                                self.nest[(0i32) as usize].tail_field = self.page_tail;
                            }
                        } else {
                            { let __ix743 = self.page_tail; let __v744 = self.mem[(4999998i32) as usize].hh().rh(); self.mem[(__ix743) as usize].set_hh_rh(__v744); }
                        }
                        { let __v745 = self.mem[(4999997i32) as usize].hh().rh(); self.mem[(4999998i32) as usize].set_hh_rh(__v745); }
                        self.mem[(4999997i32) as usize].set_hh_rh(0i32);
                        self.page_tail = 4999997i32;
                    }
                }
                self.ship_out(self.eqtb[((616833i32) - 1) as usize].hh().rh());
                self.eqtb[((616833i32) - 1) as usize].set_hh_rh(0i32);
            }
        }
        // §1012
    }

    /// \TeX\ is not always in vertical mode at the time `build_page`
    /// is called; the current mode reflects what \TeX\ should return to, after
    /// the contribution list has been emptied. A call on `build_page` should
    /// be immediately followed by ``goto big_switch`', which is \TeX's central
    /// control point.
    // §994
    pub fn build_page(&mut self) {
        let mut p: halfword = 0; // §994
        let mut q: halfword = 0; // §994
        let mut r: halfword = 0; // §994
        let mut b: i32 = 0; // §994
        let mut c: i32 = 0; // §994
        let mut pi: i32 = 0; // §994
        let mut n: i32 = 0; // §994
        let mut delta: scaled = 0; // §994
        let mut h: scaled = 0; // §994
        let mut w: scaled = 0; // §994
        'l_exit_f: {
            if ((self.mem[(4999998i32) as usize].hh().rh() == 0i32) || self.output_active) {
                break 'l_exit_f;
            }
            loop {
                // goto labels: continue, L90, L80, done1, done
                let mut __goto_1: i32 = 0;
                'l_dispatch_1: loop {
                    if __goto_1 <= 0 {
                        p = self.mem[(4999998i32) as usize].hh().rh();
                        // §996
                        if (self.last_glue != 268435455i32) {
                            self.delete_glue_ref(self.last_glue);
                        }
                        self.last_penalty = 0i32;
                        self.last_kern = 0i32;
                        if (self.mem[(p) as usize].hh().b0() == 10i32) {
                            {
                                self.last_glue = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                { let __ix746 = self.last_glue; let __v747 = (self.mem[(self.last_glue) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix746) as usize].set_hh_rh(__v747); }
                            }
                        } else {
                            {
                                self.last_glue = 268435455i32;
                                if (self.mem[(p) as usize].hh().b0() == 12i32) {
                                    self.last_penalty = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                } else {
                                    if (self.mem[(p) as usize].hh().b0() == 11i32) {
                                        self.last_kern = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                    }
                                }
                            }
                        }
                        // §1000
                        match self.mem[(p) as usize].hh().b0() {
                            0 | 1 | 2 => {
                                if (self.page_contents < 2i32) {
                                    // §1001
                                    {
                                        if (self.page_contents == 0i32) {
                                            self.freeze_page_specs(2i32);
                                        } else {
                                            self.page_contents = 2i32;
                                        }
                                        q = self.new_skip_param(9i32);
                                        if (self.mem[((self.temp_ptr).wrapping_add(1i32)) as usize].int() > self.mem[((p).wrapping_add(3i32)) as usize].int()) {
                                            { let __ix748 = (self.temp_ptr).wrapping_add(1i32); let __v749 = (self.mem[((self.temp_ptr).wrapping_add(1i32)) as usize].int()).wrapping_sub(self.mem[((p).wrapping_add(3i32)) as usize].int()); self.mem[(__ix748) as usize].set_int(__v749); }
                                        } else {
                                            { let __ix750 = (self.temp_ptr).wrapping_add(1i32); self.mem[(__ix750) as usize].set_int(0i32); }
                                        }
                                        self.mem[(q) as usize].set_hh_rh(p);
                                        self.mem[(4999998i32) as usize].set_hh_rh(q);
                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                    }
                                } else {
                                    // §1002
                                    {
                                        { let __v751 = ((self.page_so_far[(1i32) as usize]).wrapping_add(self.page_so_far[(7i32) as usize])).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int()); self.page_so_far[(1i32) as usize] = __v751; }
                                        { let __v752 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.page_so_far[(7i32) as usize] = __v752; }
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            8 => {
                                // §1364
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            }
                            10 => {
                                // §1000
                                if (self.page_contents < 2i32) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    if (self.mem[(self.page_tail) as usize].hh().b0() < 9i32) {
                                        pi = 0i32;
                                    } else {
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            11 => {
                                if (self.page_contents < 2i32) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    if (self.mem[(p) as usize].hh().rh() == 0i32) {
                                        break 'l_exit_f;
                                    } else {
                                        if (self.mem[(self.mem[(p) as usize].hh().rh()) as usize].hh().b0() == 10i32) {
                                            pi = 0i32;
                                        } else {
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            }
                            12 => {
                                if (self.page_contents < 2i32) {
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                } else {
                                    pi = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                }
                            }
                            4 => {
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            }
                            3 => {
                                // §1008
                                {
                                    if (self.page_contents == 0i32) {
                                        self.freeze_page_specs(1i32);
                                    }
                                    n = self.mem[(p) as usize].hh().b1();
                                    r = 4999999i32;
                                    while (n >= self.mem[(self.mem[(r) as usize].hh().rh()) as usize].hh().b1()) {
                                        r = self.mem[(r) as usize].hh().rh();
                                    }
                                    n = (n).wrapping_sub(0i32);
                                    if (self.mem[(r) as usize].hh().b1() != (n).wrapping_add(0i32)) {
                                        // §1009
                                        {
                                            q = self.get_node(4i32);
                                            { let __v753 = self.mem[(r) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v753); }
                                            self.mem[(r) as usize].set_hh_rh(q);
                                            r = q;
                                            self.mem[(r) as usize].set_hh_b1((n).wrapping_add(0i32));
                                            self.mem[(r) as usize].set_hh_b0(0i32);
                                            self.ensure_vbox(n);
                                            if (self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh() == 0i32) {
                                                self.mem[((r).wrapping_add(3i32)) as usize].set_int(0i32);
                                            } else {
                                                { let __v754 = (self.mem[((self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh()).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((self.eqtb[(((616578i32).wrapping_add(n)) - 1) as usize].hh().rh()).wrapping_add(2i32)) as usize].int()); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v754); }
                                            }
                                            self.mem[((r).wrapping_add(2i32)) as usize].set_hh_lh(0i32);
                                            q = self.eqtb[(((615800i32).wrapping_add(n)) - 1) as usize].hh().rh();
                                            if (self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int() == 1000i32) {
                                                h = self.mem[((r).wrapping_add(3i32)) as usize].int();
                                            } else {
                                                h = (self.x_over_n(self.mem[((r).wrapping_add(3i32)) as usize].int(), 1000i32)).wrapping_mul(self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int());
                                            }
                                            { let __v755 = ((self.page_so_far[(0i32) as usize]).wrapping_sub(h)).wrapping_sub(self.mem[((q).wrapping_add(1i32)) as usize].int()); self.page_so_far[(0i32) as usize] = __v755; }
                                            { let __ix756 = (2i32).wrapping_add(self.mem[(q) as usize].hh().b0()); let __v757 = (self.page_so_far[((2i32).wrapping_add(self.mem[(q) as usize].hh().b0())) as usize]).wrapping_add(self.mem[((q).wrapping_add(2i32)) as usize].int()); self.page_so_far[(__ix756) as usize] = __v757; }
                                            { let __v758 = (self.page_so_far[(6i32) as usize]).wrapping_add(self.mem[((q).wrapping_add(3i32)) as usize].int()); self.page_so_far[(6i32) as usize] = __v758; }
                                            if ((self.mem[(q) as usize].hh().b1() != 0i32) && (self.mem[((q).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                                {
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        self.print_nl(262i32);
                                                        self.print(998i32);
                                                    }
                                                    self.print_esc(395i32);
                                                    self.print_int(n);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[(2i32) as usize] = 999i32;
                                                        self.help_line[(1i32) as usize] = 1000i32;
                                                        self.help_line[(0i32) as usize] = 922i32;
                                                    }
                                                    self.error();
                                                }
                                            }
                                        }
                                    }
                                    // §1008
                                    if (self.mem[(r) as usize].hh().b0() == 1i32) {
                                        self.insert_penalties = (self.insert_penalties).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                    } else {
                                        {
                                            self.mem[((r).wrapping_add(2i32)) as usize].set_hh_rh(p);
                                            delta = (((self.page_so_far[(0i32) as usize]).wrapping_sub(self.page_so_far[(1i32) as usize])).wrapping_sub(self.page_so_far[(7i32) as usize])).wrapping_add(self.page_so_far[(6i32) as usize]);
                                            if (self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int() == 1000i32) {
                                                h = self.mem[((p).wrapping_add(3i32)) as usize].int();
                                            } else {
                                                h = (self.x_over_n(self.mem[((p).wrapping_add(3i32)) as usize].int(), 1000i32)).wrapping_mul(self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int());
                                            }
                                            if (((h <= 0i32) || (h <= delta)) && ((self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()) <= self.eqtb[(((618751i32).wrapping_add(n)) - 1) as usize].int())) {
                                                {
                                                    { let __v759 = (self.page_so_far[(0i32) as usize]).wrapping_sub(h); self.page_so_far[(0i32) as usize] = __v759; }
                                                    { let __v760 = (self.mem[((r).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int()); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v760); }
                                                }
                                            } else {
                                                // §1010
                                                {
                                                    if (self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int() <= 0i32) {
                                                        w = 1073741823i32;
                                                    } else {
                                                        {
                                                            w = ((self.page_so_far[(0i32) as usize]).wrapping_sub(self.page_so_far[(1i32) as usize])).wrapping_sub(self.page_so_far[(7i32) as usize]);
                                                            if (self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int() != 1000i32) {
                                                                w = (self.x_over_n(w, self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int())).wrapping_mul(1000i32);
                                                            }
                                                        }
                                                    }
                                                    if (w > (self.eqtb[(((618751i32).wrapping_add(n)) - 1) as usize].int()).wrapping_sub(self.mem[((r).wrapping_add(3i32)) as usize].int())) {
                                                        w = (self.eqtb[(((618751i32).wrapping_add(n)) - 1) as usize].int()).wrapping_sub(self.mem[((r).wrapping_add(3i32)) as usize].int());
                                                    }
                                                    q = self.vert_break(self.mem[((p).wrapping_add(4i32)) as usize].hh().lh(), w, self.mem[((p).wrapping_add(2i32)) as usize].int());
                                                    { let __v761 = (self.mem[((r).wrapping_add(3i32)) as usize].int()).wrapping_add(self.best_height_plus_depth); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v761); }
                                                    if (self.eqtb[((618196i32) - 1) as usize].int() > 0i32) {
                                                        // §1011
                                                        {
                                                            self.begin_diagnostic();
                                                            self.print_nl(1001i32);
                                                            self.print_int(n);
                                                            self.print(1002i32);
                                                            self.print_scaled(w);
                                                            self.print_char(44i32);
                                                            self.print_scaled(self.best_height_plus_depth);
                                                            self.print(931i32);
                                                            if (q == 0i32) {
                                                                self.print_int((10000i32).wrapping_neg());
                                                            } else {
                                                                if (self.mem[(q) as usize].hh().b0() == 12i32) {
                                                                    self.print_int(self.mem[((q).wrapping_add(1i32)) as usize].int());
                                                                } else {
                                                                    self.print_char(48i32);
                                                                }
                                                            }
                                                            self.end_diagnostic(false);
                                                        }
                                                    }
                                                    // §1010
                                                    if (self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int() != 1000i32) {
                                                        self.best_height_plus_depth = (self.x_over_n(self.best_height_plus_depth, 1000i32)).wrapping_mul(self.eqtb[(((618218i32).wrapping_add(n)) - 1) as usize].int());
                                                    }
                                                    { let __v762 = (self.page_so_far[(0i32) as usize]).wrapping_sub(self.best_height_plus_depth); self.page_so_far[(0i32) as usize] = __v762; }
                                                    self.mem[(r) as usize].set_hh_b0(1i32);
                                                    self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(q);
                                                    self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(p);
                                                    if (q == 0i32) {
                                                        self.insert_penalties = (self.insert_penalties).wrapping_sub(10000i32);
                                                    } else {
                                                        if (self.mem[(q) as usize].hh().b0() == 12i32) {
                                                            self.insert_penalties = (self.insert_penalties).wrapping_add(self.mem[((q).wrapping_add(1i32)) as usize].int());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // §1008
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            _ => {
                                // §1000
                                self.confusion(993i32);
                            }
                        }
                        // §1005
                        if (pi < 10000i32) {
                            {
                                // §1007
                                if (self.page_so_far[(1i32) as usize] < self.page_so_far[(0i32) as usize]) {
                                    if (((self.page_so_far[(3i32) as usize] != 0i32) || (self.page_so_far[(4i32) as usize] != 0i32)) || (self.page_so_far[(5i32) as usize] != 0i32)) {
                                        b = 0i32;
                                    } else {
                                        b = self.badness((self.page_so_far[(0i32) as usize]).wrapping_sub(self.page_so_far[(1i32) as usize]), self.page_so_far[(2i32) as usize]);
                                    }
                                } else {
                                    if ((self.page_so_far[(1i32) as usize]).wrapping_sub(self.page_so_far[(0i32) as usize]) > self.page_so_far[(6i32) as usize]) {
                                        b = 1073741823i32;
                                    } else {
                                        b = self.badness((self.page_so_far[(1i32) as usize]).wrapping_sub(self.page_so_far[(0i32) as usize]), self.page_so_far[(6i32) as usize]);
                                    }
                                }
                                // §1005
                                if (b < 1073741823i32) {
                                    if (pi <= (10000i32).wrapping_neg()) {
                                        c = pi;
                                    } else {
                                        if (b < 10000i32) {
                                            c = ((b).wrapping_add(pi)).wrapping_add(self.insert_penalties);
                                        } else {
                                            c = 100000i32;
                                        }
                                    }
                                } else {
                                    c = b;
                                }
                                if (self.insert_penalties >= 10000i32) {
                                    c = 1073741823i32;
                                }
                                if (self.eqtb[((618196i32) - 1) as usize].int() > 0i32) {
                                    // §1006
                                    {
                                        self.begin_diagnostic();
                                        self.print_nl(37i32);
                                        self.print(927i32);
                                        self.print_totals();
                                        self.print(996i32);
                                        self.print_scaled(self.page_so_far[(0i32) as usize]);
                                        self.print(930i32);
                                        if (b == 1073741823i32) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(b);
                                        }
                                        self.print(931i32);
                                        self.print_int(pi);
                                        self.print(997i32);
                                        if (c == 1073741823i32) {
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
                                // §1005
                                if (c <= self.least_page_cost) {
                                    {
                                        self.best_page_break = p;
                                        self.best_size = self.page_so_far[(0i32) as usize];
                                        self.least_page_cost = c;
                                        r = self.mem[(4999999i32) as usize].hh().rh();
                                        while (r != 4999999i32) {
                                            {
                                                { let __v763 = self.mem[((r).wrapping_add(2i32)) as usize].hh().rh(); self.mem[((r).wrapping_add(2i32)) as usize].set_hh_lh(__v763); }
                                                r = self.mem[(r) as usize].hh().rh();
                                            }
                                        }
                                    }
                                }
                                if ((c == 1073741823i32) || (pi <= (10000i32).wrapping_neg())) {
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
                        // §997
                        if ((self.mem[(p) as usize].hh().b0() < 10i32) || (self.mem[(p) as usize].hh().b0() > 11i32)) {
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    if __goto_1 <= 1 { // L90
                        if (self.mem[(p) as usize].hh().b0() == 11i32) {
                            // §1004
                            q = p;
                        } else {
                            {
                                q = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                { let __ix764 = (2i32).wrapping_add(self.mem[(q) as usize].hh().b0()); let __v765 = (self.page_so_far[((2i32).wrapping_add(self.mem[(q) as usize].hh().b0())) as usize]).wrapping_add(self.mem[((q).wrapping_add(2i32)) as usize].int()); self.page_so_far[(__ix764) as usize] = __v765; }
                                { let __v766 = (self.page_so_far[(6i32) as usize]).wrapping_add(self.mem[((q).wrapping_add(3i32)) as usize].int()); self.page_so_far[(6i32) as usize] = __v766; }
                                if ((self.mem[(q) as usize].hh().b1() != 0i32) && (self.mem[((q).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                    {
                                        {
                                            if (self.interaction == 3i32) {
                                            }
                                            self.print_nl(262i32);
                                            self.print(994i32);
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[(3i32) as usize] = 995i32;
                                            self.help_line[(2i32) as usize] = 963i32;
                                            self.help_line[(1i32) as usize] = 964i32;
                                            self.help_line[(0i32) as usize] = 922i32;
                                        }
                                        self.error();
                                        r = self.new_spec(q);
                                        self.mem[(r) as usize].set_hh_b1(0i32);
                                        self.delete_glue_ref(q);
                                        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(r);
                                        q = r;
                                    }
                                }
                            }
                        }
                        { let __v767 = ((self.page_so_far[(1i32) as usize]).wrapping_add(self.page_so_far[(7i32) as usize])).wrapping_add(self.mem[((q).wrapping_add(1i32)) as usize].int()); self.page_so_far[(1i32) as usize] = __v767; }
                        self.page_so_far[(7i32) as usize] = 0i32;
                    }
                    if __goto_1 <= 2 { // L80
                        // §997
                        if (self.page_so_far[(7i32) as usize] > self.page_max_depth) {
                            // §1003
                            {
                                { let __v768 = ((self.page_so_far[(1i32) as usize]).wrapping_add(self.page_so_far[(7i32) as usize])).wrapping_sub(self.page_max_depth); self.page_so_far[(1i32) as usize] = __v768; }
                                { let __v769 = self.page_max_depth; self.page_so_far[(7i32) as usize] = __v769; }
                            }
                        }
                        // §998
                        { let __ix770 = self.page_tail; self.mem[(__ix770) as usize].set_hh_rh(p); }
                        self.page_tail = p;
                        { let __v771 = self.mem[(p) as usize].hh().rh(); self.mem[(4999998i32) as usize].set_hh_rh(__v771); }
                        self.mem[(p) as usize].set_hh_rh(0i32);
                        { __goto_1 = 4; continue 'l_dispatch_1; }
                    }
                    if __goto_1 <= 3 { // done1
                        // §997
                        { let __v772 = self.mem[(p) as usize].hh().rh(); self.mem[(4999998i32) as usize].set_hh_rh(__v772); }
                        // §999
                        self.mem[(p) as usize].set_hh_rh(0i32);
                        self.flush_node_list(p);
                    }
                    if __goto_1 <= 4 { // done
                        // §997
                    }
                    break 'l_dispatch_1;
                }
                if (self.mem[(4999998i32) as usize].hh().rh() == 0i32) { break; }
            }
            // §995
            if (self.nest_ptr == 0i32) {
                self.cur_list.tail_field = 4999998i32;
            } else {
                self.nest[(0i32) as usize].tail_field = 4999998i32;
            }
        }
        // §994
    }

    /// @<Declare act...
    // §1043
    pub fn app_space(&mut self) {
        let mut q: halfword = 0; // §1043
        if ((self.cur_list.aux_field.hh().lh() >= 2000i32) && (self.eqtb[((615795i32) - 1) as usize].hh().rh() != 0i32)) {
            q = self.new_param_glue(13i32);
        } else {
            {
                if (self.eqtb[((615794i32) - 1) as usize].hh().rh() != 0i32) {
                    self.main_p = self.eqtb[((615794i32) - 1) as usize].hh().rh();
                } else {
                    // §1042
                    {
                        self.main_p = self.font_glue[(self.eqtb[((616834i32) - 1) as usize].hh().rh()) as usize];
                        if (self.main_p == 0i32) {
                            {
                                self.main_p = self.new_spec(0i32);
                                self.main_k = (self.param_base[(self.eqtb[((616834i32) - 1) as usize].hh().rh()) as usize]).wrapping_add(2i32);
                                { let __ix773 = (self.main_p).wrapping_add(1i32); let __v774 = self.font_info[(self.main_k) as usize].int(); self.mem[(__ix773) as usize].set_int(__v774); }
                                { let __ix775 = (self.main_p).wrapping_add(2i32); let __v776 = self.font_info[((self.main_k).wrapping_add(1i32)) as usize].int(); self.mem[(__ix775) as usize].set_int(__v776); }
                                { let __ix777 = (self.main_p).wrapping_add(3i32); let __v778 = self.font_info[((self.main_k).wrapping_add(2i32)) as usize].int(); self.mem[(__ix777) as usize].set_int(__v778); }
                                { let __ix779 = self.eqtb[((616834i32) - 1) as usize].hh().rh(); let __v780 = self.main_p; self.font_glue[(__ix779) as usize] = __v780; }
                            }
                        }
                    }
                }
                // §1043
                self.main_p = self.new_spec(self.main_p);
                // §1044
                if (self.cur_list.aux_field.hh().lh() >= 2000i32) {
                    { let __ix781 = (self.main_p).wrapping_add(1i32); let __v782 = (self.mem[((self.main_p).wrapping_add(1i32)) as usize].int()).wrapping_add(self.font_info[((7i32).wrapping_add(self.param_base[(self.eqtb[((616834i32) - 1) as usize].hh().rh()) as usize])) as usize].int()); self.mem[(__ix781) as usize].set_int(__v782); }
                }
                { let __ix783 = (self.main_p).wrapping_add(2i32); let __v784 = self.xn_over_d(self.mem[((self.main_p).wrapping_add(2i32)) as usize].int(), self.cur_list.aux_field.hh().lh(), 1000i32); self.mem[(__ix783) as usize].set_int(__v784); }
                { let __ix785 = (self.main_p).wrapping_add(3i32); let __v786 = self.xn_over_d(self.mem[((self.main_p).wrapping_add(3i32)) as usize].int(), 1000i32, self.cur_list.aux_field.hh().lh()); self.mem[(__ix785) as usize].set_int(__v786); }
                // §1043
                q = self.new_glue(self.main_p);
                { let __ix787 = self.main_p; self.mem[(__ix787) as usize].set_hh_rh(0i32); }
            }
        }
        { let __ix788 = self.cur_list.tail_field; self.mem[(__ix788) as usize].set_hh_rh(q); }
        self.cur_list.tail_field = q;
    }

    /// @<Declare action...
    // §1047
    pub fn insert_dollar_sign(&mut self) {
        self.back_input();
        self.cur_tok = 804i32;
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(1017i32);
        }
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 1018i32;
            self.help_line[(0i32) as usize] = 1019i32;
        }
        self.ins_error();
    }

    /// The ``you_cant`' procedure prints a line saying that the current command
    /// is illegal in the current mode; it identifies these things symbolically.
    /// @<Declare action...
    // §1049
    pub fn you_cant(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(685i32);
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        self.print(1020i32);
        self.print_mode(self.cur_list.mode_field);
    }

}
