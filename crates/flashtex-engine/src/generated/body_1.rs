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
    /// The function `get_avail` returns a pointer to a new one-word node whose
    /// `link` field is null. However, \TeX\ will halt if there is no more room left.
    /// If the available-space list is empty, i.e., if `avail=null`,
    /// we try first to increase `mem_end`. If that cannot be done, i.e., if
    /// `mem_end=mem_max`, we try to decrease `hi_mem_min`. If that cannot be
    /// done, i.e., if `hi_mem_min=lo_mem_max+1`, we have to quit.
    // §120
    pub fn get_avail(&mut self) -> halfword {
        let mut get_avail: halfword = 0;
        let mut p: halfword = 0; // §120
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
                            self.overflow(300i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
                        }
                    }
                }
            }
        }
        self.mem[(p) as usize].set_hh_rh(0i32);
        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
        get_avail = p;
        get_avail
    }

    /// The procedure `flush_list(p)` frees an entire linked list of
    /// one-word nodes that starts at position `p`.
    // §123
    pub fn flush_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §123
        let mut r: halfword = 0; // §123
        if (p != 0i32) {
            {
                r = p;
                loop {
                    q = r;
                    r = self.mem[(r) as usize].hh().rh();
                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    if (r == 0i32) { break; }
                }
                { let __v23 = self.avail; self.mem[(q) as usize].set_hh_rh(__v23); }
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
    // §125
    pub fn get_node(&mut self, mut s: i32) -> halfword {
        let mut get_node: halfword = 0;
        let mut p: halfword = 0; // §125
        let mut q: halfword = 0; // §125
        let mut r: i32 = 0; // §125
        let mut t: i32 = 0; // §125
        // goto labels: restart, found, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                p = self.rover;
                loop {
                    // §127
                    q = (p).wrapping_add(self.mem[(p) as usize].hh().lh());
                    while (self.mem[(q) as usize].hh().rh() == 268435455i32) {
                        {
                            t = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                            if (q == self.rover) {
                                self.rover = t;
                            }
                            { let __v24 = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh(); self.mem[((t).wrapping_add(1i32)) as usize].set_hh_lh(__v24); }
                            { let __ix25 = (self.mem[((q).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix25) as usize].set_hh_rh(t); }
                            q = (q).wrapping_add(self.mem[(q) as usize].hh().lh());
                        }
                    }
                    r = (q).wrapping_sub(s);
                    if (r > (p).wrapping_add(1i32)) {
                        // §128
                        {
                            self.mem[(p) as usize].set_hh_lh((r).wrapping_sub(p));
                            self.rover = p;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    // §127
                    if (r == p) {
                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() != p) {
                            // §129
                            {
                                self.rover = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                t = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                { let __ix26 = (self.rover).wrapping_add(1i32); self.mem[(__ix26) as usize].set_hh_lh(t); }
                                { let __v27 = self.rover; self.mem[((t).wrapping_add(1i32)) as usize].set_hh_rh(__v27); }
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                    }
                    // §127
                    self.mem[(p) as usize].set_hh_lh((q).wrapping_sub(p));
                    // §125
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
                        // §126
                        {
                            if ((self.hi_mem_min).wrapping_sub(self.lo_mem_max) >= 1998i32) {
                                t = (self.lo_mem_max).wrapping_add(1000i32);
                            } else {
                                t = ((self.lo_mem_max).wrapping_add(1i32)).wrapping_add(((self.hi_mem_min).wrapping_sub(self.lo_mem_max) / 2i32));
                            }
                            p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
                            q = self.lo_mem_max;
                            self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(q);
                            { let __ix28 = (self.rover).wrapping_add(1i32); self.mem[(__ix28) as usize].set_hh_lh(q); }
                            if (t > 268435455i32) {
                                t = 268435455i32;
                            }
                            { let __v29 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v29); }
                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(p);
                            self.mem[(q) as usize].set_hh_rh(268435455i32);
                            { let __v30 = (t).wrapping_sub(self.lo_mem_max); self.mem[(q) as usize].set_hh_lh(__v30); }
                            self.lo_mem_max = t;
                            { let __ix31 = self.lo_mem_max; self.mem[(__ix31) as usize].set_hh_rh(0i32); }
                            { let __ix32 = self.lo_mem_max; self.mem[(__ix32) as usize].set_hh_lh(0i32); }
                            self.rover = q;
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §125
                self.overflow(300i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
            }
            if __goto_1 <= 1 { // found
                self.mem[(r) as usize].set_hh_rh(0i32);
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
    // §130
    pub fn free_node(&mut self, mut p: halfword, mut s: halfword) {
        let mut q: halfword = 0; // §130
        self.mem[(p) as usize].set_hh_lh(s);
        self.mem[(p) as usize].set_hh_rh(268435455i32);
        q = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(q);
        { let __v33 = self.rover; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v33); }
        { let __ix34 = (self.rover).wrapping_add(1i32); self.mem[(__ix34) as usize].set_hh_lh(p); }
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(p);
        self.var_used = (self.var_used).wrapping_sub(s);
    }

    /// Just before \.{INITEX} writes out the memory, it sorts the doubly linked
    /// available space list. The list is probably very short at such times, so a
    /// simple insertion sort is used. The smallest available location will be
    /// pointed to by `rover`, the next-smallest by `rlink(rover)`, etc.
    // §131
    pub fn sort_avail(&mut self) {
        let mut p: halfword = 0; // §131
        let mut q: halfword = 0; // §131
        let mut r: halfword = 0; // §131
        let mut old_rover: halfword = 0; // §131
        p = self.get_node(1073741824i32);
        p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().rh();
        { let __ix35 = (self.rover).wrapping_add(1i32); self.mem[(__ix35) as usize].set_hh_rh(268435455i32); }
        old_rover = self.rover;
        while (p != old_rover) {
            // §132
            if (p < self.rover) {
                {
                    q = p;
                    p = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                    { let __v36 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v36); }
                    self.rover = q;
                }
            } else {
                {
                    q = self.rover;
                    while (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() < p) {
                        q = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                    }
                    r = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                    { let __v37 = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v37); }
                    self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(p);
                    p = r;
                }
            }
        }
        // §131
        p = self.rover;
        while (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() != 268435455i32) {
            {
                { let __ix38 = (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix38) as usize].set_hh_lh(p); }
                p = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
            }
        }
        { let __v39 = self.rover; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v39); }
        { let __ix40 = (self.rover).wrapping_add(1i32); self.mem[(__ix40) as usize].set_hh_lh(p); }
    }

    /// The `new_null_box` function returns a pointer to an `hlist_node` in
    /// which all subfields have the values corresponding to `\.{\\hbox\{\}}'.
    /// (The `subtype` field is set to `min_quarterword`, for historic reasons
    /// that are no longer relevant.)
    // §136
    pub fn new_null_box(&mut self) -> halfword {
        let mut new_null_box: halfword = 0;
        let mut p: halfword = 0; // §136
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
        self.mem[((p).wrapping_add(6i32)) as usize].set_gr(((0.0f64) as f32));
        new_null_box = p;
        new_null_box
    }

    /// A new rule node is delivered by the `new_rule` function. It
    /// makes all the dimensions ``running,'' so you have to change the
    /// ones that are not allowed to run.
    // §139
    pub fn new_rule(&mut self) -> halfword {
        let mut new_rule: halfword = 0;
        let mut p: halfword = 0; // §139
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
    // §144
    pub fn new_ligature(&mut self, mut f: quarterword, mut c: quarterword, mut q: halfword) -> halfword {
        let mut new_ligature: halfword = 0;
        let mut p: halfword = 0; // §144
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
    // §144
    pub fn new_lig_item(&mut self, mut c: quarterword) -> halfword {
        let mut new_lig_item: halfword = 0;
        let mut p: halfword = 0; // §144
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
    // §145
    pub fn new_disc(&mut self) -> halfword {
        let mut new_disc: halfword = 0;
        let mut p: halfword = 0; // §145
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
    // §147
    pub fn new_math(&mut self, mut w: scaled, mut s: small_number) -> halfword {
        let mut new_math: halfword = 0;
        let mut p: halfword = 0; // §147
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(9i32);
        self.mem[(p) as usize].set_hh_b1(s);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(w);
        new_math = p;
        new_math
    }

    /// Here is a function that returns a pointer to a copy of a glue spec.
    /// The reference count in the copy is `null`, because there is assumed
    /// to be exactly one reference to the new specification.
    // §151
    pub fn new_spec(&mut self, mut p: halfword) -> halfword {
        let mut new_spec: halfword = 0;
        let mut q: halfword = 0; // §151
        q = self.get_node(4i32);
        { let __v41 = self.mem[(p) as usize]; self.mem[(q) as usize] = __v41; }
        self.mem[(q) as usize].set_hh_rh(0i32);
        { let __v42 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v42); }
        { let __v43 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v43); }
        { let __v44 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v44); }
        new_spec = q;
        new_spec
    }

    /// And here's a function that creates a glue node for a given parameter
    /// identified by its code number; for example,
    /// `new_param_glue(line_skip_code)` returns a pointer to a glue node for the
    /// current \.{\\lineskip}.
    // §152
    pub fn new_param_glue(&mut self, mut n: small_number) -> halfword {
        let mut new_param_glue: halfword = 0;
        let mut p: halfword = 0; // §152
        let mut q: halfword = 0; // §152
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(10i32);
        self.mem[(p) as usize].set_hh_b1((n).wrapping_add(1i32));
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
        q = self.eqtb[(((615782i32).wrapping_add(n)) - 1) as usize].hh().rh();
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(q);
        { let __v45 = (self.mem[(q) as usize].hh().rh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_rh(__v45); }
        new_param_glue = p;
        new_param_glue
    }

    /// Glue nodes that are more or less anonymous are created by `new_glue`,
    /// whose argument points to a glue specification.
    // §153
    pub fn new_glue(&mut self, mut q: halfword) -> halfword {
        let mut new_glue: halfword = 0;
        let mut p: halfword = 0; // §153
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(10i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(q);
        { let __v46 = (self.mem[(q) as usize].hh().rh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_rh(__v46); }
        new_glue = p;
        new_glue
    }

    /// Still another subroutine is needed: This one is sort of a combination
    /// of `new_param_glue` and `new_glue`. It creates a glue node for one of
    /// the current glue parameters, but it makes a fresh copy of the glue
    /// specification, since that specification will probably be subject to change,
    /// while the parameter will stay put. The global variable `temp_ptr` is
    /// set to the address of the new spec.
    // §154
    pub fn new_skip_param(&mut self, mut n: small_number) -> halfword {
        let mut new_skip_param: halfword = 0;
        let mut p: halfword = 0; // §154
        self.temp_ptr = self.new_spec(self.eqtb[(((615782i32).wrapping_add(n)) - 1) as usize].hh().rh());
        p = self.new_glue(self.temp_ptr);
        { let __ix47 = self.temp_ptr; self.mem[(__ix47) as usize].set_hh_rh(0i32); }
        self.mem[(p) as usize].set_hh_b1((n).wrapping_add(1i32));
        new_skip_param = p;
        new_skip_param
    }

    /// The `new_kern` function creates a kern node having a given width.
    // §156
    pub fn new_kern(&mut self, mut w: scaled) -> halfword {
        let mut new_kern: halfword = 0;
        let mut p: halfword = 0; // §156
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(11i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(w);
        new_kern = p;
        new_kern
    }

    /// Anyone who has been reading the last few sections of the program will
    /// be able to guess what comes next.
    // §158
    pub fn new_penalty(&mut self, mut m: i32) -> halfword {
        let mut new_penalty: halfword = 0;
        let mut p: halfword = 0; // §158
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(12i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(m);
        new_penalty = p;
        new_penalty
    }

    /// Boxes, rules, inserts, whatsits, marks, and things in general that are
    /// sort of ``complicated'' are indicated only by printing `\.{[]}'.
    // §174
    pub fn short_display(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §174
        while (p > mem_min) {
            {
                if (p >= self.hi_mem_min) {
                    {
                        if (p <= self.mem_end) {
                            {
                                if (self.mem[(p) as usize].hh().b0() != self.font_in_short_display) {
                                    {
                                        if ((self.mem[(p) as usize].hh().b0() < 0i32) || (self.mem[(p) as usize].hh().b0() > font_max)) {
                                            self.print_char(42i32);
                                        } else {
                                            // §267
                                            self.print_esc(self.hash[(((615524i32).wrapping_add(self.mem[(p) as usize].hh().b0())) - 514) as usize].rh());
                                        }
                                        // §174
                                        self.print_char(32i32);
                                        self.font_in_short_display = self.mem[(p) as usize].hh().b0();
                                    }
                                }
                                self.print((self.mem[(p) as usize].hh().b1()).wrapping_sub(0i32));
                            }
                        }
                    }
                } else {
                    // §175
                    match self.mem[(p) as usize].hh().b0() {
                        0 | 1 | 3 | 8 | 4 | 5 | 13 => {
                            self.print(308i32);
                        }
                        2 => {
                            self.print_char(124i32);
                        }
                        10 => {
                            if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() != 0i32) {
                                self.print_char(32i32);
                            }
                        }
                        9 => {
                            self.print_char(36i32);
                        }
                        6 => {
                            self.short_display(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                        }
                        7 => {
                            {
                                self.short_display(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                self.short_display(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                n = self.mem[(p) as usize].hh().b1();
                                while (n > 0i32) {
                                    {
                                        if (self.mem[(p) as usize].hh().rh() != 0i32) {
                                            p = self.mem[(p) as usize].hh().rh();
                                        }
                                        n = (n).wrapping_sub(1i32);
                                    }
                                }
                            }
                        }
                        _ => {
                        }
                    }
                }
                // §174
                p = self.mem[(p) as usize].hh().rh();
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §176
    pub fn print_font_and_char(&mut self, mut p: i32) {
        if (p > self.mem_end) {
            self.print_esc(309i32);
        } else {
            {
                if ((self.mem[(p) as usize].hh().b0() < 0i32) || (self.mem[(p) as usize].hh().b0() > font_max)) {
                    self.print_char(42i32);
                } else {
                    // §267
                    self.print_esc(self.hash[(((615524i32).wrapping_add(self.mem[(p) as usize].hh().b0())) - 514) as usize].rh());
                }
                // §176
                self.print_char(32i32);
                self.print((self.mem[(p) as usize].hh().b1()).wrapping_sub(0i32));
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §176
    pub fn print_mark(&mut self, mut p: i32) {
        self.print_char(123i32);
        if ((p < self.hi_mem_min) || (p > self.mem_end)) {
            self.print_esc(309i32);
        } else {
            self.show_token_list(self.mem[(p) as usize].hh().rh(), 0i32, (max_print_line).wrapping_sub(10i32));
        }
        self.print_char(125i32);
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §176
    pub fn print_rule_dimen(&mut self, mut d: scaled) {
        if (d == (1073741824i32).wrapping_neg()) {
            self.print_char(42i32);
        } else {
            self.print_scaled(d);
        }
    }

    /// Then there is a subroutine that prints glue stretch and shrink, possibly
    /// followed by the name of finite units:
    // §177
    pub fn print_glue(&mut self, mut d: scaled, mut order: i32, mut s: str_number) {
        self.print_scaled(d);
        if ((order < 0i32) || (order > 3i32)) {
            self.print(310i32);
        } else {
            if (order > 0i32) {
                {
                    self.print(311i32);
                    while (order > 1i32) {
                        {
                            self.print_char(108i32);
                            order = (order).wrapping_sub(1i32);
                        }
                    }
                }
            } else {
                if (s != 0i32) {
                    self.print(s);
                }
            }
        }
    }

    /// The next subroutine prints a whole glue specification.
    // §178
    pub fn print_spec(&mut self, mut p: i32, mut s: str_number) {
        if ((p < mem_min) || (p >= self.lo_mem_max)) {
            self.print_char(42i32);
        } else {
            {
                self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                if (s != 0i32) {
                    self.print(s);
                }
                if (self.mem[((p).wrapping_add(2i32)) as usize].int() != 0i32) {
                    {
                        self.print(312i32);
                        self.print_glue(self.mem[((p).wrapping_add(2i32)) as usize].int(), self.mem[(p) as usize].hh().b0(), s);
                    }
                }
                if (self.mem[((p).wrapping_add(3i32)) as usize].int() != 0i32) {
                    {
                        self.print(313i32);
                        self.print_glue(self.mem[((p).wrapping_add(3i32)) as usize].int(), self.mem[(p) as usize].hh().b1(), s);
                    }
                }
            }
        }
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §691
    pub fn print_fam_and_char(&mut self, mut p: halfword) {
        self.print_esc(464i32);
        self.print_int(self.mem[(p) as usize].hh().b0());
        self.print_char(32i32);
        self.print((self.mem[(p) as usize].hh().b1()).wrapping_sub(0i32));
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §691
    pub fn print_delimiter(&mut self, mut p: halfword) {
        let mut a: i32 = 0; // §691
        a = (((self.mem[(p) as usize].qqqq().b0()).wrapping_mul(256i32)).wrapping_add(self.mem[(p) as usize].qqqq().b1())).wrapping_sub(0i32);
        a = ((((a).wrapping_mul(4096i32)).wrapping_add((self.mem[(p) as usize].qqqq().b2()).wrapping_mul(256i32))).wrapping_add(self.mem[(p) as usize].qqqq().b3())).wrapping_sub(0i32);
        if (a < 0i32) {
            self.print_int(a);
        } else {
            self.print_hex(a);
        }
    }

    /// The next subroutine will descend to another level of recursion when a
    /// subsidiary mlist needs to be displayed. The parameter `c` indicates what
    /// character is to become part of the recursion history. An empty mlist is
    /// distinguished from a field with `math_type(p)=empty`, because these are
    /// not equivalent (as explained above).
    /// @<Declare procedures needed for displaying...
    // §692
    pub fn print_subsidiary_data(&mut self, mut p: halfword, mut c: ASCII_code) {
        if ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) >= self.depth_threshold) {
            {
                if (self.mem[(p) as usize].hh().rh() != 0i32) {
                    self.print(314i32);
                }
            }
        } else {
            {
                {
                    self.str_pool[(self.pool_ptr) as usize] = c;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                self.temp_ptr = p;
                match self.mem[(p) as usize].hh().rh() {
                    1 => {
                        {
                            self.print_ln();
                            self.print_current_string();
                            self.print_fam_and_char(p);
                        }
                    }
                    2 => {
                        self.show_info();
                    }
                    3 => {
                        if (self.mem[(p) as usize].hh().lh() == 0i32) {
                            {
                                self.print_ln();
                                self.print_current_string();
                                self.print(860i32);
                            }
                        } else {
                            self.show_info();
                        }
                    }
                    _ => {
                    }
                }
                self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
            }
        }
    }

    /// @<Declare procedures needed for displaying...
    // §694
    pub fn print_style(&mut self, mut c: i32) {
        match (c / 2i32) {
            0 => {
                self.print_esc(861i32);
            }
            1 => {
                self.print_esc(862i32);
            }
            2 => {
                self.print_esc(863i32);
            }
            3 => {
                self.print_esc(864i32);
            }
            _ => {
                self.print(865i32);
            }
        }
    }

    /// Sometimes we need to convert \TeX's internal code numbers into symbolic
    /// form. The `print_skip_param` routine gives the symbolic name of a glue
    /// parameter.
    /// @<Declare the procedure called `print_skip_param`
    // §225
    pub fn print_skip_param(&mut self, mut n: i32) {
        match n {
            0 => {
                self.print_esc(376i32);
            }
            1 => {
                self.print_esc(377i32);
            }
            2 => {
                self.print_esc(378i32);
            }
            3 => {
                self.print_esc(379i32);
            }
            4 => {
                self.print_esc(380i32);
            }
            5 => {
                self.print_esc(381i32);
            }
            6 => {
                self.print_esc(382i32);
            }
            7 => {
                self.print_esc(383i32);
            }
            8 => {
                self.print_esc(384i32);
            }
            9 => {
                self.print_esc(385i32);
            }
            10 => {
                self.print_esc(386i32);
            }
            11 => {
                self.print_esc(387i32);
            }
            12 => {
                self.print_esc(388i32);
            }
            13 => {
                self.print_esc(389i32);
            }
            14 => {
                self.print_esc(390i32);
            }
            15 => {
                self.print_esc(391i32);
            }
            16 => {
                self.print_esc(392i32);
            }
            17 => {
                self.print_esc(393i32);
            }
            _ => {
                self.print(394i32);
            }
        }
    }

    /// Now we are ready for `show_node_list` itself. This procedure has been
    /// written to be ``extra robust'' in the sense that it should not crash or get
    /// into a loop even if the data structures have been messed up by bugs in
    /// the rest of the program. You can safely call its parent routine
    /// `show_box(p)` for arbitrary values of `p` when you are debugging \TeX.
    /// However, in the presence of bad data, the procedure may
    /// fetch a `memory_word` whose variant is different from the way it was stored;
    /// for example, it might try to read `mem[p].hh` when `mem[p]`
    /// contains a scaled integer, if `p` is a pointer that has been
    /// clobbered or chosen at random.
    // §182
    pub fn show_node_list(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §182
        let mut g: f64 = 0.0; // §182
        'l_exit_f: {
            if ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) > self.depth_threshold) {
                {
                    if (p > 0i32) {
                        self.print(314i32);
                    }
                    break 'l_exit_f;
                }
            }
            n = 0i32;
            while (p > mem_min) {
                {
                    self.print_ln();
                    self.print_current_string();
                    if (p > self.mem_end) {
                        {
                            self.print(315i32);
                            break 'l_exit_f;
                        }
                    }
                    n = (n).wrapping_add(1i32);
                    if (n > self.breadth_max) {
                        {
                            self.print(316i32);
                            break 'l_exit_f;
                        }
                    }
                    // §183
                    if (p >= self.hi_mem_min) {
                        self.print_font_and_char(p);
                    } else {
                        match self.mem[(p) as usize].hh().b0() {
                            0 | 1 | 13 => {
                                // §184
                                {
                                    if (self.mem[(p) as usize].hh().b0() == 0i32) {
                                        self.print_esc(104i32);
                                    } else {
                                        if (self.mem[(p) as usize].hh().b0() == 1i32) {
                                            self.print_esc(118i32);
                                        } else {
                                            self.print_esc(318i32);
                                        }
                                    }
                                    self.print(319i32);
                                    self.print_scaled(self.mem[((p).wrapping_add(3i32)) as usize].int());
                                    self.print_char(43i32);
                                    self.print_scaled(self.mem[((p).wrapping_add(2i32)) as usize].int());
                                    self.print(320i32);
                                    self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                    if (self.mem[(p) as usize].hh().b0() == 13i32) {
                                        // §185
                                        {
                                            if (self.mem[(p) as usize].hh().b1() != 0i32) {
                                                {
                                                    self.print(286i32);
                                                    self.print_int((self.mem[(p) as usize].hh().b1()).wrapping_add(1i32));
                                                    self.print(322i32);
                                                }
                                            }
                                            if (self.mem[((p).wrapping_add(6i32)) as usize].int() != 0i32) {
                                                {
                                                    self.print(323i32);
                                                    self.print_glue(self.mem[((p).wrapping_add(6i32)) as usize].int(), self.mem[((p).wrapping_add(5i32)) as usize].hh().b1(), 0i32);
                                                }
                                            }
                                            if (self.mem[((p).wrapping_add(4i32)) as usize].int() != 0i32) {
                                                {
                                                    self.print(324i32);
                                                    self.print_glue(self.mem[((p).wrapping_add(4i32)) as usize].int(), self.mem[((p).wrapping_add(5i32)) as usize].hh().b0(), 0i32);
                                                }
                                            }
                                        }
                                    } else {
                                        // §184
                                        {
                                            // §186
                                            g = ((self.mem[((p).wrapping_add(6i32)) as usize].gr()) as f64);
                                            if ((g != 0.0f64) && (self.mem[((p).wrapping_add(5i32)) as usize].hh().b0() != 0i32)) {
                                                {
                                                    self.print(325i32);
                                                    if (self.mem[((p).wrapping_add(5i32)) as usize].hh().b0() == 2i32) {
                                                        self.print(326i32);
                                                    }
                                                    if ((self.mem[((p).wrapping_add(6i32)) as usize].int()).wrapping_abs() < 1048576i32) {
                                                        self.print(327i32);
                                                    } else {
                                                        if ((g).abs() > 20000.0f64) {
                                                            {
                                                                if (g > 0.0f64) {
                                                                    self.print_char(62i32);
                                                                } else {
                                                                    self.print(328i32);
                                                                }
                                                                self.print_glue((20000i32).wrapping_mul(65536i32), self.mem[((p).wrapping_add(5i32)) as usize].hh().b1(), 0i32);
                                                            }
                                                        } else {
                                                            self.print_glue(crate::system::pas_round((((65536i32) as f64) * g)), self.mem[((p).wrapping_add(5i32)) as usize].hh().b1(), 0i32);
                                                        }
                                                    }
                                                }
                                            }
                                            // §184
                                            if (self.mem[((p).wrapping_add(4i32)) as usize].int() != 0i32) {
                                                {
                                                    self.print(321i32);
                                                    self.print_scaled(self.mem[((p).wrapping_add(4i32)) as usize].int());
                                                }
                                            }
                                        }
                                    }
                                    {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            2 => {
                                // §187
                                {
                                    self.print_esc(329i32);
                                    self.print_rule_dimen(self.mem[((p).wrapping_add(3i32)) as usize].int());
                                    self.print_char(43i32);
                                    self.print_rule_dimen(self.mem[((p).wrapping_add(2i32)) as usize].int());
                                    self.print(320i32);
                                    self.print_rule_dimen(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                }
                            }
                            3 => {
                                // §188
                                {
                                    self.print_esc(330i32);
                                    self.print_int((self.mem[(p) as usize].hh().b1()).wrapping_sub(0i32));
                                    self.print(331i32);
                                    self.print_scaled(self.mem[((p).wrapping_add(3i32)) as usize].int());
                                    self.print(332i32);
                                    self.print_spec(self.mem[((p).wrapping_add(4i32)) as usize].hh().rh(), 0i32);
                                    self.print_char(44i32);
                                    self.print_scaled(self.mem[((p).wrapping_add(2i32)) as usize].int());
                                    self.print(333i32);
                                    self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                    {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[((p).wrapping_add(4i32)) as usize].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            8 => {
                                // §1356
                                match self.mem[(p) as usize].hh().b1() {
                                    0 => {
                                        {
                                            self.print_write_whatsit(1285i32, p);
                                            self.print_char(61i32);
                                            self.print_file_name(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(), self.mem[((p).wrapping_add(2i32)) as usize].hh().lh(), self.mem[((p).wrapping_add(2i32)) as usize].hh().rh());
                                        }
                                    }
                                    1 => {
                                        {
                                            self.print_write_whatsit(594i32, p);
                                            self.print_mark(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                        }
                                    }
                                    2 => {
                                        self.print_write_whatsit(1286i32, p);
                                    }
                                    3 => {
                                        {
                                            self.print_esc(1287i32);
                                            self.print_mark(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                        }
                                    }
                                    4 => {
                                        {
                                            self.print_esc(1289i32);
                                            self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                            self.print(1292i32);
                                            self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].hh().b0());
                                            self.print_char(44i32);
                                            self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].hh().b1());
                                            self.print_char(41i32);
                                        }
                                    }
                                    _ => {
                                        self.print(1293i32);
                                    }
                                }
                            }
                            10 => {
                                // §189
                                if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                    // §190
                                    {
                                        self.print_esc(338i32);
                                        if (self.mem[(p) as usize].hh().b1() == 101i32) {
                                            self.print_char(99i32);
                                        } else {
                                            if (self.mem[(p) as usize].hh().b1() == 102i32) {
                                                self.print_char(120i32);
                                            }
                                        }
                                        self.print(339i32);
                                        self.print_spec(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(), 0i32);
                                        {
                                            {
                                                self.str_pool[(self.pool_ptr) as usize] = 46i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                        }
                                    }
                                } else {
                                    // §189
                                    {
                                        self.print_esc(334i32);
                                        if (self.mem[(p) as usize].hh().b1() != 0i32) {
                                            {
                                                self.print_char(40i32);
                                                if (self.mem[(p) as usize].hh().b1() < 98i32) {
                                                    self.print_skip_param((self.mem[(p) as usize].hh().b1()).wrapping_sub(1i32));
                                                } else {
                                                    if (self.mem[(p) as usize].hh().b1() == 98i32) {
                                                        self.print_esc(335i32);
                                                    } else {
                                                        self.print_esc(336i32);
                                                    }
                                                }
                                                self.print_char(41i32);
                                            }
                                        }
                                        if (self.mem[(p) as usize].hh().b1() != 98i32) {
                                            {
                                                self.print_char(32i32);
                                                if (self.mem[(p) as usize].hh().b1() < 98i32) {
                                                    self.print_spec(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(), 0i32);
                                                } else {
                                                    self.print_spec(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(), 337i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            11 => {
                                // §191
                                if (self.mem[(p) as usize].hh().b1() != 99i32) {
                                    {
                                        self.print_esc(340i32);
                                        if (self.mem[(p) as usize].hh().b1() != 0i32) {
                                            self.print_char(32i32);
                                        }
                                        self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        if (self.mem[(p) as usize].hh().b1() == 2i32) {
                                            self.print(341i32);
                                        }
                                    }
                                } else {
                                    {
                                        self.print_esc(342i32);
                                        self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        self.print(337i32);
                                    }
                                }
                            }
                            9 => {
                                // §192
                                {
                                    self.print_esc(343i32);
                                    if (self.mem[(p) as usize].hh().b1() == 0i32) {
                                        self.print(344i32);
                                    } else {
                                        self.print(345i32);
                                    }
                                    if (self.mem[((p).wrapping_add(1i32)) as usize].int() != 0i32) {
                                        {
                                            self.print(346i32);
                                            self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        }
                                    }
                                }
                            }
                            6 => {
                                // §193
                                {
                                    self.print_font_and_char((p).wrapping_add(1i32));
                                    self.print(347i32);
                                    if (self.mem[(p) as usize].hh().b1() > 1i32) {
                                        self.print_char(124i32);
                                    }
                                    self.font_in_short_display = self.mem[((p).wrapping_add(1i32)) as usize].hh().b0();
                                    self.short_display(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                    if (((self.mem[(p) as usize].hh().b1()) % 2) != 0) {
                                        self.print_char(124i32);
                                    }
                                    self.print_char(41i32);
                                }
                            }
                            12 => {
                                // §194
                                {
                                    self.print_esc(348i32);
                                    self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                }
                            }
                            7 => {
                                // §195
                                {
                                    self.print_esc(349i32);
                                    if (self.mem[(p) as usize].hh().b1() > 0i32) {
                                        {
                                            self.print(350i32);
                                            self.print_int(self.mem[(p) as usize].hh().b1());
                                        }
                                    }
                                    {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = 124i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            4 => {
                                // §196
                                {
                                    self.print_esc(351i32);
                                    self.print_mark(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                }
                            }
                            5 => {
                                // §197
                                {
                                    self.print_esc(352i32);
                                    {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            14 => {
                                // §690
                                self.print_style(self.mem[(p) as usize].hh().b1());
                            }
                            15 => {
                                // §695
                                {
                                    self.print_esc(525i32);
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = 68i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = 84i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = 83i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = 115i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 27 | 26 | 29 | 28 | 30 | 31 => {
                                // §696
                                {
                                    match self.mem[(p) as usize].hh().b0() {
                                        16 => {
                                            self.print_esc(866i32);
                                        }
                                        17 => {
                                            self.print_esc(867i32);
                                        }
                                        18 => {
                                            self.print_esc(868i32);
                                        }
                                        19 => {
                                            self.print_esc(869i32);
                                        }
                                        20 => {
                                            self.print_esc(870i32);
                                        }
                                        21 => {
                                            self.print_esc(871i32);
                                        }
                                        22 => {
                                            self.print_esc(872i32);
                                        }
                                        23 => {
                                            self.print_esc(873i32);
                                        }
                                        27 => {
                                            self.print_esc(874i32);
                                        }
                                        26 => {
                                            self.print_esc(875i32);
                                        }
                                        29 => {
                                            self.print_esc(539i32);
                                        }
                                        24 => {
                                            {
                                                self.print_esc(533i32);
                                                self.print_delimiter((p).wrapping_add(4i32));
                                            }
                                        }
                                        28 => {
                                            {
                                                self.print_esc(508i32);
                                                self.print_fam_and_char((p).wrapping_add(4i32));
                                            }
                                        }
                                        30 => {
                                            {
                                                self.print_esc(876i32);
                                                self.print_delimiter((p).wrapping_add(1i32));
                                            }
                                        }
                                        31 => {
                                            {
                                                self.print_esc(877i32);
                                                self.print_delimiter((p).wrapping_add(1i32));
                                            }
                                        }
                                        _ => {}
                                    }
                                    if (self.mem[(p) as usize].hh().b1() != 0i32) {
                                        if (self.mem[(p) as usize].hh().b1() == 1i32) {
                                            self.print_esc(878i32);
                                        } else {
                                            self.print_esc(879i32);
                                        }
                                    }
                                    if (self.mem[(p) as usize].hh().b0() < 30i32) {
                                        self.print_subsidiary_data((p).wrapping_add(1i32), 46i32);
                                    }
                                    self.print_subsidiary_data((p).wrapping_add(2i32), 94i32);
                                    self.print_subsidiary_data((p).wrapping_add(3i32), 95i32);
                                }
                            }
                            25 => {
                                // §697
                                {
                                    self.print_esc(880i32);
                                    if (self.mem[((p).wrapping_add(1i32)) as usize].int() == 1073741824i32) {
                                        self.print(881i32);
                                    } else {
                                        self.print_scaled(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                    }
                                    if ((((self.mem[((p).wrapping_add(4i32)) as usize].qqqq().b0() != 0i32) || (self.mem[((p).wrapping_add(4i32)) as usize].qqqq().b1() != 0i32)) || (self.mem[((p).wrapping_add(4i32)) as usize].qqqq().b2() != 0i32)) || (self.mem[((p).wrapping_add(4i32)) as usize].qqqq().b3() != 0i32)) {
                                        {
                                            self.print(882i32);
                                            self.print_delimiter((p).wrapping_add(4i32));
                                        }
                                    }
                                    if ((((self.mem[((p).wrapping_add(5i32)) as usize].qqqq().b0() != 0i32) || (self.mem[((p).wrapping_add(5i32)) as usize].qqqq().b1() != 0i32)) || (self.mem[((p).wrapping_add(5i32)) as usize].qqqq().b2() != 0i32)) || (self.mem[((p).wrapping_add(5i32)) as usize].qqqq().b3() != 0i32)) {
                                        {
                                            self.print(883i32);
                                            self.print_delimiter((p).wrapping_add(5i32));
                                        }
                                    }
                                    self.print_subsidiary_data((p).wrapping_add(2i32), 92i32);
                                    self.print_subsidiary_data((p).wrapping_add(3i32), 47i32);
                                }
                            }
                            _ => {
                                // §183
                                self.print(317i32);
                            }
                        }
                    }
                    // §182
                    p = self.mem[(p) as usize].hh().rh();
                }
            }
        }
    }

    /// The recursive machinery is started by calling `show_box`.
    // §198
    pub fn show_box(&mut self, mut p: halfword) {
        // §236
        self.depth_threshold = self.eqtb[((618188i32) - 1) as usize].int();
        self.breadth_max = self.eqtb[((618187i32) - 1) as usize].int();
        // §198
        if (self.breadth_max <= 0i32) {
            self.breadth_max = 5i32;
        }
        if ((self.pool_ptr).wrapping_add(self.depth_threshold) >= pool_size) {
            self.depth_threshold = ((pool_size).wrapping_sub(self.pool_ptr)).wrapping_sub(1i32);
        }
        self.show_node_list(p);
        self.print_ln();
    }

    /// First, however, we shall consider two non-recursive procedures that do
    /// simpler tasks. The first of these, `delete_token_ref`, is called when
    /// a pointer to a token list's reference count is being removed. This means
    /// that the token list should disappear if the reference count was `null`,
    /// otherwise the count should be decreased by one.
    // §200
    pub fn delete_token_ref(&mut self, mut p: halfword) {
        if (self.mem[(p) as usize].hh().lh() == 0i32) {
            self.flush_list(p);
        } else {
            { let __v48 = (self.mem[(p) as usize].hh().lh()).wrapping_sub(1i32); self.mem[(p) as usize].set_hh_lh(__v48); }
        }
    }

    /// Similarly, `delete_glue_ref` is called when a pointer to a glue
    /// specification is being withdrawn.
    // §201
    pub fn delete_glue_ref(&mut self, mut p: halfword) {
        if (self.mem[(p) as usize].hh().rh() == 0i32) {
            self.free_node(p, 4i32);
        } else {
            { let __v49 = (self.mem[(p) as usize].hh().rh()).wrapping_sub(1i32); self.mem[(p) as usize].set_hh_rh(__v49); }
        }
    }

    /// Now we are ready to delete any node list, recursively.
    /// In practice, the nodes deleted are usually charnodes (about 2/3 of the time),
    /// and they are glue nodes in about half of the remaining cases.
    // §202
    pub fn flush_node_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §202
        while (p != 0i32) {
            {
                q = self.mem[(p) as usize].hh().rh();
                if (p >= self.hi_mem_min) {
                    {
                        { let __v50 = self.avail; self.mem[(p) as usize].set_hh_rh(__v50); }
                        self.avail = p;
                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    }
                } else {
                    {
                        'l_done_f: {
                            match self.mem[(p) as usize].hh().b0() {
                                0 | 1 | 13 => {
                                    {
                                        self.flush_node_list(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh());
                                        self.free_node(p, 7i32);
                                        break 'l_done_f;
                                    }
                                }
                                2 => {
                                    {
                                        self.free_node(p, 4i32);
                                        break 'l_done_f;
                                    }
                                }
                                3 => {
                                    {
                                        self.flush_node_list(self.mem[((p).wrapping_add(4i32)) as usize].hh().lh());
                                        self.delete_glue_ref(self.mem[((p).wrapping_add(4i32)) as usize].hh().rh());
                                        self.free_node(p, 5i32);
                                        break 'l_done_f;
                                    }
                                }
                                8 => {
                                    // §1358
                                    {
                                        match self.mem[(p) as usize].hh().b1() {
                                            0 => {
                                                self.free_node(p, 3i32);
                                            }
                                            1 | 3 => {
                                                {
                                                    self.delete_token_ref(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                                    self.free_node(p, 2i32);
                                                    break 'l_done_f;
                                                }
                                            }
                                            2 | 4 => {
                                                self.free_node(p, 2i32);
                                            }
                                            _ => {
                                                self.confusion(1295i32);
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                                10 => {
                                    // §202
                                    {
                                        {
                                            if (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().rh() == 0i32) {
                                                self.free_node(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(), 4i32);
                                            } else {
                                                { let __ix51 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); let __v52 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().rh()).wrapping_sub(1i32); self.mem[(__ix51) as usize].set_hh_rh(__v52); }
                                            }
                                        }
                                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() != 0i32) {
                                            self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                        }
                                    }
                                }
                                11 | 9 | 12 => {
                                }
                                6 => {
                                    self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                }
                                4 => {
                                    self.delete_token_ref(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                }
                                7 => {
                                    {
                                        self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                        self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                    }
                                }
                                5 => {
                                    self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].int());
                                }
                                14 => {
                                    // §698
                                    {
                                        self.free_node(p, 3i32);
                                        break 'l_done_f;
                                    }
                                }
                                15 => {
                                    {
                                        self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                        self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh());
                                        self.flush_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                        self.flush_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().rh());
                                        self.free_node(p, 3i32);
                                        break 'l_done_f;
                                    }
                                }
                                16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 27 | 26 | 29 | 28 => {
                                    {
                                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() >= 2i32) {
                                            self.flush_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                        }
                                        if (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh() >= 2i32) {
                                            self.flush_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                        }
                                        if (self.mem[((p).wrapping_add(3i32)) as usize].hh().rh() >= 2i32) {
                                            self.flush_node_list(self.mem[((p).wrapping_add(3i32)) as usize].hh().lh());
                                        }
                                        if (self.mem[(p) as usize].hh().b0() == 24i32) {
                                            self.free_node(p, 5i32);
                                        } else {
                                            if (self.mem[(p) as usize].hh().b0() == 28i32) {
                                                self.free_node(p, 5i32);
                                            } else {
                                                self.free_node(p, 4i32);
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                                30 | 31 => {
                                    {
                                        self.free_node(p, 4i32);
                                        break 'l_done_f;
                                    }
                                }
                                25 => {
                                    {
                                        self.flush_node_list(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                        self.flush_node_list(self.mem[((p).wrapping_add(3i32)) as usize].hh().lh());
                                        self.free_node(p, 6i32);
                                        break 'l_done_f;
                                    }
                                }
                                _ => {
                                    // §202
                                    self.confusion(353i32);
                                }
                            }
                            self.free_node(p, 2i32);
                        }
                    }
                }
                p = q;
            }
        }
    }

    /// The copying procedure copies words en masse without bothering
    /// to look at their individual fields. If the node format changes---for
    /// example, if the size is altered, or if some link field is moved to another
    /// relative position---then this code may need to be changed too.
    // §204
    pub fn copy_node_list(&mut self, mut p: halfword) -> halfword {
        let mut copy_node_list: halfword = 0;
        let mut h: halfword = 0; // §204
        let mut q: halfword = 0; // §204
        let mut r: halfword = 0; // §204
        let mut words: i32 = 0; // §204
        h = self.get_avail();
        q = h;
        while (p != 0i32) {
            {
                // §205
                words = 1i32;
                if (p >= self.hi_mem_min) {
                    r = self.get_avail();
                } else {
                    // §206
                    match self.mem[(p) as usize].hh().b0() {
                        0 | 1 | 13 => {
                            {
                                r = self.get_node(7i32);
                                { let __v53 = self.mem[((p).wrapping_add(6i32)) as usize]; self.mem[((r).wrapping_add(6i32)) as usize] = __v53; }
                                { let __v54 = self.mem[((p).wrapping_add(5i32)) as usize]; self.mem[((r).wrapping_add(5i32)) as usize] = __v54; }
                                { let __v55 = self.copy_node_list(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh()); self.mem[((r).wrapping_add(5i32)) as usize].set_hh_rh(__v55); }
                                words = 5i32;
                            }
                        }
                        2 => {
                            {
                                r = self.get_node(4i32);
                                words = 4i32;
                            }
                        }
                        3 => {
                            {
                                r = self.get_node(5i32);
                                { let __v56 = self.mem[((p).wrapping_add(4i32)) as usize]; self.mem[((r).wrapping_add(4i32)) as usize] = __v56; }
                                { let __ix57 = self.mem[((p).wrapping_add(4i32)) as usize].hh().rh(); let __v58 = (self.mem[(self.mem[((p).wrapping_add(4i32)) as usize].hh().rh()) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix57) as usize].set_hh_rh(__v58); }
                                { let __v59 = self.copy_node_list(self.mem[((p).wrapping_add(4i32)) as usize].hh().lh()); self.mem[((r).wrapping_add(4i32)) as usize].set_hh_lh(__v59); }
                                words = 4i32;
                            }
                        }
                        8 => {
                            // §1357
                            match self.mem[(p) as usize].hh().b1() {
                                0 => {
                                    {
                                        r = self.get_node(3i32);
                                        words = 3i32;
                                    }
                                }
                                1 | 3 => {
                                    {
                                        r = self.get_node(2i32);
                                        { let __ix60 = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(); let __v61 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix60) as usize].set_hh_lh(__v61); }
                                        words = 2i32;
                                    }
                                }
                                2 | 4 => {
                                    {
                                        r = self.get_node(2i32);
                                        words = 2i32;
                                    }
                                }
                                _ => {
                                    self.confusion(1294i32);
                                }
                            }
                        }
                        10 => {
                            // §206
                            {
                                r = self.get_node(2i32);
                                { let __ix62 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); let __v63 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix62) as usize].set_hh_rh(__v63); }
                                { let __v64 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(__v64); }
                                { let __v65 = self.copy_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(__v65); }
                            }
                        }
                        11 | 9 | 12 => {
                            {
                                r = self.get_node(2i32);
                                words = 2i32;
                            }
                        }
                        6 => {
                            {
                                r = self.get_node(2i32);
                                { let __v66 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[((r).wrapping_add(1i32)) as usize] = __v66; }
                                { let __v67 = self.copy_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(__v67); }
                            }
                        }
                        7 => {
                            {
                                r = self.get_node(2i32);
                                { let __v68 = self.copy_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(__v68); }
                                { let __v69 = self.copy_node_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(__v69); }
                            }
                        }
                        4 => {
                            {
                                r = self.get_node(2i32);
                                { let __ix70 = self.mem[((p).wrapping_add(1i32)) as usize].int(); let __v71 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].int()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix70) as usize].set_hh_lh(__v71); }
                                words = 2i32;
                            }
                        }
                        5 => {
                            {
                                r = self.get_node(2i32);
                                { let __v72 = self.copy_node_list(self.mem[((p).wrapping_add(1i32)) as usize].int()); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v72); }
                            }
                        }
                        _ => {
                            self.confusion(354i32);
                        }
                    }
                }
                // §205
                while (words > 0i32) {
                    {
                        words = (words).wrapping_sub(1i32);
                        { let __v73 = self.mem[((p).wrapping_add(words)) as usize]; self.mem[((r).wrapping_add(words)) as usize] = __v73; }
                    }
                }
                // §204
                self.mem[(q) as usize].set_hh_rh(r);
                q = r;
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        self.mem[(q) as usize].set_hh_rh(0i32);
        q = self.mem[(h) as usize].hh().rh();
        {
            { let __v74 = self.avail; self.mem[(h) as usize].set_hh_rh(__v74); }
            self.avail = h;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        copy_node_list = q;
        copy_node_list
    }

    /// \[16] The semantic nest.
    /// \TeX\ is typically in the midst of building many lists at once. For example,
    /// when a math formula is being processed, \TeX\ is in math mode and
    /// working on an mlist; this formula has temporarily interrupted \TeX\ from
    /// being in horizontal mode and building the hlist of a paragraph; and this
    /// paragraph has temporarily interrupted \TeX\ from being in vertical mode
    /// and building the vlist for the next page of a document. Similarly, when a
    /// \.{\\vbox} occurs inside of an \.{\\hbox}, \TeX\ is temporarily
    /// interrupted from working in restricted horizontal mode, and it enters
    /// internal vertical mode.  The ``semantic nest'' is a stack that
    /// keeps track of what lists and modes are currently suspended.
    /// At each level of processing we are in one of six modes:
    /// \yskip\hang`vmode` stands for vertical mode (the page builder);
    /// \hang`hmode` stands for horizontal mode (the paragraph builder);
    /// ...
    // §211
    pub fn print_mode(&mut self, mut m: i32) {
        if (m > 0i32) {
            match (m / 101i32) {
                0 => {
                    self.print(355i32);
                }
                1 => {
                    self.print(356i32);
                }
                2 => {
                    self.print(357i32);
                }
                _ => {}
            }
        } else {
            if (m == 0i32) {
                self.print(358i32);
            } else {
                match ((m).wrapping_neg() / 101i32) {
                    0 => {
                        self.print(359i32);
                    }
                    1 => {
                        self.print(360i32);
                    }
                    2 => {
                        self.print(343i32);
                    }
                    _ => {}
                }
            }
        }
        self.print(361i32);
    }

    /// When \TeX's work on one level is interrupted, the state is saved by
    /// calling `push_nest`. This routine changes `head` and `tail` so that
    /// a new (empty) list is begun; it does not change `mode` or `aux`.
    // §216
    pub fn push_nest(&mut self) {
        if (self.nest_ptr > self.max_nest_stack) {
            {
                self.max_nest_stack = self.nest_ptr;
                if (self.nest_ptr == nest_size) {
                    self.overflow(362i32, nest_size);
                }
            }
        }
        { let __ix75 = self.nest_ptr; let __v76 = self.cur_list; self.nest[(__ix75) as usize] = __v76; }
        self.nest_ptr = (self.nest_ptr).wrapping_add(1i32);
        self.cur_list.head_field = self.get_avail();
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.pg_field = 0i32;
        self.cur_list.ml_field = self.line;
    }

    /// Conversely, when \TeX\ is finished on the current level, the former
    /// state is restored by calling `pop_nest`. This routine will never be
    /// called at the lowest semantic level, nor will it be called unless `head`
    /// is a node that should be returned to free memory.
    // §217
    pub fn pop_nest(&mut self) {
        {
            { let __ix77 = self.cur_list.head_field; let __v78 = self.avail; self.mem[(__ix77) as usize].set_hh_rh(__v78); }
            self.avail = self.cur_list.head_field;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.nest_ptr = (self.nest_ptr).wrapping_sub(1i32);
        self.cur_list = self.nest[(self.nest_ptr) as usize];
    }

    /// Here is a procedure that displays what \TeX\ is working on, at all levels.
    // §218
    pub fn show_activities(&mut self) {
        let mut p: i32 = 0; // §218
        let mut m: i32 = 0; // §218
        let mut a: memory_word = memory_word::default(); // §218
        let mut q: halfword = 0; // §218
        let mut r: halfword = 0; // §218
        let mut t: i32 = 0; // §218
        { let __ix79 = self.nest_ptr; let __v80 = self.cur_list; self.nest[(__ix79) as usize] = __v80; }
        self.print_nl(338i32);
        self.print_ln();
        {
            let __for_end_2 = 0i32;
            p = self.nest_ptr;
            while p >= __for_end_2 {
                {
                    m = self.nest[(p) as usize].mode_field;
                    a = self.nest[(p) as usize].aux_field;
                    self.print_nl(363i32);
                    self.print_mode(m);
                    self.print(364i32);
                    self.print_int((self.nest[(p) as usize].ml_field).wrapping_abs());
                    if (m == 102i32) {
                        if (self.nest[(p) as usize].pg_field != 8585216i32) {
                            {
                                self.print(365i32);
                                self.print_int((self.nest[(p) as usize].pg_field % 65536i32));
                                self.print(366i32);
                                self.print_int((self.nest[(p) as usize].pg_field / 4194304i32));
                                self.print_char(44i32);
                                self.print_int(((self.nest[(p) as usize].pg_field / 65536i32) % 64i32));
                                self.print_char(41i32);
                            }
                        }
                    }
                    if (self.nest[(p) as usize].ml_field < 0i32) {
                        self.print(367i32);
                    }
                    if (p == 0i32) {
                        {
                            // §986
                            if (4999997i32 != self.page_tail) {
                                {
                                    self.print_nl(980i32);
                                    if self.output_active {
                                        self.print(981i32);
                                    }
                                    self.show_box(self.mem[(4999997i32) as usize].hh().rh());
                                    if (self.page_contents > 0i32) {
                                        {
                                            self.print_nl(982i32);
                                            self.print_totals();
                                            self.print_nl(983i32);
                                            self.print_scaled(self.page_so_far[(0i32) as usize]);
                                            r = self.mem[(4999999i32) as usize].hh().rh();
                                            while (r != 4999999i32) {
                                                {
                                                    self.print_ln();
                                                    self.print_esc(330i32);
                                                    t = (self.mem[(r) as usize].hh().b1()).wrapping_sub(0i32);
                                                    self.print_int(t);
                                                    self.print(984i32);
                                                    if (self.eqtb[(((618218i32).wrapping_add(t)) - 1) as usize].int() == 1000i32) {
                                                        t = self.mem[((r).wrapping_add(3i32)) as usize].int();
                                                    } else {
                                                        t = (self.x_over_n(self.mem[((r).wrapping_add(3i32)) as usize].int(), 1000i32)).wrapping_mul(self.eqtb[(((618218i32).wrapping_add(t)) - 1) as usize].int());
                                                    }
                                                    self.print_scaled(t);
                                                    if (self.mem[(r) as usize].hh().b0() == 1i32) {
                                                        {
                                                            q = 4999997i32;
                                                            t = 0i32;
                                                            loop {
                                                                q = self.mem[(q) as usize].hh().rh();
                                                                if ((self.mem[(q) as usize].hh().b0() == 3i32) && (self.mem[(q) as usize].hh().b1() == self.mem[(r) as usize].hh().b1())) {
                                                                    t = (t).wrapping_add(1i32);
                                                                }
                                                                if (q == self.mem[((r).wrapping_add(1i32)) as usize].hh().lh()) { break; }
                                                            }
                                                            self.print(985i32);
                                                            self.print_int(t);
                                                            self.print(986i32);
                                                        }
                                                    }
                                                    r = self.mem[(r) as usize].hh().rh();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §218
                            if (self.mem[(4999998i32) as usize].hh().rh() != 0i32) {
                                self.print_nl(368i32);
                            }
                        }
                    }
                    self.show_box(self.mem[(self.nest[(p) as usize].head_field) as usize].hh().rh());
                    // §219
                    match ((m).wrapping_abs() / 101i32) {
                        0 => {
                            {
                                self.print_nl(369i32);
                                if (a.int() <= (65536000i32).wrapping_neg()) {
                                    self.print(370i32);
                                } else {
                                    self.print_scaled(a.int());
                                }
                                if (self.nest[(p) as usize].pg_field != 0i32) {
                                    {
                                        self.print(371i32);
                                        self.print_int(self.nest[(p) as usize].pg_field);
                                        self.print(372i32);
                                        if (self.nest[(p) as usize].pg_field != 1i32) {
                                            self.print_char(115i32);
                                        }
                                    }
                                }
                            }
                        }
                        1 => {
                            {
                                self.print_nl(373i32);
                                self.print_int(a.hh().lh());
                                if (m > 0i32) {
                                    if (a.hh().rh() > 0i32) {
                                        {
                                            self.print(374i32);
                                            self.print_int(a.hh().rh());
                                        }
                                    }
                                }
                            }
                        }
                        2 => {
                            if (a.int() != 0i32) {
                                {
                                    self.print(375i32);
                                    self.show_box(a.int());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                p = p.wrapping_sub(1);
            }
        }
    }

    /// We can print the symbolic name of an integer parameter as follows.
    // §237
    pub fn print_param(&mut self, mut n: i32) {
        match n {
            0 => {
                self.print_esc(420i32);
            }
            1 => {
                self.print_esc(421i32);
            }
            2 => {
                self.print_esc(422i32);
            }
            3 => {
                self.print_esc(423i32);
            }
            4 => {
                self.print_esc(424i32);
            }
            5 => {
                self.print_esc(425i32);
            }
            6 => {
                self.print_esc(426i32);
            }
            7 => {
                self.print_esc(427i32);
            }
            8 => {
                self.print_esc(428i32);
            }
            9 => {
                self.print_esc(429i32);
            }
            10 => {
                self.print_esc(430i32);
            }
            11 => {
                self.print_esc(431i32);
            }
            12 => {
                self.print_esc(432i32);
            }
            13 => {
                self.print_esc(433i32);
            }
            14 => {
                self.print_esc(434i32);
            }
            15 => {
                self.print_esc(435i32);
            }
            16 => {
                self.print_esc(436i32);
            }
            17 => {
                self.print_esc(437i32);
            }
            18 => {
                self.print_esc(438i32);
            }
            19 => {
                self.print_esc(439i32);
            }
            20 => {
                self.print_esc(440i32);
            }
            21 => {
                self.print_esc(441i32);
            }
            22 => {
                self.print_esc(442i32);
            }
            23 => {
                self.print_esc(443i32);
            }
            24 => {
                self.print_esc(444i32);
            }
            25 => {
                self.print_esc(445i32);
            }
            26 => {
                self.print_esc(446i32);
            }
            27 => {
                self.print_esc(447i32);
            }
            28 => {
                self.print_esc(448i32);
            }
            29 => {
                self.print_esc(449i32);
            }
            30 => {
                self.print_esc(450i32);
            }
            31 => {
                self.print_esc(451i32);
            }
            32 => {
                self.print_esc(452i32);
            }
            33 => {
                self.print_esc(453i32);
            }
            34 => {
                self.print_esc(454i32);
            }
            35 => {
                self.print_esc(455i32);
            }
            36 => {
                self.print_esc(456i32);
            }
            37 => {
                self.print_esc(457i32);
            }
            38 => {
                self.print_esc(458i32);
            }
            39 => {
                self.print_esc(459i32);
            }
            40 => {
                self.print_esc(460i32);
            }
            41 => {
                self.print_esc(461i32);
            }
            42 => {
                self.print_esc(462i32);
            }
            43 => {
                self.print_esc(463i32);
            }
            44 => {
                self.print_esc(464i32);
            }
            45 => {
                self.print_esc(465i32);
            }
            46 => {
                self.print_esc(466i32);
            }
            47 => {
                self.print_esc(467i32);
            }
            48 => {
                self.print_esc(468i32);
            }
            49 => {
                self.print_esc(469i32);
            }
            50 => {
                self.print_esc(470i32);
            }
            51 => {
                self.print_esc(471i32);
            }
            52 => {
                self.print_esc(472i32);
            }
            53 => {
                self.print_esc(473i32);
            }
            54 => {
                self.print_esc(474i32);
            }
            _ => {
                self.print(475i32);
            }
        }
    }

    /// The following procedure, which is called just before \TeX\ initializes its
    /// input and output, establishes the initial values of the date and time.
    /// Since standard \PASCAL\ cannot provide such information, something special
    /// is needed. The program here simply assumes that suitable values appear in
    /// the global variables \\{sys\_time}, \\{sys\_day}, \\{sys\_month}, and
    /// \\{sys\_year} (which are initialized to noon on 4 July 1776,
    /// in case the implementor is careless).
    // §241
    pub fn fix_date_and_time(&mut self) {
        self.sys_time = (12i32).wrapping_mul(60i32);
        self.sys_day = 4i32;
        self.sys_month = 7i32;
        self.sys_year = 1776i32;
        { let __v81 = self.sys_time; self.eqtb[((618183i32) - 1) as usize].set_int(__v81); }
        { let __v82 = self.sys_day; self.eqtb[((618184i32) - 1) as usize].set_int(__v82); }
        { let __v83 = self.sys_month; self.eqtb[((618185i32) - 1) as usize].set_int(__v83); }
        { let __v84 = self.sys_year; self.eqtb[((618186i32) - 1) as usize].set_int(__v84); }
    }

}
