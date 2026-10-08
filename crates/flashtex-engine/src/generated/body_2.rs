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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.last = self.first;
        p = __av_mem[crate::ix::U((self.pseudo_files) as usize)].hh().lh();
        if (p == null) {
            pseudo_input = false;
        } else {
            {
                { let __ix165 = self.pseudo_files; let __v166 = __av_mem[crate::ix::U((p) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix165) as usize)].set_hh_lh(__v166); }
                sz = (__av_mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(0i32);
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
                            w = __av_mem[crate::ix::U((r) as usize)].qqqq();
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = __av_mem[crate::ix::U((self.pseudo_files) as usize)].hh().rh();
        q = __av_mem[crate::ix::U((self.pseudo_files) as usize)].hh().lh();
        {
            { let __ix167 = self.pseudo_files; let __v168 = self.avail; __av_mem[crate::ix::U((__ix167) as usize)].set_hh_rh(__v168); }
            self.avail = self.pseudo_files;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.pseudo_files = p;
        while (q != null) {
            {
                p = q;
                q = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, (__av_mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(0i32));
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.base_ptr = self.input_ptr;
        { let __ix169 = self.base_ptr; let __v170 = self.cur_input; self.input_stack[crate::ix::U((__ix169) as usize)] = __v170; }
        i = self.in_open;
        w = false;
        while ((self.grp_stack[crate::ix::U((i) as usize)] == self.cur_boundary) && (i > 0i32)) {
            {
                // §1775
                if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == token_list) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field > i)) {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field > 17i32) {
                            w = true;
                        }
                    }
                }
                // §1774
                { let __v171 = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh(); self.grp_stack[crate::ix::U((i) as usize)] = __v171; }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(2040i32);
                self.print_group(true);
                self.print(2041i32);
                self.print_ln();
                if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == spotless) {
                    self.history = warning_issued;
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.base_ptr = self.input_ptr;
        { let __ix172 = self.base_ptr; let __v173 = self.cur_input; self.input_stack[crate::ix::U((__ix172) as usize)] = __v173; }
        i = self.in_open;
        w = false;
        while (self.if_stack[crate::ix::U((i) as usize)] == self.cond_ptr) {
            {
                // §1775
                if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == token_list) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field > i)) {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field > 17i32) {
                            w = true;
                        }
                    }
                }
                // §1776
                { let __v174 = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh(); self.if_stack[crate::ix::U((i) as usize)] = __v174; }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(2040i32);
                self.print_cmd_chr(if_test, self.cur_if);
                if (self.if_line != 0i32) {
                    {
                        self.ls_print_unknown(self.if_line);
                        self.print(2010i32);
                        self.print_int(((self.if_line) as i64));
                    }
                }
                self.print(2041i32);
                self.print_ln();
                if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == spotless) {
                    self.history = warning_issued;
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.save_ptr;
        l = self.cur_level;
        c = self.cur_group;
        self.save_ptr = self.cur_boundary;
        while (self.grp_stack[crate::ix::U((self.in_open) as usize)] != self.save_ptr) {
            {
                self.cur_level = (self.cur_level).wrapping_sub(1i32);
                self.print_nl(2042i32);
                self.print_group(true);
                self.print(2043i32);
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
                self.print_nl(2042i32);
                self.print_cmd_chr(if_test, self.cur_if);
                if (self.if_limit == fi_code) {
                    self.print_esc(929i32);
                }
                if (self.if_line != 0i32) {
                    {
                        self.ls_print_unknown(self.if_line);
                        self.print(2010i32);
                        self.print_int(((self.if_line) as i64));
                    }
                }
                self.print(2043i32);
                self.if_line = __av_mem[crate::ix::U(((self.cond_ptr).wrapping_add(1i32)) as usize)].int();
                self.cur_if = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().b1();
                self.if_limit = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().b0();
                self.cond_ptr = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
            }
        }
        self.cond_ptr = p;
        self.if_limit = l;
        self.cur_if = c;
        self.if_line = i;
        self.print_ln();
        if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 1i32) {
            self.show_context();
        }
        if (self.history == spotless) {
            self.history = warning_issued;
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            { let __v175 = (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(1i32); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v175); }
            if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                break 'l_exit_f;
            }
            if (__av_mem[crate::ix::U((q) as usize)].hh().b0() < dimen_val_limit) {
                if (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    s = word_node_size;
                } else {
                    break 'l_exit_f;
                }
            } else {
                {
                    if (__av_mem[crate::ix::U((q) as usize)].hh().b0() < mu_val_limit) {
                        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == zero_glue) {
                            self.delete_glue_ref(zero_glue);
                        } else {
                            break 'l_exit_f;
                        }
                    } else {
                        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() != null) {
                            break 'l_exit_f;
                        }
                    }
                    s = pointer_node_size;
                }
            }
            loop {
                i = (__av_mem[crate::ix::U((q) as usize)].hh().b0() % 16i32);
                p = q;
                q = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, s);
                if (q == null) {
                    {
                        self.sa_root[crate::ix::U((i) as usize)] = null;
                        break 'l_exit_f;
                    }
                }
                {
                    if (((i) % 2) != 0) {
                        __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                    } else {
                        __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(null);
                    }
                    { let __v176 = (__av_mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v176); }
                }
                s = index_node_size;
                if (__av_mem[crate::ix::U((q) as usize)].hh().b1() > 0i32) { break; }
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.begin_diagnostic();
        self.print_char(123i32);
        self.print(s);
        self.print_char(32i32);
        if (p == null) {
            self.print_char(63i32);
        } else {
            {
                t = (__av_mem[crate::ix::U((p) as usize)].hh().b0() / 16i32);
                if (t < box_val) {
                    self.print_cmd_chr(register, p);
                } else {
                    if (t == box_val) {
                        {
                            self.print_esc(422i32);
                            self.print_sa_num(p);
                        }
                    } else {
                        if (t == tok_val) {
                            self.print_cmd_chr(toks_register, p);
                        } else {
                            self.print_char(63i32);
                        }
                    }
                }
                self.print_char(61i32);
                if (t == int_val) {
                    self.print_int(((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()) as i64));
                } else {
                    if (t == dimen_val) {
                        {
                            self.print_scaled(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                            self.print(312i32);
                        }
                    } else {
                        {
                            p = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                            if (t == glue_val) {
                                self.print_spec(p, 312i32);
                            } else {
                                if (t == mu_val) {
                                    self.print_spec(p, 344i32);
                                } else {
                                    if (t == box_val) {
                                        if (p == null) {
                                            self.print(423i32);
                                        } else {
                                            {
                                                self.depth_threshold = 0i32;
                                                self.breadth_max = 1i32;
                                                self.show_node_list(p);
                                            }
                                        }
                                    } else {
                                        if (t == tok_val) {
                                            {
                                                if (p != null) {
                                                    self.show_token_list(__av_mem[crate::ix::U((p) as usize)].hh().rh(), null, 32i32);
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.cur_level != self.sa_level) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(617i32, save_size);
                        }
                    }
                }
                { let __ix177 = self.save_ptr; self.save_stack[crate::ix::U((__ix177) as usize)].set_hh_b0(restore_sa); }
                { let __ix178 = self.save_ptr; let __v179 = self.sa_level; self.save_stack[crate::ix::U((__ix178) as usize)].set_hh_b1(__v179); }
                { let __ix180 = self.save_ptr; let __v181 = self.sa_chain; self.save_stack[crate::ix::U((__ix180) as usize)].set_hh_rh(__v181); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                self.sa_chain = null;
                self.sa_level = self.cur_level;
            }
        }
        i = __av_mem[crate::ix::U((p) as usize)].hh().b0();
        if (i < dimen_val_limit) {
            {
                if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    {
                        q = self.get_node(pointer_node_size);
                        i = tok_val_limit;
                    }
                } else {
                    {
                        q = self.get_node(word_node_size);
                        { let __v182 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v182); }
                    }
                }
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(null);
            }
        } else {
            {
                q = self.get_node(pointer_node_size);
                { let __v183 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v183); }
            }
        }
        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
        __av_mem[crate::ix::U((q) as usize)].set_hh_b0(i);
        { let __v184 = __av_mem[crate::ix::U((p) as usize)].hh().b1(); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v184); }
        { let __v185 = self.sa_chain; __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v185); }
        self.sa_chain = q;
        { let __v186 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v186); }
    }

    /// @<Declare \eTeX\ procedures for tr...
    // §1838
    pub fn sa_destroy(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() < mu_val_limit) {
            self.delete_glue_ref(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
        } else {
            if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != null) {
                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() < box_val_limit) {
                    self.flush_node_list(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                } else {
                    self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v187 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v187); }
        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == e) {
            {
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 619i32);
                }
                self.sa_destroy(p);
            }
        } else {
            {
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 620i32);
                }
                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == self.cur_level) {
                    self.sa_destroy(p);
                } else {
                    self.sa_save(p);
                }
                { let __v188 = self.cur_level; __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v188); }
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 621i32);
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v189 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v189); }
        if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == w) {
            {
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 619i32);
                }
            }
        } else {
            {
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 620i32);
                }
                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() != self.cur_level) {
                    self.sa_save(p);
                }
                { let __v190 = self.cur_level; __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v190); }
                __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
                if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 621i32);
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v191 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v191); }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 622i32);
        }
        self.sa_destroy(p);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(level_one);
        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 621i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_def` and `sa_w_def` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1840
    pub fn gsa_w_def(&mut self, mut p: halfword, mut w: i32) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.intr_rec_on {
            self.flashtex_intr_abort(3i32);
        }
        { let __v192 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v192); }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 622i32);
        }
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(level_one);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 621i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_restore` procedure restores the sparse array entries pointed
    /// at by `sa_chain`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1841
    pub fn sa_restore(&mut self) {
        let mut p: halfword = 0; // §1841
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        loop {
            p = __av_mem[crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)].hh().lh();
            if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == level_one) {
                {
                    if (__av_mem[crate::ix::U((p) as usize)].hh().b0() >= dimen_val_limit) {
                        self.sa_destroy(self.sa_chain);
                    }
                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 624i32);
                    }
                }
            } else {
                {
                    if (__av_mem[crate::ix::U((p) as usize)].hh().b0() < dimen_val_limit) {
                        if (__av_mem[crate::ix::U((self.sa_chain) as usize)].hh().b0() < dimen_val_limit) {
                            { let __v193 = __av_mem[crate::ix::U(((self.sa_chain).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v193); }
                        } else {
                            __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
                        }
                    } else {
                        {
                            self.sa_destroy(p);
                            { let __v194 = __av_mem[crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)].hh().rh(); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v194); }
                        }
                    }
                    { let __v195 = __av_mem[crate::ix::U((self.sa_chain) as usize)].hh().b1(); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v195); }
                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 625i32);
                    }
                }
            }
            self.delete_sa_ref(p);
            p = self.sa_chain;
            self.sa_chain = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            if (__av_mem[crate::ix::U((p) as usize)].hh().b0() < dimen_val_limit) {
                self.free_node(p, word_node_size);
            } else {
                self.free_node(p, pointer_node_size);
            }
            if (self.sa_chain == null) { break; }
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if self.intr_rec_on {
            self.flashtex_intr_group(c);
        }
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(617i32, save_size);
                }
            }
        }
        if (self.eTeX_mode == 1i32) {
            {
                { let __ix196 = (self.save_ptr).wrapping_add(0i32); let __v197 = self.line; self.save_stack[crate::ix::U((__ix196) as usize)].set_int(__v197); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix198 = self.save_ptr; self.save_stack[crate::ix::U((__ix198) as usize)].set_hh_b0(level_boundary); }
        { let __ix199 = self.save_ptr; let __v200 = self.cur_group; self.save_stack[crate::ix::U((__ix199) as usize)].set_hh_b1(__v200); }
        { let __ix201 = self.save_ptr; let __v202 = self.cur_boundary; self.save_stack[crate::ix::U((__ix201) as usize)].set_hh_rh(__v202); }
        if (self.cur_level == max_quarterword) {
            self.overflow(618i32, 255i32);
        }
        self.cur_boundary = self.save_ptr;
        self.cur_group = c;
        if (__av_eqtb[crate::ix::U(((29380i32) - 1) as usize)].int() > 0i32) {
            self.group_trace(false);
        }
        self.cur_level = (self.cur_level).wrapping_add(1i32);
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        { let __ix203 = self.cur_level; let __v204 = self.cur_input.synctex_tag_field; self.ls_grp_tag[crate::ix::U((__ix203) as usize)] = __v204; }
    }

    /// Just before an entry of `eqtb` is changed, the following procedure should
    /// be called to update the other data structures properly. It is important
    /// to keep in mind that reference counts in `mem` include references from
    /// within `save_stack`, so these counts must be handled carefully.
    // §297
    pub fn eq_destroy(&mut self, mut w: memory_word) {
        let mut q: halfword = 0; // §297
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        match w.hh().b0() {
            call | long_call | outer_call | long_outer_call => {
                self.delete_token_ref(w.hh().rh());
            }
            glue_ref => {
                self.delete_glue_ref(w.hh().rh());
            }
            shape_ref => {
                {
                    q = w.hh().rh();
                    if (q != null) {
                        self.free_node(q, ((__av_mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(__av_mem[crate::ix::U((q) as usize)].hh().lh())).wrapping_add(1i32));
                    }
                }
            }
            box_ref => {
                self.flush_node_list(w.hh().rh());
            }
            toks_register | register => {
                // §1834
                if ((w.hh().rh() < mem_bot) || (w.hh().rh() > lo_mem_stat_max)) {
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(617i32, save_size);
                }
            }
        }
        if (l == level_zero) {
            { let __ix205 = self.save_ptr; self.save_stack[crate::ix::U((__ix205) as usize)].set_hh_b0(restore_zero); }
        } else {
            {
                { let __ix206 = self.save_ptr; let __v207 = __av_eqtb[crate::ix::U(((p) - 1) as usize)]; self.save_stack[crate::ix::U((__ix206) as usize)] = __v207; }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix208 = self.save_ptr; self.save_stack[crate::ix::U((__ix208) as usize)].set_hh_b0(restore_old_value); }
            }
        }
        { let __ix209 = self.save_ptr; self.save_stack[crate::ix::U((__ix209) as usize)].set_hh_b1(l); }
        { let __ix210 = self.save_ptr; self.save_stack[crate::ix::U((__ix210) as usize)].set_hh_rh(p); }
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        'l_exit_f: {
            if self.intr_rec_on {
                self.flashtex_intr_def(p, t, e, 0i32);
            }
            if (((self.eTeX_mode == 1i32) && (__av_eqtb[crate::ix::U(((p) - 1) as usize)].hh().b0() == t)) && (__av_eqtb[crate::ix::U(((p) - 1) as usize)].hh().rh() == e)) {
                {
                    if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 619i32);
                    }
                    self.eq_destroy(__av_eqtb[crate::ix::U(((p) - 1) as usize)]);
                    break 'l_exit_f;
                }
            }
            if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 620i32);
            }
            if (__av_eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1() == self.cur_level) {
                self.eq_destroy(__av_eqtb[crate::ix::U(((p) - 1) as usize)]);
            } else {
                if (self.cur_level > level_one) {
                    self.eq_save(p, __av_eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1());
                }
            }
            { let __v211 = self.cur_level; __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(__v211); }
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
            if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
                self.flashtex_intr_touch(p);
            }
            if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 621i32);
            }
        }
    }

    /// The counterpart of `eq_define` for the remaining (fullword) positions in
    /// `eqtb` is called `eq_word_define`. Since `xeq_level[p]>=level_one` for all
    /// `p`, a ``restore_zero`' will never be used in this case.
    // §300
    pub fn eq_word_define(&mut self, mut p: halfword, mut w: i32) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        'l_exit_f: {
            if self.intr_rec_on {
                self.flashtex_intr_def(p, 0i32, w, 1i32);
            }
            if ((self.eTeX_mode == 1i32) && (__av_eqtb[crate::ix::U(((p) - 1) as usize)].int() == w)) {
                {
                    if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 619i32);
                    }
                    break 'l_exit_f;
                }
            }
            if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 620i32);
            }
            if (self.xeq_level[crate::ix::U(((p) - 29277) as usize)] != self.cur_level) {
                {
                    self.eq_save(p, self.xeq_level[crate::ix::U(((p) - 29277) as usize)]);
                    { let __v212 = self.cur_level; self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = __v212; }
                }
            }
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 621i32);
            }
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §301
    pub fn geq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if self.intr_rec_on {
            self.flashtex_intr_def(p, t, e, 2i32);
        }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 622i32);
        }
        {
            self.eq_destroy(__av_eqtb[crate::ix::U(((p) - 1) as usize)]);
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(level_one);
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
        }
        if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
            self.flashtex_intr_touch(p);
        }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 621i32);
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §301
    pub fn geq_word_define(&mut self, mut p: halfword, mut w: i32) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if self.intr_rec_on {
            self.flashtex_intr_def(p, 0i32, w, 3i32);
        }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 622i32);
        }
        {
            __av_eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = level_one;
        }
        if (__av_eqtb[crate::ix::U(((29379i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 621i32);
        }
    }

    /// Subroutine `save_for_after` puts a token on the stack for save-keeping.
    // §302
    pub fn save_for_after(&mut self, mut t: halfword) {
        if (self.cur_level > level_one) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(617i32, save_size);
                        }
                    }
                }
                { let __ix213 = self.save_ptr; self.save_stack[crate::ix::U((__ix213) as usize)].set_hh_b0(insert_token); }
                { let __ix214 = self.save_ptr; self.save_stack[crate::ix::U((__ix214) as usize)].set_hh_b1(level_zero); }
                { let __ix215 = self.save_ptr; self.save_stack[crate::ix::U((__ix215) as usize)].set_hh_rh(t); }
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.intr_rec_on {
            self.flashtex_intr_unsave();
        }
        a = false;
        if (self.cur_level > level_one) {
            {
                'l_done_f: {
                    self.cur_level = (self.cur_level).wrapping_sub(1i32);
                    // §304
                    while true {
                        {
                            self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == level_boundary) {
                                break 'l_done_f;
                            }
                            p = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().rh();
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == insert_token) {
                                // §348
                                {
                                    t = self.cur_tok;
                                    self.cur_tok = p;
                                    if a {
                                        {
                                            p = self.get_avail();
                                            { let __v216 = self.cur_tok; __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v216); }
                                            { let __v217 = self.cur_input.loc_field; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v217); }
                                            self.cur_input.loc_field = p;
                                            self.cur_input.start_field = p;
                                            if (self.cur_tok < right_brace_limit) {
                                                if (self.cur_tok < left_brace_limit) {
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
                                if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == restore_sa) {
                                    {
                                        self.sa_restore();
                                        self.sa_chain = p;
                                        self.sa_level = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                                    }
                                } else {
                                    {
                                        if (self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b0() == restore_old_value) {
                                            {
                                                l = self.save_stack[crate::ix::U((self.save_ptr) as usize)].hh().b1();
                                                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                                            }
                                        } else {
                                            { let __ix218 = self.save_ptr; let __v219 = __av_eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)]; self.save_stack[crate::ix::U((__ix218) as usize)] = __v219; }
                                        }
                                        // §305
                                        if ((p < int_base) || (p > eqtb_size)) {
                                            if (__av_eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1() == level_one) {
                                                {
                                                    self.eq_destroy(self.save_stack[crate::ix::U((self.save_ptr) as usize)]);
                                                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 624i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.eq_destroy(__av_eqtb[crate::ix::U(((p) - 1) as usize)]);
                                                    { let __v220 = self.save_stack[crate::ix::U((self.save_ptr) as usize)]; __av_eqtb[crate::ix::U(((p) - 1) as usize)] = __v220; }
                                                    if (self.intr_watch[crate::ix::U((p) as usize)] != 0i32) {
                                                        self.flashtex_intr_touch(p);
                                                    }
                                                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 625i32);
                                                    }
                                                }
                                            }
                                        } else {
                                            if (self.xeq_level[crate::ix::U(((p) - 29277) as usize)] != level_one) {
                                                {
                                                    { let __v221 = self.save_stack[crate::ix::U((self.save_ptr) as usize)]; __av_eqtb[crate::ix::U(((p) - 1) as usize)] = __v221; }
                                                    self.xeq_level[crate::ix::U(((p) - 29277) as usize)] = l;
                                                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 625i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    if (__av_eqtb[crate::ix::U(((29314i32) - 1) as usize)].int() > 0i32) {
                                                        self.restore_trace(p, 624i32);
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
                if (__av_eqtb[crate::ix::U(((29380i32) - 1) as usize)].int() > 0i32) {
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
            self.confusion(623i32);
        }
    }

    /// The `prepare_mag` subroutine is called whenever \TeX\ wants to use `mag`
    /// for magnification.
    // §310
    pub fn prepare_mag(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if ((self.mag_set > 0i32) && (__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() != self.mag_set)) {
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
                    self.print(627i32);
                }
                self.print_int(((__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int()) as i64));
                self.print(628i32);
                self.print_nl(629i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 630i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 631i32;
                }
                self.int_error(self.mag_set);
                self.geq_word_define(29294i32, self.mag_set);
            }
        }
        if ((__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() <= 0i32) || (__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() > 32768i32)) {
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
                    self.print(632i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 633i32;
                }
                self.int_error(__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                self.geq_word_define(29294i32, 1000i32);
            }
        }
        self.mag_set = __av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int();
    }

    /// Here's the way we sometimes want to display a token list, given a pointer
    /// to its reference count; the pointer may be null.
    // §317
    pub fn token_show(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (p != null) {
            self.show_token_list(__av_mem[crate::ix::U((p) as usize)].hh().rh(), null, 10000000i32);
        }
    }

    /// The `print_meaning` subroutine displays `cur_cmd` and `cur_chr` in
    /// symbolic form, including the expansion of a macro or mark.
    // §318
    pub fn print_meaning(&mut self) {
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.cur_cmd >= call) {
            {
                self.print_char(58i32);
                self.print_ln();
                self.token_show(self.cur_chr);
            }
        } else {
            if ((self.cur_cmd == top_bot_mark) && (self.cur_chr < marks_code)) {
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.begin_diagnostic();
        self.print_nl(123i32);
        if (self.cur_list.mode_field != self.shown_mode) {
            {
                self.print_mode(self.cur_list.mode_field);
                self.print(647i32);
                self.shown_mode = self.cur_list.mode_field;
            }
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (__av_eqtb[crate::ix::U(((29381i32) - 1) as usize)].int() > 0i32) {
            if (self.cur_cmd >= if_test) {
                if (self.cur_cmd <= fi_or_else) {
                    {
                        self.print(647i32);
                        if (self.cur_cmd == fi_or_else) {
                            {
                                self.print_cmd_chr(if_test, self.cur_if);
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
                        while (p != null) {
                            {
                                n = (n).wrapping_add(1i32);
                                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                            }
                        }
                        self.print(648i32);
                        self.print_int(((n) as i64));
                        self.print_char(41i32);
                        if (l != 0i32) {
                            {
                                self.ls_print_unknown(l);
                                self.print(2010i32);
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_done_f: {
            self.base_ptr = self.input_ptr;
            { let __ix222 = self.base_ptr; let __v223 = self.cur_input; self.input_stack[crate::ix::U((__ix222) as usize)] = __v223; }
            nn = (1i32).wrapping_neg();
            bottom_line = false;
            while true {
                {
                    self.cur_input = self.input_stack[crate::ix::U((self.base_ptr) as usize)];
                    if (self.cur_input.state_field != token_list) {
                        if ((self.cur_input.name_field > 19i32) || (self.base_ptr == 0i32)) {
                            bottom_line = true;
                        }
                    }
                    if (((self.base_ptr == self.input_ptr) || bottom_line) || (nn < __av_eqtb[crate::ix::U(((29331i32) - 1) as usize)].int())) {
                        // §334
                        {
                            if ((((self.base_ptr == self.input_ptr) || (self.cur_input.state_field != token_list)) || (self.cur_input.index_field != backed_up)) || (self.cur_input.loc_field != null)) {
                                {
                                    self.tally = 0i32;
                                    old_setting = self.selector;
                                    if (self.cur_input.state_field != token_list) {
                                        {
                                            // §335
                                            if (self.cur_input.name_field <= 17i32) {
                                                if (self.cur_input.name_field == 0i32) {
                                                    if (self.base_ptr == 0i32) {
                                                        self.print_nl(654i32);
                                                    } else {
                                                        self.print_nl(655i32);
                                                    }
                                                } else {
                                                    {
                                                        self.print_nl(656i32);
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
                                                    self.print_nl(657i32);
                                                    self.ls_print_level(self.cur_input.index_field);
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
                                                self.selector = pseudo;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.buffer[crate::ix::U((self.cur_input.limit_field) as usize)] == __av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int()) {
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
                                                parameter => {
                                                    self.print_nl(658i32);
                                                }
                                                u_template | v_template => {
                                                    self.print_nl(659i32);
                                                }
                                                backed_up => {
                                                    if (self.cur_input.loc_field == null) {
                                                        self.print_nl(660i32);
                                                    } else {
                                                        self.print_nl(661i32);
                                                    }
                                                }
                                                inserted => {
                                                    self.print_nl(662i32);
                                                }
                                                macro_ => {
                                                    {
                                                        self.print_ln();
                                                        self.print_cs(self.cur_input.name_field);
                                                    }
                                                }
                                                output_text => {
                                                    self.print_nl(663i32);
                                                }
                                                every_par_text => {
                                                    self.print_nl(664i32);
                                                }
                                                every_math_text => {
                                                    self.print_nl(665i32);
                                                }
                                                every_display_text => {
                                                    self.print_nl(666i32);
                                                }
                                                every_hbox_text => {
                                                    self.print_nl(667i32);
                                                }
                                                every_vbox_text => {
                                                    self.print_nl(668i32);
                                                }
                                                every_job_text => {
                                                    self.print_nl(669i32);
                                                }
                                                every_cr_text => {
                                                    self.print_nl(670i32);
                                                }
                                                mark_text => {
                                                    self.print_nl(671i32);
                                                }
                                                every_eof_text => {
                                                    self.print_nl(672i32);
                                                }
                                                write_text => {
                                                    self.print_nl(673i32);
                                                }
                                                _ => {
                                                    self.print_nl(63i32);
                                                }
                                            }
                                            // §341
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = pseudo;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.cur_input.index_field < macro_) {
                                                self.show_token_list(self.cur_input.start_field, self.cur_input.loc_field, 100000i32);
                                            } else {
                                                self.show_token_list(__av_mem[crate::ix::U((self.cur_input.start_field) as usize)].hh().rh(), self.cur_input.loc_field, 100000i32);
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
                                            self.print(277i32);
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
                                        self.print(277i32);
                                    }
                                    // §334
                                    nn = (nn).wrapping_add(1i32);
                                }
                            }
                        }
                    } else {
                        // §333
                        if (nn == __av_eqtb[crate::ix::U(((29331i32) - 1) as usize)].int()) {
                            {
                                self.print_nl(277i32);
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
    #[inline(always)]
    pub fn begin_token_list(&mut self, mut p: halfword, mut t: quarterword) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(674i32, stack_size);
                    }
                }
            }
            { let __ix224 = self.input_ptr; let __v225 = self.cur_input; self.input_stack[crate::ix::U((__ix224) as usize)] = __v225; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = token_list;
        self.cur_input.start_field = p;
        self.cur_input.index_field = t;
        if (t >= macro_) {
            {
                { let __v226 = (__av_mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v226); }
                if (t == macro_) {
                    self.cur_input.limit_field = self.param_ptr;
                } else {
                    {
                        self.cur_input.loc_field = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                        if (__av_eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 1i32) {
                            {
                                self.begin_diagnostic();
                                self.print_nl(345i32);
                                match t {
                                    mark_text => {
                                        self.print_esc(360i32);
                                    }
                                    write_text => {
                                        self.print_esc(675i32);
                                    }
                                    _ => {
                                        self.print_cmd_chr(assign_toks, (t).wrapping_add(27153i32));
                                    }
                                }
                                self.print(635i32);
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
    #[inline(always)]
    pub fn end_token_list(&mut self) {
        if (self.cur_input.index_field >= backed_up) {
            {
                if (self.cur_input.index_field <= inserted) {
                    self.flush_list(self.cur_input.start_field);
                } else {
                    {
                        self.delete_token_ref(self.cur_input.start_field);
                        if (self.cur_input.index_field == macro_) {
                            while (self.param_ptr > self.cur_input.limit_field) {
                                {
                                    self.param_ptr = (self.param_ptr).wrapping_sub(1i32);
                                    self.flush_list(self.param_stack[crate::ix::U((self.param_ptr) as usize)]);
                                }
                            }
                        } else {
                            if ((self.cur_input.index_field == output_text) && (!self.output_can_end)) {
                                self.fatal_error(676i32);
                            }
                        }
                    }
                }
            }
        } else {
            if (self.cur_input.index_field == u_template) {
                if (self.align_state > 500000i32) {
                    self.align_state = 0i32;
                } else {
                    self.fatal_error(677i32);
                }
            }
        }
        if self.macro_prof_on {
            if (self.cur_input.index_field == macro_) {
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
    #[inline(always)]
    pub fn back_input(&mut self) {
        let mut p: halfword = 0; // §347
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        while (((self.cur_input.loc_field == null) && (self.cur_input.index_field != v_template)) && (self.cur_input.index_field != output_text)) {
            self.end_token_list();
        }
        self.dl_token_begin();
        p = self.get_avail();
        self.dl_token_end();
        { let __v227 = self.cur_tok; __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v227); }
        if (self.cur_tok < right_brace_limit) {
            if (self.cur_tok < left_brace_limit) {
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
                        self.overflow(674i32, stack_size);
                    }
                }
            }
            { let __ix228 = self.input_ptr; let __v229 = self.cur_input; self.input_stack[crate::ix::U((__ix228) as usize)] = __v229; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = token_list;
        self.cur_input.start_field = p;
        self.cur_input.index_field = backed_up;
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
        self.cur_input.index_field = inserted;
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
            self.overflow(678i32, max_in_open);
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
                        self.overflow(674i32, stack_size);
                    }
                }
            }
            { let __ix230 = self.input_ptr; let __v231 = self.cur_input; self.input_stack[crate::ix::U((__ix230) as usize)] = __v231; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.index_field = self.in_open;
        self.full_source_filename_stack[crate::ix::U((self.cur_input.index_field) as usize)] = 0i32;
        { let __ix232 = self.cur_input.index_field; let __v233 = false; self.eof_seen[crate::ix::U(((__ix232) - 1) as usize)] = __v233; }
        { let __ix234 = self.cur_input.index_field; let __v235 = self.cur_boundary; self.grp_stack[crate::ix::U((__ix234) as usize)] = __v235; }
        { let __ix236 = self.cur_input.index_field; let __v237 = self.cond_ptr; self.if_stack[crate::ix::U((__ix236) as usize)] = __v237; }
        { let __ix238 = self.cur_input.index_field; let __v239 = self.line; self.line_stack[crate::ix::U(((__ix238) - 1) as usize)] = __v239; }
        self.cur_input.start_field = self.first;
        self.cur_input.state_field = mid_line;
        self.cur_input.name_field = 0i32;
        // §1892
        self.cur_input.synctex_tag_field = 0i32;
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
        while ((((self.cur_input.state_field != token_list) && (self.cur_input.name_field == 0i32)) && (self.input_ptr > 0i32)) && (self.cur_input.loc_field > self.cur_input.limit_field)) {
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.scanner_status != normal) {
            {
                self.deletions_allowed = false;
                // §359
                if (self.cur_cs != 0i32) {
                    {
                        if (((self.cur_input.state_field == token_list) || (self.cur_input.name_field < 1i32)) || (self.cur_input.name_field > 17i32)) {
                            {
                                p = self.get_avail();
                                { let __v240 = (cs_token_flag).wrapping_add(self.cur_cs); __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v240); }
                                self.begin_token_list(p, backed_up);
                            }
                        }
                        self.cur_cmd = spacer;
                        self.cur_chr = 32i32;
                    }
                }
                // §358
                if (self.scanner_status > skipping) {
                    // §360
                    {
                        self.runaway();
                        if (self.cur_cs == 0i32) {
                            {
                                self.dg_mark();
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(686i32);
                            }
                        } else {
                            {
                                self.cur_cs = 0i32;
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(687i32);
                                }
                            }
                        }
                        self.print(688i32);
                        // §361
                        p = self.get_avail();
                        match self.scanner_status {
                            defining => {
                                {
                                    self.print(650i32);
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                }
                            }
                            matching => {
                                {
                                    self.print(694i32);
                                    { let __v241 = self.par_token; __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v241); }
                                    self.long_state = outer_call;
                                }
                            }
                            aligning => {
                                {
                                    self.print(652i32);
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                    q = p;
                                    p = self.get_avail();
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(19610i32);
                                    self.align_state = (1000000i32).wrapping_neg();
                                }
                            }
                            absorbing => {
                                {
                                    self.print(653i32);
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(637i32);
                                }
                            }
                            _ => {}
                        }
                        self.begin_token_list(p, inserted);
                        // §360
                        self.print(689i32);
                        self.sprint_cs(self.warning_index);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 690i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 691i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 692i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 693i32;
                        }
                        self.error();
                    }
                } else {
                    // §358
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
                            self.print(680i32);
                        }
                        self.print_cmd_chr(if_test, self.cur_if);
                        self.ls_print_unknown(self.skip_line);
                        self.print(681i32);
                        self.print_int(((self.skip_line) as i64));
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 682i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 683i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 684i32;
                        }
                        if (self.cur_cs != 0i32) {
                            self.cur_cs = 0i32;
                        } else {
                            self.help_line[crate::ix::U((2i32) as usize)] = 685i32;
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
    pub fn get_next_file(&mut self) -> i32 {
        let mut get_next_file: i32 = 0;
        let mut k: i32 = 0; // §363
        let mut cat: i32 = 0; // §363
        let mut c: ASCII_code = 0; // §363
        let mut cc: ASCII_code = 0; // §363
        let mut d: i32 = 0; // §363
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        'l_exit_f: {
            'l_restart_f: {
                get_next_file = 1i32;
                // §365
                {
                    'l_L25_b: loop {
                        if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                            {
                                self.cur_chr = self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)];
                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                'l_reswitch_b: loop {
                                    self.cur_cmd = __av_eqtb[crate::ix::U((((cat_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
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
                                                        self.cur_cs = null_cs;
                                                    } else {
                                                        {
                                                            'l_L26_b: loop {
                                                                k = self.cur_input.loc_field;
                                                                self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                cat = __av_eqtb[crate::ix::U((((cat_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                                k = (k).wrapping_add(1i32);
                                                                if (cat == letter) {
                                                                    self.cur_input.state_field = skip_blanks;
                                                                } else {
                                                                    if (cat == spacer) {
                                                                        self.cur_input.state_field = skip_blanks;
                                                                    } else {
                                                                        self.cur_input.state_field = mid_line;
                                                                    }
                                                                }
                                                                if ((cat == letter) && (k <= self.cur_input.limit_field)) {
                                                                    // §378
                                                                    {
                                                                        loop {
                                                                            self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                            cat = __av_eqtb[crate::ix::U((((cat_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                                            k = (k).wrapping_add(1i32);
                                                                            if ((cat != letter) || (k > self.cur_input.limit_field)) { break; }
                                                                        }
                                                                        // §377
                                                                        {
                                                                            if (self.buffer[crate::ix::U((k) as usize)] == self.cur_chr) {
                                                                                if (cat == sup_mark) {
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
                                                                                                            { let __v242 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v242; }
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
                                                                                                            { let __v243 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v243; }
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
                                                                        if (cat != letter) {
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
                                                                            if (cat == sup_mark) {
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
                                                                                                        { let __v244 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v244; }
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
                                                                                                        { let __v245 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v245; }
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
                                                                self.cur_cs = (single_base).wrapping_add(self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)]);
                                                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                                                break 'l_L26_b;
                                                            }
                                                        }
                                                    }
                                                }
                                                self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                if self.rs_on {
                                                    if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                        self.flashtex_cs_read(self.cur_cs);
                                                    }
                                                }
                                                if (self.cur_cmd >= outer_call) {
                                                    self.check_outer_validity();
                                                }
                                            }
                                        }
                                        14 | 30 | 46 => {
                                            // §375
                                            {
                                                self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                                self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                self.cur_input.state_field = mid_line;
                                                if self.rs_on {
                                                    if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                        self.flashtex_cs_read(self.cur_cs);
                                                    }
                                                }
                                                if (self.cur_cmd >= outer_call) {
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
                                                self.cur_input.state_field = mid_line;
                                            }
                                        }
                                        16 | 32 | 48 => {
                                            // §368
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
                                                    self.print(695i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 696i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 697i32;
                                                }
                                                self.deletions_allowed = false;
                                                self.error();
                                                self.deletions_allowed = true;
                                                break 'l_restart_f;
                                            }
                                        }
                                        11 => {
                                            // §371
                                            {
                                                self.cur_input.state_field = skip_blanks;
                                                self.cur_chr = 32i32;
                                            }
                                        }
                                        6 => {
                                            // §370
                                            {
                                                self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                self.cur_cmd = spacer;
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
                                                self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                if self.rs_on {
                                                    if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                        self.flashtex_cs_read(self.cur_cs);
                                                    }
                                                }
                                                if (self.cur_cmd >= outer_call) {
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
                                                self.cur_input.state_field = mid_line;
                                                self.align_state = (self.align_state).wrapping_add(1i32);
                                            }
                                        }
                                        3 => {
                                            self.align_state = (self.align_state).wrapping_sub(1i32);
                                        }
                                        19 | 35 => {
                                            {
                                                self.cur_input.state_field = mid_line;
                                                self.align_state = (self.align_state).wrapping_sub(1i32);
                                            }
                                        }
                                        20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41 | 44 | 45 => {
                                            self.cur_input.state_field = mid_line;
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
                                self.cur_input.state_field = new_line;
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
                                                        if ((__av_eqtb[crate::ix::U(((every_eof_loc) - 1) as usize)].hh().rh() != null) && (!self.eof_seen[crate::ix::U(((self.cur_input.index_field) - 1) as usize)])) {
                                                            {
                                                                self.cur_input.limit_field = (self.first).wrapping_sub(1i32);
                                                                { let __ix246 = self.cur_input.index_field; let __v247 = true; self.eof_seen[crate::ix::U(((__ix246) - 1) as usize)] = __v247; }
                                                                self.begin_token_list(__av_eqtb[crate::ix::U(((every_eof_loc) - 1) as usize)].hh().rh(), every_eof_text);
                                                                break 'l_restart_f;
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
                                                        if ((__av_eqtb[crate::ix::U(((every_eof_loc) - 1) as usize)].hh().rh() != null) && (!self.eof_seen[crate::ix::U(((self.cur_input.index_field) - 1) as usize)])) {
                                                            {
                                                                self.cur_input.limit_field = (self.first).wrapping_sub(1i32);
                                                                { let __ix248 = self.cur_input.index_field; let __v249 = true; self.eof_seen[crate::ix::U(((__ix248) - 1) as usize)] = __v249; }
                                                                self.begin_token_list(__av_eqtb[crate::ix::U(((every_eof_loc) - 1) as usize)].hh().rh(), every_eof_text);
                                                                break 'l_restart_f;
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
                                                if (__av_eqtb[crate::ix::U(((29383i32) - 1) as usize)].int() > 0i32) {
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
                                                break 'l_restart_f;
                                            }
                                        }
                                        if ((__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                            self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                        } else {
                                            { let __ix250 = self.cur_input.limit_field; let __v251 = __av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix250) as usize)] = __v251; }
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
                                                break 'l_exit_f;
                                            }
                                        }
                                        if (self.input_ptr > 0i32) {
                                            {
                                                self.end_file_reading();
                                                break 'l_restart_f;
                                            }
                                        }
                                        if (self.selector < log_only) {
                                            self.open_log_file();
                                        }
                                        if (self.interaction > nonstop_mode) {
                                            {
                                                if ((__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                }
                                                if (self.cur_input.limit_field == self.cur_input.start_field) {
                                                    self.print_nl(699i32);
                                                }
                                                self.print_ln();
                                                self.first = self.cur_input.start_field;
                                                {
                                                    self.print(42i32);
                                                    self.term_input();
                                                }
                                                self.cur_input.limit_field = self.last;
                                                if ((__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (__av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                                                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                                } else {
                                                    { let __ix252 = self.cur_input.limit_field; let __v253 = __av_eqtb[crate::ix::U(((29325i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix252) as usize)] = __v253; }
                                                }
                                                self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                                                self.cur_input.loc_field = self.cur_input.start_field;
                                            }
                                        } else {
                                            self.fatal_error(700i32);
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
                // §363
                get_next_file = 2i32;
                break 'l_exit_f;
            }
            get_next_file = 0i32;
        }
        get_next_file
    }

    /// Now we're ready to take the plunge into `get_next` itself. Parts of
    /// this routine are executed more often than any other instructions of \TeX.
    // §363
    #[inline(never)]
    pub fn get_next_slow(&mut self) {
        let mut t: halfword = 0; // §363
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.cur_cs = 0i32;
                if (self.cur_input.state_field != token_list) {
                    match self.get_next_file() {
                        0 => {
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                        1 => {
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                        _ => {
                        }
                    }
                } else {
                    // §379
                    if (self.cur_input.loc_field != null) {
                        {
                            t = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh();
                            self.cur_input.loc_field = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().rh();
                            if (t >= cs_token_flag) {
                                {
                                    self.cur_cs = (t).wrapping_sub(4095i32);
                                    self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                    self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                    if self.rs_on {
                                        if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                            self.flashtex_cs_read(self.cur_cs);
                                        }
                                    }
                                    if (self.cur_cmd >= outer_call) {
                                        if (self.cur_cmd == dont_expand) {
                                            // §380
                                            {
                                                self.cur_cs = (__av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh()).wrapping_sub(4095i32);
                                                self.cur_input.loc_field = null;
                                                self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                                self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                                if self.rs_on {
                                                    if (!self.rs_seen[crate::ix::U((self.cur_cs) as usize)]) {
                                                        self.flashtex_cs_read(self.cur_cs);
                                                    }
                                                }
                                                if (self.cur_cmd > max_command) {
                                                    {
                                                        self.cur_cmd = relax;
                                                        self.cur_chr = no_expand_flag;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §379
                                            {
                                                if ((self.cur_cs == end_write) && (self.cur_list.mode_field == 0i32)) {
                                                    self.fatal_error(698i32);
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
                                        left_brace => {
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                        }
                                        right_brace => {
                                            self.align_state = (self.align_state).wrapping_sub(1i32);
                                        }
                                        out_param => {
                                            // §381
                                            {
                                                self.begin_token_list(self.param_stack[crate::ix::U((((self.cur_input.limit_field).wrapping_add(self.cur_chr)).wrapping_sub(1i32)) as usize)], parameter);
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
                if (self.cur_cmd <= car_ret) {
                    if (self.cur_cmd >= tab_mark) {
                        if (self.align_state == 0i32) {
                            // §965
                            {
                                if ((self.scanner_status == aligning) || (self.cur_align == null)) {
                                    self.fatal_error(677i32);
                                }
                                self.cur_cmd = __av_mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh();
                                { let __ix254 = (self.cur_align).wrapping_add(5i32); let __v255 = self.cur_chr; __av_mem[crate::ix::U((__ix254) as usize)].set_hh_lh(__v255); }
                                if (self.cur_cmd == omit) {
                                    self.begin_token_list(omit_template, v_template);
                                } else {
                                    self.begin_token_list(__av_mem[crate::ix::U(((self.cur_align).wrapping_add(2i32)) as usize)].int(), v_template);
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

    /// Now we're ready to take the plunge into `get_next` itself. Parts of
    /// this routine are executed more often than any other instructions of \TeX.
    // §363
    #[inline(always)]
    pub fn get_next(&mut self) {
        let mut t: halfword = 0; // §363
        let mut n: halfword = 0; // §363
        let mut q: halfword = 0; // §363
        let mut c: i32 = 0; // §363
        let mut e: halfword = 0; // §363
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        c = (1i32).wrapping_neg();
        if (self.cur_input.state_field == token_list) {
            if (self.cur_input.loc_field != null) {
                {
                    t = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh();
                    n = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().rh();
                    if (t >= cs_token_flag) {
                        {
                            q = (t).wrapping_sub(4095i32);
                            c = __av_eqtb[crate::ix::U(((q) - 1) as usize)].hh().b0();
                            e = __av_eqtb[crate::ix::U(((q) - 1) as usize)].hh().rh();
                            if ((c >= outer_call) || (((c <= car_ret) && (c >= tab_mark)) && (self.align_state == 0i32))) {
                                c = (1i32).wrapping_neg();
                            } else {
                                {
                                    self.cur_cs = q;
                                    self.cur_input.loc_field = n;
                                    self.cur_cmd = c;
                                    self.cur_chr = e;
                                    if self.rs_on {
                                        if (!self.rs_seen[crate::ix::U((q) as usize)]) {
                                            self.flashtex_cs_read(q);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        {
                            c = (t / 256i32);
                            if ((c == out_param) || ((c == tab_mark) && (self.align_state == 0i32))) {
                                c = (1i32).wrapping_neg();
                            } else {
                                {
                                    self.cur_cs = 0i32;
                                    self.cur_input.loc_field = n;
                                    self.cur_cmd = c;
                                    self.cur_chr = (t % 256i32);
                                    if (c == left_brace) {
                                        self.align_state = (self.align_state).wrapping_add(1i32);
                                    } else {
                                        if (c == right_brace) {
                                            self.align_state = (self.align_state).wrapping_sub(1i32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if (c < 0i32) {
            self.get_next_slow();
        } else {
            if self.intr_rec_on {
                self.flashtex_intr_next();
            }
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.cur_input.limit_field = self.last;
        if (__av_eqtb[crate::ix::U(((29305i32) - 1) as usize)].int() > 0i32) {
            if (self.interaction > nonstop_mode) {
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
                        self.print(701i32);
                        self.term_input();
                    }
                    if (self.last > self.first) {
                        {
                            {
                                let __for_end_7 = (self.last).wrapping_sub(1i32);
                                k = self.first;
                                while k <= __for_end_7 {
                                    { let __ix256 = ((k).wrapping_add(self.cur_input.start_field)).wrapping_sub(self.first); let __v257 = self.buffer[crate::ix::U((k) as usize)]; self.buffer[crate::ix::U((__ix256) as usize)] = __v257; }
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
    #[inline(always)]
    pub fn get_token(&mut self) {
        self.no_new_control_sequence = false;
        self.get_next();
        self.no_new_control_sequence = true;
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
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
        let mut ft: halfword = 0; // §415
        let mut fq: halfword = 0; // §415
        let mut fc: i32 = 0; // §415
        let mut in_place: bool = false; // §415
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
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
            self.ls_use(ref_count);
            r = __av_mem[crate::ix::U((ref_count) as usize)].hh().rh();
            n = 0i32;
            if (__av_eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 0i32) {
                // §427
                {
                    self.begin_diagnostic();
                    if (__av_eqtb[crate::ix::U(((29335i32) - 1) as usize)].int() > 0i32) {
                        if (self.input_ptr < __av_eqtb[crate::ix::U(((29335i32) - 1) as usize)].int()) {
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
            if (__av_mem[crate::ix::U((r) as usize)].hh().lh() == protected_token) {
                r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
            }
            if (__av_mem[crate::ix::U((r) as usize)].hh().lh() != end_match_token) {
                // §417
                {
                    self.scanner_status = matching;
                    unbalance = 0i32;
                    self.long_state = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                    if (self.long_state >= outer_call) {
                        self.long_state = (self.long_state).wrapping_sub(2i32);
                    }
                    loop {
                        // goto labels: continue, found
                        let mut __goto_1: i32 = 0;
                        'l_dispatch_1: loop {
                            if __goto_1 <= 0 {
                                __av_mem[crate::ix::U((temp_head) as usize)].set_hh_rh(null);
                                if ((__av_mem[crate::ix::U((r) as usize)].hh().lh() > 3583i32) || (__av_mem[crate::ix::U((r) as usize)].hh().lh() < match_token)) {
                                    s = null;
                                } else {
                                    {
                                        match_chr = (__av_mem[crate::ix::U((r) as usize)].hh().lh()).wrapping_sub(3328i32);
                                        s = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                        r = s;
                                        p = temp_head;
                                        m = 0i32;
                                    }
                                }
                            }
                            if __goto_1 <= 1 { // continue
                                // §418
                                self.get_token();
                                if (self.cur_tok == __av_mem[crate::ix::U((r) as usize)].hh().lh()) {
                                    // §420
                                    {
                                        r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                        if ((__av_mem[crate::ix::U((r) as usize)].hh().lh() >= match_token) && (__av_mem[crate::ix::U((r) as usize)].hh().lh() <= end_match_token)) {
                                            {
                                                if (self.cur_tok < left_brace_limit) {
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
                                    if (s == null) {
                                        // §424
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
                                                self.print(734i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(735i32);
                                            {
                                                self.help_ptr = 4i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] = 736i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 737i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 738i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 739i32;
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
                                                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                        { let __v258 = __av_mem[crate::ix::U((t) as usize)].hh().lh(); __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v258); }
                                                        p = q;
                                                    }
                                                    m = (m).wrapping_add(1i32);
                                                    u = __av_mem[crate::ix::U((t) as usize)].hh().rh();
                                                    v = s;
                                                    while true {
                                                        {
                                                            if (u == r) {
                                                                if (self.cur_tok != __av_mem[crate::ix::U((v) as usize)].hh().lh()) {
                                                                    break 'l_done_f;
                                                                } else {
                                                                    {
                                                                        r = __av_mem[crate::ix::U((v) as usize)].hh().rh();
                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                    }
                                                                }
                                                            }
                                                            if (__av_mem[crate::ix::U((u) as usize)].hh().lh() != __av_mem[crate::ix::U((v) as usize)].hh().lh()) {
                                                                break 'l_done_f;
                                                            }
                                                            u = __av_mem[crate::ix::U((u) as usize)].hh().rh();
                                                            v = __av_mem[crate::ix::U((v) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                                t = __av_mem[crate::ix::U((t) as usize)].hh().rh();
                                                if (t == r) { break; }
                                            }
                                            r = s;
                                        }
                                    }
                                }
                                // §418
                                if (self.cur_tok == self.par_token) {
                                    if (self.long_state != long_call) {
                                        // §422
                                        {
                                            if (self.long_state == call) {
                                                {
                                                    self.runaway();
                                                    {
                                                        self.dg_mark();
                                                        if (self.interaction == error_stop_mode) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(264i32);
                                                        }
                                                        self.print(729i32);
                                                    }
                                                    self.sprint_cs(self.warning_index);
                                                    self.print(730i32);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[crate::ix::U((2i32) as usize)] = 731i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 732i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 733i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                            { let __v259 = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v259; }
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
                                if (self.cur_tok < right_brace_limit) {
                                    if (self.cur_tok < left_brace_limit) {
                                        // §425
                                        {
                                            'l_done1_f: {
                                                unbalance = 1i32;
                                                while true {
                                                    {
                                                        {
                                                            {
                                                                q = self.avail;
                                                                if (q == null) {
                                                                    q = self.get_avail();
                                                                } else {
                                                                    {
                                                                        self.avail = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                                                                        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                                                        self.dl_new_node(q);
                                                                    }
                                                                }
                                                            }
                                                            __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            { let __v260 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v260); }
                                                            p = q;
                                                        }
                                                        in_place = false;
                                                        if (self.cur_input.state_field == token_list) {
                                                            if (self.cur_input.loc_field != null) {
                                                                if (!self.intr_rec_on) {
                                                                    {
                                                                        ft = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().lh();
                                                                        if (ft >= cs_token_flag) {
                                                                            {
                                                                                if (ft != self.par_token) {
                                                                                    {
                                                                                        fq = (ft).wrapping_sub(4095i32);
                                                                                        fc = __av_eqtb[crate::ix::U(((fq) - 1) as usize)].hh().b0();
                                                                                        if ((fc < outer_call) && (((fc > car_ret) || (fc < tab_mark)) || (self.align_state != 0i32))) {
                                                                                            {
                                                                                                self.cur_input.loc_field = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().rh();
                                                                                                in_place = true;
                                                                                                if self.rs_on {
                                                                                                    if (!self.rs_seen[crate::ix::U((fq) as usize)]) {
                                                                                                        self.flashtex_cs_read(fq);
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            {
                                                                                fc = (ft / 256i32);
                                                                                if ((fc != out_param) && ((fc != tab_mark) || (self.align_state != 0i32))) {
                                                                                    {
                                                                                        self.cur_input.loc_field = __av_mem[crate::ix::U((self.cur_input.loc_field) as usize)].hh().rh();
                                                                                        in_place = true;
                                                                                        if (fc == left_brace) {
                                                                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                                                                        } else {
                                                                                            if (fc == right_brace) {
                                                                                                self.align_state = (self.align_state).wrapping_sub(1i32);
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
                                                        if in_place {
                                                            {
                                                                self.cur_tok = ft;
                                                                self.no_new_control_sequence = true;
                                                            }
                                                        } else {
                                                            {
                                                                self.get_token();
                                                                if (self.cur_tok == self.par_token) {
                                                                    if (self.long_state != long_call) {
                                                                        // §422
                                                                        {
                                                                            if (self.long_state == call) {
                                                                                {
                                                                                    self.runaway();
                                                                                    {
                                                                                        self.dg_mark();
                                                                                        if (self.interaction == error_stop_mode) {
                                                                                        }
                                                                                        if self.file_line_error_style_p {
                                                                                            self.print_file_line();
                                                                                        } else {
                                                                                            self.print_nl(264i32);
                                                                                        }
                                                                                        self.print(729i32);
                                                                                    }
                                                                                    self.sprint_cs(self.warning_index);
                                                                                    self.print(730i32);
                                                                                    {
                                                                                        self.help_ptr = 3i32;
                                                                                        self.help_line[crate::ix::U((2i32) as usize)] = 731i32;
                                                                                        self.help_line[crate::ix::U((1i32) as usize)] = 732i32;
                                                                                        self.help_line[crate::ix::U((0i32) as usize)] = 733i32;
                                                                                    }
                                                                                    self.back_error();
                                                                                }
                                                                            }
                                                                            { let __v261 = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v261; }
                                                                            self.align_state = (self.align_state).wrapping_sub(unbalance);
                                                                            {
                                                                                let __for_end_19 = n;
                                                                                m = 0i32;
                                                                                while m <= __for_end_19 {
                                                                                    self.flush_list(self.pstack[crate::ix::U((m) as usize)]);
                                                                                    m = m.wrapping_add(1);
                                                                                }
                                                                            }
                                                                            break 'l_exit_f;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §425
                                                        if (self.cur_tok < right_brace_limit) {
                                                            if (self.cur_tok < left_brace_limit) {
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
                                            if in_place {
                                                {
                                                    self.cur_cs = 0i32;
                                                    self.cur_cmd = (self.cur_tok / 256i32);
                                                    self.cur_chr = (self.cur_tok % 256i32);
                                                }
                                            }
                                            rbrace_ptr = p;
                                            {
                                                q = self.get_avail();
                                                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                { let __v262 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v262); }
                                                p = q;
                                            }
                                        }
                                    } else {
                                        // §421
                                        {
                                            self.back_input();
                                            {
                                                self.dg_mark();
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(721i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(722i32);
                                            {
                                                self.help_ptr = 6i32;
                                                self.help_line[crate::ix::U((5i32) as usize)] = 723i32;
                                                self.help_line[crate::ix::U((4i32) as usize)] = 724i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] = 725i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 726i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 727i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 728i32;
                                            }
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                            self.long_state = call;
                                            self.cur_tok = self.par_token;
                                            self.ins_error();
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                } else {
                                    // §419
                                    {
                                        if (self.cur_tok == space_token) {
                                            if (__av_mem[crate::ix::U((r) as usize)].hh().lh() <= end_match_token) {
                                                if (__av_mem[crate::ix::U((r) as usize)].hh().lh() >= match_token) {
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                        {
                                            q = self.get_avail();
                                            __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v263 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v263); }
                                            p = q;
                                        }
                                    }
                                }
                                // §418
                                m = (m).wrapping_add(1i32);
                                if (__av_mem[crate::ix::U((r) as usize)].hh().lh() > end_match_token) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                                if (__av_mem[crate::ix::U((r) as usize)].hh().lh() < match_token) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                            if __goto_1 <= 2 { // found
                                if (s != null) {
                                    // §426
                                    {
                                        if ((m == 1i32) && (__av_mem[crate::ix::U((p) as usize)].hh().lh() < right_brace_limit)) {
                                            {
                                                __av_mem[crate::ix::U((rbrace_ptr) as usize)].set_hh_rh(null);
                                                {
                                                    { let __v264 = self.avail; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v264); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                                p = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh();
                                                { let __v265 = __av_mem[crate::ix::U((p) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v265; }
                                                {
                                                    { let __v266 = self.avail; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v266); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                        } else {
                                            { let __v267 = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.pstack[crate::ix::U((n) as usize)] = __v267; }
                                        }
                                        n = (n).wrapping_add(1i32);
                                        if (__av_eqtb[crate::ix::U(((29307i32) - 1) as usize)].int() > 0i32) {
                                            if ((__av_eqtb[crate::ix::U(((29335i32) - 1) as usize)].int() == 0i32) || (self.input_ptr < __av_eqtb[crate::ix::U(((29335i32) - 1) as usize)].int())) {
                                                {
                                                    self.begin_diagnostic();
                                                    self.print_nl(match_chr);
                                                    self.print_int(((n) as i64));
                                                    self.print(740i32);
                                                    self.show_token_list(self.pstack[crate::ix::U(((n).wrapping_sub(1i32)) as usize)], null, 1000i32);
                                                    self.end_diagnostic(false);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            break 'l_dispatch_1;
                        }
                        if (__av_mem[crate::ix::U((r) as usize)].hh().lh() == end_match_token) { break; }
                    }
                }
            }
            // §416
            while (((self.cur_input.loc_field == null) && (self.cur_input.index_field != v_template)) && (self.cur_input.index_field != output_text)) {
                self.end_token_list();
            }
            if self.intr_at_switch {
                if self.intr_args_on {
                    if ((self.intr_cand[crate::ix::U((self.warning_index) as usize)] != 0i32) || self.intr_all_args) {
                        if self.flashtex_intr_call_args(self.warning_index, ref_count, n, save_scanner_status, save_warning_index) {
                            break 'l_exit_f;
                        }
                    }
                }
            }
            self.begin_token_list(ref_count, macro_);
            self.cur_input.name_field = self.warning_index;
            self.cur_input.loc_field = __av_mem[crate::ix::U((r) as usize)].hh().rh();
            if (self.ckpt_arm_cs != null) {
                if (self.warning_index == self.ckpt_arm_cs) {
                    {
                        self.ckpt_arm_level = self.input_ptr;
                        self.ckpt_arm_cs = null;
                        if (self.ckpt_on_arm != 0i32) {
                            self.ckpt_request = self.ckpt_on_arm;
                        }
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
                                self.overflow(720i32, param_size);
                            }
                        }
                    }
                    {
                        let __for_end_5 = (n).wrapping_sub(1i32);
                        m = 0i32;
                        while m <= __for_end_5 {
                            { let __ix268 = (self.param_ptr).wrapping_add(m); let __v269 = self.pstack[crate::ix::U((m) as usize)]; self.param_stack[crate::ix::U((__ix268) as usize)] = __v269; }
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
        self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
        self.back_input();
        self.cur_tok = 19616i32;
        self.back_input();
        self.cur_input.index_field = inserted;
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.cur_ptr = self.get_node(index_node_size);
        { let __ix270 = self.cur_ptr; __av_mem[crate::ix::U((__ix270) as usize)].set_hh_b0(i); }
        { let __ix271 = self.cur_ptr; __av_mem[crate::ix::U((__ix271) as usize)].set_hh_b1(0i32); }
        { let __ix272 = self.cur_ptr; __av_mem[crate::ix::U((__ix272) as usize)].set_hh_rh(q); }
        {
            let __for_end_2 = 8i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __ix273 = (self.cur_ptr).wrapping_add(k); let __v274 = self.sa_null; __av_mem[crate::ix::U((__ix273) as usize)] = __v274; }
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
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
                                    if (self.cur_ptr == null) {
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
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == null) {
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
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == null) {
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
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                {
                                    if (self.cur_ptr == null) {
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
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                } else {
                                    self.cur_ptr = __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                }
                                if ((self.cur_ptr == null) && w) {
                                    break 'l_L49_f;
                                }
                                break 'l_exit_f;
                            }
                            self.new_index(t, null);
                            { let __v275 = self.cur_ptr; self.sa_root[crate::ix::U((t) as usize)] = __v275; }
                            q = self.cur_ptr;
                            i = (n / 4096i32);
                        }
                        self.new_index(i, q);
                        {
                            if (((i) % 2) != 0) {
                                { let __v276 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v276); }
                            } else {
                                { let __v277 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v277); }
                            }
                            { let __v278 = (__av_mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v278); }
                        }
                        q = self.cur_ptr;
                        i = ((n / 256i32) % 16i32);
                    }
                    self.new_index(i, q);
                    {
                        if (((i) % 2) != 0) {
                            { let __v279 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v279); }
                        } else {
                            { let __v280 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v280); }
                        }
                        { let __v281 = (__av_mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v281); }
                    }
                    q = self.cur_ptr;
                    i = ((n / 16i32) % 16i32);
                }
                self.new_index(i, q);
                {
                    if (((i) % 2) != 0) {
                        { let __v282 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v282); }
                    } else {
                        { let __v283 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v283); }
                    }
                    { let __v284 = (__av_mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v284); }
                }
                q = self.cur_ptr;
                i = (n % 16i32);
            }
            if (t == mark_val) {
                // §1820
                {
                    self.cur_ptr = self.get_node(mark_class_node_size);
                    { let __ix285 = (self.cur_ptr).wrapping_add(1i32); let __v286 = self.sa_null; __av_mem[crate::ix::U((__ix285) as usize)] = __v286; }
                    { let __ix287 = (self.cur_ptr).wrapping_add(2i32); let __v288 = self.sa_null; __av_mem[crate::ix::U((__ix287) as usize)] = __v288; }
                    { let __ix289 = (self.cur_ptr).wrapping_add(3i32); let __v290 = self.sa_null; __av_mem[crate::ix::U((__ix289) as usize)] = __v290; }
                }
            } else {
                {
                    if (t <= dimen_val) {
                        {
                            self.cur_ptr = self.get_node(word_node_size);
                            { let __ix291 = (self.cur_ptr).wrapping_add(2i32); __av_mem[crate::ix::U((__ix291) as usize)].set_int(0i32); }
                            { let __ix292 = (self.cur_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix292) as usize)].set_hh_rh(n); }
                        }
                    } else {
                        {
                            self.cur_ptr = self.get_node(pointer_node_size);
                            if (t <= mu_val) {
                                {
                                    { let __ix293 = (self.cur_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix293) as usize)].set_hh_rh(zero_glue); }
                                    { let __v294 = (__av_mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v294); }
                                }
                            } else {
                                { let __ix295 = (self.cur_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix295) as usize)].set_hh_rh(null); }
                            }
                        }
                    }
                    { let __ix296 = (self.cur_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix296) as usize)].set_hh_lh(null); }
                }
            }
            { let __ix297 = self.cur_ptr; __av_mem[crate::ix::U((__ix297) as usize)].set_hh_b0(((16i32).wrapping_mul(t)).wrapping_add(i)); }
            { let __ix298 = self.cur_ptr; __av_mem[crate::ix::U((__ix298) as usize)].set_hh_b1(level_one); }
            // §1819
            { let __ix299 = self.cur_ptr; __av_mem[crate::ix::U((__ix299) as usize)].set_hh_rh(q); }
            {
                if (((i) % 2) != 0) {
                    { let __v300 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_rh(__v300); }
                } else {
                    { let __v301 = self.cur_ptr; __av_mem[crate::ix::U((((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize)].set_hh_lh(__v301); }
                }
                { let __v302 = (__av_mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v302); }
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
        let mut save_at_switch: bool = false; // §388
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(702i32, self.expand_depth);
        }
        cv_backup = self.cur_val;
        cvl_backup = self.cur_val_level;
        radix_backup = self.radix;
        co_backup = self.cur_order;
        backup_backup = __av_mem[crate::ix::U((backup_head) as usize)].hh().rh();
        save_at_switch = self.intr_at_switch;
        'l_reswitch_b: loop {
            self.intr_at_switch = false;
            if self.intr_rec_on {
                self.flashtex_intr_expand();
            }
            if (self.cur_cmd < call) {
                // §391
                {
                    if (__av_eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() > 1i32) {
                        self.show_cur_cmd_chr();
                    }
                    match self.cur_cmd {
                        top_bot_mark => {
                            // §412
                            {
                                t = (self.cur_chr % marks_code);
                                if (self.cur_chr >= marks_code) {
                                    self.scan_register_num();
                                } else {
                                    self.cur_val = 0i32;
                                }
                                if (self.cur_val == 0i32) {
                                    self.cur_ptr = self.cur_mark[crate::ix::U((t) as usize)];
                                } else {
                                    // §1824
                                    {
                                        self.find_sa_element(mark_val, self.cur_val, false);
                                        if (self.cur_ptr != null) {
                                            if (((t) % 2) != 0) {
                                                self.cur_ptr = __av_mem[crate::ix::U((((self.cur_ptr).wrapping_add((t / 2i32))).wrapping_add(1i32)) as usize)].hh().rh();
                                            } else {
                                                self.cur_ptr = __av_mem[crate::ix::U((((self.cur_ptr).wrapping_add((t / 2i32))).wrapping_add(1i32)) as usize)].hh().lh();
                                            }
                                        }
                                    }
                                }
                                // §412
                                if (self.cur_ptr != null) {
                                    self.begin_token_list(self.cur_ptr, mark_text);
                                }
                            }
                        }
                        expand_after => {
                            // §391
                            if (self.cur_chr == 0i32) {
                                // §392
                                {
                                    self.get_token();
                                    t = self.cur_tok;
                                    self.get_token();
                                    if (self.cur_cmd > max_command) {
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
                                    if ((self.cur_cmd == if_test) && (self.cur_chr != if_case_code)) {
                                        {
                                            self.cur_chr = (self.cur_chr).wrapping_add(32i32);
                                            continue 'l_reswitch_b;
                                        }
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
                                        self.print(783i32);
                                    }
                                    self.print_esc(926i32);
                                    self.print(2039i32);
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print_char(39i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 697i32;
                                    }
                                    self.back_error();
                                }
                            }
                        }
                        no_expand => {
                            // §391
                            if (self.cur_chr == 0i32) {
                                // §393
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = normal;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    t = self.cur_tok;
                                    self.back_input();
                                    if ((t >= cs_token_flag) && (t != end_write_token)) {
                                        {
                                            p = self.get_avail();
                                            __av_mem[crate::ix::U((p) as usize)].set_hh_lh(19618i32);
                                            { let __v303 = self.cur_input.loc_field; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v303); }
                                            self.cur_input.start_field = p;
                                            self.cur_input.loc_field = p;
                                        }
                                    }
                                }
                            } else {
                                // §394
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = normal;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    if (self.cur_cs < hash_base) {
                                        self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                    } else {
                                        self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                                    }
                                    if (self.cur_cs != undefined_primitive) {
                                        {
                                            t = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                            if (t > max_command) {
                                                {
                                                    self.cur_cmd = t;
                                                    self.cur_chr = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                                    self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
                                                    self.cur_cs = 0i32;
                                                    continue 'l_reswitch_b;
                                                }
                                            } else {
                                                {
                                                    self.back_input();
                                                    p = self.get_avail();
                                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(19620i32);
                                                    { let __v304 = self.cur_input.loc_field; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v304); }
                                                    self.cur_input.loc_field = p;
                                                    self.cur_input.start_field = p;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        cs_name => {
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
                                            __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v305 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v305); }
                                            p = q;
                                        }
                                    }
                                    if (self.cur_cs != 0i32) { break; }
                                }
                                if (self.cur_cmd != end_cs_name) {
                                    // §399
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
                                            self.print(709i32);
                                        }
                                        self.print_esc(578i32);
                                        self.print(710i32);
                                        {
                                            self.help_ptr = 2i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 711i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 712i32;
                                        }
                                        self.back_error();
                                    }
                                }
                                // §398
                                self.is_in_csname = b;
                                // §400
                                j = self.first;
                                p = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                while (p != null) {
                                    {
                                        if (j >= self.max_buf_stack) {
                                            {
                                                self.max_buf_stack = (j).wrapping_add(1i32);
                                                if (self.max_buf_stack == buf_size) {
                                                    self.overflow(258i32, buf_size);
                                                }
                                            }
                                        }
                                        { let __v306 = (__av_mem[crate::ix::U((p) as usize)].hh().lh() % 256i32); self.buffer[crate::ix::U((j) as usize)] = __v306; }
                                        j = (j).wrapping_add(1i32);
                                        p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
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
                                        self.cur_cs = null_cs;
                                    } else {
                                        self.cur_cs = (single_base).wrapping_add(self.buffer[crate::ix::U((self.first) as usize)]);
                                    }
                                }
                                // §398
                                self.flush_list(r);
                                if self.intr_rec_on {
                                    self.flashtex_intr_read(self.cur_cs);
                                }
                                if (__av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0() == undefined_cs) {
                                    {
                                        self.eq_define(self.cur_cs, relax, 256i32);
                                    }
                                }
                                self.cur_tok = (self.cur_cs).wrapping_add(4095i32);
                                self.back_input();
                            }
                        }
                        convert => {
                            // §391
                            self.conv_toks();
                        }
                        the => {
                            self.ins_the_toks();
                        }
                        if_test => {
                            self.conditional();
                        }
                        fi_or_else => {
                            // §536
                            {
                                if (__av_eqtb[crate::ix::U(((29381i32) - 1) as usize)].int() > 0i32) {
                                    if (__av_eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() <= 1i32) {
                                        self.show_cur_cmd_chr();
                                    }
                                }
                                if (self.cur_chr > self.if_limit) {
                                    if (self.if_limit == if_code) {
                                        self.insert_relax();
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
                                                self.print(930i32);
                                            }
                                            self.print_cmd_chr(fi_or_else, self.cur_chr);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 931i32;
                                            }
                                            self.error();
                                        }
                                    }
                                } else {
                                    {
                                        while (self.cur_chr != fi_code) {
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
                                            self.if_line = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                            self.cur_if = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                                            self.if_limit = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                            self.cond_ptr = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                            self.ls_cond_depth = (self.ls_cond_depth).wrapping_sub(1i32);
                                            self.free_node(p, if_node_size);
                                        }
                                    }
                                }
                            }
                        }
                        input => {
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
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(703i32);
                                }
                                {
                                    self.help_ptr = 5i32;
                                    self.help_line[crate::ix::U((4i32) as usize)] = 704i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 705i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 706i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 707i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 708i32;
                                }
                                self.error();
                            }
                        }
                    }
                }
            } else {
                // §388
                if (self.cur_cmd < end_template) {
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
            __av_mem[crate::ix::U((backup_head) as usize)].set_hh_rh(backup_backup);
            self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
            if self.intr_args_on {
                self.intr_at_switch = save_at_switch;
            }
            break 'l_reswitch_b;
        }
    }

    /// Here is a recursive procedure that is \TeX's usual way to get the
    /// next token of input. It has been slightly optimized to take account of
    /// common cases.
    // §406
    #[inline(always)]
    pub fn get_x_token(&mut self) {
        // goto labels: restart, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.get_next();
                if (self.cur_cmd <= max_command) {
                    { __goto_1 = 1; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd >= call) {
                    if (self.cur_cmd < end_template) {
                        self.macro_call();
                    } else {
                        {
                            self.cur_cs = frozen_endv;
                            self.cur_cmd = endv;
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
                    self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                }
            }
            break 'l_dispatch_1;
        }
    }

    /// The `get_x_token` procedure is essentially equivalent to two consecutive
    /// procedure calls: `get_next; x_token`.
    // §407
    pub fn x_token(&mut self) {
        while (self.cur_cmd > max_command) {
            {
                self.expand();
                self.get_next();
            }
        }
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
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
            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
        }
        // §429
        if (self.cur_cmd != left_brace) {
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
                    self.print(741i32);
                }
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 742i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 743i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 744i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 745i32;
                }
                self.back_error();
                self.cur_tok = 379i32;
                self.cur_cmd = left_brace;
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
            if (self.cur_cmd != spacer) { break; }
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            p = backup_head;
            __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
            k = self.str_start[crate::ix::U((s) as usize)];
            save_cur_cs = self.cur_cs;
            while (k < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
                {
                    self.get_x_token();
                    if ((self.cur_cs == 0i32) && ((self.cur_chr == self.str_pool[crate::ix::U((k) as usize)]) || (self.cur_chr == (self.str_pool[crate::ix::U((k) as usize)]).wrapping_sub(32i32)))) {
                        {
                            {
                                q = self.get_avail();
                                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                { let __v307 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v307); }
                                p = q;
                            }
                            k = (k).wrapping_add(1i32);
                        }
                    } else {
                        if ((self.cur_cmd != spacer) || (p != backup_head)) {
                            {
                                self.back_input();
                                if (p != backup_head) {
                                    self.begin_token_list(__av_mem[crate::ix::U((backup_head) as usize)].hh().rh(), backed_up);
                                }
                                self.cur_cs = save_cur_cs;
                                scan_keyword = false;
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            self.flush_list(__av_mem[crate::ix::U((backup_head) as usize)].hh().rh());
            scan_keyword = true;
        }
        scan_keyword
    }

    /// Here is a procedure that sounds an alarm when mu and non-mu units
    /// are being switched.
    // §434
    pub fn mu_error(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(746i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 747i32;
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
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 786i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
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
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(790i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 791i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(792i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 793i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(794i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 795i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
                {
                    self.help_ptr = 2i32;
                    { let __v308 = self.max_reg_help_line; self.help_line[crate::ix::U((1i32) as usize)] = __v308; }
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
    // §1905
    pub fn scan_four_bit_int_or_18(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || ((self.cur_val > 15i32) && (self.cur_val != 18i32))) {
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
                    self.print(790i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 791i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 787i32;
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            while true {
                {
                    self.get_token();
                    if (self.cur_cmd <= max_command) {
                        break 'l_exit_f;
                    }
                    if ((self.cur_cmd >= call) && (self.cur_cmd < end_template)) {
                        if (__av_mem[crate::ix::U((__av_mem[crate::ix::U((self.cur_chr) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
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
                    if (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword) {
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
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
            {
                i = ((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b2()).wrapping_sub(0i32) % 4i32);
                if (i == lig_tag) {
                    get_tag_code = 1i32;
                } else {
                    if (i == list_tag) {
                        get_tag_code = 2i32;
                    } else {
                        if (i == ext_tag) {
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        // §432
        loop {
            self.get_x_token();
            if (self.cur_cmd != spacer) { break; }
        }
        // §604
        if (((self.cur_cmd == def_font) || (self.cur_cmd == letterspace_font)) || (self.cur_cmd == pdf_copy_font)) {
            f = __av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
        } else {
            if (self.cur_cmd == set_font) {
                f = self.cur_chr;
            } else {
                if (self.cur_cmd == def_family) {
                    {
                        m = self.cur_chr;
                        self.scan_four_bit_int();
                        f = __av_eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
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
                            self.print(978i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 979i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 980i32;
                        }
                        self.back_error();
                        f = null_font;
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
                if (((writing && (n <= space_shrink_code)) && (n >= space_code)) && (self.font_glue[crate::ix::U((f) as usize)] != null)) {
                    {
                        self.delete_glue_ref(self.font_glue[crate::ix::U((f) as usize)]);
                        self.font_glue[crate::ix::U((f) as usize)] = null;
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
                                { let __ix309 = self.fmem_ptr; self.font_info[crate::ix::U((__ix309) as usize)].set_int(0i32); }
                                self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                                { let __v310 = (self.font_params[crate::ix::U((f) as usize)]).wrapping_add(1i32); self.font_params[crate::ix::U((f) as usize)] = __v310; }
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
                    self.dg_mark();
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(958i32);
                }
                self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].rh());
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                m = self.cur_chr;
                if self.intr_rec_on {
                    self.flashtex_intr_internal();
                }
                match self.cur_cmd {
                    def_code => {
                        // §440
                        {
                            self.scan_char_num();
                            if self.intr_rec_on {
                                self.flashtex_intr_read((m).wrapping_add(self.cur_val));
                            }
                            if (m == math_code_base) {
                                {
                                    self.cur_val = (__av_eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh()).wrapping_sub(0i32);
                                    self.cur_val_level = int_val;
                                }
                            } else {
                                if (m < math_code_base) {
                                    {
                                        self.cur_val = __av_eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                        self.cur_val_level = int_val;
                                    }
                                } else {
                                    {
                                        self.cur_val = __av_eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                        self.cur_val_level = int_val;
                                    }
                                }
                            }
                        }
                    }
                    toks_register | assign_toks | def_family | set_font | def_font | letterspace_font | pdf_copy_font => {
                        // §441
                        if (level != tok_val) {
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
                                    self.print(748i32);
                                }
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 749i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 750i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 751i32;
                                }
                                self.back_error();
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = dimen_val;
                                }
                            }
                        } else {
                            if (self.cur_cmd <= assign_toks) {
                                {
                                    if (self.cur_cmd < assign_toks) {
                                        if (m == mem_bot) {
                                            {
                                                self.scan_register_num();
                                                if (self.cur_val < 256i32) {
                                                    self.cur_val = __av_eqtb[crate::ix::U((((toks_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                                } else {
                                                    {
                                                        self.find_sa_element(tok_val, self.cur_val, false);
                                                        if (self.cur_ptr == null) {
                                                            self.cur_val = null;
                                                        } else {
                                                            self.cur_val = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            self.cur_val = __av_mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                        }
                                    } else {
                                        self.cur_val = __av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                                    }
                                    self.cur_val_level = tok_val;
                                }
                            } else {
                                {
                                    self.back_input();
                                    self.scan_font_ident();
                                    {
                                        self.cur_val = (font_id_base).wrapping_add(self.cur_val);
                                        self.cur_val_level = ident_val;
                                    }
                                }
                            }
                        }
                    }
                    assign_int => {
                        // §439
                        {
                            self.cur_val = __av_eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = int_val;
                        }
                    }
                    assign_dimen => {
                        {
                            self.cur_val = __av_eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = dimen_val;
                        }
                    }
                    assign_glue => {
                        {
                            self.cur_val = __av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = glue_val;
                        }
                    }
                    assign_mu_glue => {
                        {
                            self.cur_val = __av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = mu_val;
                        }
                    }
                    set_aux => {
                        // §444
                        if ((self.cur_list.mode_field).wrapping_abs() != m) {
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
                                    self.print(778i32);
                                }
                                self.print_cmd_chr(set_aux, m);
                                {
                                    self.help_ptr = 4i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 779i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 780i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 781i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 782i32;
                                }
                                self.error();
                                if (level != tok_val) {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = dimen_val;
                                    }
                                } else {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = int_val;
                                    }
                                }
                            }
                        } else {
                            if (m == vmode) {
                                {
                                    self.cur_val = self.cur_list.aux_field.int();
                                    self.cur_val_level = dimen_val;
                                }
                            } else {
                                {
                                    self.cur_val = self.cur_list.aux_field.hh().lh();
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                    set_prev_graf => {
                        // §448
                        if (self.cur_list.mode_field == 0i32) {
                            {
                                self.cur_val = 0i32;
                                self.cur_val_level = int_val;
                            }
                        } else {
                            {
                                { let __ix311 = self.nest_ptr; let __v312 = self.cur_list; self.nest[crate::ix::U((__ix311) as usize)] = __v312; }
                                p = self.nest_ptr;
                                while ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() != vmode) {
                                    p = (p).wrapping_sub(1i32);
                                }
                                {
                                    self.cur_val = self.nest[crate::ix::U((p) as usize)].pg_field;
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                    set_page_int => {
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
                            self.cur_val_level = int_val;
                        }
                    }
                    set_page_dimen => {
                        // §447
                        {
                            if ((self.page_contents == empty) && (!self.output_active)) {
                                if (m == 0i32) {
                                    self.cur_val = max_dimen;
                                } else {
                                    self.cur_val = 0i32;
                                }
                            } else {
                                self.cur_val = self.page_so_far[crate::ix::U((m) as usize)];
                            }
                            self.cur_val_level = dimen_val;
                        }
                    }
                    set_shape => {
                        // §449
                        {
                            if (m > par_shape_loc) {
                                // §1866
                                {
                                    self.scan_int();
                                    if ((__av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh() == null) || (self.cur_val < 0i32)) {
                                        self.cur_val = 0i32;
                                    } else {
                                        {
                                            if (self.cur_val > __av_mem[crate::ix::U(((__av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()) {
                                                self.cur_val = __av_mem[crate::ix::U(((__av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int();
                                            }
                                            self.cur_val = __av_mem[crate::ix::U((((__av_eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(self.cur_val)).wrapping_add(1i32)) as usize)].int();
                                        }
                                    }
                                }
                            } else {
                                // §449
                                if (__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) {
                                    self.cur_val = 0i32;
                                } else {
                                    self.cur_val = __av_mem[crate::ix::U((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                }
                            }
                            self.cur_val_level = int_val;
                        }
                    }
                    set_box_dimen => {
                        // §446
                        {
                            self.scan_register_num();
                            if (self.cur_val < 256i32) {
                                q = __av_eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(box_val, self.cur_val, false);
                                    if (self.cur_ptr == null) {
                                        q = null;
                                    } else {
                                        q = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                            if (q == null) {
                                self.cur_val = 0i32;
                            } else {
                                self.cur_val = __av_mem[crate::ix::U(((q).wrapping_add(m)) as usize)].int();
                            }
                            self.cur_val_level = dimen_val;
                        }
                    }
                    char_given | math_given => {
                        // §439
                        {
                            self.cur_val = self.cur_chr;
                            self.cur_val_level = int_val;
                        }
                    }
                    assign_font_dimen => {
                        // §451
                        {
                            self.find_font_dimen(false);
                            { let __ix313 = self.fmem_ptr; self.font_info[crate::ix::U((__ix313) as usize)].set_int(0i32); }
                            {
                                self.cur_val = self.font_info[crate::ix::U((self.cur_val) as usize)].int();
                                self.cur_val_level = dimen_val;
                            }
                        }
                    }
                    assign_font_int => {
                        // §452
                        {
                            self.scan_font_ident();
                            if (m == 0i32) {
                                {
                                    self.cur_val = self.hyphen_char[crate::ix::U((self.cur_val) as usize)];
                                    self.cur_val_level = int_val;
                                }
                            } else {
                                if (m == 1i32) {
                                    {
                                        self.cur_val = self.skew_char[crate::ix::U((self.cur_val) as usize)];
                                        self.cur_val_level = int_val;
                                    }
                                } else {
                                    if (m == no_lig_code) {
                                        {
                                            self.cur_val = self.test_no_ligatures(self.cur_val);
                                            self.cur_val_level = int_val;
                                        }
                                    } else {
                                        {
                                            n = self.cur_val;
                                            self.scan_char_num();
                                            k = self.cur_val;
                                            match m {
                                                lp_code_base => {
                                                    {
                                                        self.cur_val = self.get_lp_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                rp_code_base => {
                                                    {
                                                        self.cur_val = self.get_rp_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                ef_code_base => {
                                                    {
                                                        self.cur_val = self.get_ef_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                tag_code => {
                                                    {
                                                        self.cur_val = self.get_tag_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                kn_bs_code_base => {
                                                    {
                                                        self.cur_val = self.get_kn_bs_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                st_bs_code_base => {
                                                    {
                                                        self.cur_val = self.get_st_bs_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                sh_bs_code_base => {
                                                    {
                                                        self.cur_val = self.get_sh_bs_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                kn_bc_code_base => {
                                                    {
                                                        self.cur_val = self.get_kn_bc_code(n, k);
                                                        self.cur_val_level = int_val;
                                                    }
                                                }
                                                kn_ac_code_base => {
                                                    {
                                                        self.cur_val = self.get_kn_ac_code(n, k);
                                                        self.cur_val_level = int_val;
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
                    register => {
                        // §453
                        {
                            if ((m < mem_bot) || (m > lo_mem_stat_max)) {
                                {
                                    self.cur_val_level = (__av_mem[crate::ix::U((m) as usize)].hh().b0() / 16i32);
                                    if (self.cur_val_level < glue_val) {
                                        self.cur_val = __av_mem[crate::ix::U(((m).wrapping_add(2i32)) as usize)].int();
                                    } else {
                                        self.cur_val = __av_mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            } else {
                                {
                                    self.scan_register_num();
                                    self.cur_val_level = (m).wrapping_sub(0i32);
                                    if (self.cur_val > 255i32) {
                                        {
                                            self.find_sa_element(self.cur_val_level, self.cur_val, false);
                                            if (self.cur_ptr == null) {
                                                if (self.cur_val_level < glue_val) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    self.cur_val = zero_glue;
                                                }
                                            } else {
                                                if (self.cur_val_level < glue_val) {
                                                    self.cur_val = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].int();
                                                } else {
                                                    self.cur_val = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    } else {
                                        match self.cur_val_level {
                                            int_val => {
                                                self.cur_val = __av_eqtb[crate::ix::U((((count_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            dimen_val => {
                                                self.cur_val = __av_eqtb[crate::ix::U((((scaled_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            glue_val => {
                                                self.cur_val = __av_eqtb[crate::ix::U((((skip_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            mu_val => {
                                                self.cur_val = __av_eqtb[crate::ix::U((((mu_skip_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    last_item => {
                        // §450
                        if (m >= input_line_no_code) {
                            if (m >= eTeX_glue) {
                                // §1780
                                {
                                    if (m < eTeX_mu) {
                                        {
                                            match m {
                                                mu_to_glue_code => {
                                                    // §1807
                                                    self.scan_mu_glue();
                                                }
                                                _ => {}
                                            }
                                            // §1780
                                            self.cur_val_level = glue_val;
                                        }
                                    } else {
                                        if (m < eTeX_expr) {
                                            {
                                                match m {
                                                    glue_to_mu_code => {
                                                        // §1808
                                                        self.scan_normal_glue();
                                                    }
                                                    _ => {}
                                                }
                                                // §1780
                                                self.cur_val_level = mu_val;
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
                                            if (self.cur_val_level == glue_val) {
                                                {
                                                    m = self.cur_val;
                                                    self.cur_val = __av_mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].int();
                                                    self.delete_glue_ref(m);
                                                }
                                            } else {
                                                if (self.cur_val_level == mu_val) {
                                                    self.mu_error();
                                                }
                                            }
                                            self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                                        }
                                    }
                                    if negative {
                                        if (self.cur_val_level >= glue_val) {
                                            {
                                                m = self.cur_val;
                                                self.cur_val = self.new_spec(m);
                                                self.delete_glue_ref(m);
                                                // §457
                                                {
                                                    { let __ix314 = (self.cur_val).wrapping_add(1i32); let __v315 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix314) as usize)].set_int(__v315); }
                                                    { let __ix316 = (self.cur_val).wrapping_add(2i32); let __v317 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix316) as usize)].set_int(__v317); }
                                                    { let __ix318 = (self.cur_val).wrapping_add(3i32); let __v319 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix318) as usize)].set_int(__v319); }
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
                                if (m >= eTeX_dim) {
                                    {
                                        match m {
                                            font_char_wd_code | font_char_ht_code | font_char_dp_code | font_char_ic_code => {
                                                // §1671
                                                {
                                                    self.scan_font_ident();
                                                    q = self.cur_val;
                                                    self.scan_char_num();
                                                    if ((self.font_bc[crate::ix::U((q) as usize)] <= self.cur_val) && (self.font_ec[crate::ix::U((q) as usize)] >= self.cur_val)) {
                                                        {
                                                            i = self.font_info[crate::ix::U((((self.char_base[crate::ix::U((q) as usize)]).wrapping_add(self.cur_val)).wrapping_add(0i32)) as usize)].qqqq();
                                                            match m {
                                                                font_char_wd_code => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((q) as usize)]).wrapping_add(i.b0())) as usize)].int();
                                                                }
                                                                font_char_ht_code => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((q) as usize)]).wrapping_add(((i.b1()).wrapping_sub(0i32) / 16i32))) as usize)].int();
                                                                }
                                                                font_char_dp_code => {
                                                                    self.cur_val = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((q) as usize)]).wrapping_add(((i.b1()).wrapping_sub(0i32) % 16i32))) as usize)].int();
                                                                }
                                                                font_char_ic_code => {
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
                                            par_shape_length_code | par_shape_indent_code | par_shape_dimen_code => {
                                                // §1674
                                                {
                                                    q = (self.cur_chr).wrapping_sub(32i32);
                                                    self.scan_int();
                                                    if ((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) || (self.cur_val <= 0i32)) {
                                                        self.cur_val = 0i32;
                                                    } else {
                                                        {
                                                            if (q == 2i32) {
                                                                {
                                                                    q = (self.cur_val % 2i32);
                                                                    self.cur_val = ((self.cur_val).wrapping_add(q) / 2i32);
                                                                }
                                                            }
                                                            if (self.cur_val > __av_mem[crate::ix::U((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh()) {
                                                                self.cur_val = __av_mem[crate::ix::U((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                                            }
                                                            self.cur_val = __av_mem[crate::ix::U((((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(self.cur_val))).wrapping_sub(q)) as usize)].int();
                                                        }
                                                    }
                                                    self.cur_val_level = dimen_val;
                                                }
                                            }
                                            glue_stretch_code | glue_shrink_code => {
                                                // §1804
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == glue_stretch_code) {
                                                        self.cur_val = __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                                                    } else {
                                                        self.cur_val = __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §450
                                        self.cur_val_level = dimen_val;
                                    }
                                } else {
                                    {
                                        match m {
                                            input_line_no_code => {
                                                {
                                                    self.cur_val = self.line;
                                                    self.ls_line_read();
                                                }
                                            }
                                            badness_code => {
                                                self.cur_val = self.last_badness;
                                            }
                                            pdftex_version_code => {
                                                self.cur_val = pdftex_version;
                                            }
                                            pdf_last_obj_code => {
                                                self.cur_val = self.pdf_last_obj;
                                            }
                                            pdf_last_xform_code => {
                                                self.cur_val = self.pdf_last_xform;
                                            }
                                            pdf_last_ximage_code => {
                                                self.cur_val = self.pdf_last_ximage;
                                            }
                                            pdf_last_ximage_pages_code => {
                                                self.cur_val = self.pdf_last_ximage_pages;
                                            }
                                            pdf_last_annot_code => {
                                                self.cur_val = self.pdf_last_annot;
                                            }
                                            pdf_last_x_pos_code => {
                                                self.cur_val = self.pdf_last_x_pos;
                                            }
                                            pdf_last_y_pos_code => {
                                                self.cur_val = self.pdf_last_y_pos;
                                            }
                                            pdf_retval_code => {
                                                self.cur_val = self.pdf_retval;
                                            }
                                            pdf_last_ximage_colordepth_code => {
                                                self.cur_val = self.pdf_last_ximage_colordepth;
                                            }
                                            elapsed_time_code => {
                                                self.cur_val = self.get_microinterval();
                                            }
                                            random_seed_code => {
                                                self.cur_val = self.random_seed;
                                            }
                                            pdf_shell_escape_code => {
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
                                            pdf_last_link_code => {
                                                self.cur_val = self.pdf_last_link;
                                            }
                                            eTeX_version_code => {
                                                // §1651
                                                self.cur_val = eTeX_version;
                                            }
                                            current_group_level_code => {
                                                // §1665
                                                self.cur_val = (self.cur_level).wrapping_sub(1i32);
                                            }
                                            current_group_type_code => {
                                                self.cur_val = self.cur_group;
                                            }
                                            current_if_level_code => {
                                                // §1668
                                                {
                                                    q = self.cond_ptr;
                                                    self.cur_val = 0i32;
                                                    while (q != null) {
                                                        {
                                                            self.cur_val = (self.cur_val).wrapping_add(1i32);
                                                            q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                            current_if_type_code => {
                                                if (self.cond_ptr == null) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    if (self.cur_if < unless_code) {
                                                        self.cur_val = (self.cur_if).wrapping_add(1i32);
                                                    } else {
                                                        self.cur_val = ((self.cur_if).wrapping_sub(31i32)).wrapping_neg();
                                                    }
                                                }
                                            }
                                            current_if_branch_code => {
                                                if ((self.if_limit == or_code) || (self.if_limit == else_code)) {
                                                    self.cur_val = 1i32;
                                                } else {
                                                    if (self.if_limit == fi_code) {
                                                        self.cur_val = (1i32).wrapping_neg();
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            glue_stretch_order_code | glue_shrink_order_code => {
                                                // §1803
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == glue_stretch_order_code) {
                                                        self.cur_val = __av_mem[crate::ix::U((q) as usize)].hh().b0();
                                                    } else {
                                                        self.cur_val = __av_mem[crate::ix::U((q) as usize)].hh().b1();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §450
                                        self.cur_val_level = int_val;
                                    }
                                }
                            }
                        } else {
                            {
                                if (self.cur_chr == glue_val) {
                                    self.cur_val = zero_glue;
                                } else {
                                    self.cur_val = 0i32;
                                }
                                tx = self.cur_list.tail_field;
                                if (!(tx >= self.hi_mem_min)) {
                                    if ((__av_mem[crate::ix::U((tx) as usize)].hh().b0() == math_node) && (__av_mem[crate::ix::U((tx) as usize)].hh().b1() == end_M_code)) {
                                        {
                                            r = self.cur_list.head_field;
                                            loop {
                                                q = r;
                                                r = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                if (r == tx) { break; }
                                            }
                                            tx = q;
                                        }
                                    }
                                }
                                if (self.cur_chr == last_node_type_code) {
                                    {
                                        self.cur_val_level = int_val;
                                        if ((tx == self.cur_list.head_field) || (self.cur_list.mode_field == 0i32)) {
                                            self.cur_val = (1i32).wrapping_neg();
                                        }
                                    }
                                } else {
                                    self.cur_val_level = self.cur_chr;
                                }
                                if ((!(tx >= self.hi_mem_min)) && (self.cur_list.mode_field != 0i32)) {
                                    match self.cur_chr {
                                        int_val => {
                                            if (__av_mem[crate::ix::U((tx) as usize)].hh().b0() == penalty_node) {
                                                self.cur_val = __av_mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        dimen_val => {
                                            if (__av_mem[crate::ix::U((tx) as usize)].hh().b0() == kern_node) {
                                                self.cur_val = __av_mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        glue_val => {
                                            if (__av_mem[crate::ix::U((tx) as usize)].hh().b0() == glue_node) {
                                                {
                                                    self.cur_val = __av_mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].hh().lh();
                                                    if (__av_mem[crate::ix::U((tx) as usize)].hh().b1() == mu_glue) {
                                                        self.cur_val_level = mu_val;
                                                    }
                                                }
                                            }
                                        }
                                        last_node_type_code => {
                                            if (__av_mem[crate::ix::U((tx) as usize)].hh().b0() <= unset_node) {
                                                self.cur_val = (__av_mem[crate::ix::U((tx) as usize)].hh().b0()).wrapping_add(1i32);
                                            } else {
                                                self.cur_val = 15i32;
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    if ((self.cur_list.mode_field == vmode) && (tx == self.cur_list.head_field)) {
                                        match self.cur_chr {
                                            int_val => {
                                                self.cur_val = self.last_penalty;
                                            }
                                            dimen_val => {
                                                self.cur_val = self.last_kern;
                                            }
                                            glue_val => {
                                                if (self.last_glue != max_halfword) {
                                                    self.cur_val = self.last_glue;
                                                }
                                            }
                                            last_node_type_code => {
                                                self.cur_val = self.last_node_type;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ignore_spaces => {
                        // §439
                        if (self.cur_chr == 1i32) {
                            // §395
                            {
                                self.get_token();
                                if (self.cur_cs < hash_base) {
                                    self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                } else {
                                    self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                                }
                                if (self.cur_cs != undefined_primitive) {
                                    {
                                        self.cur_cmd = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                        self.cur_chr = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                        self.cur_cs = (prim_eqtb_base).wrapping_add(self.cur_cs);
                                        self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                                    }
                                } else {
                                    {
                                        self.cur_cmd = relax;
                                        self.cur_chr = 0i32;
                                        self.cur_tok = 19616i32;
                                        self.cur_cs = frozen_relax;
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
                                self.dg_mark();
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(783i32);
                            }
                            self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                            self.print(784i32);
                            self.print_esc(613i32);
                            {
                                self.help_ptr = 1i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 782i32;
                            }
                            self.error();
                            if (level != tok_val) {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = dimen_val;
                                }
                            } else {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                }
                // §439
                while (self.cur_val_level > level) {
                    // §455
                    {
                        if (self.cur_val_level == glue_val) {
                            self.cur_val = __av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                        } else {
                            if (self.cur_val_level == mu_val) {
                                self.mu_error();
                            }
                        }
                        self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                    }
                }
                // §456
                if negative {
                    if (self.cur_val_level >= glue_val) {
                        {
                            self.cur_val = self.new_spec(self.cur_val);
                            // §457
                            {
                                { let __ix320 = (self.cur_val).wrapping_add(1i32); let __v321 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix320) as usize)].set_int(__v321); }
                                { let __ix322 = (self.cur_val).wrapping_add(2i32); let __v323 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix322) as usize)].set_int(__v323); }
                                { let __ix324 = (self.cur_val).wrapping_add(3i32); let __v325 = (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix324) as usize)].set_int(__v325); }
                            }
                        }
                    } else {
                        // §456
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                } else {
                    if ((self.cur_val_level >= glue_val) && (self.cur_val_level <= mu_val)) {
                        { let __ix326 = self.cur_val; let __v327 = (__av_mem[crate::ix::U((self.cur_val) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix326) as usize)].set_hh_rh(__v327); }
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.radix = 0i32;
        OK_so_far = true;
        // §467
        negative = false;
        loop {
            // §432
            loop {
                self.get_x_token();
                if (self.cur_cmd != spacer) { break; }
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
            if (self.cur_tok == alpha_token) {
                // §468
                {
                    self.get_token();
                    if (self.cur_tok < cs_token_flag) {
                        {
                            self.cur_val = self.cur_chr;
                            if (self.cur_cmd <= right_brace) {
                                if (self.cur_cmd == right_brace) {
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
                                self.dg_mark();
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(796i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 797i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 798i32;
                            }
                            self.cur_val = 48i32;
                            self.back_error();
                        }
                    } else {
                        // §469
                        {
                            self.get_x_token();
                            if (self.cur_cmd != spacer) {
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
                        if (self.cur_cs < hash_base) {
                            self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                        } else {
                            self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                        }
                        if (self.cur_cs != undefined_primitive) {
                            {
                                self.cur_cmd = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                self.cur_chr = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                self.cur_cs = (prim_eqtb_base).wrapping_add(self.cur_cs);
                                self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                            }
                        } else {
                            {
                                self.cur_cmd = relax;
                                self.cur_chr = 0i32;
                                self.cur_tok = 19616i32;
                                self.cur_cs = frozen_relax;
                            }
                        }
                        continue 'l_restart_b;
                    }
                } else {
                    // §466
                    if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                        self.scan_something_internal(int_val, false);
                    } else {
                        // §470
                        {
                            'l_done_f: {
                                self.radix = 10i32;
                                m = 214748364i32;
                                if (self.cur_tok == octal_token) {
                                    {
                                        self.radix = 8i32;
                                        m = 268435456i32;
                                        self.get_x_token();
                                    }
                                } else {
                                    if (self.cur_tok == hex_token) {
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
                                        if (((self.cur_tok < (zero_token).wrapping_add(self.radix)) && (self.cur_tok >= zero_token)) && (self.cur_tok <= 3129i32)) {
                                            d = (self.cur_tok).wrapping_sub(3120i32);
                                        } else {
                                            if (self.radix == 16i32) {
                                                if ((self.cur_tok <= 2886i32) && (self.cur_tok >= A_token)) {
                                                    d = (self.cur_tok).wrapping_sub(2871i32);
                                                } else {
                                                    if ((self.cur_tok <= 3142i32) && (self.cur_tok >= other_A_token)) {
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
                                                            self.dg_mark();
                                                            if (self.interaction == error_stop_mode) {
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
                                                        self.error();
                                                        self.cur_val = infinity;
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
                                        self.dg_mark();
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(748i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 749i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 750i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 751i32;
                                    }
                                    self.back_error();
                                }
                            } else {
                                // §470
                                if (self.cur_cmd != spacer) {
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
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
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
                                self.cur_order = normal;
                                negative = false;
                                if (!shortcut) {
                                    {
                                        // §467
                                        negative = false;
                                        loop {
                                            // §432
                                            loop {
                                                self.get_x_token();
                                                if (self.cur_cmd != spacer) { break; }
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
                                        if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                                            // §475
                                            if mu {
                                                {
                                                    self.scan_something_internal(mu_val, false);
                                                    if (self.cur_val_level != int_val) {
                                                        {
                                                            // §477
                                                            if (self.cur_val_level >= glue_val) {
                                                                {
                                                                    v = __av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                                    self.delete_glue_ref(self.cur_val);
                                                                    self.cur_val = v;
                                                                }
                                                            }
                                                            // §475
                                                            if (self.cur_val_level != mu_val) {
                                                                self.mu_error();
                                                            }
                                                            break 'l_L89_f;
                                                        }
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.scan_something_internal(dimen_val, false);
                                                    if (self.cur_val_level == dimen_val) {
                                                        break 'l_L89_f;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §474
                                            {
                                                self.back_input();
                                                if (self.cur_tok == continental_point_token) {
                                                    self.cur_tok = point_token;
                                                }
                                                if (self.cur_tok != point_token) {
                                                    self.scan_int();
                                                } else {
                                                    {
                                                        self.radix = 10i32;
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                                if (self.cur_tok == continental_point_token) {
                                                    self.cur_tok = point_token;
                                                }
                                                if ((self.radix == 10i32) && (self.cur_tok == point_token)) {
                                                    // §478
                                                    {
                                                        'l_done1_f: {
                                                            k = 0i32;
                                                            p = null;
                                                            self.get_token();
                                                            while true {
                                                                {
                                                                    self.get_x_token();
                                                                    if ((self.cur_tok > 3129i32) || (self.cur_tok < zero_token)) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    if (k < 17i32) {
                                                                        {
                                                                            q = self.get_avail();
                                                                            __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                                            { let __v328 = (self.cur_tok).wrapping_sub(3120i32); __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v328); }
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
                                                                    { let __v329 = __av_mem[crate::ix::U((p) as usize)].hh().lh(); self.dig[crate::ix::U(((kk).wrapping_sub(1i32)) as usize)] = __v329; }
                                                                    q = p;
                                                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                                    {
                                                                        { let __v330 = self.avail; __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v330); }
                                                                        self.avail = q;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                                kk = kk.wrapping_sub(1);
                                                            }
                                                        }
                                                        f = self.round_decimals(k);
                                                        if (self.cur_cmd != spacer) {
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
                                    if self.scan_keyword(316i32) {
                                        {
                                            self.cur_order = fil;
                                            while self.scan_keyword(108i32) {
                                                {
                                                    if (self.cur_order == filll) {
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
                                                                self.print(803i32);
                                                            }
                                                            self.print(804i32);
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[crate::ix::U((0i32) as usize)] = 805i32;
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
                                    if (self.cur_cmd != spacer) { break; }
                                }
                                // §481
                                if ((self.cur_cmd < min_internal) || (self.cur_cmd > max_internal)) {
                                    self.back_input();
                                } else {
                                    {
                                        if mu {
                                            {
                                                self.scan_something_internal(mu_val, false);
                                                // §477
                                                if (self.cur_val_level >= glue_val) {
                                                    {
                                                        v = __av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                        self.delete_glue_ref(self.cur_val);
                                                        self.cur_val = v;
                                                    }
                                                }
                                                // §481
                                                if (self.cur_val_level != mu_val) {
                                                    self.mu_error();
                                                }
                                            }
                                        } else {
                                            self.scan_something_internal(dimen_val, false);
                                        }
                                        v = self.cur_val;
                                        break 'l_found_f;
                                    }
                                }
                                if mu {
                                    break 'l_not_found_f;
                                }
                                if self.scan_keyword(806i32) {
                                    v = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                } else {
                                    if self.scan_keyword(807i32) {
                                        v = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                    } else {
                                        if self.scan_keyword(808i32) {
                                            v = __av_eqtb[crate::ix::U(((29936i32) - 1) as usize)].int();
                                        } else {
                                            break 'l_not_found_f;
                                        }
                                    }
                                }
                                // §469
                                {
                                    self.get_x_token();
                                    if (self.cur_cmd != spacer) {
                                        self.back_input();
                                    }
                                }
                            }
                            // §481
                            self.cur_val = { let __a331_0 = save_cur_val; let __a331_1 = v; let __a331_2 = self.xn_over_d(v, f, 65536i32); let __a331_3 = 1073741823i32; self.mult_and_add(__a331_0, __a331_1, __a331_2, __a331_3) };
                            break 'l_L89_f;
                        }
                        // §479
                        if mu {
                            // §482
                            if self.scan_keyword(344i32) {
                                break 'l_L88_f;
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
                                        self.print(803i32);
                                    }
                                    self.print(809i32);
                                    {
                                        self.help_ptr = 4i32;
                                        self.help_line[crate::ix::U((3i32) as usize)] = 810i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 811i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 812i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 813i32;
                                    }
                                    self.error();
                                    break 'l_L88_f;
                                }
                            }
                        }
                        // §479
                        if self.scan_keyword(802i32) {
                            // §483
                            {
                                self.prepare_mag();
                                if (__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() != 1000i32) {
                                    {
                                        self.cur_val = self.xn_over_d(self.cur_val, 1000i32, __av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                                        f = (((1000i32).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / __av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                                        self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                                        f = (f % 65536i32);
                                    }
                                }
                            }
                        }
                        // §479
                        if self.scan_keyword(312i32) {
                            break 'l_L88_f;
                        }
                        // §484
                        if self.scan_keyword(814i32) {
                            {
                                num = 7227i32;
                                denom = 100i32;
                            }
                        } else {
                            if self.scan_keyword(815i32) {
                                {
                                    num = 12i32;
                                    denom = 1i32;
                                }
                            } else {
                                if self.scan_keyword(816i32) {
                                    {
                                        num = 7227i32;
                                        denom = 254i32;
                                    }
                                } else {
                                    if self.scan_keyword(817i32) {
                                        {
                                            num = 7227i32;
                                            denom = 2540i32;
                                        }
                                    } else {
                                        if self.scan_keyword(818i32) {
                                            {
                                                num = 7227i32;
                                                denom = 7200i32;
                                            }
                                        } else {
                                            if self.scan_keyword(819i32) {
                                                {
                                                    num = 1238i32;
                                                    denom = 1157i32;
                                                }
                                            } else {
                                                if self.scan_keyword(820i32) {
                                                    {
                                                        num = 14856i32;
                                                        denom = 1157i32;
                                                    }
                                                } else {
                                                    if self.scan_keyword(821i32) {
                                                        {
                                                            num = 685i32;
                                                            denom = 642i32;
                                                        }
                                                    } else {
                                                        if self.scan_keyword(822i32) {
                                                            {
                                                                num = 1370i32;
                                                                denom = 107i32;
                                                            }
                                                        } else {
                                                            if self.scan_keyword(823i32) {
                                                                break 'l_done_f;
                                                            } else {
                                                                // §485
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
                                                                        self.print(803i32);
                                                                    }
                                                                    self.print(824i32);
                                                                    {
                                                                        self.help_ptr = 6i32;
                                                                        self.help_line[crate::ix::U((5i32) as usize)] = 825i32;
                                                                        self.help_line[crate::ix::U((4i32) as usize)] = 826i32;
                                                                        self.help_line[crate::ix::U((3i32) as usize)] = 827i32;
                                                                        self.help_line[crate::ix::U((2i32) as usize)] = 811i32;
                                                                        self.help_line[crate::ix::U((1i32) as usize)] = 812i32;
                                                                        self.help_line[crate::ix::U((0i32) as usize)] = 813i32;
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
                    self.cur_val = ((self.cur_val).wrapping_mul(unity)).wrapping_add(f);
                }
            }
            // §469
            {
                self.get_x_token();
                if (self.cur_cmd != spacer) {
                    self.back_input();
                }
            }
        }
        // §474
        if (self.arith_error || ((self.cur_val).wrapping_abs() >= 1073741824i32)) {
            // §486
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
                    self.print(828i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 829i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 830i32;
                }
                self.error();
                self.cur_val = max_dimen;
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            mu = (level == mu_val);
            // §467
            negative = false;
            loop {
                // §432
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) { break; }
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
            if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                {
                    self.scan_something_internal(level, negative);
                    if (self.cur_val_level >= glue_val) {
                        {
                            if (self.cur_val_level != level) {
                                self.mu_error();
                            }
                            break 'l_exit_f;
                        }
                    }
                    if (self.cur_val_level == int_val) {
                        self.scan_dimen(mu, false, true);
                    } else {
                        if (level == mu_val) {
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
            q = self.new_spec(zero_glue);
            { let __v332 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v332); }
            if self.scan_keyword(831i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v333 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v333); }
                    { let __v334 = self.cur_order; __av_mem[crate::ix::U((q) as usize)].set_hh_b0(__v334); }
                }
            }
            if self.scan_keyword(832i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v335 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v335); }
                    { let __v336 = self.cur_order; __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v336); }
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        l = self.cur_val_level;
        a = self.arith_error;
        b = false;
        p = null;
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(702i32, self.expand_depth);
        }
        'l_restart_b: loop {
            // §1783
            r = expr_none;
            e = 0i32;
            s = expr_none;
            t = 0i32;
            n = 0i32;
            'l_continue_b: loop {
                if (s == expr_none) {
                    o = l;
                } else {
                    o = int_val;
                }
                // §432
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) { break; }
                }
                // §1785
                if (self.cur_tok == 3112i32) {
                    // §1788
                    {
                        q = self.get_node(expr_node_size);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_b0(l);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_b1(((4i32).wrapping_mul(s)).wrapping_add(r));
                        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(e);
                        __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(t);
                        __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(n);
                        p = q;
                        l = o;
                        continue 'l_restart_b;
                    }
                }
                // §1785
                self.back_input();
                if (o == int_val) {
                    self.scan_int();
                } else {
                    if (o == dimen_val) {
                        self.scan_dimen(false, false, false);
                    } else {
                        if (o == glue_val) {
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
                        if (self.cur_cmd != spacer) { break; }
                    }
                    // §1784
                    if (self.cur_tok == 3115i32) {
                        o = expr_add;
                    } else {
                        if (self.cur_tok == 3117i32) {
                            o = expr_sub;
                        } else {
                            if (self.cur_tok == 3114i32) {
                                o = expr_mult;
                            } else {
                                if (self.cur_tok == 3119i32) {
                                    o = expr_div;
                                } else {
                                    {
                                        o = expr_none;
                                        if (p == null) {
                                            {
                                                if (self.cur_cmd != relax) {
                                                    self.back_input();
                                                }
                                            }
                                        } else {
                                            if (self.cur_tok != 3113i32) {
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
                                                        self.print(2049i32);
                                                    }
                                                    {
                                                        self.help_ptr = 1i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 2050i32;
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
                    if ((l == int_val) || (s > expr_sub)) {
                        {
                            if ((f > infinity) || (f < (2147483647i32).wrapping_neg())) {
                                {
                                    self.arith_error = true;
                                    f = 0i32;
                                }
                            }
                        }
                    } else {
                        if (l == dimen_val) {
                            {
                                if ((f).wrapping_abs() > max_dimen) {
                                    {
                                        self.arith_error = true;
                                        f = 0i32;
                                    }
                                }
                            }
                        } else {
                            {
                                if ((((__av_mem[crate::ix::U(((f).wrapping_add(1i32)) as usize)].int()).wrapping_abs() > max_dimen) || ((__av_mem[crate::ix::U(((f).wrapping_add(2i32)) as usize)].int()).wrapping_abs() > max_dimen)) || ((__av_mem[crate::ix::U(((f).wrapping_add(3i32)) as usize)].int()).wrapping_abs() > max_dimen)) {
                                    {
                                        self.arith_error = true;
                                        self.delete_glue_ref(f);
                                        f = self.new_spec(zero_glue);
                                    }
                                }
                            }
                        }
                    }
                    // §1783
                    match s {
                        expr_none => {
                            // §1791
                            if ((l >= glue_val) && (o != expr_none)) {
                                {
                                    t = self.new_spec(f);
                                    self.delete_glue_ref(f);
                                    if (__av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                        __av_mem[crate::ix::U((t) as usize)].set_hh_b0(normal);
                                    }
                                    if (__av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                        __av_mem[crate::ix::U((t) as usize)].set_hh_b1(normal);
                                    }
                                }
                            } else {
                                t = f;
                            }
                        }
                        expr_mult => {
                            // §1795
                            if (o == expr_div) {
                                {
                                    n = f;
                                    o = expr_scale;
                                }
                            } else {
                                if (l == int_val) {
                                    t = self.mult_and_add(t, f, 0i32, 2147483647i32);
                                } else {
                                    if (l == dimen_val) {
                                        t = self.mult_and_add(t, f, 0i32, 1073741823i32);
                                    } else {
                                        {
                                            { let __v337 = self.mult_and_add(__av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f, 0i32, 1073741823i32); __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v337); }
                                            { let __v338 = self.mult_and_add(__av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f, 0i32, 1073741823i32); __av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v338); }
                                            { let __v339 = self.mult_and_add(__av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f, 0i32, 1073741823i32); __av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v339); }
                                        }
                                    }
                                }
                            }
                        }
                        expr_div => {
                            // §1796
                            if (l < glue_val) {
                                t = self.quotient(t, f);
                            } else {
                                {
                                    { let __v340 = self.quotient(__av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f); __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v340); }
                                    { let __v341 = self.quotient(__av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f); __av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v341); }
                                    { let __v342 = self.quotient(__av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f); __av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v342); }
                                }
                            }
                        }
                        expr_scale => {
                            // §1798
                            if (l == int_val) {
                                t = self.fract(t, n, f, infinity);
                            } else {
                                if (l == dimen_val) {
                                    t = self.fract(t, n, f, max_dimen);
                                } else {
                                    {
                                        { let __v343 = self.fract(__av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), n, f, max_dimen); __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v343); }
                                        { let __v344 = self.fract(__av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), n, f, max_dimen); __av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v344); }
                                        { let __v345 = self.fract(__av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), n, f, max_dimen); __av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v345); }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    // §1783
                    if (o > expr_sub) {
                        s = o;
                    } else {
                        // §1792
                        {
                            s = expr_none;
                            if (r == expr_none) {
                                e = t;
                            } else {
                                if (l == int_val) {
                                    e = self.add_or_sub(e, t, infinity, (r == expr_sub));
                                } else {
                                    if (l == dimen_val) {
                                        e = self.add_or_sub(e, t, max_dimen, (r == expr_sub));
                                    } else {
                                        // §1794
                                        {
                                            { let __v346 = self.add_or_sub(__av_mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), max_dimen, (r == expr_sub)); __av_mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].set_int(__v346); }
                                            if (__av_mem[crate::ix::U((e) as usize)].hh().b0() == __av_mem[crate::ix::U((t) as usize)].hh().b0()) {
                                                { let __v347 = self.add_or_sub(__av_mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), max_dimen, (r == expr_sub)); __av_mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v347); }
                                            } else {
                                                if ((__av_mem[crate::ix::U((e) as usize)].hh().b0() < __av_mem[crate::ix::U((t) as usize)].hh().b0()) && (__av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v348 = __av_mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v348); }
                                                        { let __v349 = __av_mem[crate::ix::U((t) as usize)].hh().b0(); __av_mem[crate::ix::U((e) as usize)].set_hh_b0(__v349); }
                                                    }
                                                }
                                            }
                                            if (__av_mem[crate::ix::U((e) as usize)].hh().b1() == __av_mem[crate::ix::U((t) as usize)].hh().b1()) {
                                                { let __v350 = self.add_or_sub(__av_mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int(), __av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), max_dimen, (r == expr_sub)); __av_mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v350); }
                                            } else {
                                                if ((__av_mem[crate::ix::U((e) as usize)].hh().b1() < __av_mem[crate::ix::U((t) as usize)].hh().b1()) && (__av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v351 = __av_mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v351); }
                                                        { let __v352 = __av_mem[crate::ix::U((t) as usize)].hh().b1(); __av_mem[crate::ix::U((e) as usize)].set_hh_b1(__v352); }
                                                    }
                                                }
                                            }
                                            self.delete_glue_ref(t);
                                            if (__av_mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                                __av_mem[crate::ix::U((e) as usize)].set_hh_b0(normal);
                                            }
                                            if (__av_mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                                __av_mem[crate::ix::U((e) as usize)].set_hh_b1(normal);
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
                    if (o != expr_none) {
                        continue 'l_continue_b;
                    }
                    if (p != null) {
                        // §1789
                        {
                            f = e;
                            q = p;
                            e = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            t = __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                            n = __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                            s = (__av_mem[crate::ix::U((q) as usize)].hh().b1() / 4i32);
                            r = (__av_mem[crate::ix::U((q) as usize)].hh().b1() % 4i32);
                            l = __av_mem[crate::ix::U((q) as usize)].hh().b0();
                            p = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                            self.free_node(q, expr_node_size);
                            continue 'l_found_b;
                        }
                    }
                    // §1782
                    self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
                    if b {
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
                                self.print(1627i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 2048i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1629i32;
                            }
                            self.error();
                            if (l >= glue_val) {
                                {
                                    self.delete_glue_ref(e);
                                    e = zero_glue;
                                    { let __v353 = (__av_mem[crate::ix::U((e) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((e) as usize)].set_hh_rh(__v353); }
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
        self.scan_glue(glue_val);
    }

    /// Here we declare two trivial procedures in order to avoid mutually
    /// recursive procedures with parameters.
    /// @<Declare procedures needed for expressions
    // §1787
    pub fn scan_mu_glue(&mut self) {
        self.scan_glue(mu_val);
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
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        q = self.new_rule();
        if (self.cur_cmd == vrule) {
            __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(default_rule);
        } else {
            {
                __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(default_rule);
                __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
            }
        }
        'l_reswitch_b: loop {
            if self.scan_keyword(833i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v354 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v354); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(834i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v355 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v355); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(835i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v356 = self.cur_val; __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v356); }
                    continue 'l_reswitch_b;
                }
            }
            scan_rule_spec = q;
            break 'l_reswitch_b;
        }
        scan_rule_spec
    }

}
