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
    /// Here we read a line from the current pseudo file into `buffer`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1756
    pub fn pseudo_input(&mut self) -> bool {
        let mut pseudo_input: bool = false;
        let mut p: halfword = 0; // §1756
        let mut sz: i32 = 0; // §1756
        let mut w: four_quarters = four_quarters::default(); // §1756
        let mut r: halfword = 0; // §1756
        self.last = self.first;
        p = self.mem[crate::ix::U((self.pseudo_files) as usize)].hh().lh();
        if (p == 0i32) {
            pseudo_input = false;
        } else {
            {
                { let __ix157 = self.pseudo_files; let __v158 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((__ix157) as usize)].set_hh_lh(__v158); }
                sz = (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(0i32);
                if (((4i32).wrapping_mul(sz)).wrapping_sub(3i32) >= (buf_size).wrapping_sub(self.last)) {
                    // §35
                    if (self.format_ident == 0i32) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "Buffer size exceeded!");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            crate::system::final_end(self);
                        }
                    } else {
                        {
                            self.cur_input.loc_field = self.first;
                            self.cur_input.limit_field = (self.last).wrapping_sub(1i32);
                            self.overflow(258i32, buf_size);
                        }
                    }
                }
                // §1756
                self.last = self.first;
                {
                    let __for_end_4 = ((p).wrapping_add(sz)).wrapping_sub(1i32);
                    r = (p).wrapping_add(1i32);
                    while r <= __for_end_4 {
                        {
                            w = self.mem[crate::ix::U((r) as usize)].qqqq();
                            self.buffer[crate::ix::U((self.last) as usize)] = w.b0();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(1i32)) as usize)] = w.b1();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(2i32)) as usize)] = w.b2();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(3i32)) as usize)] = w.b3();
                            self.last = (self.last).wrapping_add(4i32);
                        }
                        r = r.wrapping_add(1);
                    }
                }
                if (self.last >= self.max_buf_stack) {
                    self.max_buf_stack = (self.last).wrapping_add(1i32);
                }
                while ((self.last > self.first) && (self.buffer[crate::ix::U(((self.last).wrapping_sub(1i32)) as usize)] == 32i32)) {
                    self.last = (self.last).wrapping_sub(1i32);
                }
                self.free_node(p, sz);
                pseudo_input = true;
            }
        }
        pseudo_input
    }

    /// When we are done with a pseudo file we `close' it.
    /// @<Declare \eTeX\ procedures for tr...
    // §1757
    pub fn pseudo_close(&mut self) {
        let mut p: halfword = 0; // §1757
        let mut q: halfword = 0; // §1757
        p = self.mem[crate::ix::U((self.pseudo_files) as usize)].hh().rh();
        q = self.mem[crate::ix::U((self.pseudo_files) as usize)].hh().lh();
        {
            { let __ix159 = self.pseudo_files; let __v160 = self.avail; self.mem[crate::ix::U((__ix159) as usize)].set_hh_rh(__v160); }
            self.avail = self.pseudo_files;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.pseudo_files = p;
        while (q != 0i32) {
            {
                p = q;
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(0i32));
            }
        }
    }

    /// When a group ends that was apparently entered in a different input
    /// file, the `group_warning` procedure is invoked in order to update the
    /// `grp_stack`.  If moreover \.{\\tracingnesting} is positive we want to
    /// give a warning message.  The situation is, however, somewhat complicated
    /// by two facts:  (1)~There may be `grp_stack` elements without a
    /// corresponding \.{\\input} file or \.{\\scantokens} pseudo file (e.g.,
    /// error insertions from the terminal); and (2)~the relevant information is
    /// recorded in the `name_field` of the `input_stack` only loosely
    /// synchronized with the `in_open` variable indexing `grp_stack`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1774
    pub fn group_warning(&mut self) {
        let mut i: i32 = 0; // §1774
        let mut w: bool = false; // §1774
        self.base_ptr = self.input_ptr;
        { let __ix161 = self.base_ptr; let __v162 = self.cur_input; self.input_stack[crate::ix::U((__ix161) as usize)] = __v162; }
        i = self.in_open;
        w = false;
        while ((self.grp_stack[crate::ix::U((i) as usize)] == self.cur_boundary) && (i > 0i32)) {
            {
                // §1775
                if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == 0i32) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field > i)) {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field > 17i32) {
                            w = true;
                        }
                    }
                }
                // §1774
                { let __v163 = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh(); self.grp_stack[crate::ix::U((i) as usize)] = __v163; }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(2038i32);
                self.print_group(true);
                self.print(2039i32);
                self.print_ln();
                if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == 0i32) {
                    self.history = 1i32;
                }
            }
        }
    }

    /// When a conditional ends that was apparently started in a different
    /// input file, the `if_warning` procedure is invoked in order to update the
    /// `if_stack`.  If moreover \.{\\tracingnesting} is positive we want to
    /// give a warning message (with the same complications as above).
    /// @<Declare \eTeX\ procedures for tr...
    // §1776
    pub fn if_warning(&mut self) {
        let mut i: i32 = 0; // §1776
        let mut w: bool = false; // §1776
        self.base_ptr = self.input_ptr;
        { let __ix164 = self.base_ptr; let __v165 = self.cur_input; self.input_stack[crate::ix::U((__ix164) as usize)] = __v165; }
        i = self.in_open;
        w = false;
        while (self.if_stack[crate::ix::U((i) as usize)] == self.cond_ptr) {
            {
                // §1775
                if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == 0i32) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field > i)) {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field > 17i32) {
                            w = true;
                        }
                    }
                }
                // §1776
                { let __v166 = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh(); self.if_stack[crate::ix::U((i) as usize)] = __v166; }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(2038i32);
                self.print_cmd_chr(108i32, self.cur_if);
                if (self.if_line != 0i32) {
                    {
                        self.print(2008i32);
                        self.print_int(((self.if_line) as i64));
                    }
                }
                self.print(2039i32);
                self.print_ln();
                if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == 0i32) {
                    self.history = 1i32;
                }
            }
        }
    }

    /// Conversely, the `file_warning` procedure is invoked when a file ends
    /// and some groups entered or conditionals started while reading from that
    /// file are still incomplete.
    /// @<Declare \eTeX\ procedures for tr...
    // §1777
    pub fn file_warning(&mut self) {
        let mut p: halfword = 0; // §1777
        let mut l: quarterword = 0; // §1777
        let mut c: quarterword = 0; // §1777
        let mut i: i32 = 0; // §1777
        p = self.save_ptr;
        l = self.cur_level;
        c = self.cur_group;
        self.save_ptr = self.cur_boundary;
        while (self.grp_stack[crate::ix::U((self.in_open) as usize)] != self.save_ptr) {
            {
                self.cur_level = (self.cur_level).wrapping_sub(1i32);
                self.print_nl(2040i32);
                self.print_group(true);
                self.print(2041i32);
                self.cur_group = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                self.save_ptr = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh();
            }
        }
        self.save_ptr = p;
        self.cur_level = l;
        self.cur_group = c;
        p = self.cond_ptr;
        l = self.if_limit;
        c = self.cur_if;
        i = self.if_line;
        while (self.if_stack[crate::ix::U((self.in_open) as usize)] != self.cond_ptr) {
            {
                self.print_nl(2040i32);
                self.print_cmd_chr(108i32, self.cur_if);
                if (self.if_limit == 2i32) {
                    self.print_esc(932i32);
                }
                if (self.if_line != 0i32) {
                    {
                        self.print(2008i32);
                        self.print_int(((self.if_line) as i64));
                    }
                }
                self.print(2041i32);
                self.if_line = self.mem[crate::ix::U(((self.cond_ptr).wrapping_add(1i32)) as usize)].int();
                self.cur_if = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().b1();
                self.if_limit = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().b0();
                self.cond_ptr = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
            }
        }
        self.cond_ptr = p;
        self.if_limit = l;
        self.cur_if = c;
        self.if_line = i;
        self.print_ln();
        if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
            self.show_context();
        }
        if (self.history == 0i32) {
            self.history = 1i32;
        }
    }

    /// The `delete_sa_ref` procedure is called when a pointer to an array
    /// element representing a register is being removed; this means that the
    /// reference count should be decreased by one.  If the reduced reference
    /// count is `null` and the register has been (globally) assigned its
    /// default value the array element should disappear, possibly together with
    /// some index nodes.  This procedure will never be used for mark class
    /// nodes.
    // §1821
    pub fn delete_sa_ref(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §1821
        let mut i: small_number = 0; // §1821
        let mut s: small_number = 0; // §1821
        'l_exit_f: {
            { let __v167 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v167); }
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                break 'l_exit_f;
            }
            if (self.mem[crate::ix::U((q) as usize)].hh().b0() < 32i32) {
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    s = 3i32;
                } else {
                    break 'l_exit_f;
                }
            } else {
                {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() < 64i32) {
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == 0i32) {
                            self.delete_glue_ref(0i32);
                        } else {
                            break 'l_exit_f;
                        }
                    } else {
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() != 0i32) {
                            break 'l_exit_f;
                        }
                    }
                    s = 2i32;
                }
            }
            loop {
                i = (self.mem[crate::ix::U((q) as usize)].hh().b0() % 16i32);
                p = q;
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, s);
                if (q == 0i32) {
                    {
                        self.sa_root[crate::ix::U((i) as usize)] = 0i32;
                        break 'l_exit_f;
                    }
                }
                {
                    if (((i) % 2) != 0) {
                        self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(0i32);
                    } else {
                        self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(0i32);
                    }
                    { let __v168 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v168); }
                }
                s = 9i32;
                if (self.mem[crate::ix::U((q) as usize)].hh().b1() > 0i32) { break; }
            }
        }
    }

    /// Here is a procedure that displays the contents of an array element
    /// symbolically.  It is used under similar circumstances as is
    /// `restore_trace` (together with `show_eqtb`) for the quantities kept in
    /// the `eqtb` array.
    /// @<Declare \eTeX\ procedures for tr...
    // §1823
    pub fn show_sa(&mut self, mut p: halfword, mut s: str_number) {
        let mut t: small_number = 0; // §1823
        self.begin_diagnostic();
        self.print_char(123i32);
        self.print(s);
        self.print_char(32i32);
        if (p == 0i32) {
            self.print_char(63i32);
        } else {
            {
                t = (self.mem[crate::ix::U((p) as usize)].hh().b0() / 16i32);
                if (t < 4i32) {
                    self.print_cmd_chr(89i32, p);
                } else {
                    if (t == 4i32) {
                        {
                            self.print_esc(425i32);
                            self.print_sa_num(p);
                        }
                    } else {
                        if (t == 5i32) {
                            self.print_cmd_chr(71i32, p);
                        } else {
                            self.print_char(63i32);
                        }
                    }
                }
                self.print_char(61i32);
                if (t == 0i32) {
                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()) as i64));
                } else {
                    if (t == 1i32) {
                        {
                            self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                            self.print(314i32);
                        }
                    } else {
                        {
                            p = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                            if (t == 2i32) {
                                self.print_spec(p, 314i32);
                            } else {
                                if (t == 3i32) {
                                    self.print_spec(p, 347i32);
                                } else {
                                    if (t == 4i32) {
                                        if (p == 0i32) {
                                            self.print(426i32);
                                        } else {
                                            {
                                                self.depth_threshold = 0i32;
                                                self.breadth_max = 1i32;
                                                self.show_node_list(p);
                                            }
                                        }
                                    } else {
                                        if (t == 5i32) {
                                            {
                                                if (p != 0i32) {
                                                    self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), 0i32, 32i32);
                                                }
                                            }
                                        } else {
                                            self.print_char(63i32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The individual saved items are kept in pointer or word nodes similar
    /// to those used for the array elements: a word node with value zero is,
    /// however, saved as pointer node with the otherwise impossible `sa_index`
    /// value `tok_val_limit`.
    // §1837
    pub fn sa_save(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §1837
        let mut i: quarterword = 0; // §1837
        if (self.cur_level != self.sa_level) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(620i32, save_size);
                        }
                    }
                }
                { let __ix169 = self.save_ptr; self.save_stack[crate::ix::U((__ix169) as usize)].set_hh_b0(4i32); }
                { let __ix170 = self.save_ptr; let __v171 = self.sa_level; self.save_stack[crate::ix::U((__ix170) as usize)].set_hh_b1(__v171); }
                { let __ix172 = self.save_ptr; let __v173 = self.sa_chain; self.save_stack[crate::ix::U((__ix172) as usize)].set_hh_rh(__v173); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                self.sa_chain = 0i32;
                self.sa_level = self.cur_level;
            }
        }
        i = self.mem[crate::ix::U((p) as usize)].hh().b0();
        if (i < 32i32) {
            {
                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    {
                        q = self.get_node(2i32);
                        i = 96i32;
                    }
                } else {
                    {
                        q = self.get_node(3i32);
                        { let __v174 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v174); }
                    }
                }
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(0i32);
            }
        } else {
            {
                q = self.get_node(2i32);
                { let __v175 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v175); }
            }
        }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
        self.mem[crate::ix::U((q) as usize)].set_hh_b0(i);
        { let __v176 = self.mem[crate::ix::U((p) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v176); }
        { let __v177 = self.sa_chain; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v177); }
        self.sa_chain = q;
        { let __v178 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v178); }
    }

    /// @<Declare \eTeX\ procedures for tr...
    // §1838
    pub fn sa_destroy(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().b0() < 64i32) {
            self.delete_glue_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
        } else {
            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != 0i32) {
                if (self.mem[crate::ix::U((p) as usize)].hh().b0() < 80i32) {
                    self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                } else {
                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                }
            }
        }
    }

    /// The procedure `sa_def` assigns a new value to sparse array elements,
    /// and saves the former value if appropriate.  This procedure is used only
    /// for skip, muskip, box, and token list registers.  The counterpart of
    /// `sa_def` for count and dimen registers is called `sa_w_def`.
    // §1839
    pub fn sa_def(&mut self, mut p: halfword, mut e: halfword) {
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v179 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v179); }
        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == e) {
            {
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 622i32);
                }
                self.sa_destroy(p);
            }
        } else {
            {
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 623i32);
                }
                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == self.cur_level) {
                    self.sa_destroy(p);
                } else {
                    self.sa_save(p);
                }
                { let __v180 = self.cur_level; self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v180); }
                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 624i32);
                }
            }
        }
        self.delete_sa_ref(p);
    }

    /// The procedure `sa_def` assigns a new value to sparse array elements,
    /// and saves the former value if appropriate.  This procedure is used only
    /// for skip, muskip, box, and token list registers.  The counterpart of
    /// `sa_def` for count and dimen registers is called `sa_w_def`.
    // §1839
    pub fn sa_w_def(&mut self, mut p: halfword, mut w: i32) {
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v181 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v181); }
        if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == w) {
            {
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 622i32);
                }
            }
        } else {
            {
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 623i32);
                }
                if (self.mem[crate::ix::U((p) as usize)].hh().b1() != self.cur_level) {
                    self.sa_save(p);
                }
                { let __v182 = self.cur_level; self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v182); }
                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
                if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 624i32);
                }
            }
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_def` and `sa_w_def` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1840
    pub fn gsa_def(&mut self, mut p: halfword, mut e: halfword) {
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v183 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v183); }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 625i32);
        }
        self.sa_destroy(p);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(1i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 624i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_def` and `sa_w_def` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1840
    pub fn gsa_w_def(&mut self, mut p: halfword, mut w: i32) {
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v184 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v184); }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 625i32);
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(1i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 624i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_restore` procedure restores the sparse array entries pointed
    /// at by `sa_chain`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1841
    pub fn sa_restore(&mut self) {
        let mut p: halfword = 0; // §1841
        loop {
            p = self.mem[crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)].hh().lh();
            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 1i32) {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= 32i32) {
                        self.sa_destroy(self.sa_chain);
                    }
                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 627i32);
                    }
                }
            } else {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() < 32i32) {
                        if (self.mem[crate::ix::U((self.sa_chain) as usize)].hh().b0() < 32i32) {
                            { let __v185 = self.mem[crate::ix::U(((self.sa_chain).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v185); }
                        } else {
                            self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
                        }
                    } else {
                        {
                            self.sa_destroy(p);
                            { let __v186 = self.mem[crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v186); }
                        }
                    }
                    { let __v187 = self.mem[crate::ix::U((self.sa_chain) as usize)].hh().b1(); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v187); }
                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 628i32);
                    }
                }
            }
            self.delete_sa_ref(p);
            p = self.sa_chain;
            self.sa_chain = self.mem[crate::ix::U((p) as usize)].hh().rh();
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() < 32i32) {
                self.free_node(p, 3i32);
            } else {
                self.free_node(p, 2i32);
            }
            if (self.sa_chain == 0i32) { break; }
        }
    }

    /// Procedure `new_save_level` is called when a group begins. The
    /// argument is a group identification code like ``hbox_group`'. After
    /// calling this routine, it is safe to put five more entries on `save_stack`.
    /// In some cases integer-valued items are placed onto the
    /// `save_stack` just below a `level_boundary` word, because this is a
    /// convenient place to keep information that is supposed to ``pop up'' just
    /// when the group has finished.
    /// For example, when `\.{\\hbox to 100pt}\grp' is being treated, the 100pt
    /// dimension is stored on `save_stack` just before `new_save_level` is
    /// called.
    /// We use the notation `saved(k)` to stand for an integer item that
    /// appears in location `save_ptr+k` of the save stack.
    // §296
    pub fn new_save_level(&mut self, mut c: group_code) {
        if self.intr_rec_on {
            self.flashtex_intr_group(c);
        }
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(620i32, save_size);
                }
            }
        }
        if (self.eTeX_mode == 1i32) {
            {
                { let __ix188 = (self.save_ptr).wrapping_add(0i32); let __v189 = self.line; self.save_stack[crate::ix::U((__ix188) as usize)].set_int(__v189); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix190 = self.save_ptr; self.save_stack[crate::ix::U((__ix190) as usize)].set_hh_b0(3i32); }
        { let __ix191 = self.save_ptr; let __v192 = self.cur_group; self.save_stack[crate::ix::U((__ix191) as usize)].set_hh_b1(__v192); }
        { let __ix193 = self.save_ptr; let __v194 = self.cur_boundary; self.save_stack[crate::ix::U((__ix193) as usize)].set_hh_rh(__v194); }
        if (self.cur_level == 255i32) {
            self.overflow(621i32, 255i32);
        }
        self.cur_boundary = self.save_ptr;
        self.cur_group = c;
        if (self.eqtb[crate::ix::U(((29380i32) - 1) as usize)].int() > 0i32) {
            self.group_trace(false);
        }
        self.cur_level = (self.cur_level).wrapping_add(1i32);
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
    }

    /// Just before an entry of `eqtb` is changed, the following procedure should
    /// be called to update the other data structures properly. It is important
    /// to keep in mind that reference counts in `mem` include references from
    /// within `save_stack`, so these counts must be handled carefully.
    // §297
    pub fn eq_destroy(&mut self, mut w: memory_word) {
        let mut q: halfword = 0; // §297
        match w.hh().b0() {
            114 | 115 | 116 | 117 => {
                self.delete_token_ref(w.hh().rh());
            }
            120 => {
                self.delete_glue_ref(w.hh().rh());
            }
            121 => {
                {
                    q = w.hh().rh();
                    if (q != 0i32) {
                        self.free_node(q, ((self.mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().lh())).wrapping_add(1i32));
                    }
                }
            }
            122 => {
                self.flush_node_list(w.hh().rh());
            }
            71 | 89 => {
                // §1834
                if ((w.hh().rh() < 0i32) || (w.hh().rh() > 19i32)) {
                    self.delete_sa_ref(w.hh().rh());
                }
            }
            _ => {
                // §297
            }
        }
    }

    /// To save a value of `eqtb[p]` that was established at level `l`, we
    /// can use the following subroutine.
    // §298
    pub fn eq_save(&mut self, mut p: halfword, mut l: quarterword) {
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(620i32, save_size);
                }
            }
        }
        if (l == 0i32) {
            { let __ix195 = self.save_ptr; self.save_stack[crate::ix::U((__ix195) as usize)].set_hh_b0(1i32); }
        } else {
            {
                { let __ix196 = self.save_ptr; let __v197 = self.eqtb[crate::ix::U(((p) - 1) as usize)]; self.save_stack[crate::ix::U((__ix196) as usize)] = __v197; }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix198 = self.save_ptr; self.save_stack[crate::ix::U((__ix198) as usize)].set_hh_b0(0i32); }
            }
        }
        { let __ix199 = self.save_ptr; self.save_stack[crate::ix::U((__ix199) as usize)].set_hh_b1(l); }
        { let __ix200 = self.save_ptr; self.save_stack[crate::ix::U((__ix200) as usize)].set_hh_rh(p); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
    }

    /// The procedure `eq_define` defines an `eqtb` entry having specified
    /// `eq_type` and `equiv` fields, and saves the former value if appropriate.
    /// This procedure is used only for entries in the first four regions of `eqtb`,
    /// i.e., only for entries that have `eq_type` and `equiv` fields.
    /// After calling this routine, it is safe to put four more entries on
    /// `save_stack`, provided that there was room for four more entries before
    /// the call, since `eq_save` makes the necessary test.
    // §299
    pub fn eq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        'l_exit_f: {
            if self.intr_rec_on {
                self.flashtex_intr_def(p, t, e, 0i32);
            }
            if (((self.eTeX_mode == 1i32) && (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b0() == t)) && (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().rh() == e)) {
                {
                    if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 622i32);
                    }
                    self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
                    break 'l_exit_f;
                }
            }
            if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 623i32);
            }
            if (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1() == self.cur_level) {
                self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
            } else {
                if (self.cur_level > 1i32) {
                    self.eq_save(p, self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1());
                }
            }
            { let __v201 = self.cur_level; self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(__v201); }
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
            if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
                self.flashtex_intr_touch(p);
            }
            if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 624i32);
            }
        }
    }

    /// The counterpart of `eq_define` for the remaining (fullword) positions in
    /// `eqtb` is called `eq_word_define`. Since `xeq_level[p]>=level_one` for all
    /// `p`, a ``restore_zero`' will never be used in this case.
    // §300
    pub fn eq_word_define(&mut self, mut p: halfword, mut w: i32) {
        'l_exit_f: {
            if self.intr_rec_on {
                self.flashtex_intr_def(p, 0i32, w, 1i32);
            }
            if ((self.eTeX_mode == 1i32) && (self.eqtb[crate::ix::U(((p) - 1) as usize)].int() == w)) {
                {
                    if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 622i32);
                    }
                    break 'l_exit_f;
                }
            }
            if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 623i32);
            }
            if (self.xeq_level[crate::ix::U(((p) - 29277) as usize)] != self.cur_level) {
                {
                    self.eq_save(p, self.xeq_level[crate::ix::U(((p) - 29277) as usize)]);
                    { let __v202 = self.cur_level; self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = __v202; }
                }
            }
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 624i32);
            }
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §301
    pub fn geq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        if self.intr_rec_on {
            self.flashtex_intr_def(p, t, e, 2i32);
        }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 625i32);
        }
        {
            self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(1i32);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
        }
        if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
            self.flashtex_intr_touch(p);
        }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 624i32);
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §301
    pub fn geq_word_define(&mut self, mut p: halfword, mut w: i32) {
        if self.intr_rec_on {
            self.flashtex_intr_def(p, 0i32, w, 3i32);
        }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 625i32);
        }
        {
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = 1i32;
        }
        if (self.eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 624i32);
        }
    }

    /// Subroutine `save_for_after` puts a token on the stack for save-keeping.
    // §302
    pub fn save_for_after(&mut self, mut t: halfword) {
        if (self.cur_level > 1i32) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(620i32, save_size);
                        }
                    }
                }
                { let __ix203 = self.save_ptr; self.save_stack[crate::ix::U((__ix203) as usize)].set_hh_b0(2i32); }
                { let __ix204 = self.save_ptr; self.save_stack[crate::ix::U((__ix204) as usize)].set_hh_b1(0i32); }
                { let __ix205 = self.save_ptr; self.save_stack[crate::ix::U((__ix205) as usize)].set_hh_rh(t); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
    }

    /// The `unsave` routine goes the other way, taking items off of `save_stack`.
    /// This routine takes care of restoration when a level ends; everything
    /// belonging to the topmost group is cleared off of the save stack.
    // §303
    pub fn unsave(&mut self) {
        let mut p: halfword = 0; // §303
        let mut l: quarterword = 0; // §303
        let mut t: halfword = 0; // §303
        let mut a: bool = false; // §303
        if self.intr_rec_on {
            self.flashtex_intr_unsave();
        }
        a = false;
        if (self.cur_level > 1i32) {
            {
                'l_done_f: {
                    self.cur_level = (self.cur_level).wrapping_sub(1i32);
                    // §304
                    while true {
                        {
                            self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == 3i32) {
                                break 'l_done_f;
                            }
                            p = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh();
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == 2i32) {
                                // §348
                                {
                                    t = self.cur_tok;
                                    self.cur_tok = p;
                                    if a {
                                        {
                                            p = self.get_avail();
                                            { let __v206 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v206); }
                                            { let __v207 = self.cur_input.loc_field; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v207); }
                                            self.cur_input.loc_field = p;
                                            self.cur_input.start_field = p;
                                            if (self.cur_tok < 768i32) {
                                                if (self.cur_tok < 512i32) {
                                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                                } else {
                                                    self.align_state = (self.align_state).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            self.back_input();
                                            a = (self.eTeX_mode == 1i32);
                                        }
                                    }
                                    self.cur_tok = t;
                                }
                            } else {
                                // §304
                                if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == 4i32) {
                                    {
                                        self.sa_restore();
                                        self.sa_chain = p;
                                        self.sa_level = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                                    }
                                } else {
                                    {
                                        if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == 0i32) {
                                            {
                                                l = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                                                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                                            }
                                        } else {
                                            { let __ix208 = self.save_ptr; let __v209 = self.eqtb[crate::ix::U(((26627i32) - 1) as usize)]; self.save_stack[crate::ix::U((__ix208) as usize)] = __v209; }
                                        }
                                        // §305
                                        if ((p < 29277i32) || (p > 30192i32)) {
                                            if (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1() == 1i32) {
                                                {
                                                    self.eq_destroy(self.save_stack[crate::ix::U((self.save_ptr) as usize)]);
                                                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 627i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
                                                    { let __v210 = self.save_stack[crate::ix::U((self.save_ptr) as usize)]; self.eqtb[crate::ix::U(((p) - 1) as usize)] = __v210; }
                                                    if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
                                                        self.flashtex_intr_touch(p);
                                                    }
                                                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 628i32);
                                                    }
                                                }
                                            }
                                        } else {
                                            if (self.xeq_level[crate::ix::U(((p) - 29277) as usize)] != 1i32) {
                                                {
                                                    { let __v211 = self.save_stack[crate::ix::U((self.save_ptr) as usize)]; self.eqtb[crate::ix::U(((p) - 1) as usize)] = __v211; }
                                                    self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = l;
                                                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 628i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    if (self.eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 627i32);
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
                // §304
                if (self.eqtb[crate::ix::U(((29380i32) - 1) as usize)].int() > 0i32) {
                    self.group_trace(true);
                }
                if (self.grp_stack[crate::ix::U((self.in_open) as usize)] == self.cur_boundary) {
                    self.group_warning();
                }
                self.cur_group = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                self.cur_boundary = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh();
                if (self.eTeX_mode == 1i32) {
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                }
            }
        } else {
            // §303
            self.confusion(626i32);
        }
    }

    /// The `prepare_mag` subroutine is called whenever \TeX\ wants to use `mag`
    /// for magnification.
    // §310
    pub fn prepare_mag(&mut self) {
        if ((self.mag_set > 0i32) && (self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() != self.mag_set)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(630i32);
                }
                self.print_int(((self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int()) as i64));
                self.print(631i32);
                self.print_nl(632i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 633i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 634i32;
                }
                self.int_error(self.mag_set);
                self.geq_word_define(29294i32, self.mag_set);
            }
        }
        if ((self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() <= 0i32) || (self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() > 32768i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(635i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 636i32;
                }
                self.int_error(self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                self.geq_word_define(29294i32, 1000i32);
            }
        }
        self.mag_set = self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int();
    }

    /// Here's the way we sometimes want to display a token list, given a pointer
    /// to its reference count; the pointer may be null.
    // §317
    pub fn token_show(&mut self, mut p: halfword) {
        if (p != 0i32) {
            self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), 0i32, 10000000i32);
        }
    }

    /// The `print_meaning` subroutine displays `cur_cmd` and `cur_chr` in
    /// symbolic form, including the expansion of a macro or mark.
    // §318
    pub fn print_meaning(&mut self) {
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.cur_cmd >= 114i32) {
            {
                self.print_char(58i32);
                self.print_ln();
                self.token_show(self.cur_chr);
            }
        } else {
            if ((self.cur_cmd == 113i32) && (self.cur_chr < 5i32)) {
                {
                    self.print_char(58i32);
                    self.print_ln();
                    self.token_show(self.cur_mark[crate::ix::U((self.cur_chr) as usize)]);
                }
            }
        }
    }

    /// Here is a procedure that displays the current command.
    // §321
    pub fn show_cur_cmd_chr(&mut self) {
        let mut n: i32 = 0; // §321
        let mut l: i32 = 0; // §321
        let mut p: halfword = 0; // §321
        self.begin_diagnostic();
        self.print_nl(123i32);
        if (self.cur_list.mode_field != self.shown_mode) {
            {
                self.print_mode(self.cur_list.mode_field);
                self.print(650i32);
                self.shown_mode = self.cur_list.mode_field;
            }
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.eqtb[crate::ix::U(((29381i32) - 1) as usize)].int() > 0i32) {
            if (self.cur_cmd >= 108i32) {
                if (self.cur_cmd <= 109i32) {
                    {
                        self.print(650i32);
                        if (self.cur_cmd == 109i32) {
                            {
                                self.print_cmd_chr(108i32, self.cur_if);
                                self.print_char(32i32);
                                n = 0i32;
                                l = self.if_line;
                            }
                        } else {
                            {
                                n = 1i32;
                                l = self.line;
                            }
                        }
                        p = self.cond_ptr;
                        while (p != 0i32) {
                            {
                                n = (n).wrapping_add(1i32);
                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            }
                        }
                        self.print(651i32);
                        self.print_int(((n) as i64));
                        self.print_char(41i32);
                        if (l != 0i32) {
                            {
                                self.print(2008i32);
                                self.print_int(((l) as i64));
                            }
                        }
                    }
                }
            }
        }
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The status at each level is indicated by printing two lines, where the first
    /// line indicates what was read so far and the second line shows what remains
    /// to be read. The context is cropped, if necessary, so that the first line
    /// contains at most `half_error_line` characters, and the second contains
    /// at most `error_line`. Non-current input levels whose `token_type` is
    /// ``backed_up`' are shown only if they have not been fully read.
    // §333
    pub fn show_context(&mut self) {
        let mut old_setting: i32 = 0; // §333
        let mut nn: i32 = 0; // §333
        let mut bottom_line: bool = false; // §333
        let mut i: i32 = 0; // §337
        let mut j: i32 = 0; // §337
        let mut l: i32 = 0; // §337
        let mut m: i32 = 0; // §337
        let mut n: i32 = 0; // §337
        let mut p: i32 = 0; // §337
        let mut q: i32 = 0; // §337
        'l_done_f: {
            self.base_ptr = self.input_ptr;
            { let __ix212 = self.base_ptr; let __v213 = self.cur_input; self.input_stack[crate::ix::U((__ix212) as usize)] = __v213; }
            nn = (1i32).wrapping_neg();
            bottom_line = false;
            while true {
                {
                    self.cur_input = self.input_stack[crate::ix::U((self.base_ptr) as usize)];
                    if (self.cur_input.state_field != 0i32) {
                        if ((self.cur_input.name_field > 19i32) || (self.base_ptr == 0i32)) {
                            bottom_line = true;
                        }
                    }
                    if (((self.base_ptr == self.input_ptr) || bottom_line) || (nn < self.eqtb[crate::ix::U(((29331i32) - 1) as usize)].int())) {
                        // §334
                        {
                            if ((((self.base_ptr == self.input_ptr) || (self.cur_input.state_field != 0i32)) || (self.cur_input.index_field != 3i32)) || (self.cur_input.loc_field != 0i32)) {
                                {
                                    self.tally = 0i32;
                                    old_setting = self.selector;
                                    if (self.cur_input.state_field != 0i32) {
                                        {
                                            // §335
                                            if (self.cur_input.name_field <= 17i32) {
                                                if (self.cur_input.name_field == 0i32) {
                                                    if (self.base_ptr == 0i32) {
                                                        self.print_nl(657i32);
                                                    } else {
                                                        self.print_nl(658i32);
                                                    }
                                                } else {
                                                    {
                                                        self.print_nl(659i32);
                                                        if (self.cur_input.name_field == 17i32) {
                                                            self.print_char(42i32);
                                                        } else {
                                                            self.print_int((((self.cur_input.name_field).wrapping_sub(1i32)) as i64));
                                                        }
                                                        self.print_char(62i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.print_nl(660i32);
                                                    if (self.cur_input.index_field == self.in_open) {
                                                        self.print_int(((self.line) as i64));
                                                    } else {
                                                        self.print_int(((self.line_stack[crate::ix::U((((self.cur_input.index_field).wrapping_add(1i32)) - 1) as usize)]) as i64));
                                                    }
                                                }
                                            }
                                            self.print_char(32i32);
                                            // §340
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = 20i32;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.buffer[crate::ix::U((self.cur_input.limit_field) as usize)] == self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int()) {
                                                j = self.cur_input.limit_field;
                                            } else {
                                                j = (self.cur_input.limit_field).wrapping_add(1i32);
                                            }
                                            if (j > 0i32) {
                                                {
                                                    let __for_end_12 = (j).wrapping_sub(1i32);
                                                    i = self.cur_input.start_field;
                                                    while i <= __for_end_12 {
                                                        {
                                                            if (i == self.cur_input.loc_field) {
                                                                {
                                                                    self.first_count = self.tally;
                                                                    self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(self.error_line)).wrapping_sub(self.half_error_line);
                                                                    if (self.trick_count < self.error_line) {
                                                                        self.trick_count = self.error_line;
                                                                    }
                                                                }
                                                            }
                                                            self.print(self.buffer[crate::ix::U((i) as usize)]);
                                                        }
                                                        i = i.wrapping_add(1);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // §334
                                        {
                                            // §336
                                            match self.cur_input.index_field {
                                                0 => {
                                                    self.print_nl(661i32);
                                                }
                                                1 | 2 => {
                                                    self.print_nl(662i32);
                                                }
                                                3 => {
                                                    if (self.cur_input.loc_field == 0i32) {
                                                        self.print_nl(663i32);
                                                    } else {
                                                        self.print_nl(664i32);
                                                    }
                                                }
                                                4 => {
                                                    self.print_nl(665i32);
                                                }
                                                5 => {
                                                    {
                                                        self.print_ln();
                                                        self.print_cs(self.cur_input.name_field);
                                                    }
                                                }
                                                6 => {
                                                    self.print_nl(666i32);
                                                }
                                                7 => {
                                                    self.print_nl(667i32);
                                                }
                                                8 => {
                                                    self.print_nl(668i32);
                                                }
                                                9 => {
                                                    self.print_nl(669i32);
                                                }
                                                10 => {
                                                    self.print_nl(670i32);
                                                }
                                                11 => {
                                                    self.print_nl(671i32);
                                                }
                                                12 => {
                                                    self.print_nl(672i32);
                                                }
                                                13 => {
                                                    self.print_nl(673i32);
                                                }
                                                14 => {
                                                    self.print_nl(674i32);
                                                }
                                                19 => {
                                                    self.print_nl(675i32);
                                                }
                                                20 => {
                                                    self.print_nl(676i32);
                                                }
                                                _ => {
                                                    self.print_nl(63i32);
                                                }
                                            }
                                            // §341
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = 20i32;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.cur_input.index_field < 5i32) {
                                                self.show_token_list(self.cur_input.start_field, self.cur_input.loc_field, 100000i32);
                                            } else {
                                                self.show_token_list(self.mem[crate::ix::U((self.cur_input.start_field) as usize)].hh().rh(), self.cur_input.loc_field, 100000i32);
                                            }
                                        }
                                    }
                                    // §334
                                    self.selector = old_setting;
                                    // §339
                                    if (self.trick_count == 1000000i32) {
                                        {
                                            self.first_count = self.tally;
                                            self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(self.error_line)).wrapping_sub(self.half_error_line);
                                            if (self.trick_count < self.error_line) {
                                                self.trick_count = self.error_line;
                                            }
                                        }
                                    }
                                    if (self.tally < self.trick_count) {
                                        m = (self.tally).wrapping_sub(self.first_count);
                                    } else {
                                        m = (self.trick_count).wrapping_sub(self.first_count);
                                    }
                                    if ((l).wrapping_add(self.first_count) <= self.half_error_line) {
                                        {
                                            p = 0i32;
                                            n = (l).wrapping_add(self.first_count);
                                        }
                                    } else {
                                        {
                                            self.print(279i32);
                                            p = (((l).wrapping_add(self.first_count)).wrapping_sub(self.half_error_line)).wrapping_add(3i32);
                                            n = self.half_error_line;
                                        }
                                    }
                                    {
                                        let __for_end_9 = (self.first_count).wrapping_sub(1i32);
                                        q = p;
                                        while q <= __for_end_9 {
                                            self.print_char(self.trick_buf[crate::ix::U(((q % self.error_line)) as usize)]);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    self.print_ln();
                                    {
                                        let __for_end_9 = n;
                                        q = 1i32;
                                        while q <= __for_end_9 {
                                            self.print_char(32i32);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) <= self.error_line) {
                                        p = (self.first_count).wrapping_add(m);
                                    } else {
                                        p = (self.first_count).wrapping_add(((self.error_line).wrapping_sub(n)).wrapping_sub(3i32));
                                    }
                                    {
                                        let __for_end_9 = (p).wrapping_sub(1i32);
                                        q = self.first_count;
                                        while q <= __for_end_9 {
                                            self.print_char(self.trick_buf[crate::ix::U(((q % self.error_line)) as usize)]);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) > self.error_line) {
                                        self.print(279i32);
                                    }
                                    // §334
                                    nn = (nn).wrapping_add(1i32);
                                }
                            }
                        }
                    } else {
                        // §333
                        if (nn == self.eqtb[crate::ix::U(((29331i32) - 1) as usize)].int()) {
                            {
                                self.print_nl(279i32);
                                nn = (nn).wrapping_add(1i32);
                            }
                        }
                    }
                    if bottom_line {
                        break 'l_done_f;
                    }
                    self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                }
            }
        }
        self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
    }

    /// Here is a procedure that starts a new level of token-list input, given
    /// a token list `p` and its type `t`. If `t=macro`, the calling routine should
    /// set `name` and `loc`.
    // §345
    pub fn begin_token_list(&mut self, mut p: halfword, mut t: quarterword) {
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(677i32, stack_size);
                    }
                }
            }
            { let __ix214 = self.input_ptr; let __v215 = self.cur_input; self.input_stack[crate::ix::U((__ix214) as usize)] = __v215; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = 0i32;
        self.cur_input.start_field = p;
        self.cur_input.index_field = t;
        if (t >= 5i32) {
            {
                { let __v216 = (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v216); }
                if (t == 5i32) {
                    self.cur_input.limit_field = self.param_ptr;
                } else {
                    {
                        self.cur_input.loc_field = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        if (self.eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 1i32) {
                            {
                                self.begin_diagnostic();
                                self.print_nl(348i32);
                                match t {
                                    14 => {
                                        self.print_esc(363i32);
                                    }
                                    20 => {
                                        self.print_esc(678i32);
                                    }
                                    _ => {
                                        self.print_cmd_chr(72i32, (t).wrapping_add(27153i32));
                                    }
                                }
                                self.print(638i32);
                                self.token_show(p);
                                self.end_diagnostic(false);
                            }
                        }
                    }
                }
            }
        } else {
            self.cur_input.loc_field = p;
        }
    }

    /// When a token list has been fully scanned, the following computations
    /// should be done as we leave that level of input. The `token_type` tends
    /// to be equal to either `backed_up` or `inserted` about 2/3 of the time.
    // §346
    pub fn end_token_list(&mut self) {
        if (self.cur_input.index_field >= 3i32) {
            {
                if (self.cur_input.index_field <= 4i32) {
                    self.flush_list(self.cur_input.start_field);
                } else {
                    {
                        self.delete_token_ref(self.cur_input.start_field);
                        if (self.cur_input.index_field == 5i32) {
                            while (self.param_ptr > self.cur_input.limit_field) {
                                {
                                    self.param_ptr = (self.param_ptr).wrapping_sub(1i32);
                                    self.flush_list(self.param_stack[crate::ix::U((self.param_ptr) as usize)]);
                                }
                            }
                        } else {
                            if ((self.cur_input.index_field == 6i32) && (!self.output_can_end)) {
                                self.fatal_error(679i32);
                            }
                        }
                    }
                }
            }
        } else {
            if (self.cur_input.index_field == 1i32) {
                if (self.align_state > 500000i32) {
                    self.align_state = 0i32;
                } else {
                    self.fatal_error(680i32);
                }
            }
        }
        if self.macro_prof_on {
            if (self.cur_input.index_field == 5i32) {
                self.flashtex_prof_leave();
            }
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
            if (self.input_ptr < self.ckpt_arm_level) {
                {
                    self.ckpt_arm_level = 0i32;
                    self.ckpt_request = 2i32;
                }
            }
        }
        {
            if (self.interrupt != 0i32) {
                self.pause_for_instructions();
            }
        }
    }

    /// Sometimes \TeX\ has read too far and wants to ``unscan'' what it has
    /// seen. The `back_input` procedure takes care of this by putting the token
    /// just scanned back into the input stream, ready to be read again. This
    /// procedure can be used only if `cur_tok` represents the token to be
    /// replaced. Some applications of \TeX\ use this procedure a lot,
    /// so it has been slightly optimized for speed.
    // §347
    pub fn back_input(&mut self) {
        let mut p: halfword = 0; // §347
        while (((self.cur_input.loc_field == 0i32) && (self.cur_input.index_field != 2i32)) && (self.cur_input.index_field != 6i32)) {
            self.end_token_list();
        }
        p = self.get_avail();
        { let __v217 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v217); }
        if (self.cur_tok < 768i32) {
            if (self.cur_tok < 512i32) {
                self.align_state = (self.align_state).wrapping_sub(1i32);
            } else {
                self.align_state = (self.align_state).wrapping_add(1i32);
            }
        }
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(677i32, stack_size);
                    }
                }
            }
            { let __ix218 = self.input_ptr; let __v219 = self.cur_input; self.input_stack[crate::ix::U((__ix218) as usize)] = __v219; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = 0i32;
        self.cur_input.start_field = p;
        self.cur_input.index_field = 3i32;
        self.cur_input.loc_field = p;
    }

    /// The `back_error` routine is used when we want to replace an offending token
    /// just before issuing an error message. This routine, like `back_input`,
    /// requires that `cur_tok` has been set. We disable interrupts during the
    /// call of `back_input` so that the help message won't be lost.
    // §349
    pub fn back_error(&mut self) {
        self.OK_to_interrupt = false;
        self.back_input();
        self.OK_to_interrupt = true;
        self.error();
    }

    /// The `back_error` routine is used when we want to replace an offending token
    /// just before issuing an error message. This routine, like `back_input`,
    /// requires that `cur_tok` has been set. We disable interrupts during the
    /// call of `back_input` so that the help message won't be lost.
    // §349
    pub fn ins_error(&mut self) {
        self.OK_to_interrupt = false;
        self.back_input();
        self.cur_input.index_field = 4i32;
        self.OK_to_interrupt = true;
        self.error();
    }

    /// The `begin_file_reading` procedure starts a new level of input for lines
    /// of characters to be read from a file, or as an insertion from the
    /// terminal. It does not take care of opening the file, nor does it set `loc`
    /// or `limit` or `line`.
    // §350
    pub fn begin_file_reading(&mut self) {
        if (self.in_open == max_in_open) {
            self.overflow(681i32, max_in_open);
        }
        if (self.first == buf_size) {
            self.overflow(258i32, buf_size);
        }
        self.in_open = (self.in_open).wrapping_add(1i32);
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(677i32, stack_size);
                    }
                }
            }
            { let __ix220 = self.input_ptr; let __v221 = self.cur_input; self.input_stack[crate::ix::U((__ix220) as usize)] = __v221; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.index_field = self.in_open;
        self.full_source_filename_stack[crate::ix::U((self.cur_input.index_field) as usize)] = 0i32;
        { let __ix222 = self.cur_input.index_field; let __v223 = false; self.eof_seen[crate::ix::U(((__ix222) - 1) as usize)] = __v223; }
        { let __ix224 = self.cur_input.index_field; let __v225 = self.cur_boundary; self.grp_stack[crate::ix::U((__ix224) as usize)] = __v225; }
        { let __ix226 = self.cur_input.index_field; let __v227 = self.cond_ptr; self.if_stack[crate::ix::U((__ix226) as usize)] = __v227; }
        { let __ix228 = self.cur_input.index_field; let __v229 = self.line; self.line_stack[crate::ix::U(((__ix228) - 1) as usize)] = __v229; }
        self.cur_input.start_field = self.first;
        self.cur_input.state_field = 1i32;
        self.cur_input.name_field = 0i32;
    }

    /// Conversely, the variables must be downdated when such a level of input
    /// is finished:
    // §351
    pub fn end_file_reading(&mut self) {
        self.first = self.cur_input.start_field;
        self.line = self.line_stack[crate::ix::U(((self.cur_input.index_field) - 1) as usize)];
        if ((self.cur_input.name_field == 18i32) || (self.cur_input.name_field == 19i32)) {
            self.pseudo_close();
        } else {
            if (self.cur_input.name_field > 17i32) {
                { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)]); let __r = self.a_close(&mut __f0); self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)] = __f0; __r };
            }
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
            if (self.input_ptr < self.ckpt_arm_level) {
                {
                    self.ckpt_arm_level = 0i32;
                    self.ckpt_request = 2i32;
                }
            }
        }
        self.in_open = (self.in_open).wrapping_sub(1i32);
    }

    /// In order to keep the stack from overflowing during a long sequence of
    /// inserted `\.{\\show}' commands, the following routine removes completed
    /// error-inserted lines from memory.
    // §352
    pub fn clear_for_error_prompt(&mut self) {
        while ((((self.cur_input.state_field != 0i32) && (self.cur_input.name_field == 0i32)) && (self.input_ptr > 0i32)) && (self.cur_input.loc_field > self.cur_input.limit_field)) {
            self.end_file_reading();
        }
        self.print_ln();
        crate::system::break_in(&mut self.term_in, true);
    }

    /// Before getting into `get_next`, let's consider the subroutine that
    /// is called when an `\.{\\outer}' control sequence has been scanned or
    /// when the end of a file has been reached. These two cases are distinguished
    /// by `cur_cs`, which is zero at the end of a file.
    // §358
    pub fn check_outer_validity(&mut self) {
        let mut p: halfword = 0; // §358
        let mut q: halfword = 0; // §358
        if (self.scanner_status != 0i32) {
            {
                self.deletions_allowed = false;
                // §359
                if (self.cur_cs != 0i32) {
                    {
                        if (((self.cur_input.state_field == 0i32) || (self.cur_input.name_field < 1i32)) || (self.cur_input.name_field > 17i32)) {
                            {
                                p = self.get_avail();
                                { let __v230 = (4095i32).wrapping_add(self.cur_cs); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v230); }
                                self.begin_token_list(p, 3i32);
                            }
                        }
                        self.cur_cmd = 10i32;
                        self.cur_chr = 32i32;
                    }
                }
                // §358
                if (self.scanner_status > 1i32) {
                    // §360
                    {
                        self.runaway();
                        if (self.cur_cs == 0i32) {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(689i32);
                            }
                        } else {
                            {
                                self.cur_cs = 0i32;
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(690i32);
                                }
                            }
                        }
                        self.print(691i32);
                        // §361
                        p = self.get_avail();
                        match self.scanner_status {
                            2 => {
                                {
                                    self.print(653i32);
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                }
                            }
                            3 => {
                                {
                                    self.print(697i32);
                                    { let __v231 = self.par_token; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v231); }
                                    self.long_state = 116i32;
                                }
                            }
                            4 => {
                                {
                                    self.print(655i32);
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                    q = p;
                                    p = self.get_avail();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(19610i32);
                                    self.align_state = (1000000i32).wrapping_neg();
                                }
                            }
                            5 => {
                                {
                                    self.print(656i32);
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                }
                            }
                            _ => {}
                        }
                        self.begin_token_list(p, 4i32);
                        // §360
                        self.print(692i32);
                        self.sprint_cs(self.warning_index);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 693i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 694i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 695i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 696i32;
                        }
                        self.error();
                    }
                } else {
                    // §358
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(683i32);
                        }
                        self.print_cmd_chr(108i32, self.cur_if);
                        self.print(684i32);
                        self.print_int(((self.skip_line) as i64));
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 685i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 686i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 687i32;
                        }
                        if (self.cur_cs != 0i32) {
                            self.cur_cs = 0i32;
                        } else {
                            self.help_line[crate::ix::U((2i32) as usize)] = 688i32;
                        }
                        self.cur_tok = 19613i32;
                        self.ins_error();
                    }
                }
                self.deletions_allowed = true;
            }
        }
    }

    /// Now we're ready to take the plunge into `get_next` itself. Parts of
    /// this routine are executed more often than any other instructions of \TeX.
    // §363
    pub fn get_next(&mut self) {
        let mut k: i32 = 0; // §363
        let mut t: halfword = 0; // §363
        let mut cat: i32 = 0; // §363
        let mut c: ASCII_code = 0; // §363
        let mut cc: ASCII_code = 0; // §363
        let mut d: i32 = 0; // §363
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.cur_cs = 0i32;
                if (self.cur_input.state_field != 0i32) {
                    // §365
                    {
                        'l_L25_b: loop {
                            if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                                {
                                    self.cur_chr = self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)];
                                    self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                    'l_reswitch_b: loop {
                                        self.cur_cmd = self.eqtb[crate::ix::U((((27741i32).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                        // §366
                                        match (self.cur_input.state_field).wrapping_add(self.cur_cmd) {
                                            10 | 26 | 42 | 27 | 43 => {
                                                continue 'l_L25_b;
                                            }
                                            1 | 17 | 33 => {
                                                // §376
                                                {
                                                    'l_found_f: {
                                                        if (self.cur_input.loc_field > self.cur_input.limit_field) {
                                                            self.cur_cs = 513i32;
                                                        } else {
                                                            {
                                                                'l_L26_b: loop {
                                                                    k = self.cur_input.loc_field;
                                                                    self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                    cat = self.eqtb[crate::ix::U((((27741i32).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                                    k = (k).wrapping_add(1i32);
                                                                    if (cat == 11i32) {
                                                                        self.cur_input.state_field = 17i32;
                                                                    } else {
                                                                        if (cat == 10i32) {
                                                                            self.cur_input.state_field = 17i32;
                                                                        } else {
                                                                            self.cur_input.state_field = 1i32;
                                                                        }
                                                                    }
                                                                    if ((cat == 11i32) && (k <= self.cur_input.limit_field)) {
                                                                        // §378
                                                                        {
                                                                            loop {
                                                                                self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                                cat = self.eqtb[crate::ix::U((((27741i32).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                                                k = (k).wrapping_add(1i32);
                                                                                if ((cat != 11i32) || (k > self.cur_input.limit_field)) { break; }
                                                                            }
                                                                            // §377
                                                                            {
                                                                                if (self.buffer[crate::ix::U((k) as usize)] == self.cur_chr) {
                                                                                    if (cat == 7i32) {
                                                                                        if (k < self.cur_input.limit_field) {
                                                                                            {
                                                                                                c = self.buffer[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
                                                                                                if (c < 128i32) {
                                                                                                    {
                                                                                                        d = 2i32;
                                                                                                        if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                                                            if ((k).wrapping_add(2i32) <= self.cur_input.limit_field) {
                                                                                                                {
                                                                                                                    cc = self.buffer[crate::ix::U(((k).wrapping_add(2i32)) as usize)];
                                                                                                                    if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                                                        d = (d).wrapping_add(1i32);
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                        if (d > 2i32) {
                                                                                                            {
                                                                                                                if (c <= 57i32) {
                                                                                                                    self.cur_chr = (c).wrapping_sub(48i32);
                                                                                                                } else {
                                                                                                                    self.cur_chr = (c).wrapping_sub(87i32);
                                                                                                                }
                                                                                                                if (cc <= 57i32) {
                                                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                                                } else {
                                                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                                                }
                                                                                                                { let __v232 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v232; }
                                                                                                            }
                                                                                                        } else {
                                                                                                            if (c < 64i32) {
                                                                                                                self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_add(64i32);
                                                                                                            } else {
                                                                                                                self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_sub(64i32);
                                                                                                            }
                                                                                                        }
                                                                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                        self.first = (self.first).wrapping_sub(d);
                                                                                                        while (k <= self.cur_input.limit_field) {
                                                                                                            {
                                                                                                                { let __v233 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v233; }
                                                                                                                k = (k).wrapping_add(1i32);
                                                                                                            }
                                                                                                        }
                                                                                                        continue 'l_L26_b;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            // §378
                                                                            if (cat != 11i32) {
                                                                                k = (k).wrapping_sub(1i32);
                                                                            }
                                                                            if (k > (self.cur_input.loc_field).wrapping_add(1i32)) {
                                                                                {
                                                                                    self.cur_cs = self.id_lookup(self.cur_input.loc_field, (k).wrapping_sub(self.cur_input.loc_field));
                                                                                    self.cur_input.loc_field = k;
                                                                                    break 'l_found_f;
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        // §377
                                                                        {
                                                                            if (self.buffer[crate::ix::U((k) as usize)] == self.cur_chr) {
                                                                                if (cat == 7i32) {
                                                                                    if (k < self.cur_input.limit_field) {
                                                                                        {
                                                                                            c = self.buffer[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
                                                                                            if (c < 128i32) {
                                                                                                {
                                                                                                    d = 2i32;
                                                                                                    if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                                                        if ((k).wrapping_add(2i32) <= self.cur_input.limit_field) {
                                                                                                            {
                                                                                                                cc = self.buffer[crate::ix::U(((k).wrapping_add(2i32)) as usize)];
                                                                                                                if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                                                    d = (d).wrapping_add(1i32);
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                    if (d > 2i32) {
                                                                                                        {
                                                                                                            if (c <= 57i32) {
                                                                                                                self.cur_chr = (c).wrapping_sub(48i32);
                                                                                                            } else {
                                                                                                                self.cur_chr = (c).wrapping_sub(87i32);
                                                                                                            }
                                                                                                            if (cc <= 57i32) {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                                            } else {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                                            }
                                                                                                            { let __v234 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v234; }
                                                                                                        }
                                                                                                    } else {
                                                                                                        if (c < 64i32) {
                                                                                                            self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_add(64i32);
                                                                                                        } else {
                                                                                                            self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_sub(64i32);
                                                                                                        }
                                                                                                    }
                                                                                                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                    self.first = (self.first).wrapping_sub(d);
                                                                                                    while (k <= self.cur_input.limit_field) {
                                                                                                        {
                                                                                                            { let __v235 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v235; }
                                                                                                            k = (k).wrapping_add(1i32);
                                                                                                        }
                                                                                                    }
                                                                                                    continue 'l_L26_b;
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §376
                                                                    self.cur_cs = (257i32).wrapping_add(self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)]);
                                                                    self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                                                    break 'l_L26_b;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                    if self.rs_on {
                                                        if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                            self.flashtex_cs_read(self.cur_cs);
                                                        }
                                                    }
                                                    if (self.cur_cmd >= 116i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            14 | 30 | 46 => {
                                                // §375
                                                {
                                                    self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                                    self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                    self.cur_input.state_field = 1i32;
                                                    if self.rs_on {
                                                        if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                            self.flashtex_cs_read(self.cur_cs);
                                                        }
                                                    }
                                                    if (self.cur_cmd >= 116i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            8 | 24 | 40 => {
                                                // §374
                                                {
                                                    if (self.cur_chr == self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)]) {
                                                        if (self.cur_input.loc_field < self.cur_input.limit_field) {
                                                            {
                                                                c = self.buffer[crate::ix::U(((self.cur_input.loc_field).wrapping_add(1i32)) as usize)];
                                                                if (c < 128i32) {
                                                                    {
                                                                        self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(2i32);
                                                                        if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                            if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                                                                                {
                                                                                    cc = self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)];
                                                                                    if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                        {
                                                                                            self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                                                                            if (c <= 57i32) {
                                                                                                self.cur_chr = (c).wrapping_sub(48i32);
                                                                                            } else {
                                                                                                self.cur_chr = (c).wrapping_sub(87i32);
                                                                                            }
                                                                                            if (cc <= 57i32) {
                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                            } else {
                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                            }
                                                                                            continue 'l_reswitch_b;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if (c < 64i32) {
                                                                            self.cur_chr = (c).wrapping_add(64i32);
                                                                        } else {
                                                                            self.cur_chr = (c).wrapping_sub(64i32);
                                                                        }
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_input.state_field = 1i32;
                                                }
                                            }
                                            16 | 32 | 48 => {
                                                // §368
                                                {
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(264i32);
                                                        }
                                                        self.print(698i32);
                                                    }
                                                    {
                                                        self.help_ptr = 2i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 699i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 700i32;
                                                    }
                                                    self.deletions_allowed = false;
                                                    self.error();
                                                    self.deletions_allowed = true;
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            11 => {
                                                // §371
                                                {
                                                    self.cur_input.state_field = 17i32;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            6 => {
                                                // §370
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_cmd = 10i32;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            22 | 15 | 31 | 47 => {
                                                // §372
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    continue 'l_L25_b;
                                                }
                                            }
                                            38 => {
                                                // §373
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_cs = self.par_loc;
                                                    self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                    if self.rs_on {
                                                        if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                            self.flashtex_cs_read(self.cur_cs);
                                                        }
                                                    }
                                                    if (self.cur_cmd >= 116i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            2 => {
                                                // §369
                                                self.align_state = (self.align_state).wrapping_add(1i32);
                                            }
                                            18 | 34 => {
                                                {
                                                    self.cur_input.state_field = 1i32;
                                                    self.align_state = (self.align_state).wrapping_add(1i32);
                                                }
                                            }
                                            3 => {
                                                self.align_state = (self.align_state).wrapping_sub(1i32);
                                            }
                                            19 | 35 => {
                                                {
                                                    self.cur_input.state_field = 1i32;
                                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                                }
                                            }
                                            20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41 | 44 | 45 => {
                                                self.cur_input.state_field = 1i32;
                                            }
                                            _ => {
                                                // §366
                                            }
                                        }
                                        break 'l_reswitch_b;
                                    }
                                }
                            } else {
                                // §365
                                {
                                    self.cur_input.state_field = 33i32;
                                    // §382
                                    if (self.cur_input.name_field > 17i32) {
                                        // §384
                                        {
                                            self.line = (self.line).wrapping_add(1i32);
                                            self.first = self.cur_input.start_field;
                                            if (!self.force_eof) {
                                                if (self.cur_input.name_field <= 19i32) {
                                                    {
                                                        if self.pseudo_input() {
                                                            self.firm_up_the_line();
                                                        } else {
                                                            if ((self.eqtb[crate::ix::U(((27172i32) - 1) as usize)].hh().rh() != 0i32) && (!self.eof_seen[crate::ix::U(((self.cur_input.index_field) - 1) as usize)])) {
                                                                {
                                                                    self.cur_input.limit_field = (self.first).wrapping_sub(1i32);
                                                                    { let __ix236 = self.cur_input.index_field; let __v237 = true; self.eof_seen[crate::ix::U(((__ix236) - 1) as usize)] = __v237; }
                                                                    self.begin_token_list(self.eqtb[crate::ix::U(((27172i32) - 1) as usize)].hh().rh(), 19i32);
                                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                                }
                                                            } else {
                                                                self.force_eof = true;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        if { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)]); let __r = self.input_ln(&mut __f0, true); self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)] = __f0; __r } {
                                                            self.firm_up_the_line();
                                                        } else {
                                                            if ((self.eqtb[crate::ix::U(((27172i32) - 1) as usize)].hh().rh() != 0i32) && (!self.eof_seen[crate::ix::U(((self.cur_input.index_field) - 1) as usize)])) {
                                                                {
                                                                    self.cur_input.limit_field = (self.first).wrapping_sub(1i32);
                                                                    { let __ix238 = self.cur_input.index_field; let __v239 = true; self.eof_seen[crate::ix::U(((__ix238) - 1) as usize)] = __v239; }
                                                                    self.begin_token_list(self.eqtb[crate::ix::U(((27172i32) - 1) as usize)].hh().rh(), 19i32);
                                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                                }
                                                            } else {
                                                                self.force_eof = true;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if self.force_eof {
                                                {
                                                    if (self.eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
                                                        if ((self.grp_stack[crate::ix::U((self.in_open) as usize)] != self.cur_boundary) || (self.if_stack[crate::ix::U((self.in_open) as usize)] != self.cond_ptr)) {
                                                            self.file_warning();
                                                        }
                                                    }
                                                    if (self.cur_input.name_field >= 19i32) {
                                                        {
                                                            self.print_char(41i32);
                                                            self.open_parens = (self.open_parens).wrapping_sub(1i32);
                                                            crate::system::break_out(&mut self.term_out);
                                                        }
                                                    }
                                                    self.force_eof = false;
                                                    self.end_file_reading();
                                                    self.check_outer_validity();
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if ((self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                            } else {
                                                { let __ix240 = self.cur_input.limit_field; let __v241 = self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix240) as usize)] = __v241; }
                                            }
                                            self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                                            self.cur_input.loc_field = self.cur_input.start_field;
                                        }
                                    } else {
                                        // §382
                                        {
                                            if (!(self.cur_input.name_field == 0i32)) {
                                                {
                                                    self.cur_cmd = 0i32;
                                                    self.cur_chr = 0i32;
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if (self.input_ptr > 0i32) {
                                                {
                                                    self.end_file_reading();
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if (self.selector < 18i32) {
                                                self.open_log_file();
                                            }
                                            if (self.interaction > 1i32) {
                                                {
                                                    if ((self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    }
                                                    if (self.cur_input.limit_field == self.cur_input.start_field) {
                                                        self.print_nl(702i32);
                                                    }
                                                    self.print_ln();
                                                    self.first = self.cur_input.start_field;
                                                    {
                                                        self.print(42i32);
                                                        self.term_input();
                                                    }
                                                    self.cur_input.limit_field = self.last;
                                                    if ((self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                                    } else {
                                                        { let __ix242 = self.cur_input.limit_field; let __v243 = self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix242) as usize)] = __v243; }
                                                    }
                                                    self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_input.loc_field = self.cur_input.start_field;
                                                }
                                            } else {
                                                self.fatal_error(703i32);
                                            }
                                        }
                                    }
                                    // §365
                                    {
                                        if (self.interrupt != 0i32) {
                                            self.pause_for_instructions();
                                        }
                                    }
                                    continue 'l_L25_b;
                                }
                            }
                            break 'l_L25_b;
                        }
                    }
                } else {
                    // §379
                    if (self.cur_input.loc_field != 0i32) {
                        {
                            t = self.mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh();
                            self.cur_input.loc_field = self.mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().rh();
                            if (t >= 4095i32) {
                                {
                                    self.cur_cs = (t).wrapping_sub(4095i32);
                                    self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                    self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                    if self.rs_on {
                                        if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                            self.flashtex_cs_read(self.cur_cs);
                                        }
                                    }
                                    if (self.cur_cmd >= 116i32) {
                                        if (self.cur_cmd == 119i32) {
                                            // §380
                                            {
                                                self.cur_cs = (self.mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh()).wrapping_sub(4095i32);
                                                self.cur_input.loc_field = 0i32;
                                                self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                if self.rs_on {
                                                    if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                        self.flashtex_cs_read(self.cur_cs);
                                                    }
                                                }
                                                if (self.cur_cmd > 103i32) {
                                                    {
                                                        self.cur_cmd = 0i32;
                                                        self.cur_chr = 257i32;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §379
                                            {
                                                if ((self.cur_cs == 15522i32) && (self.cur_list.mode_field == 0i32)) {
                                                    self.fatal_error(701i32);
                                                }
                                                self.check_outer_validity();
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    self.cur_cmd = (t / 256i32);
                                    self.cur_chr = (t % 256i32);
                                    match self.cur_cmd {
                                        1 => {
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                        }
                                        2 => {
                                            self.align_state = (self.align_state).wrapping_sub(1i32);
                                        }
                                        5 => {
                                            // §381
                                            {
                                                self.begin_token_list(self.param_stack[crate::ix::U((((self.cur_input.limit_field).wrapping_add(self.cur_chr)).wrapping_sub(1i32)) as usize)], 0i32);
                                                { __goto_1 = 0; continue 'l_dispatch_1; }
                                            }
                                        }
                                        _ => {
                                            // §379
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        {
                            self.end_token_list();
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §364
                if (self.cur_cmd <= 5i32) {
                    if (self.cur_cmd >= 4i32) {
                        if (self.align_state == 0i32) {
                            // §965
                            {
                                if ((self.scanner_status == 4i32) || (self.cur_align == 0i32)) {
                                    self.fatal_error(680i32);
                                }
                                self.cur_cmd = self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh();
                                { let __ix244 = (self.cur_align).wrapping_add(5i32); let __v245 = self.cur_chr; self.mem[crate::ix::U((__ix244) as usize)].set_hh_lh(__v245); }
                                if (self.cur_cmd == 63i32) {
                                    self.begin_token_list(4999989i32, 2i32);
                                } else {
                                    self.begin_token_list(self.mem[crate::ix::U(((self.cur_align).wrapping_add(2i32)) as usize)].int(), 2i32);
                                }
                                self.align_state = 1000000i32;
                                { __goto_1 = 0; continue 'l_dispatch_1; }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // exit
                // §363
                if self.intr_rec_on {
                    self.flashtex_intr_next();
                }
            }
            break 'l_dispatch_1;
        }
    }

    /// If the user has set the `pausing` parameter to some positive value,
    /// and if nonstop mode has not been selected, each line of input is displayed
    /// on the terminal and the transcript file, followed by `\.{=>}'.
    /// \TeX\ waits for a response. If the response is simply `carriage_return`, the
    /// line is accepted as it stands, otherwise the line typed is
    /// used instead of the line in the file.
    // §385
    pub fn firm_up_the_line(&mut self) {
        let mut k: i32 = 0; // §385
        self.cur_input.limit_field = self.last;
        if (self.eqtb[crate::ix::U(((29305i32) - 1) as usize)].int() > 0i32) {
            if (self.interaction > 1i32) {
                {
                    self.print_ln();
                    if (self.cur_input.start_field < self.cur_input.limit_field) {
                        {
                            let __for_end_6 = (self.cur_input.limit_field).wrapping_sub(1i32);
                            k = self.cur_input.start_field;
                            while k <= __for_end_6 {
                                self.print(self.buffer[crate::ix::U((k) as usize)]);
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                    self.first = self.cur_input.limit_field;
                    {
                        self.print(704i32);
                        self.term_input();
                    }
                    if (self.last > self.first) {
                        {
                            {
                                let __for_end_7 = (self.last).wrapping_sub(1i32);
                                k = self.first;
                                while k <= __for_end_7 {
                                    { let __ix246 = ((k).wrapping_add(self.cur_input.start_field)).wrapping_sub(self.first); let __v247 = self.buffer[crate::ix::U((k) as usize)]; self.buffer[crate::ix::U((__ix246) as usize)] = __v247; }
                                    k = k.wrapping_add(1);
                                }
                            }
                            self.cur_input.limit_field = ((self.cur_input.start_field).wrapping_add(self.last)).wrapping_sub(self.first);
                        }
                    }
                }
            }
        }
    }

    /// No new control sequences will be defined except during a call of
    /// `get_token`, or when \.{\\csname} compresses a token list, because
    /// `no_new_control_sequence` is always `true` at other times.
    // §387
    pub fn get_token(&mut self) {
        self.no_new_control_sequence = false;
        self.get_next();
        self.no_new_control_sequence = true;
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
        }
    }

    /// After parameter scanning is complete, the parameters are moved to the
    /// `param_stack`. Then the macro body is fed to the scanner; in other words,
    /// `macro_call` places the defined text of the control sequence at the
    /// top of\/ \TeX's input stack, so that `get_next` will proceed to read it
    /// next.
    /// The global variable `cur_cs` contains the `eqtb` address of the control sequence
    /// being expanded, when `macro_call` begins. If this control sequence has not been
    /// declared \.{\\long}, i.e., if its command code in the `eq_type` field is
    /// not `long_call` or `long_outer_call`, its parameters are not allowed to contain
    /// the control sequence \.{\\par}. If an illegal \.{\\par} appears, the macro
    /// call is aborted, and the \.{\\par} will be rescanned.
    /// @<Declare the procedure called `macro_call`
    // §415
    pub fn macro_call(&mut self) {
        let mut r: halfword = 0; // §415
        let mut p: halfword = 0; // §415
        let mut q: halfword = 0; // §415
        let mut s: halfword = 0; // §415
        let mut t: halfword = 0; // §415
        let mut u: halfword = 0; // §415
        let mut v: halfword = 0; // §415
        let mut rbrace_ptr: halfword = 0; // §415
        let mut n: small_number = 0; // §415
        let mut unbalance: halfword = 0; // §415
        let mut m: halfword = 0; // §415
        let mut ref_count: halfword = 0; // §415
        let mut save_scanner_status: small_number = 0; // §415
        let mut save_warning_index: halfword = 0; // §415
        let mut match_chr: ASCII_code = 0; // §415
        'l_exit_f: {
            save_scanner_status = self.scanner_status;
            save_warning_index = self.warning_index;
            if self.intr_at_switch {
                if ((self.intr_cand[crate::ix::U((self.cur_cs) as usize)] != 0i32) || self.intr_all) {
                    if self.flashtex_intr_call() {
                        break 'l_exit_f;
                    }
                }
            }
            self.warning_index = self.cur_cs;
            ref_count = self.cur_chr;
            r = self.mem[crate::ix::U((ref_count) as usize)].hh().rh();
            n = 0i32;
            if (self.eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 0i32) {
                // §427
                {
                    self.begin_diagnostic();
                    if (self.eqtb[crate::ix::U(((29335i32) - 1) as usize)].int() > 0i32) {
                        if (self.input_ptr < self.eqtb[crate::ix::U(((29335i32) - 1) as usize)].int()) {
                            {
                                v = self.input_ptr;
                                self.print_ln();
                                self.print_char(126i32);
                                while (v > 0i32) {
                                    {
                                        self.print_char(46i32);
                                        v = (v).wrapping_sub(1i32);
                                    }
                                }
                                self.print_cs(self.warning_index);
                                self.token_show(ref_count);
                            }
                        } else {
                            {
                                self.print_char(126i32);
                                self.print_char(126i32);
                                self.print_cs(self.warning_index);
                            }
                        }
                    } else {
                        {
                            self.print_ln();
                            self.print_cs(self.warning_index);
                            self.token_show(ref_count);
                        }
                    }
                    self.end_diagnostic(false);
                }
            }
            // §415
            if (self.mem[crate::ix::U((r) as usize)].hh().lh() == 3585i32) {
                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
            }
            if (self.mem[crate::ix::U((r) as usize)].hh().lh() != 3584i32) {
                // §417
                {
                    self.scanner_status = 3i32;
                    unbalance = 0i32;
                    self.long_state = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                    if (self.long_state >= 116i32) {
                        self.long_state = (self.long_state).wrapping_sub(2i32);
                    }
                    loop {
                        // goto labels: continue, found
                        let mut __goto_1: i32 = 0;
                        'l_dispatch_1: loop {
                            if __goto_1 <= 0 {
                                self.mem[crate::ix::U((4999996i32) as usize)].set_hh_rh(0i32);
                                if ((self.mem[crate::ix::U((r) as usize)].hh().lh() > 3583i32) || (self.mem[crate::ix::U((r) as usize)].hh().lh() < 3328i32)) {
                                    s = 0i32;
                                } else {
                                    {
                                        match_chr = (self.mem[crate::ix::U((r) as usize)].hh().lh()).wrapping_sub(3328i32);
                                        s = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                        r = s;
                                        p = 4999996i32;
                                        m = 0i32;
                                    }
                                }
                            }
                            if __goto_1 <= 1 { // continue
                                // §418
                                self.get_token();
                                if (self.cur_tok == self.mem[crate::ix::U((r) as usize)].hh().lh()) {
                                    // §420
                                    {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                        if ((self.mem[crate::ix::U((r) as usize)].hh().lh() >= 3328i32) && (self.mem[crate::ix::U((r) as usize)].hh().lh() <= 3584i32)) {
                                            {
                                                if (self.cur_tok < 512i32) {
                                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                                }
                                                { __goto_1 = 2; continue 'l_dispatch_1; }
                                            }
                                        } else {
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                                // §423
                                if (s != r) {
                                    if (s == 0i32) {
                                        // §424
                                        {
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(737i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(738i32);
                                            {
                                                self.help_ptr = 4i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] = 739i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 740i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 741i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 742i32;
                                            }
                                            self.error();
                                            break 'l_exit_f;
                                        }
                                    } else {
                                        // §423
                                        {
                                            t = s;
                                            loop {
                                                'l_done_f: {
                                                    {
                                                        q = self.get_avail();
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                        { let __v248 = self.mem[crate::ix::U((t) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v248); }
                                                        p = q;
                                                    }
                                                    m = (m).wrapping_add(1i32);
                                                    u = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                    v = s;
                                                    while true {
                                                        {
                                                            if (u == r) {
                                                                if (self.cur_tok != self.mem[crate::ix::U((v) as usize)].hh().lh()) {
                                                                    break 'l_done_f;
                                                                } else {
                                                                    {
                                                                        r = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                    }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((u) as usize)].hh().lh() != self.mem[crate::ix::U((v) as usize)].hh().lh()) {
                                                                break 'l_done_f;
                                                            }
                                                            u = self.mem[crate::ix::U((u) as usize)].hh().rh();
                                                            v = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                                t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                if (t == r) { break; }
                                            }
                                            r = s;
                                        }
                                    }
                                }
                                // §418
                                if (self.cur_tok == self.par_token) {
                                    if (self.long_state != 115i32) {
                                        // §422
                                        {
                                            if (self.long_state == 114i32) {
                                                {
                                                    self.runaway();
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(264i32);
                                                        }
                                                        self.print(732i32);
                                                    }
                                                    self.sprint_cs(self.warning_index);
                                                    self.print(733i32);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[crate::ix::U((2i32) as usize)] = 734i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 735i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 736i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                            { let __v249 = self.mem[crate::ix::U((4999996i32) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v249; }
                                            self.align_state = (self.align_state).wrapping_sub(unbalance);
                                            {
                                                let __for_end_11 = n;
                                                m = 0i32;
                                                while m <= __for_end_11 {
                                                    self.flush_list(self.pstack[crate::ix::U((m) as usize)]);
                                                    m = m.wrapping_add(1);
                                                }
                                            }
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                // §418
                                if (self.cur_tok < 768i32) {
                                    if (self.cur_tok < 512i32) {
                                        // §425
                                        {
                                            'l_done1_f: {
                                                unbalance = 1i32;
                                                while true {
                                                    {
                                                        {
                                                            {
                                                                q = self.avail;
                                                                if (q == 0i32) {
                                                                    q = self.get_avail();
                                                                } else {
                                                                    {
                                                                        self.avail = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(0i32);
                                                                        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                                                        self.dl_new_node(q);
                                                                    }
                                                                }
                                                            }
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            { let __v250 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v250); }
                                                            p = q;
                                                        }
                                                        self.get_token();
                                                        if (self.cur_tok == self.par_token) {
                                                            if (self.long_state != 115i32) {
                                                                // §422
                                                                {
                                                                    if (self.long_state == 114i32) {
                                                                        {
                                                                            self.runaway();
                                                                            {
                                                                                if (self.interaction == 3i32) {
                                                                                }
                                                                                if self.file_line_error_style_p {
                                                                                    self.print_file_line();
                                                                                } else {
                                                                                    self.print_nl(264i32);
                                                                                }
                                                                                self.print(732i32);
                                                                            }
                                                                            self.sprint_cs(self.warning_index);
                                                                            self.print(733i32);
                                                                            {
                                                                                self.help_ptr = 3i32;
                                                                                self.help_line[crate::ix::U((2i32) as usize)] = 734i32;
                                                                                self.help_line[crate::ix::U((1i32) as usize)] = 735i32;
                                                                                self.help_line[crate::ix::U((0i32) as usize)] = 736i32;
                                                                            }
                                                                            self.back_error();
                                                                        }
                                                                    }
                                                                    { let __v251 = self.mem[crate::ix::U((4999996i32) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v251; }
                                                                    self.align_state = (self.align_state).wrapping_sub(unbalance);
                                                                    {
                                                                        let __for_end_17 = n;
                                                                        m = 0i32;
                                                                        while m <= __for_end_17 {
                                                                            self.flush_list(self.pstack[crate::ix::U((m) as usize)]);
                                                                            m = m.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    break 'l_exit_f;
                                                                }
                                                            }
                                                        }
                                                        // §425
                                                        if (self.cur_tok < 768i32) {
                                                            if (self.cur_tok < 512i32) {
                                                                unbalance = (unbalance).wrapping_add(1i32);
                                                            } else {
                                                                {
                                                                    unbalance = (unbalance).wrapping_sub(1i32);
                                                                    if (unbalance == 0i32) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            rbrace_ptr = p;
                                            {
                                                q = self.get_avail();
                                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                { let __v252 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v252); }
                                                p = q;
                                            }
                                        }
                                    } else {
                                        // §421
                                        {
                                            self.back_input();
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(724i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(725i32);
                                            {
                                                self.help_ptr = 6i32;
                                                self.help_line[crate::ix::U((5i32) as usize)] = 726i32;
                                                self.help_line[crate::ix::U((4i32) as usize)] = 727i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] = 728i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 729i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 730i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 731i32;
                                            }
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                            self.long_state = 114i32;
                                            self.cur_tok = self.par_token;
                                            self.ins_error();
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                } else {
                                    // §419
                                    {
                                        if (self.cur_tok == 2592i32) {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().lh() <= 3584i32) {
                                                if (self.mem[crate::ix::U((r) as usize)].hh().lh() >= 3328i32) {
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v253 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v253); }
                                            p = q;
                                        }
                                    }
                                }
                                // §418
                                m = (m).wrapping_add(1i32);
                                if (self.mem[crate::ix::U((r) as usize)].hh().lh() > 3584i32) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                                if (self.mem[crate::ix::U((r) as usize)].hh().lh() < 3328i32) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                            if __goto_1 <= 2 { // found
                                if (s != 0i32) {
                                    // §426
                                    {
                                        if ((m == 1i32) && (self.mem[crate::ix::U((p) as usize)].hh().lh() < 768i32)) {
                                            {
                                                self.mem[crate::ix::U((rbrace_ptr) as usize)].set_hh_rh(0i32);
                                                {
                                                    { let __v254 = self.avail; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v254); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                                p = self.mem[crate::ix::U((4999996i32) as usize)].hh().rh();
                                                { let __v255 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v255; }
                                                {
                                                    { let __v256 = self.avail; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v256); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                        } else {
                                            { let __v257 = self.mem[crate::ix::U((4999996i32) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v257; }
                                        }
                                        n = (n).wrapping_add(1i32);
                                        if (self.eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 0i32) {
                                            if ((self.eqtb[crate::ix::U(((29335i32) - 1) as usize)].int() == 0i32) || (self.input_ptr < self.eqtb[crate::ix::U(((29335i32) - 1) as usize)].int())) {
                                                {
                                                    self.begin_diagnostic();
                                                    self.print_nl(match_chr);
                                                    self.print_int(((n) as i64));
                                                    self.print(743i32);
                                                    self.show_token_list(self.pstack[crate::ix::U(((n).wrapping_sub(1i32)) as usize)], 0i32, 1000i32);
                                                    self.end_diagnostic(false);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            break 'l_dispatch_1;
                        }
                        if (self.mem[crate::ix::U((r) as usize)].hh().lh() == 3584i32) { break; }
                    }
                }
            }
            // §416
            while (((self.cur_input.loc_field == 0i32) && (self.cur_input.index_field != 2i32)) && (self.cur_input.index_field != 6i32)) {
                self.end_token_list();
            }
            self.begin_token_list(ref_count, 5i32);
            self.cur_input.name_field = self.warning_index;
            self.cur_input.loc_field = self.mem[crate::ix::U((r) as usize)].hh().rh();
            if (self.ckpt_arm_cs != 0i32) {
                if (self.warning_index == self.ckpt_arm_cs) {
                    {
                        self.ckpt_arm_level = self.input_ptr;
                        self.ckpt_arm_cs = 0i32;
                    }
                }
            }
            if self.macro_prof_on {
                self.flashtex_prof_enter(self.warning_index);
            }
            if self.intr_rec_on {
                self.flashtex_intr_fed();
            }
            if (n > 0i32) {
                {
                    if ((self.param_ptr).wrapping_add(n) > self.max_param_stack) {
                        {
                            self.max_param_stack = (self.param_ptr).wrapping_add(n);
                            if (self.max_param_stack > param_size) {
                                self.overflow(723i32, param_size);
                            }
                        }
                    }
                    {
                        let __for_end_5 = (n).wrapping_sub(1i32);
                        m = 0i32;
                        while m <= __for_end_5 {
                            { let __ix258 = (self.param_ptr).wrapping_add(m); let __v259 = self.pstack[crate::ix::U((m) as usize)]; self.param_stack[crate::ix::U((__ix258) as usize)] = __v259; }
                            m = m.wrapping_add(1);
                        }
                    }
                    self.param_ptr = (self.param_ptr).wrapping_add(n);
                }
            }
        }
        // §415
        self.scanner_status = save_scanner_status;
        self.warning_index = save_warning_index;
    }

    /// Sometimes the expansion looks too far ahead, so we want to insert
    /// a harmless \.{\\relax} into the user's input.
    /// @<Declare the procedure called `insert_relax`
    // §405
    pub fn insert_relax(&mut self) {
        self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
        self.back_input();
        self.cur_tok = 19616i32;
        self.back_input();
        self.cur_input.index_field = 4i32;
    }

    /// There are seven almost identical doubly linked trees, one for the
    /// sparse array of the up to 32512 additional registers of each kind and
    /// one for the sparse array of the up to 32767 additional mark classes.
    /// The root of each such tree, if it exists, is an index node containing 16
    /// pointers to subtrees for 4096 consecutive array elements.  Similar index
    /// nodes are the starting points for all nonempty subtrees for 4096, 256,
    /// and 16 consecutive array elements.  These four levels of index nodes are
    /// followed by a fifth level with nodes for the individual array elements.
    /// Each index node is nine words long.  The pointers to the 16 possible
    /// subtrees or are kept in the `info` and `link` fields of the last eight
    /// words.  (It would be both elegant and efficient to declare them as
    /// array, unfortunately \PASCAL\ doesn't allow this.)
    /// The fields in the first word of each index node and in the nodes for the
    /// array elements are closely related.  The `link` field points to the next
    /// ...
    // §1815
    pub fn new_index(&mut self, mut i: quarterword, mut q: halfword) {
        let mut k: small_number = 0; // §1815
        self.cur_ptr = self.get_node(9i32);
        { let __ix260 = self.cur_ptr; self.mem[crate::ix::U((__ix260) as usize)].set_hh_b0(i); }
        { let __ix261 = self.cur_ptr; self.mem[crate::ix::U((__ix261) as usize)].set_hh_b1(0i32); }
        { let __ix262 = self.cur_ptr; self.mem[crate::ix::U((__ix262) as usize)].set_hh_rh(q); }
        {
            let __for_end_2 = 8i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __ix263 = (self.cur_ptr).wrapping_add(k); let __v264 = self.sa_null; self.mem[crate::ix::U((__ix263) as usize)] = __v264; }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Given a type `t` and a sixteen-bit number `n`, the `find_sa_element`
    /// procedure returns (in `cur_ptr`) a pointer to the node for the
    /// corresponding array element, or `null` when no such element exists.  The
    /// third parameter `w` is set `true` if the element must exist, e.g.,
    /// because it is about to be modified.  The procedure has two main
    /// branches:  one follows the existing tree structure, the other (only used
    /// when `w` is `true`) creates the missing nodes.
    /// We use macros to extract the four-bit pieces from a sixteen-bit register
    /// number or mark class and to fetch or store one of the 16 pointers from
    /// an index node.
    // §1819
    pub fn find_sa_element(&mut self, mut t: small_number, mut n: halfword, mut w: bool) {
        let mut q: halfword = 0; // §1819
        let mut i: small_number = 0; // §1819
        'l_exit_f: {
            'l_L49_f: {
                'l_L48_f: {
                    'l_L47_f: {
                        'l_not_found1_f: {
                            'l_not_found_f: {
                                if self.intr_rec_on {
                                    self.flashtex_intr_abort(3i32);
                                }
                                self.cur_ptr = self.sa_root[crate::ix::U((t) as usize)];
                                {
                                    if (self.cur_ptr == 0i32) {
                                        if w {
                                            break 'l_not_found_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = (n / 4096i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == 0i32) {
                                        if w {
                                            break 'l_not_found1_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = ((n / 256i32) % 16i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == 0i32) {
                                        if w {
                                            break 'l_L47_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = ((n / 16i32) % 16i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == 0i32) {
                                        if w {
                                            break 'l_L48_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = (n % 16i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                if ((self.cur_ptr == 0i32) && w) {
                                    break 'l_L49_f;
                                }
                                break 'l_exit_f;
                            }
                            self.new_index(t, 0i32);
                            { let __v265 = self.cur_ptr; self.sa_root[crate::ix::U((t) as usize)] = __v265; }
                            q = self.cur_ptr;
                            i = (n / 4096i32);
                        }
                        self.new_index(i, q);
                        {
                            if (((i) % 2) != 0) {
                                { let __v266 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v266); }
                            } else {
                                { let __v267 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v267); }
                            }
                            { let __v268 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v268); }
                        }
                        q = self.cur_ptr;
                        i = ((n / 256i32) % 16i32);
                    }
                    self.new_index(i, q);
                    {
                        if (((i) % 2) != 0) {
                            { let __v269 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v269); }
                        } else {
                            { let __v270 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v270); }
                        }
                        { let __v271 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v271); }
                    }
                    q = self.cur_ptr;
                    i = ((n / 16i32) % 16i32);
                }
                self.new_index(i, q);
                {
                    if (((i) % 2) != 0) {
                        { let __v272 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v272); }
                    } else {
                        { let __v273 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v273); }
                    }
                    { let __v274 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v274); }
                }
                q = self.cur_ptr;
                i = (n % 16i32);
            }
            if (t == 6i32) {
                // §1820
                {
                    self.cur_ptr = self.get_node(4i32);
                    { let __ix275 = (self.cur_ptr).wrapping_add(1i32); let __v276 = self.sa_null; self.mem[crate::ix::U((__ix275) as usize)] = __v276; }
                    { let __ix277 = (self.cur_ptr).wrapping_add(2i32); let __v278 = self.sa_null; self.mem[crate::ix::U((__ix277) as usize)] = __v278; }
                    { let __ix279 = (self.cur_ptr).wrapping_add(3i32); let __v280 = self.sa_null; self.mem[crate::ix::U((__ix279) as usize)] = __v280; }
                }
            } else {
                {
                    if (t <= 1i32) {
                        {
                            self.cur_ptr = self.get_node(3i32);
                            { let __ix281 = (self.cur_ptr).wrapping_add(2i32); self.mem[crate::ix::U((__ix281) as usize)].set_int(0i32); }
                            { let __ix282 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix282) as usize)].set_hh_rh(n); }
                        }
                    } else {
                        {
                            self.cur_ptr = self.get_node(2i32);
                            if (t <= 3i32) {
                                {
                                    { let __ix283 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix283) as usize)].set_hh_rh(0i32); }
                                    { let __v284 = (self.mem[crate::ix::U((0i32) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((0i32) as usize)].set_hh_rh(__v284); }
                                }
                            } else {
                                { let __ix285 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix285) as usize)].set_hh_rh(0i32); }
                            }
                        }
                    }
                    { let __ix286 = (self.cur_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix286) as usize)].set_hh_lh(0i32); }
                }
            }
            { let __ix287 = self.cur_ptr; self.mem[crate::ix::U((__ix287) as usize)].set_hh_b0(((16i32).wrapping_mul(t)).wrapping_add(i)); }
            { let __ix288 = self.cur_ptr; self.mem[crate::ix::U((__ix288) as usize)].set_hh_b1(1i32); }
            // §1819
            { let __ix289 = self.cur_ptr; self.mem[crate::ix::U((__ix289) as usize)].set_hh_rh(q); }
            {
                if (((i) % 2) != 0) {
                    { let __v290 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v290); }
                } else {
                    { let __v291 = self.cur_ptr; self.mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v291); }
                }
                { let __v292 = (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v292); }
            }
        }
    }

    /// \[25] Expanding the next token.
    /// Only a dozen or so command codes `>max_command` can possibly be returned by
    /// `get_next`; in increasing order, they are `undefined_cs`, `expand_after`,
    /// `no_expand`, `input`, `if_test`, `fi_or_else`, `cs_name`, `convert`, `the`,
    /// `top_bot_mark`, `call`, `long_call`, `outer_call`, `long_outer_call`, and
    /// `end_template`.{\emergencystretch=40pt\par}
    /// The `expand` subroutine is used when `cur_cmd>max_command`. It removes a
    /// ``call'' or a conditional or one of the other special operations just
    /// listed.  It follows that `expand` might invoke itself recursively. In all
    /// cases, `expand` destroys the current token, but it sets things up so that
    /// the next `get_next` will deliver the appropriate next token. The value of
    /// `cur_tok` need not be known when `expand` is called.
    /// Since several of the basic scanning routines communicate via global variables,
    /// their values are saved as local variables of `expand` so that
    /// ...
    // §388
    pub fn expand(&mut self) {
        let mut t: halfword = 0; // §388
        let mut b: bool = false; // §388
        let mut p: halfword = 0; // §388
        let mut q: halfword = 0; // §388
        let mut r: halfword = 0; // §388
        let mut j: i32 = 0; // §388
        let mut cv_backup: i32 = 0; // §388
        let mut cvl_backup: small_number = 0; // §388
        let mut radix_backup: small_number = 0; // §388
        let mut co_backup: small_number = 0; // §388
        let mut backup_backup: halfword = 0; // §388
        let mut save_scanner_status: small_number = 0; // §388
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(705i32, self.expand_depth);
        }
        cv_backup = self.cur_val;
        cvl_backup = self.cur_val_level;
        radix_backup = self.radix;
        co_backup = self.cur_order;
        backup_backup = self.mem[crate::ix::U((4999986i32) as usize)].hh().rh();
        'l_reswitch_b: loop {
            self.intr_at_switch = false;
            if self.intr_rec_on {
                self.flashtex_intr_expand();
            }
            if (self.cur_cmd < 114i32) {
                // §391
                {
                    if (self.eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() > 1i32) {
                        self.show_cur_cmd_chr();
                    }
                    match self.cur_cmd {
                        113 => {
                            // §412
                            {
                                t = (self.cur_chr % 5i32);
                                if (self.cur_chr >= 5i32) {
                                    self.scan_register_num();
                                } else {
                                    self.cur_val = 0i32;
                                }
                                if (self.cur_val == 0i32) {
                                    self.cur_ptr = self.cur_mark[crate::ix::U((t) as usize)];
                                } else {
                                    // §1824
                                    {
                                        self.find_sa_element(6i32, self.cur_val, false);
                                        if (self.cur_ptr != 0i32) {
                                            if (((t) % 2) != 0) {
                                                self.cur_ptr = self.mem[crate::ix::U((((self.cur_ptr).wrapping_add((t / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                            } else {
                                                self.cur_ptr = self.mem[crate::ix::U((((self.cur_ptr).wrapping_add((t / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                            }
                                        }
                                    }
                                }
                                // §412
                                if (self.cur_ptr != 0i32) {
                                    self.begin_token_list(self.cur_ptr, 14i32);
                                }
                            }
                        }
                        105 => {
                            // §391
                            if (self.cur_chr == 0i32) {
                                // §392
                                {
                                    self.get_token();
                                    t = self.cur_tok;
                                    self.get_token();
                                    if (self.cur_cmd > 103i32) {
                                        self.expand();
                                    } else {
                                        self.back_input();
                                    }
                                    self.cur_tok = t;
                                    self.back_input();
                                }
                            } else {
                                // §1765
                                {
                                    self.get_token();
                                    if ((self.cur_cmd == 108i32) && (self.cur_chr != 16i32)) {
                                        {
                                            self.cur_chr = (self.cur_chr).wrapping_add(32i32);
                                            continue 'l_reswitch_b;
                                        }
                                    }
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(786i32);
                                    }
                                    self.print_esc(929i32);
                                    self.print(2037i32);
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print_char(39i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 700i32;
                                    }
                                    self.back_error();
                                }
                            }
                        }
                        106 => {
                            // §391
                            if (self.cur_chr == 0i32) {
                                // §393
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = 0i32;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    t = self.cur_tok;
                                    self.back_input();
                                    if (t >= 4095i32) {
                                        {
                                            p = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_lh(19618i32);
                                            { let __v293 = self.cur_input.loc_field; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v293); }
                                            self.cur_input.start_field = p;
                                            self.cur_input.loc_field = p;
                                        }
                                    }
                                }
                            } else {
                                // §394
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = 0i32;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    if (self.cur_cs < 514i32) {
                                        self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                    } else {
                                        self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                                    }
                                    if (self.cur_cs != 0i32) {
                                        {
                                            t = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                            if (t > 103i32) {
                                                {
                                                    self.cur_cmd = t;
                                                    self.cur_chr = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                                    self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
                                                    self.cur_cs = 0i32;
                                                    continue 'l_reswitch_b;
                                                }
                                            } else {
                                                {
                                                    self.back_input();
                                                    p = self.get_avail();
                                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(19620i32);
                                                    { let __v294 = self.cur_input.loc_field; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v294); }
                                                    self.cur_input.loc_field = p;
                                                    self.cur_input.start_field = p;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        110 => {
                            // §398
                            {
                                r = self.get_avail();
                                p = r;
                                b = self.is_in_csname;
                                self.is_in_csname = true;
                                loop {
                                    self.get_x_token();
                                    if (self.cur_cs == 0i32) {
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v295 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v295); }
                                            p = q;
                                        }
                                    }
                                    if (self.cur_cs != 0i32) { break; }
                                }
                                if (self.cur_cmd != 67i32) {
                                    // §399
                                    {
                                        {
                                            if (self.interaction == 3i32) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(264i32);
                                            }
                                            self.print(712i32);
                                        }
                                        self.print_esc(581i32);
                                        self.print(713i32);
                                        {
                                            self.help_ptr = 2i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 714i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 715i32;
                                        }
                                        self.back_error();
                                    }
                                }
                                // §398
                                self.is_in_csname = b;
                                // §400
                                j = self.first;
                                p = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                while (p != 0i32) {
                                    {
                                        if (j >= self.max_buf_stack) {
                                            {
                                                self.max_buf_stack = (j).wrapping_add(1i32);
                                                if (self.max_buf_stack == buf_size) {
                                                    self.overflow(258i32, buf_size);
                                                }
                                            }
                                        }
                                        { let __v296 = (self.mem[crate::ix::U((p) as usize)].hh().lh() % 256i32); self.buffer[crate::ix::U((j) as usize)] = __v296; }
                                        j = (j).wrapping_add(1i32);
                                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    }
                                }
                                if (j > (self.first).wrapping_add(1i32)) {
                                    {
                                        self.no_new_control_sequence = false;
                                        self.cur_cs = self.id_lookup(self.first, (j).wrapping_sub(self.first));
                                        self.no_new_control_sequence = true;
                                    }
                                } else {
                                    if (j == self.first) {
                                        self.cur_cs = 513i32;
                                    } else {
                                        self.cur_cs = (257i32).wrapping_add(self.buffer[crate::ix::U((self.first) as usize)]);
                                    }
                                }
                                // §398
                                self.flush_list(r);
                                if self.intr_rec_on {
                                    self.flashtex_intr_read(self.cur_cs);
                                }
                                if (self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0() == 104i32) {
                                    {
                                        self.eq_define(self.cur_cs, 0i32, 256i32);
                                    }
                                }
                                self.cur_tok = (self.cur_cs).wrapping_add(4095i32);
                                self.back_input();
                            }
                        }
                        111 => {
                            // §391
                            self.conv_toks();
                        }
                        112 => {
                            self.ins_the_toks();
                        }
                        108 => {
                            self.conditional();
                        }
                        109 => {
                            // §536
                            {
                                if (self.eqtb[crate::ix::U(((29381i32) - 1) as usize)].int() > 0i32) {
                                    if (self.eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() <= 1i32) {
                                        self.show_cur_cmd_chr();
                                    }
                                }
                                if (self.cur_chr > self.if_limit) {
                                    if (self.if_limit == 1i32) {
                                        self.insert_relax();
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
                                                self.print(933i32);
                                            }
                                            self.print_cmd_chr(109i32, self.cur_chr);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 934i32;
                                            }
                                            self.error();
                                        }
                                    }
                                } else {
                                    {
                                        while (self.cur_chr != 2i32) {
                                            self.pass_text();
                                        }
                                        // §522
                                        {
                                            if self.intr_rec_on {
                                                self.flashtex_intr_pop_cond();
                                            }
                                            if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                                                self.if_warning();
                                            }
                                            p = self.cond_ptr;
                                            self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                            self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                            self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                            self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                            self.free_node(p, 2i32);
                                        }
                                    }
                                }
                            }
                        }
                        107 => {
                            // §404
                            if (self.cur_chr == 1i32) {
                                self.force_eof = true;
                            } else {
                                // §1749
                                if (self.cur_chr == 2i32) {
                                    self.pseudo_start();
                                } else {
                                    // §404
                                    if self.name_in_progress {
                                        self.insert_relax();
                                    } else {
                                        self.start_input();
                                    }
                                }
                            }
                        }
                        _ => {
                            // §396
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(706i32);
                                }
                                {
                                    self.help_ptr = 5i32;
                                    self.help_line[crate::ix::U((4i32) as usize)] = 707i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 708i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 709i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 710i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 711i32;
                                }
                                self.error();
                            }
                        }
                    }
                }
            } else {
                // §388
                if (self.cur_cmd < 118i32) {
                    self.macro_call();
                } else {
                    // §401
                    {
                        self.cur_tok = 19615i32;
                        self.back_input();
                    }
                }
            }
            // §388
            self.cur_val = cv_backup;
            self.cur_val_level = cvl_backup;
            self.radix = radix_backup;
            self.cur_order = co_backup;
            self.mem[crate::ix::U((4999986i32) as usize)].set_hh_rh(backup_backup);
            self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
            break 'l_reswitch_b;
        }
    }

    /// Here is a recursive procedure that is \TeX's usual way to get the
    /// next token of input. It has been slightly optimized to take account of
    /// common cases.
    // §406
    pub fn get_x_token(&mut self) {
        // goto labels: restart, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.get_next();
                if (self.cur_cmd <= 103i32) {
                    { __goto_1 = 1; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd >= 114i32) {
                    if (self.cur_cmd < 118i32) {
                        self.macro_call();
                    } else {
                        {
                            self.cur_cs = 15520i32;
                            self.cur_cmd = 9i32;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                } else {
                    self.expand();
                }
                { __goto_1 = 0; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 1 { // done
                if (self.cur_cs == 0i32) {
                    self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
                } else {
                    self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
                }
            }
            break 'l_dispatch_1;
        }
    }

    /// The `get_x_token` procedure is essentially equivalent to two consecutive
    /// procedure calls: `get_next; x_token`.
    // §407
    pub fn x_token(&mut self) {
        while (self.cur_cmd > 103i32) {
            {
                self.expand();
                self.get_next();
            }
        }
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
        }
    }

    /// The `scan_left_brace` routine is called when a left brace is supposed to be
    /// the next non-blank token. (The term ``left brace'' means, more precisely,
    /// a character whose catcode is `left_brace`.) \TeX\ allows \.{\\relax} to
    /// appear before the `left_brace`.
    // §429
    pub fn scan_left_brace(&mut self) {
        // §430
        loop {
            self.get_x_token();
            if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
        }
        // §429
        if (self.cur_cmd != 1i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(744i32);
                }
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 745i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 746i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 747i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 748i32;
                }
                self.back_error();
                self.cur_tok = 379i32;
                self.cur_cmd = 1i32;
                self.cur_chr = 123i32;
                self.align_state = (self.align_state).wrapping_add(1i32);
            }
        }
    }

    /// The `scan_optional_equals` routine looks for an optional `\.=' sign preceded
    /// by optional spaces; `\.{\\relax}' is not ignored here.
    // §431
    pub fn scan_optional_equals(&mut self) {
        // §432
        loop {
            self.get_x_token();
            if (self.cur_cmd != 10i32) { break; }
        }
        // §431
        if (self.cur_tok != 3133i32) {
            self.back_input();
        }
    }

    /// In case you are getting bored, here is a slightly less trivial routine:
    /// Given a string of lowercase letters, like `\.{pt}' or `\.{plus}' or
    /// `\.{width}', the `scan_keyword` routine checks to see whether the next
    /// tokens of input match this string. The match must be exact, except that
    /// uppercase letters will match their lowercase counterparts; uppercase
    /// equivalents are determined by subtracting `"a"-"A"`, rather than using the
    /// `uc_code` table, since \TeX\ uses this routine only for its own limited
    /// set of keywords.
    /// If a match is found, the characters are effectively removed from the input
    /// and `true` is returned. Otherwise `false` is returned, and the input
    /// is left essentially unchanged (except for the fact that some macros
    /// may have been expanded, etc.).
    // §433
    pub fn scan_keyword(&mut self, mut s: str_number) -> bool {
        let mut scan_keyword: bool = false;
        let mut p: halfword = 0; // §433
        let mut q: halfword = 0; // §433
        let mut k: pool_pointer = 0; // §433
        let mut save_cur_cs: halfword = 0; // §433
        'l_exit_f: {
            p = 4999986i32;
            self.mem[crate::ix::U((p) as usize)].set_hh_rh(0i32);
            k = self.str_start[crate::ix::U((s) as usize)];
            save_cur_cs = self.cur_cs;
            while (k < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
                {
                    self.get_x_token();
                    if ((self.cur_cs == 0i32) && ((self.cur_chr == self.str_pool[crate::ix::U((k) as usize)]) || (self.cur_chr == (self.str_pool[crate::ix::U((k) as usize)]).wrapping_sub(32i32)))) {
                        {
                            {
                                q = self.get_avail();
                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                { let __v297 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v297); }
                                p = q;
                            }
                            k = (k).wrapping_add(1i32);
                        }
                    } else {
                        if ((self.cur_cmd != 10i32) || (p != 4999986i32)) {
                            {
                                self.back_input();
                                if (p != 4999986i32) {
                                    self.begin_token_list(self.mem[crate::ix::U((4999986i32) as usize)].hh().rh(), 3i32);
                                }
                                self.cur_cs = save_cur_cs;
                                scan_keyword = false;
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            self.flush_list(self.mem[crate::ix::U((4999986i32) as usize)].hh().rh());
            scan_keyword = true;
        }
        scan_keyword
    }

    /// Here is a procedure that sounds an alarm when mu and non-mu units
    /// are being switched.
    // §434
    pub fn mu_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(749i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 750i32;
        }
        self.error();
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §459
    pub fn scan_eight_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(788i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 789i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §460
    pub fn scan_char_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(791i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 792i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §461
    pub fn scan_four_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 15i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(793i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 794i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §462
    pub fn scan_fifteen_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 32767i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(795i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 796i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §463
    pub fn scan_twenty_seven_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 134217727i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(797i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 798i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §1811
    pub fn scan_register_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > self.max_reg_num)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(788i32);
                }
                {
                    self.help_ptr = 2i32;
                    { let __v298 = self.max_reg_help_line; self.help_line[crate::ix::U((1i32) as usize)] = __v298; }
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// To be able to determine whether \.{\\write18} is enabled from within
    /// \TeX\ we also implement \.{\\eof18}.  We sort of cheat by having an
    /// additional route `scan_four_bit_int_or_18` which is the same as
    /// `scan_four_bit_int` except it also accepts the value 18.
    /// @<Declare procedures that scan restricted classes of integers
    // §1891
    pub fn scan_four_bit_int_or_18(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || ((self.cur_val > 15i32) && (self.cur_val != 18i32))) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(793i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 794i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// The `get_x_or_protected` procedure is like `get_x_token` except that
    /// protected macros are not expanded.
    /// @<Declare \eTeX\ procedures for sc...
    // §1772
    pub fn get_x_or_protected(&mut self) {
        'l_exit_f: {
            while true {
                {
                    self.get_token();
                    if (self.cur_cmd <= 103i32) {
                        break 'l_exit_f;
                    }
                    if ((self.cur_cmd >= 114i32) && (self.cur_cmd < 118i32)) {
                        if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh()) as usize)].hh().lh() == 3585i32) {
                            break 'l_exit_f;
                        }
                    }
                    self.expand();
                }
            }
        }
    }

    /// Before we forget about the format of these tables, let's deal with two
    /// of \TeX's basic scanning routines related to font information.
    /// @<Declare procedures that scan font-related stuff
    // §604
    pub fn test_no_ligatures(&mut self, mut f: internal_font_number) -> i32 {
        let mut test_no_ligatures: i32 = 0;
        let mut c: i32 = 0; // §604
        'l_exit_f: {
            test_no_ligatures = 1i32;
            {
                let __for_end_3 = self.font_ec[crate::ix::U((f) as usize)];
                c = self.font_bc[crate::ix::U((f) as usize)];
                while c <= __for_end_3 {
                    if (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > 0i32) {
                        if (((((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b2()).wrapping_sub(0i32) % 4i32)) % 2) != 0) {
                            {
                                test_no_ligatures = 0i32;
                                break 'l_exit_f;
                            }
                        }
                    }
                    c = c.wrapping_add(1);
                }
            }
        }
        test_no_ligatures
    }

    /// Before we forget about the format of these tables, let's deal with two
    /// of \TeX's basic scanning routines related to font information.
    /// @<Declare procedures that scan font-related stuff
    // §604
    pub fn get_tag_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_tag_code: i32 = 0;
        let mut i: small_number = 0; // §604
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > 0i32)) {
            {
                i = ((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b2()).wrapping_sub(0i32) % 4i32);
                if (i == 1i32) {
                    get_tag_code = 1i32;
                } else {
                    if (i == 2i32) {
                        get_tag_code = 2i32;
                    } else {
                        if (i == 3i32) {
                            get_tag_code = 4i32;
                        } else {
                            get_tag_code = 0i32;
                        }
                    }
                }
            }
        } else {
            get_tag_code = (1i32).wrapping_neg();
        }
        get_tag_code
    }

    /// Before we forget about the format of these tables, let's deal with two
    /// of \TeX's basic scanning routines related to font information.
    /// @<Declare procedures that scan font-related stuff
    // §604
    pub fn scan_font_ident(&mut self) {
        let mut f: internal_font_number = 0; // §604
        let mut m: halfword = 0; // §604
        // §432
        loop {
            self.get_x_token();
            if (self.cur_cmd != 10i32) { break; }
        }
        // §604
        if (((self.cur_cmd == 88i32) || (self.cur_cmd == 101i32)) || (self.cur_cmd == 102i32)) {
            f = self.eqtb[crate::ix::U(((27689i32) - 1) as usize)].hh().rh();
        } else {
            if (self.cur_cmd == 87i32) {
                f = self.cur_chr;
            } else {
                if (self.cur_cmd == 86i32) {
                    {
                        m = self.cur_chr;
                        self.scan_four_bit_int();
                        f = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
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
                            self.print(978i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 979i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 980i32;
                        }
                        self.back_error();
                        f = 0i32;
                    }
                }
            }
        }
        self.cur_val = f;
    }

    /// The following routine is used to implement `\.{\\fontdimen} `n` `f`'.
    /// The boolean parameter `writing` is set `true` if the calling program
    /// intends to change the parameter value.
    /// @<Declare procedures that scan font-related stuff
    // §605
    pub fn find_font_dimen(&mut self, mut writing: bool) {
        let mut f: internal_font_number = 0; // §605
        let mut n: i32 = 0; // §605
        self.scan_int();
        n = self.cur_val;
        self.scan_font_ident();
        f = self.cur_val;
        if (n <= 0i32) {
            self.cur_val = self.fmem_ptr;
        } else {
            {
                if (((writing && (n <= 4i32)) && (n >= 2i32)) && (self.font_glue[crate::ix::U((f) as usize)] != 0i32)) {
                    {
                        self.delete_glue_ref(self.font_glue[crate::ix::U((f) as usize)]);
                        self.font_glue[crate::ix::U((f) as usize)] = 0i32;
                    }
                }
                if (n > self.font_params[crate::ix::U((f) as usize)]) {
                    if (f < self.font_ptr) {
                        self.cur_val = self.fmem_ptr;
                    } else {
                        // §607
                        {
                            loop {
                                if (self.fmem_ptr == font_mem_size) {
                                    self.overflow(985i32, font_mem_size);
                                }
                                { let __ix299 = self.fmem_ptr; self.font_info[crate::ix::U((__ix299) as usize)].set_int(0i32); }
                                self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                                { let __v300 = (self.font_params[crate::ix::U((f) as usize)]).wrapping_add(1i32); self.font_params[crate::ix::U((f) as usize)] = __v300; }
                                if (n == self.font_params[crate::ix::U((f) as usize)]) { break; }
                            }
                            self.cur_val = (self.fmem_ptr).wrapping_sub(1i32);
                        }
                    }
                } else {
                    // §605
                    self.cur_val = (n).wrapping_add(self.param_base[crate::ix::U((f) as usize)]);
                }
            }
        }
        // §606
        if (self.cur_val == self.fmem_ptr) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(960i32);
                }
                self.print_esc(self.hash[crate::ix::U((((17626i32).wrapping_add(f)) - 514) as usize)].rh());
                self.print(981i32);
                self.print_int(((self.font_params[crate::ix::U((f) as usize)]) as i64));
                self.print(982i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 983i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 984i32;
                }
                self.error();
            }
        }
    }

    /// OK, we're ready for `scan_something_internal` itself. A second parameter,
    /// `negative`, is set `true` if the value that is found should be negated.
    /// It is assumed that `cur_cmd` and `cur_chr` represent the first token of
    /// the internal quantity to be scanned; an error will be signalled if
    /// `cur_cmd<min_internal` or `cur_cmd>max_internal`.
    // §439
    pub fn scan_something_internal(&mut self, mut level: small_number, mut negative: bool) {
        let mut m: halfword = 0; // §439
        let mut n: i32 = 0; // §439
        let mut k: i32 = 0; // §439
        let mut q: halfword = 0; // §439
        let mut r: halfword = 0; // §439
        let mut tx: halfword = 0; // §439
        let mut i: four_quarters = four_quarters::default(); // §439
        let mut p: i32 = 0; // §439
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                m = self.cur_chr;
                if self.intr_rec_on {
                    self.flashtex_intr_internal();
                }
                match self.cur_cmd {
                    85 => {
                        // §440
                        {
                            self.scan_char_num();
                            if self.intr_rec_on {
                                self.flashtex_intr_read((m).wrapping_add(self.cur_val));
                            }
                            if (m == 28765i32) {
                                {
                                    self.cur_val = (self.eqtb[crate::ix::U((((28765i32).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh()).wrapping_sub(0i32);
                                    self.cur_val_level = 0i32;
                                }
                            } else {
                                if (m < 28765i32) {
                                    {
                                        self.cur_val = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                        self.cur_val_level = 0i32;
                                    }
                                } else {
                                    {
                                        self.cur_val = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                        self.cur_val_level = 0i32;
                                    }
                                }
                            }
                        }
                    }
                    71 | 72 | 86 | 87 | 88 | 101 | 102 => {
                        // §441
                        if (level != 5i32) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(751i32);
                                }
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 752i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 753i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 754i32;
                                }
                                self.back_error();
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = 1i32;
                                }
                            }
                        } else {
                            if (self.cur_cmd <= 72i32) {
                                {
                                    if (self.cur_cmd < 72i32) {
                                        if (m == 0i32) {
                                            {
                                                self.scan_register_num();
                                                if (self.cur_val < 256i32) {
                                                    self.cur_val = self.eqtb[crate::ix::U((((27173i32).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                                } else {
                                                    {
                                                        self.find_sa_element(5i32, self.cur_val, false);
                                                        if (self.cur_ptr == 0i32) {
                                                            self.cur_val = 0i32;
                                                        } else {
                                                            self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                        }
                                    } else {
                                        self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                                    }
                                    self.cur_val_level = 5i32;
                                }
                            } else {
                                {
                                    self.back_input();
                                    self.scan_font_ident();
                                    {
                                        self.cur_val = (17626i32).wrapping_add(self.cur_val);
                                        self.cur_val_level = 4i32;
                                    }
                                }
                            }
                        }
                    }
                    73 => {
                        // §439
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = 0i32;
                        }
                    }
                    74 => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = 1i32;
                        }
                    }
                    75 => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = 2i32;
                        }
                    }
                    76 => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = 3i32;
                        }
                    }
                    79 => {
                        // §444
                        if ((self.cur_list.mode_field).wrapping_abs() != m) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(781i32);
                                }
                                self.print_cmd_chr(79i32, m);
                                {
                                    self.help_ptr = 4i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 782i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 783i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 784i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 785i32;
                                }
                                self.error();
                                if (level != 5i32) {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = 1i32;
                                    }
                                } else {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = 0i32;
                                    }
                                }
                            }
                        } else {
                            if (m == 1i32) {
                                {
                                    self.cur_val = self.cur_list.aux_field.int();
                                    self.cur_val_level = 1i32;
                                }
                            } else {
                                {
                                    self.cur_val = self.cur_list.aux_field.hh().lh();
                                    self.cur_val_level = 0i32;
                                }
                            }
                        }
                    }
                    80 => {
                        // §448
                        if (self.cur_list.mode_field == 0i32) {
                            {
                                self.cur_val = 0i32;
                                self.cur_val_level = 0i32;
                            }
                        } else {
                            {
                                { let __ix301 = self.nest_ptr; let __v302 = self.cur_list; self.nest[crate::ix::U((__ix301) as usize)] = __v302; }
                                p = self.nest_ptr;
                                while ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() != 1i32) {
                                    p = (p).wrapping_sub(1i32);
                                }
                                {
                                    self.cur_val = self.nest[crate::ix::U((p) as usize)].pg_field;
                                    self.cur_val_level = 0i32;
                                }
                            }
                        }
                    }
                    82 => {
                        // §445
                        {
                            if (m == 0i32) {
                                self.cur_val = self.dead_cycles;
                            } else {
                                // §1694
                                if (m == 2i32) {
                                    self.cur_val = self.interaction;
                                } else {
                                    // §445
                                    self.cur_val = self.insert_penalties;
                                }
                            }
                            self.cur_val_level = 0i32;
                        }
                    }
                    81 => {
                        // §447
                        {
                            if ((self.page_contents == 0i32) && (!self.output_active)) {
                                if (m == 0i32) {
                                    self.cur_val = 1073741823i32;
                                } else {
                                    self.cur_val = 0i32;
                                }
                            } else {
                                self.cur_val = self.page_so_far[crate::ix::U((m) as usize)];
                            }
                            self.cur_val_level = 1i32;
                        }
                    }
                    84 => {
                        // §449
                        {
                            if (m > 27158i32) {
                                // §1866
                                {
                                    self.scan_int();
                                    if ((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh() == 0i32) || (self.cur_val < 0i32)) {
                                        self.cur_val = 0i32;
                                    } else {
                                        {
                                            if (self.cur_val > self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()) {
                                                self.cur_val = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int();
                                            }
                                            self.cur_val = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(self.cur_val)).wrapping_add(1i32)) as usize)].int();
                                        }
                                    }
                                }
                            } else {
                                // §449
                                if (self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh() == 0i32) {
                                    self.cur_val = 0i32;
                                } else {
                                    self.cur_val = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                }
                            }
                            self.cur_val_level = 0i32;
                        }
                    }
                    83 => {
                        // §446
                        {
                            self.scan_register_num();
                            if (self.cur_val < 256i32) {
                                q = self.eqtb[crate::ix::U((((27433i32).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(4i32, self.cur_val, false);
                                    if (self.cur_ptr == 0i32) {
                                        q = 0i32;
                                    } else {
                                        q = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                            if (q == 0i32) {
                                self.cur_val = 0i32;
                            } else {
                                self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(m)) as usize)].int();
                            }
                            self.cur_val_level = 1i32;
                        }
                    }
                    68 | 69 => {
                        // §439
                        {
                            self.cur_val = self.cur_chr;
                            self.cur_val_level = 0i32;
                        }
                    }
                    77 => {
                        // §451
                        {
                            self.find_font_dimen(false);
                            { let __ix303 = self.fmem_ptr; self.font_info[crate::ix::U((__ix303) as usize)].set_int(0i32); }
                            {
                                self.cur_val = self.font_info[crate::ix::U((self.cur_val) as usize)].int();
                                self.cur_val_level = 1i32;
                            }
                        }
                    }
                    78 => {
                        // §452
                        {
                            self.scan_font_ident();
                            if (m == 0i32) {
                                {
                                    self.cur_val = self.hyphen_char[crate::ix::U((self.cur_val) as usize)];
                                    self.cur_val_level = 0i32;
                                }
                            } else {
                                if (m == 1i32) {
                                    {
                                        self.cur_val = self.skew_char[crate::ix::U((self.cur_val) as usize)];
                                        self.cur_val_level = 0i32;
                                    }
                                } else {
                                    if (m == 6i32) {
                                        {
                                            self.cur_val = self.test_no_ligatures(self.cur_val);
                                            self.cur_val_level = 0i32;
                                        }
                                    } else {
                                        {
                                            n = self.cur_val;
                                            self.scan_char_num();
                                            k = self.cur_val;
                                            match m {
                                                2 => {
                                                    {
                                                        self.cur_val = self.get_lp_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                3 => {
                                                    {
                                                        self.cur_val = self.get_rp_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                4 => {
                                                    {
                                                        self.cur_val = self.get_ef_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                5 => {
                                                    {
                                                        self.cur_val = self.get_tag_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                7 => {
                                                    {
                                                        self.cur_val = self.get_kn_bs_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                8 => {
                                                    {
                                                        self.cur_val = self.get_st_bs_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                9 => {
                                                    {
                                                        self.cur_val = self.get_sh_bs_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                10 => {
                                                    {
                                                        self.cur_val = self.get_kn_bc_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                11 => {
                                                    {
                                                        self.cur_val = self.get_kn_ac_code(n, k);
                                                        self.cur_val_level = 0i32;
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    89 => {
                        // §453
                        {
                            if ((m < 0i32) || (m > 19i32)) {
                                {
                                    self.cur_val_level = (self.mem[crate::ix::U((m) as usize)].hh().b0() / 16i32);
                                    if (self.cur_val_level < 2i32) {
                                        self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(2i32)) as usize)].int();
                                    } else {
                                        self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            } else {
                                {
                                    self.scan_register_num();
                                    self.cur_val_level = (m).wrapping_sub(0i32);
                                    if (self.cur_val > 255i32) {
                                        {
                                            self.find_sa_element(self.cur_val_level, self.cur_val, false);
                                            if (self.cur_ptr == 0i32) {
                                                if (self.cur_val_level < 2i32) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    self.cur_val = 0i32;
                                                }
                                            } else {
                                                if (self.cur_val_level < 2i32) {
                                                    self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].int();
                                                } else {
                                                    self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    } else {
                                        match self.cur_val_level {
                                            0 => {
                                                self.cur_val = self.eqtb[crate::ix::U((((29391i32).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            1 => {
                                                self.cur_val = self.eqtb[crate::ix::U((((29937i32).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            2 => {
                                                self.cur_val = self.eqtb[crate::ix::U((((26646i32).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            3 => {
                                                self.cur_val = self.eqtb[crate::ix::U((((26902i32).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    70 => {
                        // §450
                        if (m >= 4i32) {
                            if (m >= 37i32) {
                                // §1780
                                {
                                    if (m < 38i32) {
                                        {
                                            match m {
                                                37 => {
                                                    // §1807
                                                    self.scan_mu_glue();
                                                }
                                                _ => {}
                                            }
                                            // §1780
                                            self.cur_val_level = 2i32;
                                        }
                                    } else {
                                        if (m < 39i32) {
                                            {
                                                match m {
                                                    38 => {
                                                        // §1808
                                                        self.scan_normal_glue();
                                                    }
                                                    _ => {}
                                                }
                                                // §1780
                                                self.cur_val_level = 3i32;
                                            }
                                        } else {
                                            {
                                                self.cur_val_level = (m).wrapping_sub(39i32);
                                                self.scan_expr();
                                            }
                                        }
                                    }
                                    while (self.cur_val_level > level) {
                                        {
                                            if (self.cur_val_level == 2i32) {
                                                {
                                                    m = self.cur_val;
                                                    self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].int();
                                                    self.delete_glue_ref(m);
                                                }
                                            } else {
                                                if (self.cur_val_level == 3i32) {
                                                    self.mu_error();
                                                }
                                            }
                                            self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                                        }
                                    }
                                    if negative {
                                        if (self.cur_val_level >= 2i32) {
                                            {
                                                m = self.cur_val;
                                                self.cur_val = self.new_spec(m);
                                                self.delete_glue_ref(m);
                                                // §457
                                                {
                                                    { let __ix304 = (self.cur_val).wrapping_add(1i32); let __v305 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix304) as usize)].set_int(__v305); }
                                                    { let __ix306 = (self.cur_val).wrapping_add(2i32); let __v307 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix306) as usize)].set_int(__v307); }
                                                    { let __ix308 = (self.cur_val).wrapping_add(3i32); let __v309 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix308) as usize)].set_int(__v309); }
                                                }
                                            }
                                        } else {
                                            // §1780
                                            self.cur_val = (self.cur_val).wrapping_neg();
                                        }
                                    }
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            } else {
                                // §450
                                if (m >= 28i32) {
                                    {
                                        match m {
                                            28 | 29 | 30 | 31 => {
                                                // §1671
                                                {
                                                    self.scan_font_ident();
                                                    q = self.cur_val;
                                                    self.scan_char_num();
                                                    if ((self.font_bc[crate::ix::U((q) as usize)] <= self.cur_val) && (self.font_ec[crate::ix::U((q) as usize)] >= self.cur_val)) {
                                                        {
                                                            i = self.font_info[crate::ix::U((((self.char_base[crate::ix::U((q) as usize)]).wrapping_add(self.cur_val)).wrapping_add(0i32)) as usize)].qqqq();
                                                            match m {
                                                                28 => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((q) as usize)]).wrapping_add(i.b0())) as usize)].int();
                                                                }
                                                                29 => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((q) as usize)]).wrapping_add(((i.b1()).wrapping_sub(0i32) / 16i32))) as usize)].int();
                                                                }
                                                                30 => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((q) as usize)]).wrapping_add(((i.b1()).wrapping_sub(0i32) % 16i32))) as usize)].int();
                                                                }
                                                                31 => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((q) as usize)]).wrapping_add(((i.b2()).wrapping_sub(0i32) / 4i32))) as usize)].int();
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            32 | 33 | 34 => {
                                                // §1674
                                                {
                                                    q = (self.cur_chr).wrapping_sub(32i32);
                                                    self.scan_int();
                                                    if ((self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh() == 0i32) || (self.cur_val <= 0i32)) {
                                                        self.cur_val = 0i32;
                                                    } else {
                                                        {
                                                            if (q == 2i32) {
                                                                {
                                                                    q = (self.cur_val % 2i32);
                                                                    self.cur_val = ((self.cur_val).wrapping_add(q) / 2i32);
                                                                }
                                                            }
                                                            if (self.cur_val > self.mem[crate::ix::U((self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh()) as usize)].hh().lh()) {
                                                                self.cur_val = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                                            }
                                                            self.cur_val = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((27158i32) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(self.cur_val))).wrapping_sub(q)) as usize)].int();
                                                        }
                                                    }
                                                    self.cur_val_level = 1i32;
                                                }
                                            }
                                            35 | 36 => {
                                                // §1804
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == 35i32) {
                                                        self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                                                    } else {
                                                        self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §450
                                        self.cur_val_level = 1i32;
                                    }
                                } else {
                                    {
                                        match m {
                                            4 => {
                                                self.cur_val = self.line;
                                            }
                                            5 => {
                                                self.cur_val = self.last_badness;
                                            }
                                            6 => {
                                                self.cur_val = 140i32;
                                            }
                                            7 => {
                                                self.cur_val = self.pdf_last_obj;
                                            }
                                            8 => {
                                                self.cur_val = self.pdf_last_xform;
                                            }
                                            9 => {
                                                self.cur_val = self.pdf_last_ximage;
                                            }
                                            10 => {
                                                self.cur_val = self.pdf_last_ximage_pages;
                                            }
                                            11 => {
                                                self.cur_val = self.pdf_last_annot;
                                            }
                                            12 => {
                                                self.cur_val = self.pdf_last_x_pos;
                                            }
                                            13 => {
                                                self.cur_val = self.pdf_last_y_pos;
                                            }
                                            14 => {
                                                self.cur_val = self.pdf_retval;
                                            }
                                            15 => {
                                                self.cur_val = self.pdf_last_ximage_colordepth;
                                            }
                                            16 => {
                                                self.cur_val = self.get_microinterval();
                                            }
                                            18 => {
                                                self.cur_val = self.random_seed;
                                            }
                                            17 => {
                                                {
                                                    if self.shellenabledp {
                                                        {
                                                            if self.restrictedshell {
                                                                self.cur_val = 2i32;
                                                            } else {
                                                                self.cur_val = 1i32;
                                                            }
                                                        }
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            19 => {
                                                self.cur_val = self.pdf_last_link;
                                            }
                                            20 => {
                                                // §1651
                                                self.cur_val = 2i32;
                                            }
                                            21 => {
                                                // §1665
                                                self.cur_val = (self.cur_level).wrapping_sub(1i32);
                                            }
                                            22 => {
                                                self.cur_val = self.cur_group;
                                            }
                                            23 => {
                                                // §1668
                                                {
                                                    q = self.cond_ptr;
                                                    self.cur_val = 0i32;
                                                    while (q != 0i32) {
                                                        {
                                                            self.cur_val = (self.cur_val).wrapping_add(1i32);
                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                            24 => {
                                                if (self.cond_ptr == 0i32) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    if (self.cur_if < 32i32) {
                                                        self.cur_val = (self.cur_if).wrapping_add(1i32);
                                                    } else {
                                                        self.cur_val = ((self.cur_if).wrapping_sub(31i32)).wrapping_neg();
                                                    }
                                                }
                                            }
                                            25 => {
                                                if ((self.if_limit == 4i32) || (self.if_limit == 3i32)) {
                                                    self.cur_val = 1i32;
                                                } else {
                                                    if (self.if_limit == 2i32) {
                                                        self.cur_val = (1i32).wrapping_neg();
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            26 | 27 => {
                                                // §1803
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == 26i32) {
                                                        self.cur_val = self.mem[crate::ix::U((q) as usize)].hh().b0();
                                                    } else {
                                                        self.cur_val = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §450
                                        self.cur_val_level = 0i32;
                                    }
                                }
                            }
                        } else {
                            {
                                if (self.cur_chr == 2i32) {
                                    self.cur_val = 0i32;
                                } else {
                                    self.cur_val = 0i32;
                                }
                                tx = self.cur_list.tail_field;
                                if (!(tx >= self.hi_mem_min)) {
                                    if ((self.mem[crate::ix::U((tx) as usize)].hh().b0() == 9i32) && (self.mem[crate::ix::U((tx) as usize)].hh().b1() == 3i32)) {
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
                                if (self.cur_chr == 3i32) {
                                    {
                                        self.cur_val_level = 0i32;
                                        if ((tx == self.cur_list.head_field) || (self.cur_list.mode_field == 0i32)) {
                                            self.cur_val = (1i32).wrapping_neg();
                                        }
                                    }
                                } else {
                                    self.cur_val_level = self.cur_chr;
                                }
                                if ((!(tx >= self.hi_mem_min)) && (self.cur_list.mode_field != 0i32)) {
                                    match self.cur_chr {
                                        0 => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == 12i32) {
                                                self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        1 => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == 11i32) {
                                                self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        2 => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == 10i32) {
                                                {
                                                    self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].hh().lh();
                                                    if (self.mem[crate::ix::U((tx) as usize)].hh().b1() == 99i32) {
                                                        self.cur_val_level = 3i32;
                                                    }
                                                }
                                            }
                                        }
                                        3 => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() <= 13i32) {
                                                self.cur_val = (self.mem[crate::ix::U((tx) as usize)].hh().b0()).wrapping_add(1i32);
                                            } else {
                                                self.cur_val = 15i32;
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    if ((self.cur_list.mode_field == 1i32) && (tx == self.cur_list.head_field)) {
                                        match self.cur_chr {
                                            0 => {
                                                self.cur_val = self.last_penalty;
                                            }
                                            1 => {
                                                self.cur_val = self.last_kern;
                                            }
                                            2 => {
                                                if (self.last_glue != 268435455i32) {
                                                    self.cur_val = self.last_glue;
                                                }
                                            }
                                            3 => {
                                                self.cur_val = self.last_node_type;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    39 => {
                        // §439
                        if (self.cur_chr == 1i32) {
                            // §395
                            {
                                self.get_token();
                                if (self.cur_cs < 514i32) {
                                    self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                } else {
                                    self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                                }
                                if (self.cur_cs != 0i32) {
                                    {
                                        self.cur_cmd = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                        self.cur_chr = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                        self.cur_cs = (15526i32).wrapping_add(self.cur_cs);
                                        self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
                                    }
                                } else {
                                    {
                                        self.cur_cmd = 0i32;
                                        self.cur_chr = 0i32;
                                        self.cur_tok = 19616i32;
                                        self.cur_cs = 15521i32;
                                    }
                                }
                                { __goto_1 = 0; continue 'l_dispatch_1; }
                            }
                        }
                    }
                    _ => {
                        // §454
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(786i32);
                            }
                            self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                            self.print(787i32);
                            self.print_esc(616i32);
                            {
                                self.help_ptr = 1i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 785i32;
                            }
                            self.error();
                            if (level != 5i32) {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = 1i32;
                                }
                            } else {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = 0i32;
                                }
                            }
                        }
                    }
                }
                // §439
                while (self.cur_val_level > level) {
                    // §455
                    {
                        if (self.cur_val_level == 2i32) {
                            self.cur_val = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                        } else {
                            if (self.cur_val_level == 3i32) {
                                self.mu_error();
                            }
                        }
                        self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                    }
                }
                // §456
                if negative {
                    if (self.cur_val_level >= 2i32) {
                        {
                            self.cur_val = self.new_spec(self.cur_val);
                            // §457
                            {
                                { let __ix310 = (self.cur_val).wrapping_add(1i32); let __v311 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix310) as usize)].set_int(__v311); }
                                { let __ix312 = (self.cur_val).wrapping_add(2i32); let __v313 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix312) as usize)].set_int(__v313); }
                                { let __ix314 = (self.cur_val).wrapping_add(3i32); let __v315 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix314) as usize)].set_int(__v315); }
                            }
                        }
                    } else {
                        // §456
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                } else {
                    if ((self.cur_val_level >= 2i32) && (self.cur_val_level <= 3i32)) {
                        { let __ix316 = self.cur_val; let __v317 = (self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix316) as usize)].set_hh_rh(__v317); }
                    }
                }
            }
            if __goto_1 <= 1 { // exit
                // §439
            }
            break 'l_dispatch_1;
        }
    }

    /// The `scan_int` routine is used also to scan the integer part of a
    /// fraction; for example, the `\.3' in `\.{3.14159}' will be found by
    /// `scan_int`. The `scan_dimen` routine assumes that `cur_tok=point_token`
    /// after the integer part of such a fraction has been scanned by `scan_int`,
    /// and that the decimal point has been backed up to be scanned again.
    // §466
    pub fn scan_int(&mut self) {
        let mut negative: bool = false; // §466
        let mut m: i32 = 0; // §466
        let mut d: small_number = 0; // §466
        let mut vacuous: bool = false; // §466
        let mut OK_so_far: bool = false; // §466
        self.radix = 0i32;
        OK_so_far = true;
        // §467
        negative = false;
        loop {
            // §432
            loop {
                self.get_x_token();
                if (self.cur_cmd != 10i32) { break; }
            }
            // §467
            if (self.cur_tok == 3117i32) {
                {
                    negative = (!negative);
                    self.cur_tok = 3115i32;
                }
            }
            if (self.cur_tok != 3115i32) { break; }
        }
        'l_restart_b: loop {
            // §466
            if (self.cur_tok == 3168i32) {
                // §468
                {
                    self.get_token();
                    if (self.cur_tok < 4095i32) {
                        {
                            self.cur_val = self.cur_chr;
                            if (self.cur_cmd <= 2i32) {
                                if (self.cur_cmd == 2i32) {
                                    self.align_state = (self.align_state).wrapping_add(1i32);
                                } else {
                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                }
                            }
                        }
                    } else {
                        if (self.cur_tok < 4352i32) {
                            self.cur_val = (self.cur_tok).wrapping_sub(4096i32);
                        } else {
                            self.cur_val = (self.cur_tok).wrapping_sub(4352i32);
                        }
                    }
                    if (self.cur_val > 255i32) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(799i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 800i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 801i32;
                            }
                            self.cur_val = 48i32;
                            self.back_error();
                        }
                    } else {
                        // §469
                        {
                            self.get_x_token();
                            if (self.cur_cmd != 10i32) {
                                self.back_input();
                            }
                        }
                    }
                }
            } else {
                // §466
                if (self.cur_tok == 19620i32) {
                    // §395
                    {
                        self.get_token();
                        if (self.cur_cs < 514i32) {
                            self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                        } else {
                            self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                        }
                        if (self.cur_cs != 0i32) {
                            {
                                self.cur_cmd = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                self.cur_chr = self.eqtb[crate::ix::U((((15526i32).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                self.cur_cs = (15526i32).wrapping_add(self.cur_cs);
                                self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
                            }
                        } else {
                            {
                                self.cur_cmd = 0i32;
                                self.cur_chr = 0i32;
                                self.cur_tok = 19616i32;
                                self.cur_cs = 15521i32;
                            }
                        }
                        continue 'l_restart_b;
                    }
                } else {
                    // §466
                    if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                        self.scan_something_internal(0i32, false);
                    } else {
                        // §470
                        {
                            'l_done_f: {
                                self.radix = 10i32;
                                m = 214748364i32;
                                if (self.cur_tok == 3111i32) {
                                    {
                                        self.radix = 8i32;
                                        m = 268435456i32;
                                        self.get_x_token();
                                    }
                                } else {
                                    if (self.cur_tok == 3106i32) {
                                        {
                                            self.radix = 16i32;
                                            m = 134217728i32;
                                            self.get_x_token();
                                        }
                                    }
                                }
                                vacuous = true;
                                self.cur_val = 0i32;
                                // §471
                                while true {
                                    {
                                        if (((self.cur_tok < (3120i32).wrapping_add(self.radix)) && (self.cur_tok >= 3120i32)) && (self.cur_tok <= 3129i32)) {
                                            d = (self.cur_tok).wrapping_sub(3120i32);
                                        } else {
                                            if (self.radix == 16i32) {
                                                if ((self.cur_tok <= 2886i32) && (self.cur_tok >= 2881i32)) {
                                                    d = (self.cur_tok).wrapping_sub(2871i32);
                                                } else {
                                                    if ((self.cur_tok <= 3142i32) && (self.cur_tok >= 3137i32)) {
                                                        d = (self.cur_tok).wrapping_sub(3127i32);
                                                    } else {
                                                        break 'l_done_f;
                                                    }
                                                }
                                            } else {
                                                break 'l_done_f;
                                            }
                                        }
                                        vacuous = false;
                                        if ((self.cur_val >= m) && (((self.cur_val > m) || (d > 7i32)) || (self.radix != 10i32))) {
                                            {
                                                if OK_so_far {
                                                    {
                                                        {
                                                            if (self.interaction == 3i32) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(264i32);
                                                            }
                                                            self.print(802i32);
                                                        }
                                                        {
                                                            self.help_ptr = 2i32;
                                                            self.help_line[crate::ix::U((1i32) as usize)] = 803i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 804i32;
                                                        }
                                                        self.error();
                                                        self.cur_val = 2147483647i32;
                                                        OK_so_far = false;
                                                    }
                                                }
                                            }
                                        } else {
                                            self.cur_val = ((self.cur_val).wrapping_mul(self.radix)).wrapping_add(d);
                                        }
                                        self.get_x_token();
                                    }
                                }
                            }
                            // §470
                            if vacuous {
                                // §472
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(751i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 752i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 753i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 754i32;
                                    }
                                    self.back_error();
                                }
                            } else {
                                // §470
                                if (self.cur_cmd != 10i32) {
                                    self.back_input();
                                }
                            }
                        }
                    }
                }
            }
            // §466
            if negative {
                self.cur_val = (self.cur_val).wrapping_neg();
            }
            break 'l_restart_b;
        }
    }

    /// Constructions like `\.{-\'77 pt}' are legal dimensions, so `scan_dimen`
    /// may begin with `scan_int`. This explains why it is convenient to use
    /// `scan_int` also for the integer part of a decimal fraction.
    /// Several branches of `scan_dimen` work with `cur_val` as an integer and
    /// with an auxiliary fraction `f`, so that the actual quantity of interest is
    /// $`cur_val`+`f`/2^{16}$. At the end of the routine, this ``unpacked''
    /// representation is put into the single word `cur_val`, which suddenly
    /// switches significance from `integer` to `scaled`.
    // §474
    pub fn scan_dimen(&mut self, mut mu: bool, mut inf: bool, mut shortcut: bool) {
        let mut negative: bool = false; // §474
        let mut f: i32 = 0; // §474
        let mut num: i32 = 0; // §476
        let mut denom: i32 = 0; // §476
        let mut k: small_number = 0; // §476
        let mut kk: small_number = 0; // §476
        let mut p: halfword = 0; // §476
        let mut q: halfword = 0; // §476
        let mut v: scaled = 0; // §476
        let mut save_cur_val: i32 = 0; // §476
        'l_L89_f: {
            'l_done_f: {
                'l_L88_f: {
                    'l_done2_f: {
                        'l_not_found_f: {
                            'l_found_f: {
                                if self.intr_rec_on {
                                    self.flashtex_intr_abort(1i32);
                                }
                                f = 0i32;
                                self.arith_error = false;
                                self.cur_order = 0i32;
                                negative = false;
                                if (!shortcut) {
                                    {
                                        // §467
                                        negative = false;
                                        loop {
                                            // §432
                                            loop {
                                                self.get_x_token();
                                                if (self.cur_cmd != 10i32) { break; }
                                            }
                                            // §467
                                            if (self.cur_tok == 3117i32) {
                                                {
                                                    negative = (!negative);
                                                    self.cur_tok = 3115i32;
                                                }
                                            }
                                            if (self.cur_tok != 3115i32) { break; }
                                        }
                                        // §474
                                        if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                                            // §475
                                            if mu {
                                                {
                                                    self.scan_something_internal(3i32, false);
                                                    // §477
                                                    if (self.cur_val_level >= 2i32) {
                                                        {
                                                            v = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                            self.delete_glue_ref(self.cur_val);
                                                            self.cur_val = v;
                                                        }
                                                    }
                                                    // §475
                                                    if (self.cur_val_level == 3i32) {
                                                        break 'l_L89_f;
                                                    }
                                                    if (self.cur_val_level != 0i32) {
                                                        self.mu_error();
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.scan_something_internal(1i32, false);
                                                    if (self.cur_val_level == 1i32) {
                                                        break 'l_L89_f;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §474
                                            {
                                                self.back_input();
                                                if (self.cur_tok == 3116i32) {
                                                    self.cur_tok = 3118i32;
                                                }
                                                if (self.cur_tok != 3118i32) {
                                                    self.scan_int();
                                                } else {
                                                    {
                                                        self.radix = 10i32;
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                                if (self.cur_tok == 3116i32) {
                                                    self.cur_tok = 3118i32;
                                                }
                                                if ((self.radix == 10i32) && (self.cur_tok == 3118i32)) {
                                                    // §478
                                                    {
                                                        'l_done1_f: {
                                                            k = 0i32;
                                                            p = 0i32;
                                                            self.get_token();
                                                            while true {
                                                                {
                                                                    self.get_x_token();
                                                                    if ((self.cur_tok > 3129i32) || (self.cur_tok < 3120i32)) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    if (k < 17i32) {
                                                                        {
                                                                            q = self.get_avail();
                                                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                                            { let __v318 = (self.cur_tok).wrapping_sub(3120i32); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v318); }
                                                                            p = q;
                                                                            k = (k).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            let __for_end_14 = 1i32;
                                                            kk = k;
                                                            while kk >= __for_end_14 {
                                                                {
                                                                    { let __v319 = self.mem[crate::ix::U((p) as usize)].hh().lh(); self.dig[crate::ix::U(((kk).wrapping_sub(1i32)) as usize)] = __v319; }
                                                                    q = p;
                                                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                    {
                                                                        { let __v320 = self.avail; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v320); }
                                                                        self.avail = q;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                                kk = kk.wrapping_sub(1);
                                                            }
                                                        }
                                                        f = self.round_decimals(k);
                                                        if (self.cur_cmd != 10i32) {
                                                            self.back_input();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // §474
                                if (self.cur_val < 0i32) {
                                    {
                                        negative = (!negative);
                                        self.cur_val = (self.cur_val).wrapping_neg();
                                    }
                                }
                                // §479
                                if inf {
                                    // §480
                                    if self.scan_keyword(318i32) {
                                        {
                                            self.cur_order = 1i32;
                                            while self.scan_keyword(108i32) {
                                                {
                                                    if (self.cur_order == 3i32) {
                                                        {
                                                            {
                                                                if (self.interaction == 3i32) {
                                                                }
                                                                if self.file_line_error_style_p {
                                                                    self.print_file_line();
                                                                } else {
                                                                    self.print_nl(264i32);
                                                                }
                                                                self.print(806i32);
                                                            }
                                                            self.print(807i32);
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[crate::ix::U((0i32) as usize)] = 808i32;
                                                            }
                                                            self.error();
                                                        }
                                                    } else {
                                                        self.cur_order = (self.cur_order).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            break 'l_L88_f;
                                        }
                                    }
                                }
                                // §481
                                save_cur_val = self.cur_val;
                                // §432
                                loop {
                                    self.get_x_token();
                                    if (self.cur_cmd != 10i32) { break; }
                                }
                                // §481
                                if ((self.cur_cmd < 68i32) || (self.cur_cmd > 89i32)) {
                                    self.back_input();
                                } else {
                                    {
                                        if mu {
                                            {
                                                self.scan_something_internal(3i32, false);
                                                // §477
                                                if (self.cur_val_level >= 2i32) {
                                                    {
                                                        v = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                        self.delete_glue_ref(self.cur_val);
                                                        self.cur_val = v;
                                                    }
                                                }
                                                // §481
                                                if (self.cur_val_level != 3i32) {
                                                    self.mu_error();
                                                }
                                            }
                                        } else {
                                            self.scan_something_internal(1i32, false);
                                        }
                                        v = self.cur_val;
                                        break 'l_found_f;
                                    }
                                }
                                if mu {
                                    break 'l_not_found_f;
                                }
                                if self.scan_keyword(809i32) {
                                    v = self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((27689i32) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                } else {
                                    if self.scan_keyword(810i32) {
                                        v = self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((27689i32) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                    } else {
                                        if self.scan_keyword(811i32) {
                                            v = self.eqtb[crate::ix::U(((29936i32) - 1) as usize)].int();
                                        } else {
                                            break 'l_not_found_f;
                                        }
                                    }
                                }
                                // §469
                                {
                                    self.get_x_token();
                                    if (self.cur_cmd != 10i32) {
                                        self.back_input();
                                    }
                                }
                            }
                            // §481
                            self.cur_val = { let __a321_0 = save_cur_val; let __a321_1 = v; let __a321_2 = self.xn_over_d(v, f, 65536i32); let __a321_3 = 1073741823i32; self.mult_and_add(__a321_0, __a321_1, __a321_2, __a321_3) };
                            break 'l_L89_f;
                        }
                        // §479
                        if mu {
                            // §482
                            if self.scan_keyword(347i32) {
                                break 'l_L88_f;
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
                                        self.print(806i32);
                                    }
                                    self.print(812i32);
                                    {
                                        self.help_ptr = 4i32;
                                        self.help_line[crate::ix::U((3i32) as usize)] = 813i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 814i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 815i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 816i32;
                                    }
                                    self.error();
                                    break 'l_L88_f;
                                }
                            }
                        }
                        // §479
                        if self.scan_keyword(805i32) {
                            // §483
                            {
                                self.prepare_mag();
                                if (self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() != 1000i32) {
                                    {
                                        self.cur_val = self.xn_over_d(self.cur_val, 1000i32, self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                                        f = (((1000i32).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / self.eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                                        self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                                        f = (f % 65536i32);
                                    }
                                }
                            }
                        }
                        // §479
                        if self.scan_keyword(314i32) {
                            break 'l_L88_f;
                        }
                        // §484
                        if self.scan_keyword(817i32) {
                            {
                                num = 7227i32;
                                denom = 100i32;
                            }
                        } else {
                            if self.scan_keyword(818i32) {
                                {
                                    num = 12i32;
                                    denom = 1i32;
                                }
                            } else {
                                if self.scan_keyword(819i32) {
                                    {
                                        num = 7227i32;
                                        denom = 254i32;
                                    }
                                } else {
                                    if self.scan_keyword(820i32) {
                                        {
                                            num = 7227i32;
                                            denom = 2540i32;
                                        }
                                    } else {
                                        if self.scan_keyword(821i32) {
                                            {
                                                num = 7227i32;
                                                denom = 7200i32;
                                            }
                                        } else {
                                            if self.scan_keyword(822i32) {
                                                {
                                                    num = 1238i32;
                                                    denom = 1157i32;
                                                }
                                            } else {
                                                if self.scan_keyword(823i32) {
                                                    {
                                                        num = 14856i32;
                                                        denom = 1157i32;
                                                    }
                                                } else {
                                                    if self.scan_keyword(824i32) {
                                                        {
                                                            num = 685i32;
                                                            denom = 642i32;
                                                        }
                                                    } else {
                                                        if self.scan_keyword(825i32) {
                                                            {
                                                                num = 1370i32;
                                                                denom = 107i32;
                                                            }
                                                        } else {
                                                            if self.scan_keyword(826i32) {
                                                                break 'l_done_f;
                                                            } else {
                                                                // §485
                                                                {
                                                                    {
                                                                        if (self.interaction == 3i32) {
                                                                        }
                                                                        if self.file_line_error_style_p {
                                                                            self.print_file_line();
                                                                        } else {
                                                                            self.print_nl(264i32);
                                                                        }
                                                                        self.print(806i32);
                                                                    }
                                                                    self.print(827i32);
                                                                    {
                                                                        self.help_ptr = 6i32;
                                                                        self.help_line[crate::ix::U((5i32) as usize)] = 828i32;
                                                                        self.help_line[crate::ix::U((4i32) as usize)] = 829i32;
                                                                        self.help_line[crate::ix::U((3i32) as usize)] = 830i32;
                                                                        self.help_line[crate::ix::U((2i32) as usize)] = 814i32;
                                                                        self.help_line[crate::ix::U((1i32) as usize)] = 815i32;
                                                                        self.help_line[crate::ix::U((0i32) as usize)] = 816i32;
                                                                    }
                                                                    self.error();
                                                                    break 'l_done2_f;
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
                        // §484
                        self.cur_val = self.xn_over_d(self.cur_val, num, denom);
                        f = (((num).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / denom);
                        self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                        f = (f % 65536i32);
                    }
                }
                // §479
                if (self.cur_val >= 16384i32) {
                    self.arith_error = true;
                } else {
                    self.cur_val = ((self.cur_val).wrapping_mul(65536i32)).wrapping_add(f);
                }
            }
            // §469
            {
                self.get_x_token();
                if (self.cur_cmd != 10i32) {
                    self.back_input();
                }
            }
        }
        // §474
        if (self.arith_error || ((self.cur_val).wrapping_abs() >= 1073741824i32)) {
            // §486
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(831i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 832i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 833i32;
                }
                self.error();
                self.cur_val = 1073741823i32;
                self.arith_error = false;
            }
        }
        // §474
        if negative {
            self.cur_val = (self.cur_val).wrapping_neg();
        }
    }

    /// The final member of \TeX's value-scanning trio is `scan_glue`, which
    /// makes `cur_val` point to a glue specification. The reference count of that
    /// glue spec will take account of the fact that `cur_val` is pointing to~it.
    /// The `level` parameter should be either `glue_val` or `mu_val`.
    /// Since `scan_dimen` was so much more complex than `scan_int`, we might expect
    /// `scan_glue` to be even worse. But fortunately, it is very simple, since
    /// most of the work has already been done.
    // §487
    pub fn scan_glue(&mut self, mut level: small_number) {
        let mut negative: bool = false; // §487
        let mut q: halfword = 0; // §487
        let mut mu: bool = false; // §487
        'l_exit_f: {
            mu = (level == 3i32);
            // §467
            negative = false;
            loop {
                // §432
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != 10i32) { break; }
                }
                // §467
                if (self.cur_tok == 3117i32) {
                    {
                        negative = (!negative);
                        self.cur_tok = 3115i32;
                    }
                }
                if (self.cur_tok != 3115i32) { break; }
            }
            // §487
            if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                {
                    self.scan_something_internal(level, negative);
                    if (self.cur_val_level >= 2i32) {
                        {
                            if (self.cur_val_level != level) {
                                self.mu_error();
                            }
                            break 'l_exit_f;
                        }
                    }
                    if (self.cur_val_level == 0i32) {
                        self.scan_dimen(mu, false, true);
                    } else {
                        if (level == 3i32) {
                            self.mu_error();
                        }
                    }
                }
            } else {
                {
                    self.back_input();
                    self.scan_dimen(mu, false, false);
                    if negative {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                }
            }
            // §488
            q = self.new_spec(0i32);
            { let __v322 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v322); }
            if self.scan_keyword(834i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v323 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v323); }
                    { let __v324 = self.cur_order; self.mem[crate::ix::U((q) as usize)].set_hh_b0(__v324); }
                }
            }
            if self.scan_keyword(835i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v325 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v325); }
                    { let __v326 = self.cur_order; self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v326); }
                }
            }
            self.cur_val = q;
        }
        // §487
    }

    /// The function `add_or_sub(x,y,max_answer,negative)` computes the sum
    /// (for `negative=false`) or difference (for `negative=true`) of `x` and
    /// `y`, provided the absolute value of the result does not exceed
    /// `max_answer`.
    /// @<Declare subprocedures for `scan_expr`
    // §1793
    pub fn add_or_sub(&mut self, mut x: i32, mut y: i32, mut max_answer: i32, mut negative: bool) -> i32 {
        let mut add_or_sub: i32 = 0;
        let mut a: i32 = 0; // §1793
        if negative {
            y = (y).wrapping_neg();
        }
        if (x >= 0i32) {
            if (y <= (max_answer).wrapping_sub(x)) {
                a = (x).wrapping_add(y);
            } else {
                {
                    self.arith_error = true;
                    a = 0i32;
                }
            }
        } else {
            if (y >= ((max_answer).wrapping_neg()).wrapping_sub(x)) {
                a = (x).wrapping_add(y);
            } else {
                {
                    self.arith_error = true;
                    a = 0i32;
                }
            }
        }
        add_or_sub = a;
        add_or_sub
    }

    /// The function `quotient(n,d)` computes the rounded quotient
    /// $q=\lfloor n/d+{1\over2}\rfloor$, when $n$ and $d$ are positive.
    /// @<Declare subprocedures for `scan_expr`
    // §1797
    pub fn quotient(&mut self, mut n: i32, mut d: i32) -> i32 {
        let mut quotient: i32 = 0;
        let mut negative: bool = false; // §1797
        let mut a: i32 = 0; // §1797
        if (d == 0i32) {
            {
                self.arith_error = true;
                a = 0i32;
            }
        } else {
            {
                if (d > 0i32) {
                    negative = false;
                } else {
                    {
                        d = (d).wrapping_neg();
                        negative = true;
                    }
                }
                if (n < 0i32) {
                    {
                        n = (n).wrapping_neg();
                        negative = (!negative);
                    }
                }
                a = (n / d);
                n = (n).wrapping_sub((a).wrapping_mul(d));
                d = (n).wrapping_sub(d);
                if ((d).wrapping_add(n) >= 0i32) {
                    a = (a).wrapping_add(1i32);
                }
                if negative {
                    a = (a).wrapping_neg();
                }
            }
        }
        quotient = a;
        quotient
    }

    /// Finally, the function `fract(x,n,d,max_answer)` computes the integer
    /// $q=\lfloor xn/d+{1\over2}\rfloor$, when $x$, $n$, and $d$ are positive
    /// and the result does not exceed `max_answer`.  We can't use floating
    /// point arithmetic since the routine must produce identical results in all
    /// cases; and it would be too dangerous to multiply by~`n` and then divide
    /// by~`d`, in separate operations, since overflow might well occur.  Hence
    /// this subroutine simulates double precision arithmetic, somewhat
    /// analogous to \MF's `make_fraction` and `take_fraction` routines.
    // §1799
    pub fn fract(&mut self, mut x: i32, mut n: i32, mut d: i32, mut max_answer: i32) -> i32 {
        let mut fract: i32 = 0;
        let mut negative: bool = false; // §1799
        let mut a: i32 = 0; // §1799
        let mut f: i32 = 0; // §1799
        let mut h: i32 = 0; // §1799
        let mut r: i32 = 0; // §1799
        let mut t: i32 = 0; // §1799
        'l_done_f: {
            'l_L88_f: {
                'l_found_f: {
                    'l_found1_f: {
                        if (d == 0i32) {
                            break 'l_L88_f;
                        }
                        a = 0i32;
                        if (d > 0i32) {
                            negative = false;
                        } else {
                            {
                                d = (d).wrapping_neg();
                                negative = true;
                            }
                        }
                        if (x < 0i32) {
                            {
                                x = (x).wrapping_neg();
                                negative = (!negative);
                            }
                        } else {
                            if (x == 0i32) {
                                break 'l_done_f;
                            }
                        }
                        if (n < 0i32) {
                            {
                                n = (n).wrapping_neg();
                                negative = (!negative);
                            }
                        }
                        t = (n / d);
                        if (t > (max_answer / x)) {
                            break 'l_L88_f;
                        }
                        a = (t).wrapping_mul(x);
                        n = (n).wrapping_sub((t).wrapping_mul(d));
                        if (n == 0i32) {
                            break 'l_found_f;
                        }
                        t = (x / d);
                        if (t > ((max_answer).wrapping_sub(a) / n)) {
                            break 'l_L88_f;
                        }
                        a = (a).wrapping_add((t).wrapping_mul(n));
                        x = (x).wrapping_sub((t).wrapping_mul(d));
                        if (x == 0i32) {
                            break 'l_found_f;
                        }
                        if (x < n) {
                            {
                                t = x;
                                x = n;
                                n = t;
                            }
                        }
                        // §1800
                        f = 0i32;
                        r = ((d / 2i32)).wrapping_sub(d);
                        h = (r).wrapping_neg();
                        while true {
                            {
                                if (((n) % 2) != 0) {
                                    {
                                        r = (r).wrapping_add(x);
                                        if (r >= 0i32) {
                                            {
                                                r = (r).wrapping_sub(d);
                                                f = (f).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                                n = (n / 2i32);
                                if (n == 0i32) {
                                    break 'l_found1_f;
                                }
                                if (x < h) {
                                    x = (x).wrapping_add(x);
                                } else {
                                    {
                                        t = (x).wrapping_sub(d);
                                        x = (t).wrapping_add(x);
                                        f = (f).wrapping_add(n);
                                        if (x < n) {
                                            {
                                                if (x == 0i32) {
                                                    break 'l_found1_f;
                                                }
                                                t = x;
                                                x = n;
                                                n = t;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if (f > (max_answer).wrapping_sub(a)) {
                        // §1799
                        break 'l_L88_f;
                    }
                    a = (a).wrapping_add(f);
                }
                if negative {
                    a = (a).wrapping_neg();
                }
                break 'l_done_f;
            }
            {
                self.arith_error = true;
                a = 0i32;
            }
        }
        fract = a;
        fract
    }

    /// The `scan_expr` procedure scans and evaluates an expression.
    /// @<Declare procedures needed for expressions
    // §1782
    pub fn scan_expr(&mut self) {
        let mut a: bool = false; // §1782
        let mut b: bool = false; // §1782
        let mut l: small_number = 0; // §1782
        let mut r: small_number = 0; // §1782
        let mut s: small_number = 0; // §1782
        let mut o: small_number = 0; // §1782
        let mut e: i32 = 0; // §1782
        let mut t: i32 = 0; // §1782
        let mut f: i32 = 0; // §1782
        let mut n: i32 = 0; // §1782
        let mut p: halfword = 0; // §1782
        let mut q: halfword = 0; // §1782
        l = self.cur_val_level;
        a = self.arith_error;
        b = false;
        p = 0i32;
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(705i32, self.expand_depth);
        }
        'l_restart_b: loop {
            // §1783
            r = 0i32;
            e = 0i32;
            s = 0i32;
            t = 0i32;
            n = 0i32;
            'l_continue_b: loop {
                if (s == 0i32) {
                    o = l;
                } else {
                    o = 0i32;
                }
                // §432
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != 10i32) { break; }
                }
                // §1785
                if (self.cur_tok == 3112i32) {
                    // §1788
                    {
                        q = self.get_node(4i32);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b0(l);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(((4i32).wrapping_mul(s)).wrapping_add(r));
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(e);
                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(t);
                        self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(n);
                        p = q;
                        l = o;
                        continue 'l_restart_b;
                    }
                }
                // §1785
                self.back_input();
                if (o == 0i32) {
                    self.scan_int();
                } else {
                    if (o == 1i32) {
                        self.scan_dimen(false, false, false);
                    } else {
                        if (o == 2i32) {
                            self.scan_normal_glue();
                        } else {
                            self.scan_mu_glue();
                        }
                    }
                }
                f = self.cur_val;
                'l_found_b: loop {
                    // §1783
                    loop {
                        // §432
                        self.get_x_token();
                        if (self.cur_cmd != 10i32) { break; }
                    }
                    // §1784
                    if (self.cur_tok == 3115i32) {
                        o = 1i32;
                    } else {
                        if (self.cur_tok == 3117i32) {
                            o = 2i32;
                        } else {
                            if (self.cur_tok == 3114i32) {
                                o = 3i32;
                            } else {
                                if (self.cur_tok == 3119i32) {
                                    o = 4i32;
                                } else {
                                    {
                                        o = 0i32;
                                        if (p == 0i32) {
                                            {
                                                if (self.cur_cmd != 0i32) {
                                                    self.back_input();
                                                }
                                            }
                                        } else {
                                            if (self.cur_tok != 3113i32) {
                                                {
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(264i32);
                                                        }
                                                        self.print(2047i32);
                                                    }
                                                    {
                                                        self.help_ptr = 1i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 2048i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §1783
                    self.arith_error = b;
                    // §1790
                    if ((l == 0i32) || (s > 2i32)) {
                        {
                            if ((f > 2147483647i32) || (f < (2147483647i32).wrapping_neg())) {
                                {
                                    self.arith_error = true;
                                    f = 0i32;
                                }
                            }
                        }
                    } else {
                        if (l == 1i32) {
                            {
                                if ((f).wrapping_abs() > 1073741823i32) {
                                    {
                                        self.arith_error = true;
                                        f = 0i32;
                                    }
                                }
                            }
                        } else {
                            {
                                if ((((self.mem[crate::ix::U(((f).wrapping_add(1i32)) as usize)].int()).wrapping_abs() > 1073741823i32) || ((self.mem[crate::ix::U(((f).wrapping_add(2i32)) as usize)].int()).wrapping_abs() > 1073741823i32)) || ((self.mem[crate::ix::U(((f).wrapping_add(3i32)) as usize)].int()).wrapping_abs() > 1073741823i32)) {
                                    {
                                        self.arith_error = true;
                                        self.delete_glue_ref(f);
                                        f = self.new_spec(0i32);
                                    }
                                }
                            }
                        }
                    }
                    // §1783
                    match s {
                        0 => {
                            // §1791
                            if ((l >= 2i32) && (o != 0i32)) {
                                {
                                    t = self.new_spec(f);
                                    self.delete_glue_ref(f);
                                    if (self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                        self.mem[crate::ix::U((t) as usize)].set_hh_b0(0i32);
                                    }
                                    if (self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                        self.mem[crate::ix::U((t) as usize)].set_hh_b1(0i32);
                                    }
                                }
                            } else {
                                t = f;
                            }
                        }
                        3 => {
                            // §1795
                            if (o == 4i32) {
                                {
                                    n = f;
                                    o = 5i32;
                                }
                            } else {
                                if (l == 0i32) {
                                    t = self.mult_and_add(t, f, 0i32, 2147483647i32);
                                } else {
                                    if (l == 1i32) {
                                        t = self.mult_and_add(t, f, 0i32, 1073741823i32);
                                    } else {
                                        {
                                            { let __v327 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v327); }
                                            { let __v328 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v328); }
                                            { let __v329 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v329); }
                                        }
                                    }
                                }
                            }
                        }
                        4 => {
                            // §1796
                            if (l < 2i32) {
                                t = self.quotient(t, f);
                            } else {
                                {
                                    { let __v330 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v330); }
                                    { let __v331 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v331); }
                                    { let __v332 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v332); }
                                }
                            }
                        }
                        5 => {
                            // §1798
                            if (l == 0i32) {
                                t = self.fract(t, n, f, 2147483647i32);
                            } else {
                                if (l == 1i32) {
                                    t = self.fract(t, n, f, 1073741823i32);
                                } else {
                                    {
                                        { let __v333 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), n, f, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v333); }
                                        { let __v334 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), n, f, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v334); }
                                        { let __v335 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), n, f, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v335); }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    // §1783
                    if (o > 2i32) {
                        s = o;
                    } else {
                        // §1792
                        {
                            s = 0i32;
                            if (r == 0i32) {
                                e = t;
                            } else {
                                if (l == 0i32) {
                                    e = self.add_or_sub(e, t, 2147483647i32, (r == 2i32));
                                } else {
                                    if (l == 1i32) {
                                        e = self.add_or_sub(e, t, 1073741823i32, (r == 2i32));
                                    } else {
                                        // §1794
                                        {
                                            { let __v336 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), 1073741823i32, (r == 2i32)); self.mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].set_int(__v336); }
                                            if (self.mem[crate::ix::U((e) as usize)].hh().b0() == self.mem[crate::ix::U((t) as usize)].hh().b0()) {
                                                { let __v337 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), 1073741823i32, (r == 2i32)); self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v337); }
                                            } else {
                                                if ((self.mem[crate::ix::U((e) as usize)].hh().b0() < self.mem[crate::ix::U((t) as usize)].hh().b0()) && (self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v338 = self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v338); }
                                                        { let __v339 = self.mem[crate::ix::U((t) as usize)].hh().b0(); self.mem[crate::ix::U((e) as usize)].set_hh_b0(__v339); }
                                                    }
                                                }
                                            }
                                            if (self.mem[crate::ix::U((e) as usize)].hh().b1() == self.mem[crate::ix::U((t) as usize)].hh().b1()) {
                                                { let __v340 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), 1073741823i32, (r == 2i32)); self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v340); }
                                            } else {
                                                if ((self.mem[crate::ix::U((e) as usize)].hh().b1() < self.mem[crate::ix::U((t) as usize)].hh().b1()) && (self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v341 = self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v341); }
                                                        { let __v342 = self.mem[crate::ix::U((t) as usize)].hh().b1(); self.mem[crate::ix::U((e) as usize)].set_hh_b1(__v342); }
                                                    }
                                                }
                                            }
                                            self.delete_glue_ref(t);
                                            if (self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                                self.mem[crate::ix::U((e) as usize)].set_hh_b0(0i32);
                                            }
                                            if (self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                                self.mem[crate::ix::U((e) as usize)].set_hh_b1(0i32);
                                            }
                                        }
                                    }
                                }
                            }
                            // §1792
                            r = o;
                        }
                    }
                    // §1783
                    b = self.arith_error;
                    if (o != 0i32) {
                        continue 'l_continue_b;
                    }
                    if (p != 0i32) {
                        // §1789
                        {
                            f = e;
                            q = p;
                            e = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            t = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                            n = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                            s = (self.mem[crate::ix::U((q) as usize)].hh().b1() / 4i32);
                            r = (self.mem[crate::ix::U((q) as usize)].hh().b1() % 4i32);
                            l = self.mem[crate::ix::U((q) as usize)].hh().b0();
                            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            self.free_node(q, 4i32);
                            continue 'l_found_b;
                        }
                    }
                    // §1782
                    self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
                    if b {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1626i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 2046i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1628i32;
                            }
                            self.error();
                            if (l >= 2i32) {
                                {
                                    self.delete_glue_ref(e);
                                    e = 0i32;
                                    { let __v343 = (self.mem[crate::ix::U((e) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((e) as usize)].set_hh_rh(__v343); }
                                }
                            } else {
                                e = 0i32;
                            }
                        }
                    }
                    self.arith_error = a;
                    self.cur_val = e;
                    self.cur_val_level = l;
                    break 'l_found_b;
                }
                break 'l_continue_b;
            }
            break 'l_restart_b;
        }
    }

    /// Here we declare two trivial procedures in order to avoid mutually
    /// recursive procedures with parameters.
    /// @<Declare procedures needed for expressions
    // §1787
    pub fn scan_normal_glue(&mut self) {
        self.scan_glue(2i32);
    }

    /// Here we declare two trivial procedures in order to avoid mutually
    /// recursive procedures with parameters.
    /// @<Declare procedures needed for expressions
    // §1787
    pub fn scan_mu_glue(&mut self) {
        self.scan_glue(3i32);
    }

    /// Here's a similar procedure that returns a pointer to a rule node. This
    /// routine is called just after \TeX\ has seen \.{\\hrule} or \.{\\vrule};
    /// therefore `cur_cmd` will be either `hrule` or `vrule`. The idea is to store
    /// the default rule dimensions in the node, then to override them if
    /// `\.{height}' or `\.{width}' or `\.{depth}' specifications are
    /// found (in any order).
    // §489
    pub fn scan_rule_spec(&mut self) -> halfword {
        let mut scan_rule_spec: halfword = 0;
        let mut q: halfword = 0; // §489
        q = self.new_rule();
        if (self.cur_cmd == 35i32) {
            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(26214i32);
        } else {
            {
                self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(26214i32);
                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
            }
        }
        'l_reswitch_b: loop {
            if self.scan_keyword(836i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v344 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v344); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(837i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v345 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v345); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(838i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v346 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v346); }
                    continue 'l_reswitch_b;
                }
            }
            scan_rule_spec = q;
            break 'l_reswitch_b;
        }
        scan_rule_spec
    }

    /// The token list (balanced text) created by `scan_general_text` begins
    /// at `link(temp_head)` and ends at `cur_val`.  (If `cur_val=temp_head`,
    /// the list is empty.)
    /// @<Declare \eTeX\ procedures for tok...
    // §1683
    pub fn scan_general_text(&mut self) {
        let mut s: i32 = 0; // §1683
        let mut w: halfword = 0; // §1683
        let mut d: halfword = 0; // §1683
        let mut p: halfword = 0; // §1683
        let mut q: halfword = 0; // §1683
        let mut unbalance: halfword = 0; // §1683
        'l_found_f: {
            s = self.scanner_status;
            w = self.warning_index;
            d = self.def_ref;
            self.scanner_status = 5i32;
            self.warning_index = self.cur_cs;
            self.def_ref = self.get_avail();
            { let __ix347 = self.def_ref; self.mem[crate::ix::U((__ix347) as usize)].set_hh_lh(0i32); }
            p = self.def_ref;
            self.scan_left_brace();
            unbalance = 1i32;
            while true {
                {
                    self.get_token();
                    if (self.cur_tok < 768i32) {
                        if (self.cur_cmd < 2i32) {
                            unbalance = (unbalance).wrapping_add(1i32);
                        } else {
                            {
                                unbalance = (unbalance).wrapping_sub(1i32);
                                if (unbalance == 0i32) {
                                    break 'l_found_f;
                                }
                            }
                        }
                    }
                    {
                        q = self.get_avail();
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                        { let __v348 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v348); }
                        p = q;
                    }
                }
            }
        }
        q = self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh();
        {
            { let __ix349 = self.def_ref; let __v350 = self.avail; self.mem[crate::ix::U((__ix349) as usize)].set_hh_rh(__v350); }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        if (q == 0i32) {
            self.cur_val = 4999996i32;
        } else {
            self.cur_val = p;
        }
        self.mem[crate::ix::U((4999996i32) as usize)].set_hh_rh(q);
        self.scanner_status = s;
        self.warning_index = w;
        self.def_ref = d;
    }

}
