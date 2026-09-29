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
    /// The `dvi_four` procedure outputs four bytes in two's complement notation,
    /// without risking arithmetic overflow.
    // §600
    pub fn dvi_four(&mut self, mut x: i32) {
        if (x >= 0i32) {
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = (x / 16777216i32);
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        } else {
            {
                x = (x).wrapping_add(1073741824i32);
                x = (x).wrapping_add(1073741824i32);
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = ((x / 16777216i32)).wrapping_add(128i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        }
        x = (x % 16777216i32);
        {
            self.dvi_buf[(self.dvi_ptr) as usize] = (x / 65536i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        x = (x % 65536i32);
        {
            self.dvi_buf[(self.dvi_ptr) as usize] = (x / 256i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            self.dvi_buf[(self.dvi_ptr) as usize] = (x % 256i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
    }

    /// A mild optimization of the output is performed by the `dvi_pop`
    /// routine, which issues a `pop` unless it is possible to cancel a
    /// ``push` `pop`' pair. The parameter to `dvi_pop` is the byte address
    /// following the old `push` that matches the new `pop`.
    // §601
    pub fn dvi_pop(&mut self, mut l: i32) {
        if ((l == (self.dvi_offset).wrapping_add(self.dvi_ptr)) && (self.dvi_ptr > 0i32)) {
            self.dvi_ptr = (self.dvi_ptr).wrapping_sub(1i32);
        } else {
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = 142i32;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
    }

    /// Here's a procedure that outputs a font definition. Since \TeX82 uses at
    /// most 256 different fonts per job, `fnt_def1` is always used as the command code.
    // §602
    pub fn dvi_font_def(&mut self, mut f: internal_font_number) {
        let mut k: pool_pointer = 0; // §602
        {
            self.dvi_buf[(self.dvi_ptr) as usize] = 243i32;
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            self.dvi_buf[(self.dvi_ptr) as usize] = (f).wrapping_sub(1i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix239 = self.dvi_ptr; let __v240 = (self.font_check[(f) as usize].b0()).wrapping_sub(0i32); self.dvi_buf[(__ix239) as usize] = __v240; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix241 = self.dvi_ptr; let __v242 = (self.font_check[(f) as usize].b1()).wrapping_sub(0i32); self.dvi_buf[(__ix241) as usize] = __v242; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix243 = self.dvi_ptr; let __v244 = (self.font_check[(f) as usize].b2()).wrapping_sub(0i32); self.dvi_buf[(__ix243) as usize] = __v244; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix245 = self.dvi_ptr; let __v246 = (self.font_check[(f) as usize].b3()).wrapping_sub(0i32); self.dvi_buf[(__ix245) as usize] = __v246; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        self.dvi_four(self.font_size[(f) as usize]);
        self.dvi_four(self.font_dsize[(f) as usize]);
        {
            { let __ix247 = self.dvi_ptr; let __v248 = (self.str_start[((self.font_area[(f) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.font_area[(f) as usize]) as usize]); self.dvi_buf[(__ix247) as usize] = __v248; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix249 = self.dvi_ptr; let __v250 = (self.str_start[((self.font_name[(f) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.font_name[(f) as usize]) as usize]); self.dvi_buf[(__ix249) as usize] = __v250; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        // §603
        {
            let __for_end_2 = (self.str_start[((self.font_area[(f) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            k = self.str_start[(self.font_area[(f) as usize]) as usize];
            while k <= __for_end_2 {
                {
                    { let __ix251 = self.dvi_ptr; let __v252 = self.str_pool[(k) as usize]; self.dvi_buf[(__ix251) as usize] = __v252; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[((self.font_name[(f) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            k = self.str_start[(self.font_name[(f) as usize]) as usize];
            while k <= __for_end_2 {
                {
                    { let __ix253 = self.dvi_ptr; let __v254 = self.str_pool[(k) as usize]; self.dvi_buf[(__ix253) as usize] = __v254; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Here is a subroutine that produces a \.{DVI} command for some specified
    /// downward or rightward motion. It has two parameters: `w` is the amount
    /// of motion, and `o` is either `down1` or `right1`. We use the fact that
    /// the command codes have convenient arithmetic properties: `y1-down1=w1-right1`
    /// and `z1-down1=x1-right1`.
    // §607
    pub fn movement(&mut self, mut w: scaled, mut o: eight_bits) {
        let mut mstate: small_number = 0; // §607
        let mut p: halfword = 0; // §607
        let mut q: halfword = 0; // §607
        let mut k: i32 = 0; // §607
        'l_exit_f: {
            'l_found_f: {
                'l_start_of_TEX_f: {
                    'l_L2_f: {
                        'l_not_found_f: {
                            q = self.get_node(3i32);
                            self.mem[((q).wrapping_add(1i32)) as usize].set_int(w);
                            { let __v255 = (self.dvi_offset).wrapping_add(self.dvi_ptr); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v255); }
                            if (o == 157i32) {
                                {
                                    { let __v256 = self.down_ptr; self.mem[(q) as usize].set_hh_rh(__v256); }
                                    self.down_ptr = q;
                                }
                            } else {
                                {
                                    { let __v257 = self.right_ptr; self.mem[(q) as usize].set_hh_rh(__v257); }
                                    self.right_ptr = q;
                                }
                            }
                            // §611
                            p = self.mem[(q) as usize].hh().rh();
                            mstate = 0i32;
                            while (p != 0i32) {
                                {
                                    if (self.mem[((p).wrapping_add(1i32)) as usize].int() == w) {
                                        // §612
                                        match (mstate).wrapping_add(self.mem[(p) as usize].hh().lh()) {
                                            3 | 4 | 15 | 16 => {
                                                if (self.mem[((p).wrapping_add(2i32)) as usize].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §613
                                                    {
                                                        k = (self.mem[((p).wrapping_add(2i32)) as usize].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v258 = (self.dvi_buf[(k) as usize]).wrapping_add(5i32); self.dvi_buf[(k) as usize] = __v258; }
                                                        self.mem[(p) as usize].set_hh_lh(1i32);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            5 | 9 | 11 => {
                                                // §612
                                                if (self.mem[((p).wrapping_add(2i32)) as usize].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §614
                                                    {
                                                        k = (self.mem[((p).wrapping_add(2i32)) as usize].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v259 = (self.dvi_buf[(k) as usize]).wrapping_add(10i32); self.dvi_buf[(k) as usize] = __v259; }
                                                        self.mem[(p) as usize].set_hh_lh(2i32);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            1 | 2 | 8 | 13 => {
                                                // §612
                                                break 'l_found_f;
                                            }
                                            _ => {
                                            }
                                        }
                                    } else {
                                        // §611
                                        match (mstate).wrapping_add(self.mem[(p) as usize].hh().lh()) {
                                            1 => {
                                                mstate = 6i32;
                                            }
                                            2 => {
                                                mstate = 12i32;
                                            }
                                            8 | 13 => {
                                                break 'l_not_found_f;
                                            }
                                            _ => {
                                            }
                                        }
                                    }
                                    p = self.mem[(p) as usize].hh().rh();
                                }
                            }
                        }
                        // §610
                        self.mem[(q) as usize].set_hh_lh(3i32);
                        if ((w).wrapping_abs() >= 8388608i32) {
                            {
                                {
                                    self.dvi_buf[(self.dvi_ptr) as usize] = (o).wrapping_add(3i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                self.dvi_four(w);
                                break 'l_exit_f;
                            }
                        }
                        if ((w).wrapping_abs() >= 32768i32) {
                            {
                                {
                                    self.dvi_buf[(self.dvi_ptr) as usize] = (o).wrapping_add(2i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                if (w < 0i32) {
                                    w = (w).wrapping_add(16777216i32);
                                }
                                {
                                    self.dvi_buf[(self.dvi_ptr) as usize] = (w / 65536i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                w = (w % 65536i32);
                                break 'l_L2_f;
                            }
                        }
                        if ((w).wrapping_abs() >= 128i32) {
                            {
                                {
                                    self.dvi_buf[(self.dvi_ptr) as usize] = (o).wrapping_add(1i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                if (w < 0i32) {
                                    w = (w).wrapping_add(65536i32);
                                }
                                break 'l_L2_f;
                            }
                        }
                        {
                            self.dvi_buf[(self.dvi_ptr) as usize] = o;
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        if (w < 0i32) {
                            w = (w).wrapping_add(256i32);
                        }
                        break 'l_start_of_TEX_f;
                    }
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = (w / 256i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                }
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = (w % 256i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                break 'l_exit_f;
            }
            // §607
            { let __v260 = self.mem[(p) as usize].hh().lh(); self.mem[(q) as usize].set_hh_lh(__v260); }
            // §609
            if (self.mem[(q) as usize].hh().lh() == 1i32) {
                {
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = (o).wrapping_add(4i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    while (self.mem[(q) as usize].hh().rh() != p) {
                        {
                            q = self.mem[(q) as usize].hh().rh();
                            match self.mem[(q) as usize].hh().lh() {
                                3 => {
                                    self.mem[(q) as usize].set_hh_lh(5i32);
                                }
                                4 => {
                                    self.mem[(q) as usize].set_hh_lh(6i32);
                                }
                                _ => {
                                }
                            }
                        }
                    }
                }
            } else {
                {
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = (o).wrapping_add(9i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    while (self.mem[(q) as usize].hh().rh() != p) {
                        {
                            q = self.mem[(q) as usize].hh().rh();
                            match self.mem[(q) as usize].hh().lh() {
                                3 => {
                                    self.mem[(q) as usize].set_hh_lh(4i32);
                                }
                                5 => {
                                    self.mem[(q) as usize].set_hh_lh(6i32);
                                }
                                _ => {
                                }
                            }
                        }
                    }
                }
            }
        }
        // §607
    }

    /// In case you are wondering when all the movement nodes are removed from
    /// \TeX's memory, the answer is that they are recycled just before
    /// `hlist_out` and `vlist_out` finish outputting a box. This restores the
    /// down and right stacks to the state they were in before the box was output,
    /// except that some `info`'s may have become more restrictive.
    // §615
    pub fn prune_movements(&mut self, mut l: i32) {
        let mut p: halfword = 0; // §615
        'l_exit_f: {
            'l_done_f: {
                while (self.down_ptr != 0i32) {
                    {
                        if (self.mem[((self.down_ptr).wrapping_add(2i32)) as usize].int() < l) {
                            break 'l_done_f;
                        }
                        p = self.down_ptr;
                        self.down_ptr = self.mem[(p) as usize].hh().rh();
                        self.free_node(p, 3i32);
                    }
                }
            }
            while (self.right_ptr != 0i32) {
                {
                    if (self.mem[((self.right_ptr).wrapping_add(2i32)) as usize].int() < l) {
                        break 'l_exit_f;
                    }
                    p = self.right_ptr;
                    self.right_ptr = self.mem[(p) as usize].hh().rh();
                    self.free_node(p, 3i32);
                }
            }
        }
    }

    /// After all this preliminary shuffling, we come finally to the routines
    /// that actually send out the requested data. Let's do \.{\\special} first
    /// (it's easier).
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1368
    pub fn special_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1368
        let mut k: pool_pointer = 0; // §1368
        if (self.cur_h != self.dvi_h) {
            {
                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                self.dvi_h = self.cur_h;
            }
        }
        if (self.cur_v != self.dvi_v) {
            {
                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                self.dvi_v = self.cur_v;
            }
        }
        old_setting = self.selector;
        self.selector = 21i32;
        self.show_token_list(self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        if ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) < 256i32) {
            {
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = 239i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix261 = self.dvi_ptr; let __v262 = (self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]); self.dvi_buf[(__ix261) as usize] = __v262; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        } else {
            {
                {
                    self.dvi_buf[(self.dvi_ptr) as usize] = 242i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                self.dvi_four((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]));
            }
        }
        {
            let __for_end_2 = (self.pool_ptr).wrapping_sub(1i32);
            k = self.str_start[(self.str_ptr) as usize];
            while k <= __for_end_2 {
                {
                    { let __ix263 = self.dvi_ptr; let __v264 = self.str_pool[(k) as usize]; self.dvi_buf[(__ix263) as usize] = __v264; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.pool_ptr = self.str_start[(self.str_ptr) as usize];
    }

    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1370
    pub fn write_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1370
        let mut old_mode: i32 = 0; // §1370
        let mut j: small_number = 0; // §1370
        let mut q: halfword = 0; // §1370
        let mut r: halfword = 0; // §1370
        // §1371
        q = self.get_avail();
        self.mem[(q) as usize].set_hh_lh(637i32);
        r = self.get_avail();
        self.mem[(q) as usize].set_hh_rh(r);
        self.mem[(r) as usize].set_hh_lh(6717i32);
        self.begin_token_list(q, 4i32);
        self.begin_token_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(), 15i32);
        q = self.get_avail();
        self.mem[(q) as usize].set_hh_lh(379i32);
        self.begin_token_list(q, 4i32);
        old_mode = self.cur_list.mode_field;
        self.cur_list.mode_field = 0i32;
        self.cur_cs = self.write_loc;
        q = self.scan_toks(false, true);
        self.get_token();
        if (self.cur_tok != 6717i32) {
            // §1372
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(1297i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 1298i32;
                    self.help_line[(0i32) as usize] = 1012i32;
                }
                self.error();
                loop {
                    self.get_token();
                    if (self.cur_tok == 6717i32) { break; }
                }
            }
        }
        // §1371
        self.cur_list.mode_field = old_mode;
        self.end_token_list();
        // §1370
        old_setting = self.selector;
        j = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
        if self.write_open[(j) as usize] {
            self.selector = j;
        } else {
            {
                if ((j == 17i32) && (self.selector == 19i32)) {
                    self.selector = 18i32;
                }
                self.print_nl(338i32);
            }
        }
        self.token_show(self.def_ref);
        self.print_ln();
        self.flush_list(self.def_ref);
        self.selector = old_setting;
    }

    /// The `out_what` procedure takes care of outputting whatsit nodes for
    /// `vlist_out` and `hlist_out`\kern-.3pt.
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1373
    pub fn out_what(&mut self, mut p: halfword) {
        let mut j: small_number = 0; // §1373
        match self.mem[(p) as usize].hh().b1() {
            0 | 1 | 2 => {
                // §1374
                if (!self.doing_leaders) {
                    {
                        j = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                        if (self.mem[(p) as usize].hh().b1() == 1i32) {
                            self.write_out(p);
                        } else {
                            {
                                if self.write_open[(j) as usize] {
                                    { let mut __f = ::core::mem::take(&mut self.write_file[(j) as usize]); let __r = self.a_close(&mut __f); self.write_file[(j) as usize] = __f; __r };
                                }
                                if (self.mem[(p) as usize].hh().b1() == 2i32) {
                                    { let __v265 = false; self.write_open[(j) as usize] = __v265; }
                                } else {
                                    if (j < 16i32) {
                                        {
                                            self.cur_name = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                            self.cur_area = self.mem[((p).wrapping_add(2i32)) as usize].hh().lh();
                                            self.cur_ext = self.mem[((p).wrapping_add(2i32)) as usize].hh().rh();
                                            if (self.cur_ext == 338i32) {
                                                self.cur_ext = 791i32;
                                            }
                                            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                                            while (!{ let mut __f = ::core::mem::take(&mut self.write_file[(j) as usize]); let __r = self.a_open_out(&mut __f); self.write_file[(j) as usize] = __f; __r }) {
                                                self.prompt_file_name(1300i32, 791i32);
                                            }
                                            { let __v266 = true; self.write_open[(j) as usize] = __v266; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            3 => {
                // §1373
                self.special_out(p);
            }
            4 => {
            }
            _ => {
                self.confusion(1299i32);
            }
        }
    }

    /// The recursive procedures `hlist_out` and `vlist_out` each have local variables
    /// `save_h` and `save_v` to hold the values of `dvi_h` and `dvi_v` just before
    /// entering a new level of recursion.  In effect, the values of `save_h` and
    /// `save_v` on \TeX's run-time stack correspond to the values of `h` and `v`
    /// that a \.{DVI}-reading program will push onto its coordinate stack.
    // §619
    pub fn hlist_out(&mut self) {
        let mut base_line: scaled = 0; // §619
        let mut left_edge: scaled = 0; // §619
        let mut save_h: scaled = 0; // §619
        let mut save_v: scaled = 0; // §619
        let mut this_box: halfword = 0; // §619
        let mut g_order: glue_ord = 0; // §619
        let mut g_sign: i32 = 0; // §619
        let mut p: halfword = 0; // §619
        let mut save_loc: i32 = 0; // §619
        let mut leader_box: halfword = 0; // §619
        let mut leader_wd: scaled = 0; // §619
        let mut lx: scaled = 0; // §619
        let mut outer_doing_leaders: bool = false; // §619
        let mut edge: scaled = 0; // §619
        let mut glue_temp: f64 = 0.0; // §619
        let mut cur_glue: f64 = 0.0; // §619
        let mut cur_g: scaled = 0; // §619
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().b1();
        g_sign = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().b0();
        p = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        if (self.cur_s > 0i32) {
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = 141i32;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
        if (self.cur_s > self.max_push) {
            self.max_push = self.cur_s;
        }
        save_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
        base_line = self.cur_v;
        left_edge = self.cur_h;
        while (p != 0i32) {
            'l_reswitch_b: loop {
                // §620
                if (p >= self.hi_mem_min) {
                    {
                        if (self.cur_h != self.dvi_h) {
                            {
                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                                self.dvi_h = self.cur_h;
                            }
                        }
                        if (self.cur_v != self.dvi_v) {
                            {
                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                self.dvi_v = self.cur_v;
                            }
                        }
                        loop {
                            self.f = self.mem[(p) as usize].hh().b0();
                            self.c = self.mem[(p) as usize].hh().b1();
                            if (self.f != self.dvi_f) {
                                // §621
                                {
                                    if (!self.font_used[(self.f) as usize]) {
                                        {
                                            self.dvi_font_def(self.f);
                                            { let __ix267 = self.f; let __v268 = true; self.font_used[(__ix267) as usize] = __v268; }
                                        }
                                    }
                                    if (self.f <= 64i32) {
                                        {
                                            { let __ix269 = self.dvi_ptr; let __v270 = (self.f).wrapping_add(170i32); self.dvi_buf[(__ix269) as usize] = __v270; }
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                    } else {
                                        {
                                            {
                                                self.dvi_buf[(self.dvi_ptr) as usize] = 235i32;
                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                if (self.dvi_ptr == self.dvi_limit) {
                                                    self.dvi_swap();
                                                }
                                            }
                                            {
                                                { let __ix271 = self.dvi_ptr; let __v272 = (self.f).wrapping_sub(1i32); self.dvi_buf[(__ix271) as usize] = __v272; }
                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                if (self.dvi_ptr == self.dvi_limit) {
                                                    self.dvi_swap();
                                                }
                                            }
                                        }
                                    }
                                    self.dvi_f = self.f;
                                }
                            }
                            // §620
                            if (self.c >= 128i32) {
                                {
                                    self.dvi_buf[(self.dvi_ptr) as usize] = 128i32;
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                            }
                            {
                                { let __ix273 = self.dvi_ptr; let __v274 = (self.c).wrapping_sub(0i32); self.dvi_buf[(__ix273) as usize] = __v274; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            self.cur_h = (self.cur_h).wrapping_add(self.font_info[((self.width_base[(self.f) as usize]).wrapping_add(self.font_info[((self.char_base[(self.f) as usize]).wrapping_add(self.c)) as usize].qqqq().b0())) as usize].int());
                            p = self.mem[(p) as usize].hh().rh();
                            if (!(p >= self.hi_mem_min)) { break; }
                        }
                        self.dvi_h = self.cur_h;
                    }
                } else {
                    // §622
                    {
                        'l_L15_f: {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[(p) as usize].hh().b0() {
                                        0 | 1 => {
                                            // §623
                                            if (self.mem[((p).wrapping_add(5i32)) as usize].hh().rh() == 0i32) {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                            } else {
                                                {
                                                    save_h = self.dvi_h;
                                                    save_v = self.dvi_v;
                                                    self.cur_v = (base_line).wrapping_add(self.mem[((p).wrapping_add(4i32)) as usize].int());
                                                    self.temp_ptr = p;
                                                    edge = self.cur_h;
                                                    if (self.mem[(p) as usize].hh().b0() == 1i32) {
                                                        self.vlist_out();
                                                    } else {
                                                        self.hlist_out();
                                                    }
                                                    self.dvi_h = save_h;
                                                    self.dvi_v = save_v;
                                                    self.cur_h = (edge).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                                    self.cur_v = base_line;
                                                }
                                            }
                                        }
                                        2 => {
                                            // §622
                                            {
                                                self.rule_ht = self.mem[((p).wrapping_add(3i32)) as usize].int();
                                                self.rule_dp = self.mem[((p).wrapping_add(2i32)) as usize].int();
                                                self.rule_wd = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        8 => {
                                            // §1367
                                            self.out_what(p);
                                        }
                                        10 => {
                                            // §625
                                            {
                                                self.g = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                                self.rule_wd = (self.mem[((self.g).wrapping_add(1i32)) as usize].int()).wrapping_sub(cur_g);
                                                if (g_sign != 0i32) {
                                                    {
                                                        if (g_sign == 1i32) {
                                                            {
                                                                if (self.mem[(self.g) as usize].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((self.mem[((self.g).wrapping_add(2i32)) as usize].int()) as f64));
                                                                        glue_temp = (((self.mem[((this_box).wrapping_add(6i32)) as usize].gr()) as f64) * cur_glue);
                                                                        if (glue_temp > 1000000000.0f64) {
                                                                            glue_temp = 1000000000.0f64;
                                                                        } else {
                                                                            if (glue_temp < (-1000000000.0f64)) {
                                                                                glue_temp = (-1000000000.0f64);
                                                                            }
                                                                        }
                                                                        cur_g = crate::system::pas_round(glue_temp);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            if (self.mem[(self.g) as usize].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((self.mem[((self.g).wrapping_add(3i32)) as usize].int()) as f64));
                                                                    glue_temp = (((self.mem[((this_box).wrapping_add(6i32)) as usize].gr()) as f64) * cur_glue);
                                                                    if (glue_temp > 1000000000.0f64) {
                                                                        glue_temp = 1000000000.0f64;
                                                                    } else {
                                                                        if (glue_temp < (-1000000000.0f64)) {
                                                                            glue_temp = (-1000000000.0f64);
                                                                        }
                                                                    }
                                                                    cur_g = crate::system::pas_round(glue_temp);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                self.rule_wd = (self.rule_wd).wrapping_add(cur_g);
                                                if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                                    // §626
                                                    {
                                                        leader_box = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                                        if (self.mem[(leader_box) as usize].hh().b0() == 2i32) {
                                                            {
                                                                self.rule_ht = self.mem[((leader_box).wrapping_add(3i32)) as usize].int();
                                                                self.rule_dp = self.mem[((leader_box).wrapping_add(2i32)) as usize].int();
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_wd = self.mem[((leader_box).wrapping_add(1i32)) as usize].int();
                                                        if ((leader_wd > 0i32) && (self.rule_wd > 0i32)) {
                                                            {
                                                                self.rule_wd = (self.rule_wd).wrapping_add(10i32);
                                                                edge = (self.cur_h).wrapping_add(self.rule_wd);
                                                                lx = 0i32;
                                                                // §627
                                                                if (self.mem[(p) as usize].hh().b1() == 100i32) {
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.cur_h = (left_edge).wrapping_add((leader_wd).wrapping_mul(((self.cur_h).wrapping_sub(left_edge) / leader_wd)));
                                                                        if (self.cur_h < save_h) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_wd / leader_wd);
                                                                        self.lr = (self.rule_wd % leader_wd);
                                                                        if (self.mem[(p) as usize].hh().b1() == 101i32) {
                                                                            self.cur_h = (self.cur_h).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_h = (self.cur_h).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §626
                                                                while ((self.cur_h).wrapping_add(leader_wd) <= edge) {
                                                                    // §628
                                                                    {
                                                                        self.cur_v = (base_line).wrapping_add(self.mem[((leader_box).wrapping_add(4i32)) as usize].int());
                                                                        if (self.cur_v != self.dvi_v) {
                                                                            {
                                                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                                                                self.dvi_v = self.cur_v;
                                                                            }
                                                                        }
                                                                        save_v = self.dvi_v;
                                                                        if (self.cur_h != self.dvi_h) {
                                                                            {
                                                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                                                                                self.dvi_h = self.cur_h;
                                                                            }
                                                                        }
                                                                        save_h = self.dvi_h;
                                                                        self.temp_ptr = leader_box;
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[(leader_box) as usize].hh().b0() == 1i32) {
                                                                            self.vlist_out();
                                                                        } else {
                                                                            self.hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.dvi_v = save_v;
                                                                        self.dvi_h = save_h;
                                                                        self.cur_v = base_line;
                                                                        self.cur_h = ((save_h).wrapping_add(leader_wd)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §626
                                                                self.cur_h = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §625
                                                break 'l_L13_f;
                                            }
                                        }
                                        11 | 9 => {
                                            // §622
                                            self.cur_h = (self.cur_h).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        }
                                        6 => {
                                            // §652
                                            {
                                                { let __v275 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[(29988i32) as usize] = __v275; }
                                                { let __v276 = self.mem[(p) as usize].hh().rh(); self.mem[(29988i32) as usize].set_hh_rh(__v276); }
                                                p = 29988i32;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        _ => {
                                            // §622
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_ht == (1073741824i32).wrapping_neg()) {
                                    // §624
                                    self.rule_ht = self.mem[((this_box).wrapping_add(3i32)) as usize].int();
                                }
                                if (self.rule_dp == (1073741824i32).wrapping_neg()) {
                                    self.rule_dp = self.mem[((this_box).wrapping_add(2i32)) as usize].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_h != self.dvi_h) {
                                            {
                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                                                self.dvi_h = self.cur_h;
                                            }
                                        }
                                        self.cur_v = (base_line).wrapping_add(self.rule_dp);
                                        if (self.cur_v != self.dvi_v) {
                                            {
                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                                self.dvi_v = self.cur_v;
                                            }
                                        }
                                        {
                                            self.dvi_buf[(self.dvi_ptr) as usize] = 132i32;
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                        self.dvi_four(self.rule_ht);
                                        self.dvi_four(self.rule_wd);
                                        self.cur_v = base_line;
                                        self.dvi_h = (self.dvi_h).wrapping_add(self.rule_wd);
                                    }
                                }
                            }
                            // §622
                            self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                        }
                        p = self.mem[(p) as usize].hh().rh();
                    }
                }
                break 'l_reswitch_b;
            }
        }
        // §619
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `vlist_out` routine is similar to `hlist_out`, but a bit simpler.
    // §629
    pub fn vlist_out(&mut self) {
        let mut left_edge: scaled = 0; // §629
        let mut top_edge: scaled = 0; // §629
        let mut save_h: scaled = 0; // §629
        let mut save_v: scaled = 0; // §629
        let mut this_box: halfword = 0; // §629
        let mut g_order: glue_ord = 0; // §629
        let mut g_sign: i32 = 0; // §629
        let mut p: halfword = 0; // §629
        let mut save_loc: i32 = 0; // §629
        let mut leader_box: halfword = 0; // §629
        let mut leader_ht: scaled = 0; // §629
        let mut lx: scaled = 0; // §629
        let mut outer_doing_leaders: bool = false; // §629
        let mut edge: scaled = 0; // §629
        let mut glue_temp: f64 = 0.0; // §629
        let mut cur_glue: f64 = 0.0; // §629
        let mut cur_g: scaled = 0; // §629
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().b1();
        g_sign = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().b0();
        p = self.mem[((this_box).wrapping_add(5i32)) as usize].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        if (self.cur_s > 0i32) {
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = 141i32;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
        if (self.cur_s > self.max_push) {
            self.max_push = self.cur_s;
        }
        save_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
        left_edge = self.cur_h;
        self.cur_v = (self.cur_v).wrapping_sub(self.mem[((this_box).wrapping_add(3i32)) as usize].int());
        top_edge = self.cur_v;
        while (p != 0i32) {
            // §630
            {
                'l_L15_f: {
                    if (p >= self.hi_mem_min) {
                        self.confusion(828i32);
                    } else {
                        // §631
                        {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[(p) as usize].hh().b0() {
                                        0 | 1 => {
                                            // §632
                                            if (self.mem[((p).wrapping_add(5i32)) as usize].hh().rh() == 0i32) {
                                                self.cur_v = ((self.cur_v).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int())).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int());
                                            } else {
                                                {
                                                    self.cur_v = (self.cur_v).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int());
                                                    if (self.cur_v != self.dvi_v) {
                                                        {
                                                            self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                                            self.dvi_v = self.cur_v;
                                                        }
                                                    }
                                                    save_h = self.dvi_h;
                                                    save_v = self.dvi_v;
                                                    self.cur_h = (left_edge).wrapping_add(self.mem[((p).wrapping_add(4i32)) as usize].int());
                                                    self.temp_ptr = p;
                                                    if (self.mem[(p) as usize].hh().b0() == 1i32) {
                                                        self.vlist_out();
                                                    } else {
                                                        self.hlist_out();
                                                    }
                                                    self.dvi_h = save_h;
                                                    self.dvi_v = save_v;
                                                    self.cur_v = (save_v).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int());
                                                    self.cur_h = left_edge;
                                                }
                                            }
                                        }
                                        2 => {
                                            // §631
                                            {
                                                self.rule_ht = self.mem[((p).wrapping_add(3i32)) as usize].int();
                                                self.rule_dp = self.mem[((p).wrapping_add(2i32)) as usize].int();
                                                self.rule_wd = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        8 => {
                                            // §1366
                                            self.out_what(p);
                                        }
                                        10 => {
                                            // §634
                                            {
                                                self.g = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                                self.rule_ht = (self.mem[((self.g).wrapping_add(1i32)) as usize].int()).wrapping_sub(cur_g);
                                                if (g_sign != 0i32) {
                                                    {
                                                        if (g_sign == 1i32) {
                                                            {
                                                                if (self.mem[(self.g) as usize].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((self.mem[((self.g).wrapping_add(2i32)) as usize].int()) as f64));
                                                                        glue_temp = (((self.mem[((this_box).wrapping_add(6i32)) as usize].gr()) as f64) * cur_glue);
                                                                        if (glue_temp > 1000000000.0f64) {
                                                                            glue_temp = 1000000000.0f64;
                                                                        } else {
                                                                            if (glue_temp < (-1000000000.0f64)) {
                                                                                glue_temp = (-1000000000.0f64);
                                                                            }
                                                                        }
                                                                        cur_g = crate::system::pas_round(glue_temp);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            if (self.mem[(self.g) as usize].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((self.mem[((self.g).wrapping_add(3i32)) as usize].int()) as f64));
                                                                    glue_temp = (((self.mem[((this_box).wrapping_add(6i32)) as usize].gr()) as f64) * cur_glue);
                                                                    if (glue_temp > 1000000000.0f64) {
                                                                        glue_temp = 1000000000.0f64;
                                                                    } else {
                                                                        if (glue_temp < (-1000000000.0f64)) {
                                                                            glue_temp = (-1000000000.0f64);
                                                                        }
                                                                    }
                                                                    cur_g = crate::system::pas_round(glue_temp);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                self.rule_ht = (self.rule_ht).wrapping_add(cur_g);
                                                if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                                    // §635
                                                    {
                                                        leader_box = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                                        if (self.mem[(leader_box) as usize].hh().b0() == 2i32) {
                                                            {
                                                                self.rule_wd = self.mem[((leader_box).wrapping_add(1i32)) as usize].int();
                                                                self.rule_dp = 0i32;
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_ht = (self.mem[((leader_box).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((leader_box).wrapping_add(2i32)) as usize].int());
                                                        if ((leader_ht > 0i32) && (self.rule_ht > 0i32)) {
                                                            {
                                                                self.rule_ht = (self.rule_ht).wrapping_add(10i32);
                                                                edge = (self.cur_v).wrapping_add(self.rule_ht);
                                                                lx = 0i32;
                                                                // §636
                                                                if (self.mem[(p) as usize].hh().b1() == 100i32) {
                                                                    {
                                                                        save_v = self.cur_v;
                                                                        self.cur_v = (top_edge).wrapping_add((leader_ht).wrapping_mul(((self.cur_v).wrapping_sub(top_edge) / leader_ht)));
                                                                        if (self.cur_v < save_v) {
                                                                            self.cur_v = (self.cur_v).wrapping_add(leader_ht);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_ht / leader_ht);
                                                                        self.lr = (self.rule_ht % leader_ht);
                                                                        if (self.mem[(p) as usize].hh().b1() == 101i32) {
                                                                            self.cur_v = (self.cur_v).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_v = (self.cur_v).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §635
                                                                while ((self.cur_v).wrapping_add(leader_ht) <= edge) {
                                                                    // §637
                                                                    {
                                                                        self.cur_h = (left_edge).wrapping_add(self.mem[((leader_box).wrapping_add(4i32)) as usize].int());
                                                                        if (self.cur_h != self.dvi_h) {
                                                                            {
                                                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                                                                                self.dvi_h = self.cur_h;
                                                                            }
                                                                        }
                                                                        save_h = self.dvi_h;
                                                                        self.cur_v = (self.cur_v).wrapping_add(self.mem[((leader_box).wrapping_add(3i32)) as usize].int());
                                                                        if (self.cur_v != self.dvi_v) {
                                                                            {
                                                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                                                                self.dvi_v = self.cur_v;
                                                                            }
                                                                        }
                                                                        save_v = self.dvi_v;
                                                                        self.temp_ptr = leader_box;
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[(leader_box) as usize].hh().b0() == 1i32) {
                                                                            self.vlist_out();
                                                                        } else {
                                                                            self.hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.dvi_v = save_v;
                                                                        self.dvi_h = save_h;
                                                                        self.cur_h = left_edge;
                                                                        self.cur_v = (((save_v).wrapping_sub(self.mem[((leader_box).wrapping_add(3i32)) as usize].int())).wrapping_add(leader_ht)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §635
                                                                self.cur_v = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §634
                                                break 'l_L13_f;
                                            }
                                        }
                                        11 => {
                                            // §631
                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        }
                                        _ => {
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_wd == (1073741824i32).wrapping_neg()) {
                                    // §633
                                    self.rule_wd = self.mem[((this_box).wrapping_add(1i32)) as usize].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_h != self.dvi_h) {
                                            {
                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), 143i32);
                                                self.dvi_h = self.cur_h;
                                            }
                                        }
                                        if (self.cur_v != self.dvi_v) {
                                            {
                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), 157i32);
                                                self.dvi_v = self.cur_v;
                                            }
                                        }
                                        {
                                            self.dvi_buf[(self.dvi_ptr) as usize] = 137i32;
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                        self.dvi_four(self.rule_ht);
                                        self.dvi_four(self.rule_wd);
                                    }
                                }
                                break 'l_L15_f;
                            }
                            // §631
                            self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                        }
                    }
                }
                // §630
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        // §629
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `hlist_out` and `vlist_out` procedures are now complete, so we are
    /// ready for the `ship_out` routine that gets them started in the first place.
    // §638
    pub fn ship_out(&mut self, mut p: halfword) {
        let mut page_loc: i32 = 0; // §638
        let mut j: i32 = 0; // §638
        let mut k: i32 = 0; // §638
        let mut s: pool_pointer = 0; // §638
        let mut old_setting: i32 = 0; // §638
        'l_done_f: {
            if (self.eqtb[((5297i32) - 1) as usize].int() > 0i32) {
                {
                    self.print_nl(338i32);
                    self.print_ln();
                    self.print(829i32);
                }
            }
            if (self.term_offset > (max_print_line).wrapping_sub(9i32)) {
                self.print_ln();
            } else {
                if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                    self.print_char(32i32);
                }
            }
            self.print_char(91i32);
            j = 9i32;
            while ((self.eqtb[(((5318i32).wrapping_add(j)) - 1) as usize].int() == 0i32) && (j > 0i32)) {
                j = (j).wrapping_sub(1i32);
            }
            {
                let __for_end_3 = j;
                k = 0i32;
                while k <= __for_end_3 {
                    {
                        self.print_int(self.eqtb[(((5318i32).wrapping_add(k)) - 1) as usize].int());
                        if (k < j) {
                            self.print_char(46i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            crate::system::break_out(&mut self.term_out);
            if (self.eqtb[((5297i32) - 1) as usize].int() > 0i32) {
                {
                    self.print_char(93i32);
                    self.begin_diagnostic();
                    self.show_box(p);
                    self.end_diagnostic(true);
                }
            }
            // §641
            if ((((self.mem[((p).wrapping_add(3i32)) as usize].int() > 1073741823i32) || (self.mem[((p).wrapping_add(2i32)) as usize].int() > 1073741823i32)) || (((self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int())).wrapping_add(self.eqtb[((5849i32) - 1) as usize].int()) > 1073741823i32)) || ((self.mem[((p).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5848i32) - 1) as usize].int()) > 1073741823i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(833i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 834i32;
                        self.help_line[(0i32) as usize] = 835i32;
                    }
                    self.error();
                    if (self.eqtb[((5297i32) - 1) as usize].int() <= 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(836i32);
                            self.show_box(p);
                            self.end_diagnostic(true);
                        }
                    }
                    break 'l_done_f;
                }
            }
            if (((self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int())).wrapping_add(self.eqtb[((5849i32) - 1) as usize].int()) > self.max_v) {
                self.max_v = ((self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int())).wrapping_add(self.eqtb[((5849i32) - 1) as usize].int());
            }
            if ((self.mem[((p).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5848i32) - 1) as usize].int()) > self.max_h) {
                self.max_h = (self.mem[((p).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5848i32) - 1) as usize].int());
            }
            // §617
            self.dvi_h = 0i32;
            self.dvi_v = 0i32;
            self.cur_h = self.eqtb[((5848i32) - 1) as usize].int();
            self.dvi_f = 0i32;
            if (self.output_file_name == 0i32) {
                {
                    if (self.job_name == 0i32) {
                        self.open_log_file();
                    }
                    self.pack_job_name(794i32);
                    while (!{ let mut __f = ::core::mem::take(&mut self.dvi_file); let __r = self.b_open_out(&mut __f); self.dvi_file = __f; __r }) {
                        self.prompt_file_name(795i32, 794i32);
                    }
                    self.output_file_name = { let mut __f = ::core::mem::take(&mut self.dvi_file); let __r = self.b_make_name_string(&mut __f); self.dvi_file = __f; __r };
                }
            }
            if (self.total_pages == 0i32) {
                {
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = 247i32;
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        self.dvi_buf[(self.dvi_ptr) as usize] = 2i32;
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    self.dvi_four(25400000i32);
                    self.dvi_four(473628672i32);
                    self.prepare_mag();
                    self.dvi_four(self.eqtb[((5280i32) - 1) as usize].int());
                    old_setting = self.selector;
                    self.selector = 21i32;
                    self.print(827i32);
                    self.print_int(self.eqtb[((5286i32) - 1) as usize].int());
                    self.print_char(46i32);
                    self.print_two(self.eqtb[((5285i32) - 1) as usize].int());
                    self.print_char(46i32);
                    self.print_two(self.eqtb[((5284i32) - 1) as usize].int());
                    self.print_char(58i32);
                    self.print_two((self.eqtb[((5283i32) - 1) as usize].int() / 60i32));
                    self.print_two((self.eqtb[((5283i32) - 1) as usize].int() % 60i32));
                    self.selector = old_setting;
                    {
                        { let __ix277 = self.dvi_ptr; let __v278 = (self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]); self.dvi_buf[(__ix277) as usize] = __v278; }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        let __for_end_5 = (self.pool_ptr).wrapping_sub(1i32);
                        s = self.str_start[(self.str_ptr) as usize];
                        while s <= __for_end_5 {
                            {
                                { let __ix279 = self.dvi_ptr; let __v280 = self.str_pool[(s) as usize]; self.dvi_buf[(__ix279) as usize] = __v280; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            s = s.wrapping_add(1);
                        }
                    }
                    self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                }
            }
            // §640
            page_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = 139i32;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            {
                let __for_end_3 = 9i32;
                k = 0i32;
                while k <= __for_end_3 {
                    self.dvi_four(self.eqtb[(((5318i32).wrapping_add(k)) - 1) as usize].int());
                    k = k.wrapping_add(1);
                }
            }
            self.dvi_four(self.last_bop);
            self.last_bop = page_loc;
            self.cur_v = (self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.eqtb[((5849i32) - 1) as usize].int());
            self.temp_ptr = p;
            if (self.mem[(p) as usize].hh().b0() == 1i32) {
                self.vlist_out();
            } else {
                self.hlist_out();
            }
            {
                self.dvi_buf[(self.dvi_ptr) as usize] = 140i32;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            self.total_pages = (self.total_pages).wrapping_add(1i32);
            self.cur_s = (1i32).wrapping_neg();
        }
        // §638
        if (self.eqtb[((5297i32) - 1) as usize].int() <= 0i32) {
            self.print_char(93i32);
        }
        self.dead_cycles = 0i32;
        crate::system::break_out(&mut self.term_out);
        // §639
        if (self.eqtb[((5294i32) - 1) as usize].int() > 1i32) {
            {
                self.print_nl(830i32);
                self.print_int(self.var_used);
                self.print_char(38i32);
                self.print_int(self.dyn_used);
                self.print_char(59i32);
            }
        }
        self.flush_node_list(p);
        if (self.eqtb[((5294i32) - 1) as usize].int() > 1i32) {
            {
                self.print(831i32);
                self.print_int(self.var_used);
                self.print_char(38i32);
                self.print_int(self.dyn_used);
                self.print(832i32);
                self.print_int(((self.hi_mem_min).wrapping_sub(self.lo_mem_max)).wrapping_sub(1i32));
                self.print_ln();
            }
        }
    }

    /// The parameters to `hpack` and `vpack` correspond to \TeX's primitives
    /// like `\.{\\hbox} \.{to} \.{300pt}', `\.{\\hbox} \.{spread} \.{10pt}'; note
    /// that `\.{\\hbox}' with no dimension following it is equivalent to
    /// `\.{\\hbox} \.{spread} \.{0pt}'.  The `scan_spec` subroutine scans such
    /// constructions in the user's input, including the mandatory left brace that
    /// follows them, and it puts the specification onto `save_stack` so that the
    /// desired box can later be obtained by executing the following code:
    /// $$\vbox{\halign{#\hfil\cr
    /// `save_ptr:=save_ptr-2;`\cr
    /// `hpack(p,saved(1),saved(0)).`\cr}}$$
    /// Special care is necessary to ensure that the special `save_stack` codes
    /// are placed just below the new group code, because scanning can change
    /// `save_stack` when \.{\\csname} appears.
    // §645
    pub fn scan_spec(&mut self, mut c: group_code, mut three_codes: bool) {
        let mut s: i32 = 0; // §645
        let mut spec_code: i32 = 0; // §645
        'l_found_f: {
            if three_codes {
                s = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int();
            }
            if self.scan_keyword(842i32) {
                spec_code = 0i32;
            } else {
                if self.scan_keyword(843i32) {
                    spec_code = 1i32;
                } else {
                    {
                        spec_code = 1i32;
                        self.cur_val = 0i32;
                        break 'l_found_f;
                    }
                }
            }
            self.scan_dimen(false, false, false);
        }
        if three_codes {
            {
                { let __ix281 = (self.save_ptr).wrapping_add(0i32); self.save_stack[(__ix281) as usize].set_int(s); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix282 = (self.save_ptr).wrapping_add(0i32); self.save_stack[(__ix282) as usize].set_int(spec_code); }
        { let __ix283 = (self.save_ptr).wrapping_add(1i32); let __v284 = self.cur_val; self.save_stack[(__ix283) as usize].set_int(__v284); }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(c);
        self.scan_left_brace();
    }

    /// Here now is `hpack`, which contains few if any surprises.
    // §649
    pub fn hpack(&mut self, mut p: halfword, mut w: scaled, mut m: small_number) -> halfword {
        let mut hpack: halfword = 0;
        let mut r: halfword = 0; // §649
        let mut q: halfword = 0; // §649
        let mut h: scaled = 0; // §649
        let mut d: scaled = 0; // §649
        let mut x: scaled = 0; // §649
        let mut s: scaled = 0; // §649
        let mut g: halfword = 0; // §649
        let mut o: glue_ord = 0; // §649
        let mut f: internal_font_number = 0; // §649
        let mut i: four_quarters = four_quarters::default(); // §649
        let mut hd: eight_bits = 0; // §649
        'l_exit_f: {
            'l_common_ending_f: {
                self.last_badness = 0i32;
                r = self.get_node(7i32);
                self.mem[(r) as usize].set_hh_b0(0i32);
                self.mem[(r) as usize].set_hh_b1(0i32);
                self.mem[((r).wrapping_add(4i32)) as usize].set_int(0i32);
                q = (r).wrapping_add(5i32);
                self.mem[(q) as usize].set_hh_rh(p);
                h = 0i32;
                // §650
                d = 0i32;
                x = 0i32;
                self.total_stretch[(0i32) as usize] = 0i32;
                self.total_shrink[(0i32) as usize] = 0i32;
                self.total_stretch[(1i32) as usize] = 0i32;
                self.total_shrink[(1i32) as usize] = 0i32;
                self.total_stretch[(2i32) as usize] = 0i32;
                self.total_shrink[(2i32) as usize] = 0i32;
                self.total_stretch[(3i32) as usize] = 0i32;
                self.total_shrink[(3i32) as usize] = 0i32;
                // §649
                while (p != 0i32) {
                    // §651
                    {
                        'l_reswitch_b: loop {
                            while (p >= self.hi_mem_min) {
                                // §654
                                {
                                    f = self.mem[(p) as usize].hh().b0();
                                    i = self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq();
                                    hd = (i.b1()).wrapping_sub(0i32);
                                    x = (x).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(i.b0())) as usize].int());
                                    s = self.font_info[((self.height_base[(f) as usize]).wrapping_add((hd / 16i32))) as usize].int();
                                    if (s > h) {
                                        h = s;
                                    }
                                    s = self.font_info[((self.depth_base[(f) as usize]).wrapping_add((hd % 16i32))) as usize].int();
                                    if (s > d) {
                                        d = s;
                                    }
                                    p = self.mem[(p) as usize].hh().rh();
                                }
                            }
                            // §651
                            if (p != 0i32) {
                                {
                                    match self.mem[(p) as usize].hh().b0() {
                                        0 | 1 | 2 | 13 => {
                                            // §653
                                            {
                                                x = (x).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                                if (self.mem[(p) as usize].hh().b0() >= 2i32) {
                                                    s = 0i32;
                                                } else {
                                                    s = self.mem[((p).wrapping_add(4i32)) as usize].int();
                                                }
                                                if ((self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_sub(s) > h) {
                                                    h = (self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_sub(s);
                                                }
                                                if ((self.mem[((p).wrapping_add(2i32)) as usize].int()).wrapping_add(s) > d) {
                                                    d = (self.mem[((p).wrapping_add(2i32)) as usize].int()).wrapping_add(s);
                                                }
                                            }
                                        }
                                        3 | 4 | 5 => {
                                            // §651
                                            if (self.adjust_tail != 0i32) {
                                                // §655
                                                {
                                                    while (self.mem[(q) as usize].hh().rh() != p) {
                                                        q = self.mem[(q) as usize].hh().rh();
                                                    }
                                                    if (self.mem[(p) as usize].hh().b0() == 5i32) {
                                                        {
                                                            { let __ix285 = self.adjust_tail; let __v286 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[(__ix285) as usize].set_hh_rh(__v286); }
                                                            while (self.mem[(self.adjust_tail) as usize].hh().rh() != 0i32) {
                                                                self.adjust_tail = self.mem[(self.adjust_tail) as usize].hh().rh();
                                                            }
                                                            p = self.mem[(p) as usize].hh().rh();
                                                            self.free_node(self.mem[(q) as usize].hh().rh(), 2i32);
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix287 = self.adjust_tail; self.mem[(__ix287) as usize].set_hh_rh(p); }
                                                            self.adjust_tail = p;
                                                            p = self.mem[(p) as usize].hh().rh();
                                                        }
                                                    }
                                                    self.mem[(q) as usize].set_hh_rh(p);
                                                    p = q;
                                                }
                                            }
                                        }
                                        8 => {
                                            // §651
                                        }
                                        10 => {
                                            // §656
                                            {
                                                g = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                                x = (x).wrapping_add(self.mem[((g).wrapping_add(1i32)) as usize].int());
                                                o = self.mem[(g) as usize].hh().b0();
                                                { let __v288 = (self.total_stretch[(o) as usize]).wrapping_add(self.mem[((g).wrapping_add(2i32)) as usize].int()); self.total_stretch[(o) as usize] = __v288; }
                                                o = self.mem[(g) as usize].hh().b1();
                                                { let __v289 = (self.total_shrink[(o) as usize]).wrapping_add(self.mem[((g).wrapping_add(3i32)) as usize].int()); self.total_shrink[(o) as usize] = __v289; }
                                                if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                                    {
                                                        g = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                                        if (self.mem[((g).wrapping_add(3i32)) as usize].int() > h) {
                                                            h = self.mem[((g).wrapping_add(3i32)) as usize].int();
                                                        }
                                                        if (self.mem[((g).wrapping_add(2i32)) as usize].int() > d) {
                                                            d = self.mem[((g).wrapping_add(2i32)) as usize].int();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        11 | 9 => {
                                            // §651
                                            x = (x).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        }
                                        6 => {
                                            // §652
                                            {
                                                { let __v290 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[(29988i32) as usize] = __v290; }
                                                { let __v291 = self.mem[(p) as usize].hh().rh(); self.mem[(29988i32) as usize].set_hh_rh(__v291); }
                                                p = 29988i32;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        _ => {
                                            // §651
                                        }
                                    }
                                    p = self.mem[(p) as usize].hh().rh();
                                }
                            }
                            break 'l_reswitch_b;
                        }
                    }
                }
                // §649
                if (self.adjust_tail != 0i32) {
                    { let __ix292 = self.adjust_tail; self.mem[(__ix292) as usize].set_hh_rh(0i32); }
                }
                self.mem[((r).wrapping_add(3i32)) as usize].set_int(h);
                self.mem[((r).wrapping_add(2i32)) as usize].set_int(d);
                // §657
                if (m == 1i32) {
                    w = (x).wrapping_add(w);
                }
                self.mem[((r).wrapping_add(1i32)) as usize].set_int(w);
                x = (w).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                        self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                        break 'l_exit_f;
                    }
                } else {
                    if (x > 0i32) {
                        // §658
                        {
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
                            // §658
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(o);
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(1i32);
                            if (self.total_stretch[(o) as usize] != 0i32) {
                                { let __v293 = (((((x) as f64) / ((self.total_stretch[(o) as usize]) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v293); }
                            } else {
                                {
                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                }
                            }
                            if (o == 0i32) {
                                if (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32) {
                                    // §660
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[(0i32) as usize]);
                                        if (self.last_badness > self.eqtb[((5289i32) - 1) as usize].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(844i32);
                                                } else {
                                                    self.print_nl(845i32);
                                                }
                                                self.print(846i32);
                                                self.print_int(self.last_badness);
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                            // §658
                            break 'l_exit_f;
                        }
                    } else {
                        // §664
                        {
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
                            // §664
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(o);
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(2i32);
                            if (self.total_shrink[(o) as usize] != 0i32) {
                                { let __v294 = ((((((x).wrapping_neg()) as f64) / ((self.total_shrink[(o) as usize]) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v294); }
                            } else {
                                {
                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                }
                            }
                            if (((self.total_shrink[(o) as usize] < (x).wrapping_neg()) && (o == 0i32)) && (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32)) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((1.0f64) as f32));
                                    // §666
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[(0i32) as usize]) > self.eqtb[((5838i32) - 1) as usize].int()) || (self.eqtb[((5289i32) - 1) as usize].int() < 100i32)) {
                                        {
                                            if ((self.eqtb[((5846i32) - 1) as usize].int() > 0i32) && (((x).wrapping_neg()).wrapping_sub(self.total_shrink[(0i32) as usize]) > self.eqtb[((5838i32) - 1) as usize].int())) {
                                                {
                                                    while (self.mem[(q) as usize].hh().rh() != 0i32) {
                                                        q = self.mem[(q) as usize].hh().rh();
                                                    }
                                                    { let __v295 = self.new_rule(); self.mem[(q) as usize].set_hh_rh(__v295); }
                                                    { let __ix296 = (self.mem[(q) as usize].hh().rh()).wrapping_add(1i32); let __v297 = self.eqtb[((5846i32) - 1) as usize].int(); self.mem[(__ix296) as usize].set_int(__v297); }
                                                }
                                            }
                                            self.print_ln();
                                            self.print_nl(852i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[(0i32) as usize]));
                                            self.print(853i32);
                                            break 'l_common_ending_f;
                                        }
                                    }
                                }
                            } else {
                                // §664
                                if (o == 0i32) {
                                    if (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32) {
                                        // §667
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[(0i32) as usize]);
                                            if (self.last_badness > self.eqtb[((5289i32) - 1) as usize].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(854i32);
                                                    self.print_int(self.last_badness);
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §664
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §649
            if self.output_active {
                // §663
                self.print(847i32);
            } else {
                {
                    if (self.pack_begin_line != 0i32) {
                        {
                            if (self.pack_begin_line > 0i32) {
                                self.print(848i32);
                            } else {
                                self.print(849i32);
                            }
                            self.print_int((self.pack_begin_line).wrapping_abs());
                            self.print(850i32);
                        }
                    } else {
                        self.print(851i32);
                    }
                    self.print_int(self.line);
                }
            }
            self.print_ln();
            self.font_in_short_display = 0i32;
            self.short_display(self.mem[((r).wrapping_add(5i32)) as usize].hh().rh());
            self.print_ln();
            self.begin_diagnostic();
            self.show_box(r);
            self.end_diagnostic(true);
        }
        // §649
        hpack = r;
        hpack
    }

    /// The `vpack` subroutine is actually a special case of a slightly more
    /// general routine called `vpackage`, which has four parameters. The fourth
    /// parameter, which is `max_dimen` in the case of `vpack`, specifies the
    /// maximum depth of the page box that is constructed. The depth is first
    /// computed by the normal rules; if it exceeds this limit, the reference
    /// point is simply moved down until the limiting depth is attained.
    // §668
    pub fn vpackage(&mut self, mut p: halfword, mut h: scaled, mut m: small_number, mut l: scaled) -> halfword {
        let mut vpackage: halfword = 0;
        let mut r: halfword = 0; // §668
        let mut w: scaled = 0; // §668
        let mut d: scaled = 0; // §668
        let mut x: scaled = 0; // §668
        let mut s: scaled = 0; // §668
        let mut g: halfword = 0; // §668
        let mut o: glue_ord = 0; // §668
        'l_exit_f: {
            'l_common_ending_f: {
                self.last_badness = 0i32;
                r = self.get_node(7i32);
                self.mem[(r) as usize].set_hh_b0(1i32);
                self.mem[(r) as usize].set_hh_b1(0i32);
                self.mem[((r).wrapping_add(4i32)) as usize].set_int(0i32);
                self.mem[((r).wrapping_add(5i32)) as usize].set_hh_rh(p);
                w = 0i32;
                // §650
                d = 0i32;
                x = 0i32;
                self.total_stretch[(0i32) as usize] = 0i32;
                self.total_shrink[(0i32) as usize] = 0i32;
                self.total_stretch[(1i32) as usize] = 0i32;
                self.total_shrink[(1i32) as usize] = 0i32;
                self.total_stretch[(2i32) as usize] = 0i32;
                self.total_shrink[(2i32) as usize] = 0i32;
                self.total_stretch[(3i32) as usize] = 0i32;
                self.total_shrink[(3i32) as usize] = 0i32;
                // §668
                while (p != 0i32) {
                    // §669
                    {
                        if (p >= self.hi_mem_min) {
                            self.confusion(855i32);
                        } else {
                            match self.mem[(p) as usize].hh().b0() {
                                0 | 1 | 2 | 13 => {
                                    // §670
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(self.mem[((p).wrapping_add(3i32)) as usize].int());
                                        d = self.mem[((p).wrapping_add(2i32)) as usize].int();
                                        if (self.mem[(p) as usize].hh().b0() >= 2i32) {
                                            s = 0i32;
                                        } else {
                                            s = self.mem[((p).wrapping_add(4i32)) as usize].int();
                                        }
                                        if ((self.mem[((p).wrapping_add(1i32)) as usize].int()).wrapping_add(s) > w) {
                                            w = (self.mem[((p).wrapping_add(1i32)) as usize].int()).wrapping_add(s);
                                        }
                                    }
                                }
                                8 => {
                                    // §669
                                }
                                10 => {
                                    // §671
                                    {
                                        x = (x).wrapping_add(d);
                                        d = 0i32;
                                        g = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                        x = (x).wrapping_add(self.mem[((g).wrapping_add(1i32)) as usize].int());
                                        o = self.mem[(g) as usize].hh().b0();
                                        { let __v298 = (self.total_stretch[(o) as usize]).wrapping_add(self.mem[((g).wrapping_add(2i32)) as usize].int()); self.total_stretch[(o) as usize] = __v298; }
                                        o = self.mem[(g) as usize].hh().b1();
                                        { let __v299 = (self.total_shrink[(o) as usize]).wrapping_add(self.mem[((g).wrapping_add(3i32)) as usize].int()); self.total_shrink[(o) as usize] = __v299; }
                                        if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                            {
                                                g = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                                if (self.mem[((g).wrapping_add(1i32)) as usize].int() > w) {
                                                    w = self.mem[((g).wrapping_add(1i32)) as usize].int();
                                                }
                                            }
                                        }
                                    }
                                }
                                11 => {
                                    // §669
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        d = 0i32;
                                    }
                                }
                                _ => {
                                }
                            }
                        }
                        p = self.mem[(p) as usize].hh().rh();
                    }
                }
                // §668
                self.mem[((r).wrapping_add(1i32)) as usize].set_int(w);
                if (d > l) {
                    {
                        x = ((x).wrapping_add(d)).wrapping_sub(l);
                        self.mem[((r).wrapping_add(2i32)) as usize].set_int(l);
                    }
                } else {
                    self.mem[((r).wrapping_add(2i32)) as usize].set_int(d);
                }
                // §672
                if (m == 1i32) {
                    h = (x).wrapping_add(h);
                }
                self.mem[((r).wrapping_add(3i32)) as usize].set_int(h);
                x = (h).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                        self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
                        self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                        break 'l_exit_f;
                    }
                } else {
                    if (x > 0i32) {
                        // §673
                        {
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
                            // §673
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(o);
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(1i32);
                            if (self.total_stretch[(o) as usize] != 0i32) {
                                { let __v300 = (((((x) as f64) / ((self.total_stretch[(o) as usize]) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v300); }
                            } else {
                                {
                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                }
                            }
                            if (o == 0i32) {
                                if (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32) {
                                    // §674
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[(0i32) as usize]);
                                        if (self.last_badness > self.eqtb[((5290i32) - 1) as usize].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(844i32);
                                                } else {
                                                    self.print_nl(845i32);
                                                }
                                                self.print(856i32);
                                                self.print_int(self.last_badness);
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                            // §673
                            break 'l_exit_f;
                        }
                    } else {
                        // §676
                        {
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
                            // §676
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b1(o);
                            self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(2i32);
                            if (self.total_shrink[(o) as usize] != 0i32) {
                                { let __v301 = ((((((x).wrapping_neg()) as f64) / ((self.total_shrink[(o) as usize]) as f64))) as f32); self.mem[((r).wrapping_add(6i32)) as usize].set_gr(__v301); }
                            } else {
                                {
                                    self.mem[((r).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
                                }
                            }
                            if (((self.total_shrink[(o) as usize] < (x).wrapping_neg()) && (o == 0i32)) && (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32)) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[((r).wrapping_add(6i32)) as usize].set_gr(((1.0f64) as f32));
                                    // §677
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[(0i32) as usize]) > self.eqtb[((5839i32) - 1) as usize].int()) || (self.eqtb[((5290i32) - 1) as usize].int() < 100i32)) {
                                        {
                                            self.print_ln();
                                            self.print_nl(857i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[(0i32) as usize]));
                                            self.print(858i32);
                                            break 'l_common_ending_f;
                                        }
                                    }
                                }
                            } else {
                                // §676
                                if (o == 0i32) {
                                    if (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh() != 0i32) {
                                        // §678
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[(0i32) as usize]);
                                            if (self.last_badness > self.eqtb[((5290i32) - 1) as usize].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(859i32);
                                                    self.print_int(self.last_badness);
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §676
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §668
            if self.output_active {
                // §675
                self.print(847i32);
            } else {
                {
                    if (self.pack_begin_line != 0i32) {
                        {
                            self.print(849i32);
                            self.print_int((self.pack_begin_line).wrapping_abs());
                            self.print(850i32);
                        }
                    } else {
                        self.print(851i32);
                    }
                    self.print_int(self.line);
                    self.print_ln();
                }
            }
            self.begin_diagnostic();
            self.show_box(r);
            self.end_diagnostic(true);
        }
        // §668
        vpackage = r;
        vpackage
    }

    /// When a box is being appended to the current vertical list, the
    /// baselineskip calculation is handled by the `append_to_vlist` routine.
    // §679
    pub fn append_to_vlist(&mut self, mut b: halfword) {
        let mut d: scaled = 0; // §679
        let mut p: halfword = 0; // §679
        if (self.cur_list.aux_field.int() > (65536000i32).wrapping_neg()) {
            {
                d = ((self.mem[((self.eqtb[((2883i32) - 1) as usize].hh().rh()).wrapping_add(1i32)) as usize].int()).wrapping_sub(self.cur_list.aux_field.int())).wrapping_sub(self.mem[((b).wrapping_add(3i32)) as usize].int());
                if (d < self.eqtb[((5832i32) - 1) as usize].int()) {
                    p = self.new_param_glue(0i32);
                } else {
                    {
                        p = self.new_skip_param(1i32);
                        { let __ix302 = (self.temp_ptr).wrapping_add(1i32); self.mem[(__ix302) as usize].set_int(d); }
                    }
                }
                { let __ix303 = self.cur_list.tail_field; self.mem[(__ix303) as usize].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
        { let __ix304 = self.cur_list.tail_field; self.mem[(__ix304) as usize].set_hh_rh(b); }
        self.cur_list.tail_field = b;
        { let __v305 = self.mem[((b).wrapping_add(2i32)) as usize].int(); self.cur_list.aux_field.set_int(__v305); }
    }

    /// The `new_noad` function creates an `ord_noad` that is completely null.
    // §686
    pub fn new_noad(&mut self) -> halfword {
        let mut new_noad: halfword = 0;
        let mut p: halfword = 0; // §686
        p = self.get_node(4i32);
        self.mem[(p) as usize].set_hh_b0(16i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        { let __v306 = self.empty_field; self.mem[((p).wrapping_add(1i32)) as usize].set_hh(__v306); }
        { let __v307 = self.empty_field; self.mem[((p).wrapping_add(3i32)) as usize].set_hh(__v307); }
        { let __v308 = self.empty_field; self.mem[((p).wrapping_add(2i32)) as usize].set_hh(__v308); }
        new_noad = p;
        new_noad
    }

    /// Math formulas can also contain instructions like \.{\\textstyle} that
    /// override \TeX's normal style rules. A `style_node` is inserted into the
    /// data structure to record such instructions; it is three words long, so it
    /// is considered a node instead of a noad. The `subtype` is either `display_style`
    /// or `text_style` or `script_style` or `script_script_style`. The
    /// second and third words of a `style_node` are not used, but they are
    /// present because a `choice_node` is converted to a `style_node`.
    /// \TeX\ uses even numbers 0, 2, 4, 6 to encode the basic styles
    /// `display_style`, \dots, `script_script_style`, and adds~1 to get the
    /// ``cramped'' versions of these styles. This gives a numerical order that
    /// is backwards from the convention of Appendix~G in {\sl The \TeX book\/};
    /// i.e., a smaller style has a larger numerical value.
    // §688
    pub fn new_style(&mut self, mut s: small_number) -> halfword {
        let mut new_style: halfword = 0;
        let mut p: halfword = 0; // §688
        p = self.get_node(3i32);
        self.mem[(p) as usize].set_hh_b0(14i32);
        self.mem[(p) as usize].set_hh_b1(s);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(0i32);
        self.mem[((p).wrapping_add(2i32)) as usize].set_int(0i32);
        new_style = p;
        new_style
    }

    /// Finally, the \.{\\mathchoice} primitive creates a `choice_node`, which
    /// has special subfields `display_mlist`, `text_mlist`, `script_mlist`,
    /// and `script_script_mlist` pointing to the mlists for each style.
    // §689
    pub fn new_choice(&mut self) -> halfword {
        let mut new_choice: halfword = 0;
        let mut p: halfword = 0; // §689
        p = self.get_node(3i32);
        self.mem[(p) as usize].set_hh_b0(15i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
        self.mem[((p).wrapping_add(2i32)) as usize].set_hh_lh(0i32);
        self.mem[((p).wrapping_add(2i32)) as usize].set_hh_rh(0i32);
        new_choice = p;
        new_choice
    }

    /// The inelegant introduction of `show_info` in the code above seems better
    /// than the alternative of using \PASCAL's strange `forward` declaration for a
    /// procedure with parameters. The \PASCAL\ convention about dropping parameters
    /// from a post-`forward` procedure is, frankly, so intolerable to the author
    /// of \TeX\ that he would rather stoop to communication via a global temporary
    /// variable. (A similar stoopidity occurred with respect to `hlist_out` and
    /// `vlist_out` above, and it will occur with respect to `mlist_to_hlist` below.)
    // §693
    pub fn show_info(&mut self) {
        self.show_node_list(self.mem[(self.temp_ptr) as usize].hh().lh());
    }

    /// Here is a function that returns a pointer to a rule node having a given
    /// thickness `t`. The rule will extend horizontally to the boundary of the vlist
    /// that eventually contains it.
    // §704
    pub fn fraction_rule(&mut self, mut t: scaled) -> halfword {
        let mut fraction_rule: halfword = 0;
        let mut p: halfword = 0; // §704
        p = self.new_rule();
        self.mem[((p).wrapping_add(3i32)) as usize].set_int(t);
        self.mem[((p).wrapping_add(2i32)) as usize].set_int(0i32);
        fraction_rule = p;
        fraction_rule
    }

    /// The `overbar` function returns a pointer to a vlist box that consists of
    /// a given box `b`, above which has been placed a kern of height `k` under a
    /// fraction rule of thickness `t` under additional space of height `t`.
    // §705
    pub fn overbar(&mut self, mut b: halfword, mut k: scaled, mut t: scaled) -> halfword {
        let mut overbar: halfword = 0;
        let mut p: halfword = 0; // §705
        let mut q: halfword = 0; // §705
        p = self.new_kern(k);
        self.mem[(p) as usize].set_hh_rh(b);
        q = self.fraction_rule(t);
        self.mem[(q) as usize].set_hh_rh(p);
        p = self.new_kern(t);
        self.mem[(p) as usize].set_hh_rh(q);
        overbar = self.vpackage(p, 0i32, 1i32, 1073741823i32);
        overbar
    }

    /// Here is a subroutine that creates a new box, whose list contains a
    /// single character, and whose width includes the italic correction for
    /// that character. The height or depth of the box will be negative, if
    /// the height or depth of the character is negative; thus, this routine
    /// may deliver a slightly different result than `hpack` would produce.
    /// @<Declare subprocedures for `var_delimiter`
    // §709
    pub fn char_box(&mut self, mut f: internal_font_number, mut c: quarterword) -> halfword {
        let mut char_box: halfword = 0;
        let mut q: four_quarters = four_quarters::default(); // §709
        let mut hd: eight_bits = 0; // §709
        let mut b: halfword = 0; // §709
        let mut p: halfword = 0; // §709
        q = self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq();
        hd = (q.b1()).wrapping_sub(0i32);
        b = self.new_null_box();
        { let __v309 = (self.font_info[((self.width_base[(f) as usize]).wrapping_add(q.b0())) as usize].int()).wrapping_add(self.font_info[((self.italic_base[(f) as usize]).wrapping_add(((q.b2()).wrapping_sub(0i32) / 4i32))) as usize].int()); self.mem[((b).wrapping_add(1i32)) as usize].set_int(__v309); }
        { let __v310 = self.font_info[((self.height_base[(f) as usize]).wrapping_add((hd / 16i32))) as usize].int(); self.mem[((b).wrapping_add(3i32)) as usize].set_int(__v310); }
        { let __v311 = self.font_info[((self.depth_base[(f) as usize]).wrapping_add((hd % 16i32))) as usize].int(); self.mem[((b).wrapping_add(2i32)) as usize].set_int(__v311); }
        p = self.get_avail();
        self.mem[(p) as usize].set_hh_b1(c);
        self.mem[(p) as usize].set_hh_b0(f);
        self.mem[((b).wrapping_add(5i32)) as usize].set_hh_rh(p);
        char_box = b;
        char_box
    }

    /// When we build an extensible character, it's handy to have the
    /// following subroutine, which puts a given character on top
    /// of the characters already in box `b`:
    /// @<Declare subprocedures for `var_delimiter`
    // §711
    pub fn stack_into_box(&mut self, mut b: halfword, mut f: internal_font_number, mut c: quarterword) {
        let mut p: halfword = 0; // §711
        p = self.char_box(f, c);
        { let __v312 = self.mem[((b).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(p) as usize].set_hh_rh(__v312); }
        self.mem[((b).wrapping_add(5i32)) as usize].set_hh_rh(p);
        { let __v313 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((b).wrapping_add(3i32)) as usize].set_int(__v313); }
    }

    /// Another handy subroutine computes the height plus depth of
    /// a given character:
    /// @<Declare subprocedures for `var_delimiter`
    // §712
    pub fn height_plus_depth(&mut self, mut f: internal_font_number, mut c: quarterword) -> scaled {
        let mut height_plus_depth: scaled = 0;
        let mut q: four_quarters = four_quarters::default(); // §712
        let mut hd: eight_bits = 0; // §712
        q = self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq();
        hd = (q.b1()).wrapping_sub(0i32);
        height_plus_depth = (self.font_info[((self.height_base[(f) as usize]).wrapping_add((hd / 16i32))) as usize].int()).wrapping_add(self.font_info[((self.depth_base[(f) as usize]).wrapping_add((hd % 16i32))) as usize].int());
        height_plus_depth
    }

    /// The `var_delimiter` function, which finds or constructs a sufficiently
    /// large delimiter, is the most interesting of the auxiliary functions that
    /// currently concern us. Given a pointer `d` to a delimiter field in some noad,
    /// together with a size code `s` and a vertical distance `v`, this function
    /// returns a pointer to a box that contains the smallest variant of `d` whose
    /// height plus depth is `v` or more. (And if no variant is large enough, it
    /// returns the largest available variant.) In particular, this routine will
    /// construct arbitrarily large delimiters from extensible components, if
    /// `d` leads to such characters.
    /// The value returned is a box whose `shift_amount` has been set so that
    /// the box is vertically centered with respect to the axis in the given size.
    /// If a built-up symbol is returned, the height of the box before shifting
    /// will be the height of its topmost component.
    // §706
    pub fn var_delimiter(&mut self, mut d: halfword, mut s: small_number, mut v: scaled) -> halfword {
        let mut var_delimiter: halfword = 0;
        let mut b: halfword = 0; // §706
        let mut f: internal_font_number = 0; // §706
        let mut g: internal_font_number = 0; // §706
        let mut c: quarterword = 0; // §706
        let mut x: quarterword = 0; // §706
        let mut y: quarterword = 0; // §706
        let mut m: i32 = 0; // §706
        let mut n: i32 = 0; // §706
        let mut u: scaled = 0; // §706
        let mut w: scaled = 0; // §706
        let mut q: four_quarters = four_quarters::default(); // §706
        let mut hd: eight_bits = 0; // §706
        let mut r: four_quarters = four_quarters::default(); // §706
        let mut z: small_number = 0; // §706
        let mut large_attempt: bool = false; // §706
        'l_found_f: {
            f = 0i32;
            w = 0i32;
            large_attempt = false;
            z = self.mem[(d) as usize].qqqq().b0();
            x = self.mem[(d) as usize].qqqq().b1();
            while true {
                {
                    // §707
                    if ((z != 0i32) || (x != 0i32)) {
                        {
                            z = ((z).wrapping_add(s)).wrapping_add(16i32);
                            loop {
                                z = (z).wrapping_sub(16i32);
                                g = self.eqtb[(((3935i32).wrapping_add(z)) - 1) as usize].hh().rh();
                                if (g != 0i32) {
                                    // §708
                                    {
                                        y = x;
                                        if (((y).wrapping_sub(0i32) >= self.font_bc[(g) as usize]) && ((y).wrapping_sub(0i32) <= self.font_ec[(g) as usize])) {
                                            {
                                                'l_continue_b: loop {
                                                    q = self.font_info[((self.char_base[(g) as usize]).wrapping_add(y)) as usize].qqqq();
                                                    if (q.b0() > 0i32) {
                                                        {
                                                            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 3i32) {
                                                                {
                                                                    f = g;
                                                                    c = y;
                                                                    break 'l_found_f;
                                                                }
                                                            }
                                                            hd = (q.b1()).wrapping_sub(0i32);
                                                            u = (self.font_info[((self.height_base[(g) as usize]).wrapping_add((hd / 16i32))) as usize].int()).wrapping_add(self.font_info[((self.depth_base[(g) as usize]).wrapping_add((hd % 16i32))) as usize].int());
                                                            if (u > w) {
                                                                {
                                                                    f = g;
                                                                    c = y;
                                                                    w = u;
                                                                    if (u >= v) {
                                                                        break 'l_found_f;
                                                                    }
                                                                }
                                                            }
                                                            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 2i32) {
                                                                {
                                                                    y = q.b3();
                                                                    continue 'l_continue_b;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    break 'l_continue_b;
                                                }
                                            }
                                        }
                                    }
                                }
                                if (z < 16i32) { break; }
                            }
                        }
                    }
                    // §706
                    if large_attempt {
                        break 'l_found_f;
                    }
                    large_attempt = true;
                    z = self.mem[(d) as usize].qqqq().b2();
                    x = self.mem[(d) as usize].qqqq().b3();
                }
            }
        }
        if (f != 0i32) {
            // §710
            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 3i32) {
                // §713
                {
                    b = self.new_null_box();
                    self.mem[(b) as usize].set_hh_b0(1i32);
                    r = self.font_info[((self.exten_base[(f) as usize]).wrapping_add(q.b3())) as usize].qqqq();
                    // §714
                    c = r.b3();
                    u = self.height_plus_depth(f, c);
                    w = 0i32;
                    q = self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq();
                    { let __v314 = (self.font_info[((self.width_base[(f) as usize]).wrapping_add(q.b0())) as usize].int()).wrapping_add(self.font_info[((self.italic_base[(f) as usize]).wrapping_add(((q.b2()).wrapping_sub(0i32) / 4i32))) as usize].int()); self.mem[((b).wrapping_add(1i32)) as usize].set_int(__v314); }
                    c = r.b2();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    c = r.b1();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    c = r.b0();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    n = 0i32;
                    if (u > 0i32) {
                        while (w < v) {
                            {
                                w = (w).wrapping_add(u);
                                n = (n).wrapping_add(1i32);
                                if (r.b1() != 0i32) {
                                    w = (w).wrapping_add(u);
                                }
                            }
                        }
                    }
                    // §713
                    c = r.b2();
                    if (c != 0i32) {
                        self.stack_into_box(b, f, c);
                    }
                    c = r.b3();
                    {
                        let __for_end_5 = n;
                        m = 1i32;
                        while m <= __for_end_5 {
                            self.stack_into_box(b, f, c);
                            m = m.wrapping_add(1);
                        }
                    }
                    c = r.b1();
                    if (c != 0i32) {
                        {
                            self.stack_into_box(b, f, c);
                            c = r.b3();
                            {
                                let __for_end_7 = n;
                                m = 1i32;
                                while m <= __for_end_7 {
                                    self.stack_into_box(b, f, c);
                                    m = m.wrapping_add(1);
                                }
                            }
                        }
                    }
                    c = r.b0();
                    if (c != 0i32) {
                        self.stack_into_box(b, f, c);
                    }
                    { let __v315 = (w).wrapping_sub(self.mem[((b).wrapping_add(3i32)) as usize].int()); self.mem[((b).wrapping_add(2i32)) as usize].set_int(__v315); }
                }
            } else {
                // §710
                b = self.char_box(f, c);
            }
        } else {
            // §706
            {
                b = self.new_null_box();
                { let __v316 = self.eqtb[((5841i32) - 1) as usize].int(); self.mem[((b).wrapping_add(1i32)) as usize].set_int(__v316); }
            }
        }
        { let __v317 = (self.half((self.mem[((b).wrapping_add(3i32)) as usize].int()).wrapping_sub(self.mem[((b).wrapping_add(2i32)) as usize].int()))).wrapping_sub(self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(s)) - 1) as usize].hh().rh()) as usize])) as usize].int()); self.mem[((b).wrapping_add(4i32)) as usize].set_int(__v317); }
        var_delimiter = b;
        var_delimiter
    }

    /// The next subroutine is much simpler; it is used for numerators and
    /// denominators of fractions as well as for displayed operators and
    /// their limits above and below. It takes a given box~`b` and
    /// changes it so that the new box is centered in a box of width~`w`.
    /// The centering is done by putting \.{\\hss} glue at the left and right
    /// of the list inside `b`, then packaging the new box; thus, the
    /// actual box might not really be centered, if it already contains
    /// infinite glue.
    /// The given box might contain a single character whose italic correction
    /// has been added to the width of the box; in this case a compensating
    /// kern is inserted.
    // §715
    pub fn rebox(&mut self, mut b: halfword, mut w: scaled) -> halfword {
        let mut rebox: halfword = 0;
        let mut p: halfword = 0; // §715
        let mut f: internal_font_number = 0; // §715
        let mut v: scaled = 0; // §715
        if ((self.mem[((b).wrapping_add(1i32)) as usize].int() != w) && (self.mem[((b).wrapping_add(5i32)) as usize].hh().rh() != 0i32)) {
            {
                if (self.mem[(b) as usize].hh().b0() == 1i32) {
                    b = self.hpack(b, 0i32, 1i32);
                }
                p = self.mem[((b).wrapping_add(5i32)) as usize].hh().rh();
                if ((p >= self.hi_mem_min) && (self.mem[(p) as usize].hh().rh() == 0i32)) {
                    {
                        f = self.mem[(p) as usize].hh().b0();
                        v = self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int();
                        if (v != self.mem[((b).wrapping_add(1i32)) as usize].int()) {
                            { let __v318 = self.new_kern((self.mem[((b).wrapping_add(1i32)) as usize].int()).wrapping_sub(v)); self.mem[(p) as usize].set_hh_rh(__v318); }
                        }
                    }
                }
                self.free_node(b, 7i32);
                b = self.new_glue(12i32);
                self.mem[(b) as usize].set_hh_rh(p);
                while (self.mem[(p) as usize].hh().rh() != 0i32) {
                    p = self.mem[(p) as usize].hh().rh();
                }
                { let __v319 = self.new_glue(12i32); self.mem[(p) as usize].set_hh_rh(__v319); }
                rebox = self.hpack(b, w, 0i32);
            }
        } else {
            {
                self.mem[((b).wrapping_add(1i32)) as usize].set_int(w);
                rebox = b;
            }
        }
        rebox
    }

    /// Here is a subroutine that creates a new glue specification from another
    /// one that is expressed in `\.{mu}', given the value of the math unit.
    // §716
    pub fn math_glue(&mut self, mut g: halfword, mut m: scaled) -> halfword {
        let mut math_glue: halfword = 0;
        let mut p: halfword = 0; // §716
        let mut n: i32 = 0; // §716
        let mut f: scaled = 0; // §716
        n = self.x_over_n(m, 65536i32);
        f = self.remainder;
        if (f < 0i32) {
            {
                n = (n).wrapping_sub(1i32);
                f = (f).wrapping_add(65536i32);
            }
        }
        p = self.get_node(4i32);
        { let __v320 = { let __a321_0 = n; let __a321_1 = self.mem[((g).wrapping_add(1i32)) as usize].int(); let __a321_2 = self.xn_over_d(self.mem[((g).wrapping_add(1i32)) as usize].int(), f, 65536i32); let __a321_3 = 1073741823i32; self.mult_and_add(__a321_0, __a321_1, __a321_2, __a321_3) }; self.mem[((p).wrapping_add(1i32)) as usize].set_int(__v320); }
        { let __v322 = self.mem[(g) as usize].hh().b0(); self.mem[(p) as usize].set_hh_b0(__v322); }
        if (self.mem[(p) as usize].hh().b0() == 0i32) {
            { let __v323 = { let __a324_0 = n; let __a324_1 = self.mem[((g).wrapping_add(2i32)) as usize].int(); let __a324_2 = self.xn_over_d(self.mem[((g).wrapping_add(2i32)) as usize].int(), f, 65536i32); let __a324_3 = 1073741823i32; self.mult_and_add(__a324_0, __a324_1, __a324_2, __a324_3) }; self.mem[((p).wrapping_add(2i32)) as usize].set_int(__v323); }
        } else {
            { let __v325 = self.mem[((g).wrapping_add(2i32)) as usize].int(); self.mem[((p).wrapping_add(2i32)) as usize].set_int(__v325); }
        }
        { let __v326 = self.mem[(g) as usize].hh().b1(); self.mem[(p) as usize].set_hh_b1(__v326); }
        if (self.mem[(p) as usize].hh().b1() == 0i32) {
            { let __v327 = { let __a328_0 = n; let __a328_1 = self.mem[((g).wrapping_add(3i32)) as usize].int(); let __a328_2 = self.xn_over_d(self.mem[((g).wrapping_add(3i32)) as usize].int(), f, 65536i32); let __a328_3 = 1073741823i32; self.mult_and_add(__a328_0, __a328_1, __a328_2, __a328_3) }; self.mem[((p).wrapping_add(3i32)) as usize].set_int(__v327); }
        } else {
            { let __v329 = self.mem[((g).wrapping_add(3i32)) as usize].int(); self.mem[((p).wrapping_add(3i32)) as usize].set_int(__v329); }
        }
        math_glue = p;
        math_glue
    }

    /// The `math_kern` subroutine removes `mu_glue` from a kern node, given
    /// the value of the math unit.
    // §717
    pub fn math_kern(&mut self, mut p: halfword, mut m: scaled) {
        let mut n: i32 = 0; // §717
        let mut f: scaled = 0; // §717
        if (self.mem[(p) as usize].hh().b1() == 99i32) {
            {
                n = self.x_over_n(m, 65536i32);
                f = self.remainder;
                if (f < 0i32) {
                    {
                        n = (n).wrapping_sub(1i32);
                        f = (f).wrapping_add(65536i32);
                    }
                }
                { let __v330 = { let __a331_0 = n; let __a331_1 = self.mem[((p).wrapping_add(1i32)) as usize].int(); let __a331_2 = self.xn_over_d(self.mem[((p).wrapping_add(1i32)) as usize].int(), f, 65536i32); let __a331_3 = 1073741823i32; self.mult_and_add(__a331_0, __a331_1, __a331_2, __a331_3) }; self.mem[((p).wrapping_add(1i32)) as usize].set_int(__v330); }
                self.mem[(p) as usize].set_hh_b1(1i32);
            }
        }
    }

    /// Sometimes it is necessary to destroy an mlist. The following
    /// subroutine empties the current list, assuming that `abs(mode)=mmode`.
    // §718
    pub fn flush_math(&mut self) {
        self.flush_node_list(self.mem[(self.cur_list.head_field) as usize].hh().rh());
        self.flush_node_list(self.cur_list.aux_field.int());
        { let __ix332 = self.cur_list.head_field; self.mem[(__ix332) as usize].set_hh_rh(0i32); }
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.aux_field.set_int(0i32);
    }

    /// The recursion in `mlist_to_hlist` is due primarily to a subroutine
    /// called `clean_box` that puts a given noad field into a box using a given
    /// math style; `mlist_to_hlist` can call `clean_box`, which can call
    /// `mlist_to_hlist`.
    /// The box returned by `clean_box` is ``clean'' in the
    /// sense that its `shift_amount` is zero.
    // §720
    pub fn clean_box(&mut self, mut p: halfword, mut s: small_number) -> halfword {
        let mut clean_box: halfword = 0;
        let mut q: halfword = 0; // §720
        let mut save_style: small_number = 0; // §720
        let mut x: halfword = 0; // §720
        let mut r: halfword = 0; // §720
        'l_found_f: {
            match self.mem[(p) as usize].hh().rh() {
                1 => {
                    {
                        self.cur_mlist = self.new_noad();
                        { let __ix333 = (self.cur_mlist).wrapping_add(1i32); let __v334 = self.mem[(p) as usize]; self.mem[(__ix333) as usize] = __v334; }
                    }
                }
                2 => {
                    {
                        q = self.mem[(p) as usize].hh().lh();
                        break 'l_found_f;
                    }
                }
                3 => {
                    self.cur_mlist = self.mem[(p) as usize].hh().lh();
                }
                _ => {
                    {
                        q = self.new_null_box();
                        break 'l_found_f;
                    }
                }
            }
            save_style = self.cur_style;
            self.cur_style = s;
            self.mlist_penalties = false;
            self.mlist_to_hlist();
            q = self.mem[(29997i32) as usize].hh().rh();
            self.cur_style = save_style;
            // §703
            {
                if (self.cur_style < 4i32) {
                    self.cur_size = 0i32;
                } else {
                    self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                }
                self.cur_mu = self.x_over_n(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(), 18i32);
            }
        }
        // §720
        if ((q >= self.hi_mem_min) || (q == 0i32)) {
            x = self.hpack(q, 0i32, 1i32);
        } else {
            if (((self.mem[(q) as usize].hh().rh() == 0i32) && (self.mem[(q) as usize].hh().b0() <= 1i32)) && (self.mem[((q).wrapping_add(4i32)) as usize].int() == 0i32)) {
                x = q;
            } else {
                x = self.hpack(q, 0i32, 1i32);
            }
        }
        // §721
        q = self.mem[((x).wrapping_add(5i32)) as usize].hh().rh();
        if (q >= self.hi_mem_min) {
            {
                r = self.mem[(q) as usize].hh().rh();
                if (r != 0i32) {
                    if (self.mem[(r) as usize].hh().rh() == 0i32) {
                        if (!(r >= self.hi_mem_min)) {
                            if (self.mem[(r) as usize].hh().b0() == 11i32) {
                                {
                                    self.free_node(r, 2i32);
                                    self.mem[(q) as usize].set_hh_rh(0i32);
                                }
                            }
                        }
                    }
                }
            }
        }
        // §720
        clean_box = x;
        clean_box
    }

    /// It is convenient to have a procedure that converts a `math_char`
    /// field to an ``unpacked'' form. The `fetch` routine sets `cur_f`, `cur_c`,
    /// and `cur_i` to the font code, character code, and character information bytes of
    /// a given noad field. It also takes care of issuing error messages for
    /// nonexistent characters; in such cases, `char_exists(cur_i)` will be `false`
    /// after `fetch` has acted, and the field will also have been reset to `empty`.
    // §722
    pub fn fetch(&mut self, mut a: halfword) {
        self.cur_c = self.mem[(a) as usize].hh().b1();
        self.cur_f = self.eqtb[((((3935i32).wrapping_add(self.mem[(a) as usize].hh().b0())).wrapping_add(self.cur_size)) - 1) as usize].hh().rh();
        if (self.cur_f == 0i32) {
            // §723
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(338i32);
                }
                self.print_size(self.cur_size);
                self.print_char(32i32);
                self.print_int(self.mem[(a) as usize].hh().b0());
                self.print(884i32);
                self.print((self.cur_c).wrapping_sub(0i32));
                self.print_char(41i32);
                {
                    self.help_ptr = 4i32;
                    self.help_line[(3i32) as usize] = 885i32;
                    self.help_line[(2i32) as usize] = 886i32;
                    self.help_line[(1i32) as usize] = 887i32;
                    self.help_line[(0i32) as usize] = 888i32;
                }
                self.error();
                self.cur_i = self.null_character;
                self.mem[(a) as usize].set_hh_rh(0i32);
            }
        } else {
            // §722
            {
                if (((self.cur_c).wrapping_sub(0i32) >= self.font_bc[(self.cur_f) as usize]) && ((self.cur_c).wrapping_sub(0i32) <= self.font_ec[(self.cur_f) as usize])) {
                    self.cur_i = self.font_info[((self.char_base[(self.cur_f) as usize]).wrapping_add(self.cur_c)) as usize].qqqq();
                } else {
                    self.cur_i = self.null_character;
                }
                if (!(self.cur_i.b0() > 0i32)) {
                    {
                        self.char_warning(self.cur_f, (self.cur_c).wrapping_sub(0i32));
                        self.mem[(a) as usize].set_hh_rh(0i32);
                        self.cur_i = self.null_character;
                    }
                }
            }
        }
    }

    /// Most of the actual construction work of `mlist_to_hlist` is done
    /// by procedures with names
    /// like `make_fraction`, `make_radical`, etc. To illustrate
    /// the general setup of such procedures, let's begin with a couple of
    /// simple ones.
    /// @<Declare math...
    // §734
    pub fn make_over(&mut self, mut q: halfword) {
        { let __v335 = { let __a336_0 = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32)); let __a336_1 = (3i32).wrapping_mul(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()); let __a336_2 = self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(); self.overbar(__a336_0, __a336_1, __a336_2) }; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v335); }
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
    }

    /// @<Declare math...
    // §735
    pub fn make_under(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §735
        let mut x: halfword = 0; // §735
        let mut y: halfword = 0; // §735
        let mut delta: scaled = 0; // §735
        x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
        p = self.new_kern((3i32).wrapping_mul(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()));
        self.mem[(x) as usize].set_hh_rh(p);
        { let __v337 = self.fraction_rule(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()); self.mem[(p) as usize].set_hh_rh(__v337); }
        y = self.vpackage(x, 0i32, 1i32, 1073741823i32);
        delta = ((self.mem[((y).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((y).wrapping_add(2i32)) as usize].int())).wrapping_add(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
        { let __v338 = self.mem[((x).wrapping_add(3i32)) as usize].int(); self.mem[((y).wrapping_add(3i32)) as usize].set_int(__v338); }
        { let __v339 = (delta).wrapping_sub(self.mem[((y).wrapping_add(3i32)) as usize].int()); self.mem[((y).wrapping_add(2i32)) as usize].set_int(__v339); }
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(y);
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
    }

    /// @<Declare math...
    // §736
    pub fn make_vcenter(&mut self, mut q: halfword) {
        let mut v: halfword = 0; // §736
        let mut delta: scaled = 0; // §736
        v = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh();
        if (self.mem[(v) as usize].hh().b0() != 1i32) {
            self.confusion(539i32);
        }
        delta = (self.mem[((v).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((v).wrapping_add(2i32)) as usize].int());
        { let __v340 = (self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_add(self.half(delta)); self.mem[((v).wrapping_add(3i32)) as usize].set_int(__v340); }
        { let __v341 = (delta).wrapping_sub(self.mem[((v).wrapping_add(3i32)) as usize].int()); self.mem[((v).wrapping_add(2i32)) as usize].set_int(__v341); }
    }

    /// According to the rules in the \.{DVI} file specifications, we ensure alignment
    /// between a square root sign and the rule above its nucleus by assuming that the
    /// baseline of the square-root symbol is the same as the bottom of the rule. The
    /// height of the square-root symbol will be the thickness of the rule, and the
    /// depth of the square-root symbol should exceed or equal the height-plus-depth
    /// of the nucleus plus a certain minimum clearance~`clr`. The symbol will be
    /// placed so that the actual clearance is `clr` plus half the excess.
    /// @<Declare math...
    // §737
    pub fn make_radical(&mut self, mut q: halfword) {
        let mut x: halfword = 0; // §737
        let mut y: halfword = 0; // §737
        let mut delta: scaled = 0; // §737
        let mut clr: scaled = 0; // §737
        x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
        if (self.cur_style < 2i32) {
            clr = (self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_add(((self.font_info[((5i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_abs() / 4i32));
        } else {
            {
                clr = self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                clr = (clr).wrapping_add(((clr).wrapping_abs() / 4i32));
            }
        }
        y = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, (((self.mem[((x).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_add(clr)).wrapping_add(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()));
        delta = (self.mem[((y).wrapping_add(2i32)) as usize].int()).wrapping_sub(((self.mem[((x).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_add(clr));
        if (delta > 0i32) {
            clr = (clr).wrapping_add(self.half(delta));
        }
        { let __v342 = ((self.mem[((x).wrapping_add(3i32)) as usize].int()).wrapping_add(clr)).wrapping_neg(); self.mem[((y).wrapping_add(4i32)) as usize].set_int(__v342); }
        { let __v343 = self.overbar(x, clr, self.mem[((y).wrapping_add(3i32)) as usize].int()); self.mem[(y) as usize].set_hh_rh(__v343); }
        { let __v344 = self.hpack(y, 0i32, 1i32); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(__v344); }
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
    }

    /// Slants are not considered when placing accents in math mode. The accenter is
    /// centered over the accentee, and the accent width is treated as zero with
    /// respect to the size of the final box.
    /// @<Declare math...
    // §738
    pub fn make_math_accent(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §738
        let mut x: halfword = 0; // §738
        let mut y: halfword = 0; // §738
        let mut a: i32 = 0; // §738
        let mut c: quarterword = 0; // §738
        let mut f: internal_font_number = 0; // §738
        let mut i: four_quarters = four_quarters::default(); // §738
        let mut s: scaled = 0; // §738
        let mut h: scaled = 0; // §738
        let mut delta: scaled = 0; // §738
        let mut w: scaled = 0; // §738
        self.fetch((q).wrapping_add(4i32));
        if (self.cur_i.b0() > 0i32) {
            {
                'l_done_f: {
                    'l_done1_f: {
                        i = self.cur_i;
                        c = self.cur_c;
                        f = self.cur_f;
                        // §741
                        s = 0i32;
                        if (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() == 1i32) {
                            {
                                self.fetch((q).wrapping_add(1i32));
                                if (((self.cur_i.b2()).wrapping_sub(0i32) % 4i32) == 1i32) {
                                    {
                                        a = (self.lig_kern_base[(self.cur_f) as usize]).wrapping_add(self.cur_i.b3());
                                        self.cur_i = self.font_info[(a) as usize].qqqq();
                                        if (self.cur_i.b0() > 128i32) {
                                            {
                                                a = ((((self.lig_kern_base[(self.cur_f) as usize]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                                self.cur_i = self.font_info[(a) as usize].qqqq();
                                            }
                                        }
                                        while true {
                                            {
                                                if ((self.cur_i.b1()).wrapping_sub(0i32) == self.skew_char[(self.cur_f) as usize]) {
                                                    {
                                                        if (self.cur_i.b2() >= 128i32) {
                                                            if (self.cur_i.b0() <= 128i32) {
                                                                s = self.font_info[(((self.kern_base[(self.cur_f) as usize]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())) as usize].int();
                                                            }
                                                        }
                                                        break 'l_done1_f;
                                                    }
                                                }
                                                if (self.cur_i.b0() >= 128i32) {
                                                    break 'l_done1_f;
                                                }
                                                a = ((a).wrapping_add(self.cur_i.b0())).wrapping_add(1i32);
                                                self.cur_i = self.font_info[(a) as usize].qqqq();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §738
                    x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
                    w = self.mem[((x).wrapping_add(1i32)) as usize].int();
                    h = self.mem[((x).wrapping_add(3i32)) as usize].int();
                    // §740
                    while true {
                        {
                            if (((i.b2()).wrapping_sub(0i32) % 4i32) != 2i32) {
                                break 'l_done_f;
                            }
                            y = i.b3();
                            i = self.font_info[((self.char_base[(f) as usize]).wrapping_add(y)) as usize].qqqq();
                            if (!(i.b0() > 0i32)) {
                                break 'l_done_f;
                            }
                            if (self.font_info[((self.width_base[(f) as usize]).wrapping_add(i.b0())) as usize].int() > w) {
                                break 'l_done_f;
                            }
                            c = y;
                        }
                    }
                }
                // §738
                if (h < self.font_info[((5i32).wrapping_add(self.param_base[(f) as usize])) as usize].int()) {
                    delta = h;
                } else {
                    delta = self.font_info[((5i32).wrapping_add(self.param_base[(f) as usize])) as usize].int();
                }
                if ((self.mem[((q).wrapping_add(2i32)) as usize].hh().rh() != 0i32) || (self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() != 0i32)) {
                    if (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() == 1i32) {
                        // §742
                        {
                            self.flush_node_list(x);
                            x = self.new_noad();
                            { let __v345 = self.mem[((q).wrapping_add(1i32)) as usize]; self.mem[((x).wrapping_add(1i32)) as usize] = __v345; }
                            { let __v346 = self.mem[((q).wrapping_add(2i32)) as usize]; self.mem[((x).wrapping_add(2i32)) as usize] = __v346; }
                            { let __v347 = self.mem[((q).wrapping_add(3i32)) as usize]; self.mem[((x).wrapping_add(3i32)) as usize] = __v347; }
                            { let __v348 = self.empty_field; self.mem[((q).wrapping_add(2i32)) as usize].set_hh(__v348); }
                            { let __v349 = self.empty_field; self.mem[((q).wrapping_add(3i32)) as usize].set_hh(__v349); }
                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(3i32);
                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(x);
                            x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                            delta = ((delta).wrapping_add(self.mem[((x).wrapping_add(3i32)) as usize].int())).wrapping_sub(h);
                            h = self.mem[((x).wrapping_add(3i32)) as usize].int();
                        }
                    }
                }
                // §738
                y = self.char_box(f, c);
                { let __v350 = (s).wrapping_add(self.half((w).wrapping_sub(self.mem[((y).wrapping_add(1i32)) as usize].int()))); self.mem[((y).wrapping_add(4i32)) as usize].set_int(__v350); }
                self.mem[((y).wrapping_add(1i32)) as usize].set_int(0i32);
                p = self.new_kern((delta).wrapping_neg());
                self.mem[(p) as usize].set_hh_rh(x);
                self.mem[(y) as usize].set_hh_rh(p);
                y = self.vpackage(y, 0i32, 1i32, 1073741823i32);
                { let __v351 = self.mem[((x).wrapping_add(1i32)) as usize].int(); self.mem[((y).wrapping_add(1i32)) as usize].set_int(__v351); }
                if (self.mem[((y).wrapping_add(3i32)) as usize].int() < h) {
                    // §739
                    {
                        p = self.new_kern((h).wrapping_sub(self.mem[((y).wrapping_add(3i32)) as usize].int()));
                        { let __v352 = self.mem[((y).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(p) as usize].set_hh_rh(__v352); }
                        self.mem[((y).wrapping_add(5i32)) as usize].set_hh_rh(p);
                        self.mem[((y).wrapping_add(3i32)) as usize].set_int(h);
                    }
                }
                // §738
                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(y);
                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
            }
        }
    }

    /// The `make_fraction` procedure is a bit different because it sets
    /// `new_hlist(q)` directly rather than making a sub-box.
    /// @<Declare math...
    // §743
    pub fn make_fraction(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §743
        let mut v: halfword = 0; // §743
        let mut x: halfword = 0; // §743
        let mut y: halfword = 0; // §743
        let mut z: halfword = 0; // §743
        let mut delta: scaled = 0; // §743
        let mut delta1: scaled = 0; // §743
        let mut delta2: scaled = 0; // §743
        let mut shift_up: scaled = 0; // §743
        let mut shift_down: scaled = 0; // §743
        let mut clr: scaled = 0; // §743
        if (self.mem[((q).wrapping_add(1i32)) as usize].int() == 1073741824i32) {
            { let __v353 = self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int(); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v353); }
        }
        // §744
        x = self.clean_box((q).wrapping_add(2i32), ((self.cur_style).wrapping_add(2i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        z = self.clean_box((q).wrapping_add(3i32), (((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(3i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        if (self.mem[((x).wrapping_add(1i32)) as usize].int() < self.mem[((z).wrapping_add(1i32)) as usize].int()) {
            x = self.rebox(x, self.mem[((z).wrapping_add(1i32)) as usize].int());
        } else {
            z = self.rebox(z, self.mem[((x).wrapping_add(1i32)) as usize].int());
        }
        if (self.cur_style < 2i32) {
            {
                shift_up = self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                shift_down = self.font_info[((11i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
            }
        } else {
            {
                shift_down = self.font_info[((12i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                if (self.mem[((q).wrapping_add(1i32)) as usize].int() != 0i32) {
                    shift_up = self.font_info[((9i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                } else {
                    shift_up = self.font_info[((10i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                }
            }
        }
        // §743
        if (self.mem[((q).wrapping_add(1i32)) as usize].int() == 0i32) {
            // §745
            {
                if (self.cur_style < 2i32) {
                    clr = (7i32).wrapping_mul(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                } else {
                    clr = (3i32).wrapping_mul(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                }
                delta = self.half((clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.mem[((z).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down))));
                if (delta > 0i32) {
                    {
                        shift_up = (shift_up).wrapping_add(delta);
                        shift_down = (shift_down).wrapping_add(delta);
                    }
                }
            }
        } else {
            // §746
            {
                if (self.cur_style < 2i32) {
                    clr = (3i32).wrapping_mul(self.mem[((q).wrapping_add(1i32)) as usize].int());
                } else {
                    clr = self.mem[((q).wrapping_add(1i32)) as usize].int();
                }
                delta = self.half(self.mem[((q).wrapping_add(1i32)) as usize].int());
                delta1 = (clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_add(delta)));
                delta2 = (clr).wrapping_sub(((self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_sub(delta)).wrapping_sub((self.mem[((z).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down)));
                if (delta1 > 0i32) {
                    shift_up = (shift_up).wrapping_add(delta1);
                }
                if (delta2 > 0i32) {
                    shift_down = (shift_down).wrapping_add(delta2);
                }
            }
        }
        // §747
        v = self.new_null_box();
        self.mem[(v) as usize].set_hh_b0(1i32);
        { let __v354 = (shift_up).wrapping_add(self.mem[((x).wrapping_add(3i32)) as usize].int()); self.mem[((v).wrapping_add(3i32)) as usize].set_int(__v354); }
        { let __v355 = (self.mem[((z).wrapping_add(2i32)) as usize].int()).wrapping_add(shift_down); self.mem[((v).wrapping_add(2i32)) as usize].set_int(__v355); }
        { let __v356 = self.mem[((x).wrapping_add(1i32)) as usize].int(); self.mem[((v).wrapping_add(1i32)) as usize].set_int(__v356); }
        if (self.mem[((q).wrapping_add(1i32)) as usize].int() == 0i32) {
            {
                p = self.new_kern(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.mem[((z).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down)));
                self.mem[(p) as usize].set_hh_rh(z);
            }
        } else {
            {
                y = self.fraction_rule(self.mem[((q).wrapping_add(1i32)) as usize].int());
                p = self.new_kern(((self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_sub(delta)).wrapping_sub((self.mem[((z).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down)));
                self.mem[(y) as usize].set_hh_rh(p);
                self.mem[(p) as usize].set_hh_rh(z);
                p = self.new_kern(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_add(delta)));
                self.mem[(p) as usize].set_hh_rh(y);
            }
        }
        self.mem[(x) as usize].set_hh_rh(p);
        self.mem[((v).wrapping_add(5i32)) as usize].set_hh_rh(x);
        // §748
        if (self.cur_style < 2i32) {
            delta = self.font_info[((20i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
        } else {
            delta = self.font_info[((21i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
        }
        x = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, delta);
        self.mem[(x) as usize].set_hh_rh(v);
        z = self.var_delimiter((q).wrapping_add(5i32), self.cur_size, delta);
        self.mem[(v) as usize].set_hh_rh(z);
        { let __v357 = self.hpack(x, 0i32, 1i32); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v357); }
    }

    /// If the nucleus of an `op_noad` is a single character, it is to be
    /// centered vertically with respect to the axis, after first being enlarged
    /// (via a character list in the font) if we are in display style.  The normal
    /// convention for placing displayed limits is to put them above and below the
    /// operator in display style.
    /// The italic correction is removed from the character if there is a subscript
    /// and the limits are not being displayed. The `make_op`
    /// routine returns the value that should be used as an offset between
    /// subscript and superscript.
    /// After `make_op` has acted, `subtype(q)` will be `limits` if and only if
    /// the limits have been set above and below the operator. In that case,
    /// `new_hlist(q)` will already contain the desired final box.
    /// @<Declare math...
    // §749
    pub fn make_op(&mut self, mut q: halfword) -> scaled {
        let mut make_op: scaled = 0;
        let mut delta: scaled = 0; // §749
        let mut p: halfword = 0; // §749
        let mut v: halfword = 0; // §749
        let mut x: halfword = 0; // §749
        let mut y: halfword = 0; // §749
        let mut z: halfword = 0; // §749
        let mut c: quarterword = 0; // §749
        let mut i: four_quarters = four_quarters::default(); // §749
        let mut shift_up: scaled = 0; // §749
        let mut shift_down: scaled = 0; // §749
        if ((self.mem[(q) as usize].hh().b1() == 0i32) && (self.cur_style < 2i32)) {
            self.mem[(q) as usize].set_hh_b1(1i32);
        }
        if (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() == 1i32) {
            {
                self.fetch((q).wrapping_add(1i32));
                if ((self.cur_style < 2i32) && (((self.cur_i.b2()).wrapping_sub(0i32) % 4i32) == 2i32)) {
                    {
                        c = self.cur_i.b3();
                        i = self.font_info[((self.char_base[(self.cur_f) as usize]).wrapping_add(c)) as usize].qqqq();
                        if (i.b0() > 0i32) {
                            {
                                self.cur_c = c;
                                self.cur_i = i;
                                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_b1(c);
                            }
                        }
                    }
                }
                delta = self.font_info[((self.italic_base[(self.cur_f) as usize]).wrapping_add(((self.cur_i.b2()).wrapping_sub(0i32) / 4i32))) as usize].int();
                x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                if ((self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() != 0i32) && (self.mem[(q) as usize].hh().b1() != 1i32)) {
                    { let __v358 = (self.mem[((x).wrapping_add(1i32)) as usize].int()).wrapping_sub(delta); self.mem[((x).wrapping_add(1i32)) as usize].set_int(__v358); }
                }
                { let __v359 = (self.half((self.mem[((x).wrapping_add(3i32)) as usize].int()).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int()))).wrapping_sub(self.font_info[((22i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()); self.mem[((x).wrapping_add(4i32)) as usize].set_int(__v359); }
                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
                self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(x);
            }
        } else {
            delta = 0i32;
        }
        if (self.mem[(q) as usize].hh().b1() == 1i32) {
            // §750
            {
                x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                y = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                z = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                v = self.new_null_box();
                self.mem[(v) as usize].set_hh_b0(1i32);
                { let __v360 = self.mem[((y).wrapping_add(1i32)) as usize].int(); self.mem[((v).wrapping_add(1i32)) as usize].set_int(__v360); }
                if (self.mem[((x).wrapping_add(1i32)) as usize].int() > self.mem[((v).wrapping_add(1i32)) as usize].int()) {
                    { let __v361 = self.mem[((x).wrapping_add(1i32)) as usize].int(); self.mem[((v).wrapping_add(1i32)) as usize].set_int(__v361); }
                }
                if (self.mem[((z).wrapping_add(1i32)) as usize].int() > self.mem[((v).wrapping_add(1i32)) as usize].int()) {
                    { let __v362 = self.mem[((z).wrapping_add(1i32)) as usize].int(); self.mem[((v).wrapping_add(1i32)) as usize].set_int(__v362); }
                }
                x = self.rebox(x, self.mem[((v).wrapping_add(1i32)) as usize].int());
                y = self.rebox(y, self.mem[((v).wrapping_add(1i32)) as usize].int());
                z = self.rebox(z, self.mem[((v).wrapping_add(1i32)) as usize].int());
                { let __v363 = self.half(delta); self.mem[((x).wrapping_add(4i32)) as usize].set_int(__v363); }
                { let __v364 = (self.mem[((x).wrapping_add(4i32)) as usize].int()).wrapping_neg(); self.mem[((z).wrapping_add(4i32)) as usize].set_int(__v364); }
                { let __v365 = self.mem[((y).wrapping_add(3i32)) as usize].int(); self.mem[((v).wrapping_add(3i32)) as usize].set_int(__v365); }
                { let __v366 = self.mem[((y).wrapping_add(2i32)) as usize].int(); self.mem[((v).wrapping_add(2i32)) as usize].set_int(__v366); }
                // §751
                if (self.mem[((q).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                    {
                        self.free_node(x, 7i32);
                        self.mem[((v).wrapping_add(5i32)) as usize].set_hh_rh(y);
                    }
                } else {
                    {
                        shift_up = (self.font_info[((11i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int());
                        if (shift_up < self.font_info[((9i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()) {
                            shift_up = self.font_info[((9i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                        }
                        p = self.new_kern(shift_up);
                        self.mem[(p) as usize].set_hh_rh(y);
                        self.mem[(x) as usize].set_hh_rh(p);
                        p = self.new_kern(self.font_info[((13i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                        self.mem[(p) as usize].set_hh_rh(x);
                        self.mem[((v).wrapping_add(5i32)) as usize].set_hh_rh(p);
                        { let __v367 = ((((self.mem[((v).wrapping_add(3i32)) as usize].int()).wrapping_add(self.font_info[((13i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int())).wrapping_add(self.mem[((x).wrapping_add(3i32)) as usize].int())).wrapping_add(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_add(shift_up); self.mem[((v).wrapping_add(3i32)) as usize].set_int(__v367); }
                    }
                }
                if (self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() == 0i32) {
                    self.free_node(z, 7i32);
                } else {
                    {
                        shift_down = (self.font_info[((12i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_sub(self.mem[((z).wrapping_add(3i32)) as usize].int());
                        if (shift_down < self.font_info[((10i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()) {
                            shift_down = self.font_info[((10i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                        }
                        p = self.new_kern(shift_down);
                        self.mem[(y) as usize].set_hh_rh(p);
                        self.mem[(p) as usize].set_hh_rh(z);
                        p = self.new_kern(self.font_info[((13i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                        self.mem[(z) as usize].set_hh_rh(p);
                        { let __v368 = ((((self.mem[((v).wrapping_add(2i32)) as usize].int()).wrapping_add(self.font_info[((13i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int())).wrapping_add(self.mem[((z).wrapping_add(3i32)) as usize].int())).wrapping_add(self.mem[((z).wrapping_add(2i32)) as usize].int())).wrapping_add(shift_down); self.mem[((v).wrapping_add(2i32)) as usize].set_int(__v368); }
                    }
                }
                // §750
                self.mem[((q).wrapping_add(1i32)) as usize].set_int(v);
            }
        }
        // §749
        make_op = delta;
        make_op
    }

    /// A ligature found in a math formula does not create a `ligature_node`, because
    /// there is no question of hyphenation afterwards; the ligature will simply be
    /// stored in an ordinary `char_node`, after residing in an `ord_noad`.
    /// The `math_type` is converted to `math_text_char` here if we would not want to
    /// apply an italic correction to the current character unless it belongs
    /// to a math font (i.e., a font with `space=0`).
    /// No boundary characters enter into these ligatures.
    /// @<Declare math...
    // §752
    pub fn make_ord(&mut self, mut q: halfword) {
        let mut a: i32 = 0; // §752
        let mut p: halfword = 0; // §752
        let mut r: halfword = 0; // §752
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if (self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() == 0i32) {
                    if (self.mem[((q).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                        if (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() == 1i32) {
                            {
                                p = self.mem[(q) as usize].hh().rh();
                                if (p != 0i32) {
                                    if ((self.mem[(p) as usize].hh().b0() >= 16i32) && (self.mem[(p) as usize].hh().b0() <= 22i32)) {
                                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() == 1i32) {
                                            if (self.mem[((p).wrapping_add(1i32)) as usize].hh().b0() == self.mem[((q).wrapping_add(1i32)) as usize].hh().b0()) {
                                                {
                                                    self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(4i32);
                                                    self.fetch((q).wrapping_add(1i32));
                                                    if (((self.cur_i.b2()).wrapping_sub(0i32) % 4i32) == 1i32) {
                                                        {
                                                            a = (self.lig_kern_base[(self.cur_f) as usize]).wrapping_add(self.cur_i.b3());
                                                            self.cur_c = self.mem[((p).wrapping_add(1i32)) as usize].hh().b1();
                                                            self.cur_i = self.font_info[(a) as usize].qqqq();
                                                            if (self.cur_i.b0() > 128i32) {
                                                                {
                                                                    a = ((((self.lig_kern_base[(self.cur_f) as usize]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                                                    self.cur_i = self.font_info[(a) as usize].qqqq();
                                                                }
                                                            }
                                                            while true {
                                                                {
                                                                    // §753
                                                                    if (self.cur_i.b1() == self.cur_c) {
                                                                        if (self.cur_i.b0() <= 128i32) {
                                                                            if (self.cur_i.b2() >= 128i32) {
                                                                                {
                                                                                    p = self.new_kern(self.font_info[(((self.kern_base[(self.cur_f) as usize]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())) as usize].int());
                                                                                    { let __v369 = self.mem[(q) as usize].hh().rh(); self.mem[(p) as usize].set_hh_rh(__v369); }
                                                                                    self.mem[(q) as usize].set_hh_rh(p);
                                                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                                }
                                                                            } else {
                                                                                {
                                                                                    {
                                                                                        if (self.interrupt != 0i32) {
                                                                                            self.pause_for_instructions();
                                                                                        }
                                                                                    }
                                                                                    match self.cur_i.b2() {
                                                                                        1 | 5 => {
                                                                                            { let __v370 = self.cur_i.b3(); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_b1(__v370); }
                                                                                        }
                                                                                        2 | 6 => {
                                                                                            { let __v371 = self.cur_i.b3(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b1(__v371); }
                                                                                        }
                                                                                        3 | 7 | 11 => {
                                                                                            {
                                                                                                r = self.new_noad();
                                                                                                { let __v372 = self.cur_i.b3(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_b1(__v372); }
                                                                                                { let __v373 = self.mem[((q).wrapping_add(1i32)) as usize].hh().b0(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_b0(__v373); }
                                                                                                self.mem[(q) as usize].set_hh_rh(r);
                                                                                                self.mem[(r) as usize].set_hh_rh(p);
                                                                                                if (self.cur_i.b2() < 11i32) {
                                                                                                    self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(1i32);
                                                                                                } else {
                                                                                                    self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(4i32);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        _ => {
                                                                                            {
                                                                                                { let __v374 = self.mem[(p) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v374); }
                                                                                                { let __v375 = self.cur_i.b3(); self.mem[((q).wrapping_add(1i32)) as usize].set_hh_b1(__v375); }
                                                                                                { let __v376 = self.mem[((p).wrapping_add(3i32)) as usize]; self.mem[((q).wrapping_add(3i32)) as usize] = __v376; }
                                                                                                { let __v377 = self.mem[((p).wrapping_add(2i32)) as usize]; self.mem[((q).wrapping_add(2i32)) as usize] = __v377; }
                                                                                                self.free_node(p, 4i32);
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    if (self.cur_i.b2() > 3i32) {
                                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                                    }
                                                                                    self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(1i32);
                                                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §752
                                                                    if (self.cur_i.b0() >= 128i32) {
                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                    }
                                                                    a = ((a).wrapping_add(self.cur_i.b0())).wrapping_add(1i32);
                                                                    self.cur_i = self.font_info[(a) as usize].qqqq();
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
                    }
                }
            }
            if __goto_1 <= 1 { // exit
            }
            break 'l_dispatch_1;
        }
    }

    /// The purpose of `make_scripts(q,delta)` is to attach the subscript and/or
    /// superscript of noad `q` to the list that starts at `new_hlist(q)`,
    /// given that the subscript and superscript aren't both empty. The superscript
    /// will appear to the right of the subscript by a given distance `delta`.
    /// We set `shift_down` and `shift_up` to the minimum amounts to shift the
    /// baseline of subscripts and superscripts based on the given nucleus.
    /// @<Declare math...
    // §756
    pub fn make_scripts(&mut self, mut q: halfword, mut delta: scaled) {
        let mut p: halfword = 0; // §756
        let mut x: halfword = 0; // §756
        let mut y: halfword = 0; // §756
        let mut z: halfword = 0; // §756
        let mut shift_up: scaled = 0; // §756
        let mut shift_down: scaled = 0; // §756
        let mut clr: scaled = 0; // §756
        let mut t: small_number = 0; // §756
        p = self.mem[((q).wrapping_add(1i32)) as usize].int();
        if (p >= self.hi_mem_min) {
            {
                shift_up = 0i32;
                shift_down = 0i32;
            }
        } else {
            {
                z = self.hpack(p, 0i32, 1i32);
                if (self.cur_style < 4i32) {
                    t = 16i32;
                } else {
                    t = 32i32;
                }
                shift_up = (self.mem[((z).wrapping_add(3i32)) as usize].int()).wrapping_sub(self.font_info[((18i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(t)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                shift_down = (self.mem[((z).wrapping_add(2i32)) as usize].int()).wrapping_add(self.font_info[((19i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(t)) - 1) as usize].hh().rh()) as usize])) as usize].int());
                self.free_node(z, 7i32);
            }
        }
        if (self.mem[((q).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
            // §757
            {
                x = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                { let __v378 = (self.mem[((x).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5842i32) - 1) as usize].int()); self.mem[((x).wrapping_add(1i32)) as usize].set_int(__v378); }
                if (shift_down < self.font_info[((16i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()) {
                    shift_down = self.font_info[((16i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                }
                clr = (self.mem[((x).wrapping_add(3i32)) as usize].int()).wrapping_sub((((self.font_info[((5i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_mul(4i32)).wrapping_abs() / 5i32));
                if (shift_down < clr) {
                    shift_down = clr;
                }
                self.mem[((x).wrapping_add(4i32)) as usize].set_int(shift_down);
            }
        } else {
            // §756
            {
                // §758
                {
                    x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                    { let __v379 = (self.mem[((x).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5842i32) - 1) as usize].int()); self.mem[((x).wrapping_add(1i32)) as usize].set_int(__v379); }
                    if (((self.cur_style) % 2) != 0) {
                        clr = self.font_info[((15i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                    } else {
                        if (self.cur_style < 2i32) {
                            clr = self.font_info[((13i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                        } else {
                            clr = self.font_info[((14i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                        }
                    }
                    if (shift_up < clr) {
                        shift_up = clr;
                    }
                    clr = (self.mem[((x).wrapping_add(2i32)) as usize].int()).wrapping_add(((self.font_info[((5i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_abs() / 4i32));
                    if (shift_up < clr) {
                        shift_up = clr;
                    }
                }
                // §756
                if (self.mem[((q).wrapping_add(3i32)) as usize].hh().rh() == 0i32) {
                    self.mem[((x).wrapping_add(4i32)) as usize].set_int((shift_up).wrapping_neg());
                } else {
                    // §759
                    {
                        y = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                        { let __v380 = (self.mem[((y).wrapping_add(1i32)) as usize].int()).wrapping_add(self.eqtb[((5842i32) - 1) as usize].int()); self.mem[((y).wrapping_add(1i32)) as usize].set_int(__v380); }
                        if (shift_down < self.font_info[((17i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()) {
                            shift_down = self.font_info[((17i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int();
                        }
                        clr = ((4i32).wrapping_mul(self.font_info[((8i32).wrapping_add(self.param_base[(self.eqtb[(((3938i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int())).wrapping_sub(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.mem[((y).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down)));
                        if (clr > 0i32) {
                            {
                                shift_down = (shift_down).wrapping_add(clr);
                                clr = ((((self.font_info[((5i32).wrapping_add(self.param_base[(self.eqtb[(((3937i32).wrapping_add(self.cur_size)) - 1) as usize].hh().rh()) as usize])) as usize].int()).wrapping_mul(4i32)).wrapping_abs() / 5i32)).wrapping_sub((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int()));
                                if (clr > 0i32) {
                                    {
                                        shift_up = (shift_up).wrapping_add(clr);
                                        shift_down = (shift_down).wrapping_sub(clr);
                                    }
                                }
                            }
                        }
                        self.mem[((x).wrapping_add(4i32)) as usize].set_int(delta);
                        p = self.new_kern(((shift_up).wrapping_sub(self.mem[((x).wrapping_add(2i32)) as usize].int())).wrapping_sub((self.mem[((y).wrapping_add(3i32)) as usize].int()).wrapping_sub(shift_down)));
                        self.mem[(x) as usize].set_hh_rh(p);
                        self.mem[(p) as usize].set_hh_rh(y);
                        x = self.vpackage(x, 0i32, 1i32, 1073741823i32);
                        self.mem[((x).wrapping_add(4i32)) as usize].set_int(shift_down);
                    }
                }
            }
        }
        // §756
        if (self.mem[((q).wrapping_add(1i32)) as usize].int() == 0i32) {
            self.mem[((q).wrapping_add(1i32)) as usize].set_int(x);
        } else {
            {
                p = self.mem[((q).wrapping_add(1i32)) as usize].int();
                while (self.mem[(p) as usize].hh().rh() != 0i32) {
                    p = self.mem[(p) as usize].hh().rh();
                }
                self.mem[(p) as usize].set_hh_rh(x);
            }
        }
    }

}
