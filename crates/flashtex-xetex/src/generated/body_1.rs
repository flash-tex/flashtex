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
    /// The procedure `flush_list(p)` frees an entire linked list of
    /// one-word nodes that starts at position `p`.
    // §145
    pub fn flush_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §145
        let mut r: halfword = 0; // §145
        if (p != (268435455i32).wrapping_neg()) {
            {
                r = p;
                loop {
                    q = r;
                    r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    if (r == (268435455i32).wrapping_neg()) { break; }
                }
                { let __v36 = self.avail; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v36); }
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
    // §147
    pub fn get_node(&mut self, mut s: i32) -> halfword {
        let mut get_node: halfword = 0;
        let mut p: halfword = 0; // §147
        let mut q: halfword = 0; // §147
        let mut r: i32 = 0; // §147
        let mut t: i32 = 0; // §147
        // goto labels: restart, found, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                p = self.rover;
                loop {
                    // §149
                    q = (p).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().lh());
                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() == empty_flag) {
                        {
                            t = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                            if (q == self.rover) {
                                self.rover = t;
                            }
                            { let __v37 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_hh_lh(__v37); }
                            { let __ix38 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix38) as usize)].set_hh_rh(t); }
                            q = (q).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().lh());
                        }
                    }
                    r = (q).wrapping_sub(s);
                    if (r > (p).wrapping_add(1i32)) {
                        // §150
                        {
                            self.mem[crate::ix::U((p) as usize)].set_hh_lh((r).wrapping_sub(p));
                            self.rover = p;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    // §149
                    if (r == p) {
                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != p) {
                            // §151
                            {
                                self.rover = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                t = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                { let __ix39 = (self.rover).wrapping_add(1i32); self.mem[crate::ix::U((__ix39) as usize)].set_hh_lh(t); }
                                { let __v40 = self.rover; self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_hh_rh(__v40); }
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                    }
                    // §149
                    self.mem[crate::ix::U((p) as usize)].set_hh_lh((q).wrapping_sub(p));
                    // §147
                    p = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                    if (p == self.rover) { break; }
                }
                if (s == 1073741824i32) {
                    {
                        get_node = max_halfword;
                        { __goto_1 = 2; continue 'l_dispatch_1; }
                    }
                }
                if ((self.lo_mem_max).wrapping_add(2i32) < self.hi_mem_min) {
                    if ((self.lo_mem_max).wrapping_add(2i32) <= 1073741823i32) {
                        // §148
                        {
                            if ((self.hi_mem_min).wrapping_sub(self.lo_mem_max) >= 1998i32) {
                                t = (self.lo_mem_max).wrapping_add(1000i32);
                            } else {
                                t = ((self.lo_mem_max).wrapping_add(1i32)).wrapping_add(((self.hi_mem_min).wrapping_sub(self.lo_mem_max) / 2i32));
                            }
                            p = self.mem[crate::ix::U(((self.rover).wrapping_add(1i32)) as usize)].hh().lh();
                            q = self.lo_mem_max;
                            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(q);
                            { let __ix41 = (self.rover).wrapping_add(1i32); self.mem[crate::ix::U((__ix41) as usize)].set_hh_lh(q); }
                            if (t > 1073741823i32) {
                                t = 1073741823i32;
                            }
                            { let __v42 = self.rover; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v42); }
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(empty_flag);
                            { let __v43 = (t).wrapping_sub(self.lo_mem_max); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v43); }
                            self.lo_mem_max = t;
                            { let __ix44 = self.lo_mem_max; self.mem[crate::ix::U((__ix44) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                            { let __ix45 = self.lo_mem_max; self.mem[crate::ix::U((__ix45) as usize)].set_hh_lh((268435455i32).wrapping_neg()); }
                            self.rover = q;
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §147
                self.overflow(65584i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
            }
            if __goto_1 <= 1 { // found
                self.mem[crate::ix::U((r) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                self.var_used = (self.var_used).wrapping_add(s);
                // §1705
                if (s >= medium_node_size) {
                    {
                        { let __v46 = self.cur_input.synctex_tag_field; self.mem[crate::ix::U((((r).wrapping_add(s)).wrapping_sub(1i32)) as usize)].set_hh_lh(__v46); }
                        { let __v47 = self.line; self.mem[crate::ix::U((((r).wrapping_add(s)).wrapping_sub(1i32)) as usize)].set_hh_rh(__v47); }
                    }
                }
                // §147
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
    // §152
    pub fn free_node(&mut self, mut p: halfword, mut s: halfword) {
        let mut q: halfword = 0; // §152
        self.mem[crate::ix::U((p) as usize)].set_hh_lh(s);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(empty_flag);
        q = self.mem[crate::ix::U(((self.rover).wrapping_add(1i32)) as usize)].hh().lh();
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(q);
        { let __v48 = self.rover; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v48); }
        { let __ix49 = (self.rover).wrapping_add(1i32); self.mem[crate::ix::U((__ix49) as usize)].set_hh_lh(p); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(p);
        self.var_used = (self.var_used).wrapping_sub(s);
    }

    /// Just before \.{INITEX} writes out the memory, it sorts the doubly linked
    /// available space list. The list is probably very short at such times, so a
    /// simple insertion sort is used. The smallest available location will be
    /// pointed to by `rover`, the next-smallest by `rlink(rover)`, etc.
    // §153
    pub fn sort_avail(&mut self) {
        let mut p: halfword = 0; // §153
        let mut q: halfword = 0; // §153
        let mut r: halfword = 0; // §153
        let mut old_rover: halfword = 0; // §153
        p = self.get_node(1073741824i32);
        p = self.mem[crate::ix::U(((self.rover).wrapping_add(1i32)) as usize)].hh().rh();
        { let __ix50 = (self.rover).wrapping_add(1i32); self.mem[crate::ix::U((__ix50) as usize)].set_hh_rh(max_halfword); }
        old_rover = self.rover;
        while (p != old_rover) {
            // §154
            if (p < self.rover) {
                {
                    q = p;
                    p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                    { let __v51 = self.rover; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v51); }
                    self.rover = q;
                }
            } else {
                {
                    q = self.rover;
                    while (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() < p) {
                        q = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                    }
                    r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                    { let __v52 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v52); }
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(p);
                    p = r;
                }
            }
        }
        // §153
        p = self.rover;
        while (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != max_halfword) {
            {
                { let __ix53 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix53) as usize)].set_hh_lh(p); }
                p = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
            }
        }
        { let __v54 = self.rover; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v54); }
        { let __ix55 = (self.rover).wrapping_add(1i32); self.mem[crate::ix::U((__ix55) as usize)].set_hh_lh(p); }
    }

    /// The `new_null_box` function returns a pointer to an `hlist_node` in
    /// which all subfields have the values corresponding to `\.{\\hbox\{\}}'.
    /// (The `subtype` field is set to `min_quarterword`, for historic reasons
    /// that are no longer relevant.)
    // §158
    pub fn new_null_box(&mut self) -> halfword {
        let mut new_null_box: halfword = 0;
        let mut p: halfword = 0; // §158
        p = self.get_node(box_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(hlist_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(min_quarterword);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
        self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
        self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
        new_null_box = p;
        new_null_box
    }

    /// A new rule node is delivered by the `new_rule` function. It
    /// makes all the dimensions ``running,'' so you have to change the
    /// ones that are not allowed to run.
    // §161
    pub fn new_rule(&mut self) -> halfword {
        let mut new_rule: halfword = 0;
        let mut p: halfword = 0; // §161
        p = self.get_node(rule_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(rule_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int((1073741824i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int((1073741824i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int((1073741824i32).wrapping_neg());
        new_rule = p;
        new_rule
    }

    /// The `new_ligature` function creates a ligature node having given
    /// contents of the `font`, `character`, and `lig_ptr` fields. We also have
    /// a `new_lig_item` function, which returns a two-word node having a given
    /// `character` field. Such nodes are used for temporary processing as ligatures
    /// are being created.
    // §166
    pub fn new_ligature(&mut self, mut f: quarterword, mut c: quarterword, mut q: halfword) -> halfword {
        let mut new_ligature: halfword = 0;
        let mut p: halfword = 0; // §166
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(ligature_node);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b0(f);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b1(c);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(q);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        new_ligature = p;
        new_ligature
    }

    /// The `new_ligature` function creates a ligature node having given
    /// contents of the `font`, `character`, and `lig_ptr` fields. We also have
    /// a `new_lig_item` function, which returns a two-word node having a given
    /// `character` field. Such nodes are used for temporary processing as ligatures
    /// are being created.
    // §166
    pub fn new_lig_item(&mut self, mut c: quarterword) -> halfword {
        let mut new_lig_item: halfword = 0;
        let mut p: halfword = 0; // §166
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(c);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
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
    // §167
    pub fn new_disc(&mut self) -> halfword {
        let mut new_disc: halfword = 0;
        let mut p: halfword = 0; // §167
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(disc_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        new_disc = p;
        new_disc
    }

    /// To support ``native'' fonts, we build `native_word_node`s, which are variable
    /// size whatsits.  These have the same `width`, `depth`, and `height` fields as a
    /// `box_node`, at offsets 1-3, and then a word containing a size field for the
    /// node, a font number, a length, and a glyph count.  Then there is a field
    /// containing a C pointer to a glyph info array; this and the glyph count are set
    /// by `set_native_metrics`.  Copying and freeing of these nodes needs to take
    /// account of this!  This is followed by `2*length` bytes, for the actual
    /// characters of the string (in UTF-16).
    /// So `native_node_size`, which does not include any space for the actual text, is
    /// 6.
    /// 0-3 whatsits subtypes are used for open, write, close, special; 4 is language;
    /// pdf\TeX\ uses up through 30-something, so we use subtypes starting from 40.
    /// There are also `glyph_node`s; these are like `native_word_node`s in having
    /// `width`, `depth`, and `height` fields, but then they contain a glyph ID rather
    /// ...
    // §169
    pub fn copy_native_glyph_info(&mut self, mut src: halfword, mut dest: halfword) {
        let mut glyph_count: i32 = 0; // §169
        if (self.mem[crate::ix::U(((src).wrapping_add(5i32)) as usize)].int() != null_ptr) {
            {
                glyph_count = self.mem[crate::ix::U(((src).wrapping_add(4i32)) as usize)].qqqq().b3();
                { let __v56 = self.copy_glyph_info(self.mem[crate::ix::U(((src).wrapping_add(5i32)) as usize)].int()); self.mem[crate::ix::U(((dest).wrapping_add(5i32)) as usize)].set_int(__v56); }
                self.mem[crate::ix::U(((dest).wrapping_add(4i32)) as usize)].set_qqqq_b3(glyph_count);
            }
        }
    }

    /// A `math_node`, which occurs only in horizontal lists, appears before and
    /// after mathematical formulas. The `subtype` field is `before` before the
    /// formula and `after` after it. There is a `width` field, which represents
    /// the amount of surrounding space inserted by \.{\\mathsurround}.
    /// In addition a `math_node` with `subtype>after` and `width=0` will be
    /// (ab)used to record a regular `math_node` reinserted after being
    /// discarded at a line break or one of the text direction primitives (
    /// \.{\\beginL}, \.{\\endL}, \.{\\beginR}, and \.{\\endR} ).
    // §171
    pub fn new_math(&mut self, mut w: scaled, mut s: small_number) -> halfword {
        let mut new_math: halfword = 0;
        let mut p: halfword = 0; // §171
        p = self.get_node(medium_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(math_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(w);
        new_math = p;
        new_math
    }

    /// Here is a function that returns a pointer to a copy of a glue spec.
    /// The reference count in the copy is `null`, because there is assumed
    /// to be exactly one reference to the new specification.
    // §175
    pub fn new_spec(&mut self, mut p: halfword) -> halfword {
        let mut new_spec: halfword = 0;
        let mut q: halfword = 0; // §175
        q = self.get_node(glue_spec_size);
        { let __v57 = self.mem[crate::ix::U((p) as usize)]; self.mem[crate::ix::U((q) as usize)] = __v57; }
        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        { let __v58 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v58); }
        { let __v59 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v59); }
        { let __v60 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v60); }
        new_spec = q;
        new_spec
    }

    /// And here's a function that creates a glue node for a given parameter
    /// identified by its code number; for example,
    /// `new_param_glue(line_skip_code)` returns a pointer to a glue node for the
    /// current \.{\\lineskip}.
    // §176
    pub fn new_param_glue(&mut self, mut n: small_number) -> halfword {
        let mut new_param_glue: halfword = 0;
        let mut p: halfword = 0; // §176
        let mut q: halfword = 0; // §176
        p = self.get_node(medium_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(glue_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1((n).wrapping_add(1i32));
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        q = self.eqtb[crate::ix::U((((glue_base).wrapping_add(n)) - 1) as usize)].hh().rh();
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(q);
        { let __v61 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v61); }
        new_param_glue = p;
        new_param_glue
    }

    /// Glue nodes that are more or less anonymous are created by `new_glue`,
    /// whose argument points to a glue specification.
    // §177
    pub fn new_glue(&mut self, mut q: halfword) -> halfword {
        let mut new_glue: halfword = 0;
        let mut p: halfword = 0; // §177
        p = self.get_node(medium_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(glue_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(q);
        { let __v62 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v62); }
        new_glue = p;
        new_glue
    }

    /// Still another subroutine is needed: This one is sort of a combination
    /// of `new_param_glue` and `new_glue`. It creates a glue node for one of
    /// the current glue parameters, but it makes a fresh copy of the glue
    /// specification, since that specification will probably be subject to change,
    /// while the parameter will stay put. The global variable `temp_ptr` is
    /// set to the address of the new spec.
    // §178
    pub fn new_skip_param(&mut self, mut n: small_number) -> halfword {
        let mut new_skip_param: halfword = 0;
        let mut p: halfword = 0; // §178
        self.temp_ptr = self.new_spec(self.eqtb[crate::ix::U((((glue_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        p = self.new_glue(self.temp_ptr);
        { let __ix63 = self.temp_ptr; self.mem[crate::ix::U((__ix63) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
        self.mem[crate::ix::U((p) as usize)].set_hh_b1((n).wrapping_add(1i32));
        new_skip_param = p;
        new_skip_param
    }

    /// The `new_kern` function creates a kern node having a given width.
    // §180
    pub fn new_kern(&mut self, mut w: scaled) -> halfword {
        let mut new_kern: halfword = 0;
        let mut p: halfword = 0; // §180
        p = self.get_node(medium_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(w);
        new_kern = p;
        new_kern
    }

    /// Anyone who has been reading the last few sections of the program will
    /// be able to guess what comes next.
    // §183
    pub fn new_penalty(&mut self, mut m: i32) -> halfword {
        let mut new_penalty: halfword = 0;
        let mut p: halfword = 0; // §183
        p = self.get_node(medium_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(penalty_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(m);
        new_penalty = p;
        new_penalty
    }

    /// Some stuff for character protrusion, etc.
    // §198
    pub fn pdf_error(&mut self, mut t: str_number, mut p: str_number) {
        self.normalize_selector();
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(65592i32);
        }
        if (t != 0i32) {
            {
                self.print(65566i32);
                self.print(t);
                self.print(41i32);
            }
        }
        self.print(65593i32);
        self.print(p);
        {
            if (self.interaction == error_stop_mode) {
                self.interaction = scroll_mode;
            }
            if self.log_opened {
                self.error();
            }
            self.history = fatal_error_stop;
            self.jump_out();
        }
    }

    /// Some stuff for character protrusion, etc.
    // §198
    pub fn prev_rightmost(&mut self, mut s: halfword, mut e: halfword) -> halfword {
        let mut prev_rightmost: halfword = 0;
        let mut p: halfword = 0; // §198
        prev_rightmost = (268435455i32).wrapping_neg();
        p = s;
        if (p == (268435455i32).wrapping_neg()) {
            return prev_rightmost;
        }
        while (self.mem[crate::ix::U((p) as usize)].hh().rh() != e) {
            {
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                if (p == (268435455i32).wrapping_neg()) {
                    return prev_rightmost;
                }
            }
        }
        prev_rightmost = p;
        prev_rightmost
    }

    /// Some stuff for character protrusion, etc.
    // §198
    pub fn round_xn_over_d(&mut self, mut x: scaled, mut n: i32, mut d: i32) -> scaled {
        let mut round_xn_over_d: scaled = 0;
        let mut positive: bool = false; // §198
        let mut t: nonnegative_integer = 0; // §198
        let mut u: nonnegative_integer = 0; // §198
        let mut v: nonnegative_integer = 0; // §198
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
        v = (v % d);
        if ((2i32).wrapping_mul(v) >= d) {
            u = (u).wrapping_add(1i32);
        }
        if positive {
            round_xn_over_d = u;
        } else {
            round_xn_over_d = (u).wrapping_neg();
        }
        round_xn_over_d
    }

    /// Some stuff for character protrusion, etc.
    // §198
    pub fn is_bit_set(&mut self, mut n: i32, mut s: small_number) -> bool {
        let mut is_bit_set: bool = false;
        let mut m: i32 = 0; // §198
        let mut i: i32 = 0; // §198
        m = 1i32;
        {
            let __for_end_2 = (s).wrapping_sub(1i32);
            i = 1i32;
            while i <= __for_end_2 {
                m = (m).wrapping_mul(2i32);
                i = i.wrapping_add(1);
            }
        }
        is_bit_set = ((((n / m) % 2i32)) != 0);
        is_bit_set
    }

    // §1410
    pub fn flush_str(&mut self, mut s: str_number) {
        if (s == (self.str_ptr).wrapping_sub(1i32)) {
            {
                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
            }
        }
    }

    // §1410
    pub fn tokens_to_string(&mut self, mut p: halfword) -> str_number {
        let mut tokens_to_string: str_number = 0;
        if (self.selector == new_string) {
            self.pdf_error(66109i32, 66110i32);
        }
        self.old_setting = self.selector;
        self.selector = new_string;
        self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = self.old_setting;
        tokens_to_string = self.make_string();
        tokens_to_string
    }

    // §1410
    pub fn scan_pdf_ext_toks(&mut self) {
        {
            if (self.scan_toks(false, true) != 0i32) {
            }
        }
    }

    // §1410
    pub fn compare_strings(&mut self) {
        let mut s1: str_number = 0; // §1410
        let mut s2: str_number = 0; // §1410
        let mut i1: pool_pointer = 0; // §1410
        let mut i2: pool_pointer = 0; // §1410
        let mut j1: pool_pointer = 0; // §1410
        let mut j2: pool_pointer = 0; // §1410
        let mut save_cur_cs: halfword = 0; // §1410
        'l_done_f: {
            save_cur_cs = self.cur_cs;
            {
                if (self.scan_toks(false, true) != 0i32) {
                }
            }
            s1 = self.tokens_to_string(self.def_ref);
            self.delete_token_ref(self.def_ref);
            self.cur_cs = save_cur_cs;
            {
                if (self.scan_toks(false, true) != 0i32) {
                }
            }
            s2 = self.tokens_to_string(self.def_ref);
            self.delete_token_ref(self.def_ref);
            i1 = self.str_start[crate::ix::U(((s1).wrapping_sub(65536i32)) as usize)];
            j1 = self.str_start[crate::ix::U((((s1).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)];
            i2 = self.str_start[crate::ix::U(((s2).wrapping_sub(65536i32)) as usize)];
            j2 = self.str_start[crate::ix::U((((s2).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)];
            while ((i1 < j1) && (i2 < j2)) {
                {
                    if (self.str_pool[crate::ix::U((i1) as usize)] < self.str_pool[crate::ix::U((i2) as usize)]) {
                        {
                            self.cur_val = (1i32).wrapping_neg();
                            break 'l_done_f;
                        }
                    }
                    if (self.str_pool[crate::ix::U((i1) as usize)] > self.str_pool[crate::ix::U((i2) as usize)]) {
                        {
                            self.cur_val = 1i32;
                            break 'l_done_f;
                        }
                    }
                    i1 = (i1).wrapping_add(1i32);
                    i2 = (i2).wrapping_add(1i32);
                }
            }
            if ((i1 == j1) && (i2 == j2)) {
                self.cur_val = 0i32;
            } else {
                if (i1 < j1) {
                    self.cur_val = 1i32;
                } else {
                    self.cur_val = (1i32).wrapping_neg();
                }
            }
        }
        self.flush_str(s2);
        self.flush_str(s1);
        self.cur_val_level = int_val;
    }

    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1411
    pub fn get_microinterval(&mut self) -> i32 {
        let mut get_microinterval: i32 = 0;
        let mut s: i32 = 0; // §1411
        let mut m: i32 = 0; // §1411
        { let mut __f0 = ::core::mem::take(&mut s); let mut __f1 = ::core::mem::take(&mut m); let __r = self.seconds_and_micros(&mut __f0, &mut __f1); s = __f0; m = __f1; __r };
        if ((s).wrapping_sub(self.epochseconds) > 32767i32) {
            get_microinterval = max_integer;
        } else {
            if (self.microseconds > m) {
                get_microinterval = ((((((((s).wrapping_sub(1i32)).wrapping_sub(self.epochseconds)).wrapping_mul(65536i32)) as f64) + (((((((m).wrapping_add(1000000i32)).wrapping_sub(self.microseconds)) as f64) / ((100i32) as f64)) * ((65536i32) as f64)) / ((10000i32) as f64)))) as i32);
            } else {
                get_microinterval = (((((((s).wrapping_sub(self.epochseconds)).wrapping_mul(65536i32)) as f64) + ((((((m).wrapping_sub(self.microseconds)) as f64) / ((100i32) as f64)) * ((65536i32) as f64)) / ((10000i32) as f64)))) as i32);
            }
        }
        get_microinterval
    }

    /// Boxes, rules, inserts, whatsits, marks, and things in general that are
    /// sort of ``complicated'' are indicated only by printing `\.{[]}'.
    // §200
    pub fn short_display(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §200
        while (p > mem_min) {
            {
                if (p >= self.hi_mem_min) {
                    {
                        if (p <= self.mem_end) {
                            {
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() != self.font_in_short_display) {
                                    {
                                        if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < font_base) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > font_max)) {
                                            self.print_char(42i32);
                                        } else {
                                            // §297
                                            self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b0())) - 1179650) as usize)].rh());
                                        }
                                        // §200
                                        self.print_char(32i32);
                                        self.font_in_short_display = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    }
                                }
                                self.print(self.mem[crate::ix::U((p) as usize)].hh().b1());
                            }
                        }
                    }
                } else {
                    // §201
                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                        hlist_node | vlist_node | ins_node | mark_node | adjust_node | unset_node => {
                            self.print(65594i32);
                        }
                        whatsit_node => {
                            match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                native_word_node | native_word_node_AT => {
                                    {
                                        if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1() != self.font_in_short_display) {
                                            {
                                                self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1())) - 1179650) as usize)].rh());
                                                self.print_char(32i32);
                                                self.font_in_short_display = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1();
                                            }
                                        }
                                        self.print_native_word(p);
                                    }
                                }
                                _ => {
                                    self.print(65594i32);
                                }
                            }
                        }
                        rule_node => {
                            self.print_char(124i32);
                        }
                        glue_node => {
                            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != zero_glue) {
                                self.print_char(32i32);
                            }
                        }
                        math_node => {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= L_code) {
                                self.print(65594i32);
                            } else {
                                self.print_char(36i32);
                            }
                        }
                        ligature_node => {
                            self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                        }
                        disc_node => {
                            {
                                self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                n = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                while (n > 0i32) {
                                    {
                                        if (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
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
                // §200
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §202
    pub fn print_font_and_char(&mut self, mut p: i32) {
        if (p > self.mem_end) {
            self.print_esc(65595i32);
        } else {
            {
                if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < font_base) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > font_max)) {
                    self.print_char(42i32);
                } else {
                    // §297
                    self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b0())) - 1179650) as usize)].rh());
                }
                // §202
                self.print_char(32i32);
                self.print(self.mem[crate::ix::U((p) as usize)].hh().b1());
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §202
    pub fn print_mark(&mut self, mut p: i32) {
        self.print_char(123i32);
        if ((p < self.hi_mem_min) || (p > self.mem_end)) {
            self.print_esc(65595i32);
        } else {
            self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (self.max_print_line).wrapping_sub(10i32));
        }
        self.print_char(125i32);
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §202
    pub fn print_rule_dimen(&mut self, mut d: scaled) {
        if (d == (1073741824i32).wrapping_neg()) {
            self.print_char(42i32);
        } else {
            self.print_scaled(d);
        }
    }

    /// Then there is a subroutine that prints glue stretch and shrink, possibly
    /// followed by the name of finite units:
    // §203
    pub fn print_glue(&mut self, mut d: scaled, mut order: i32, mut s: str_number) {
        self.print_scaled(d);
        if ((order < normal) || (order > filll)) {
            self.print(65596i32);
        } else {
            if (order > normal) {
                {
                    self.print(65597i32);
                    while (order > fil) {
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
    // §204
    pub fn print_spec(&mut self, mut p: i32, mut s: str_number) {
        if ((p < mem_min) || (p >= self.lo_mem_max)) {
            self.print_char(42i32);
        } else {
            {
                self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                if (s != 0i32) {
                    self.print(s);
                }
                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() != 0i32) {
                    {
                        self.print(65598i32);
                        self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U((p) as usize)].hh().b0(), s);
                    }
                }
                if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() != 0i32) {
                    {
                        self.print(65599i32);
                        self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U((p) as usize)].hh().b1(), s);
                    }
                }
            }
        }
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §733
    pub fn print_fam_and_char(&mut self, mut p: halfword) {
        let mut c: i32 = 0; // §733
        self.print_esc(65756i32);
        self.print_int(((self.mem[crate::ix::U((p) as usize)].hh().b0() % 256i32) % 256i32));
        self.print_char(32i32);
        c = (self.cast_to_ushort(self.mem[crate::ix::U((p) as usize)].hh().b1())).wrapping_add(((self.mem[crate::ix::U((p) as usize)].hh().b0() / 256i32)).wrapping_mul(65536i32));
        if (c < 65536i32) {
            self.print(c);
        } else {
            self.print_char(c);
        }
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §733
    pub fn print_delimiter(&mut self, mut p: halfword) {
        let mut a: i32 = 0; // §733
        a = (((self.mem[crate::ix::U((p) as usize)].qqqq().b0() % 256i32)).wrapping_mul(256i32)).wrapping_add((self.mem[crate::ix::U((p) as usize)].qqqq().b1()).wrapping_add(((self.mem[crate::ix::U((p) as usize)].qqqq().b0() / 256i32)).wrapping_mul(65536i32)));
        a = (((a).wrapping_mul(4096i32)).wrapping_add(((self.mem[crate::ix::U((p) as usize)].qqqq().b2() % 256i32)).wrapping_mul(256i32))).wrapping_add((self.mem[crate::ix::U((p) as usize)].qqqq().b3()).wrapping_add(((self.mem[crate::ix::U((p) as usize)].qqqq().b2() / 256i32)).wrapping_mul(65536i32)));
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
    // §734
    pub fn print_subsidiary_data(&mut self, mut p: halfword, mut c: UTF16_code) {
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]) >= self.depth_threshold) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().rh() != empty) {
                    self.print(65600i32);
                }
            }
        } else {
            {
                {
                    if (c > 65535i32) {
                        {
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((c).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32);
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((c % 1024i32)).wrapping_add(56320i32);
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = c;
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    }
                }
                self.temp_ptr = p;
                match self.mem[crate::ix::U((p) as usize)].hh().rh() {
                    math_char => {
                        {
                            self.print_ln();
                            self.print_current_string();
                            self.print_fam_and_char(p);
                        }
                    }
                    sub_box => {
                        self.show_info();
                    }
                    sub_mlist => {
                        if (self.mem[crate::ix::U((p) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
                            {
                                self.print_ln();
                                self.print_current_string();
                                self.print(66263i32);
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
    // §736
    pub fn print_style(&mut self, mut c: i32) {
        match (c / 2i32) {
            0 => {
                self.print_esc(66264i32);
            }
            1 => {
                self.print_esc(66265i32);
            }
            2 => {
                self.print_esc(66266i32);
            }
            3 => {
                self.print_esc(66267i32);
            }
            _ => {
                self.print(66268i32);
            }
        }
    }

    /// Sometimes we need to convert \TeX's internal code numbers into symbolic
    /// form. The `print_skip_param` routine gives the symbolic name of a glue
    /// parameter.
    /// @<Declare the procedure called `print_skip_param`
    // §251
    pub fn print_skip_param(&mut self, mut n: i32) {
        match n {
            line_skip_code => {
                self.print_esc(65667i32);
            }
            baseline_skip_code => {
                self.print_esc(65668i32);
            }
            par_skip_code => {
                self.print_esc(65669i32);
            }
            above_display_skip_code => {
                self.print_esc(65670i32);
            }
            below_display_skip_code => {
                self.print_esc(65671i32);
            }
            above_display_short_skip_code => {
                self.print_esc(65672i32);
            }
            below_display_short_skip_code => {
                self.print_esc(65673i32);
            }
            left_skip_code => {
                self.print_esc(65674i32);
            }
            right_skip_code => {
                self.print_esc(65675i32);
            }
            top_skip_code => {
                self.print_esc(65676i32);
            }
            split_top_skip_code => {
                self.print_esc(65677i32);
            }
            tab_skip_code => {
                self.print_esc(65678i32);
            }
            space_skip_code => {
                self.print_esc(65679i32);
            }
            xspace_skip_code => {
                self.print_esc(65680i32);
            }
            par_fill_skip_code => {
                self.print_esc(65681i32);
            }
            XeTeX_linebreak_skip_code => {
                self.print_esc(65682i32);
            }
            thin_mu_skip_code => {
                self.print_esc(65683i32);
            }
            med_mu_skip_code => {
                self.print_esc(65684i32);
            }
            thick_mu_skip_code => {
                self.print_esc(65685i32);
            }
            _ => {
                self.print(65686i32);
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
    // §208
    pub fn show_node_list(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §208
        let mut i: i32 = 0; // §208
        let mut g: f64 = 0.0; // §208
        'l_exit_f: {
            if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]) > self.depth_threshold) {
                {
                    if (p > (268435455i32).wrapping_neg()) {
                        self.print(65600i32);
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
                            self.print(65601i32);
                            break 'l_exit_f;
                        }
                    }
                    n = (n).wrapping_add(1i32);
                    if (n > self.breadth_max) {
                        {
                            self.print(65602i32);
                            break 'l_exit_f;
                        }
                    }
                    // §209
                    if (p >= self.hi_mem_min) {
                        self.print_font_and_char(p);
                    } else {
                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                            hlist_node | vlist_node | unset_node => {
                                // §210
                                {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) {
                                        self.print_esc(104i32);
                                    } else {
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                            self.print_esc(118i32);
                                        } else {
                                            self.print_esc(65607i32);
                                        }
                                    }
                                    self.print(65608i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print_char(43i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(65609i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == unset_node) {
                                        // §211
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() != min_quarterword) {
                                                {
                                                    self.print(65566i32);
                                                    self.print_int((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32));
                                                    self.print(65611i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(65612i32);
                                                    self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(65613i32);
                                                    self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0(), 0i32);
                                                }
                                            }
                                        }
                                    } else {
                                        // §210
                                        {
                                            // §212
                                            g = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr();
                                            if ((g != 0.0f64) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() != normal)) {
                                                {
                                                    self.print(65614i32);
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() == shrinking) {
                                                        self.print(65615i32);
                                                    }
                                                    if ((g).abs() > 20000.0f64) {
                                                        {
                                                            if (g > 0.0f64) {
                                                                self.print_char(62i32);
                                                            } else {
                                                                self.print(65616i32);
                                                            }
                                                            self.print_glue((20000i32).wrapping_mul(unity), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                        }
                                                    } else {
                                                        self.print_glue(crate::system::pas_round((((unity) as f64) * g)), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                    }
                                                }
                                            }
                                            // §210
                                            if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(65610i32);
                                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                }
                                            }
                                            if (self.eTeX_mode == 1i32) {
                                                // §1514
                                                if ((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == dlist)) {
                                                    self.print(66913i32);
                                                }
                                            }
                                        }
                                    }
                                    // §210
                                    {
                                        {
                                            if (46i32 > 65535i32) {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65490i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((46i32 % 1024i32)).wrapping_add(56320i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            rule_node => {
                                // §213
                                {
                                    self.print_esc(65617i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print_char(43i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(65609i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                }
                            }
                            ins_node => {
                                // §214
                                {
                                    self.print_esc(65618i32);
                                    self.print_int(self.mem[crate::ix::U((p) as usize)].hh().b1());
                                    self.print(65619i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print(65620i32);
                                    self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(), 0i32);
                                    self.print_char(44i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(65621i32);
                                    self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    {
                                        {
                                            if (46i32 > 65535i32) {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65490i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((46i32 % 1024i32)).wrapping_add(56320i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            whatsit_node => {
                                // §1416
                                match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                    open_node => {
                                        {
                                            self.print_write_whatsit(66736i32, p);
                                            self.print_char(61i32);
                                            self.print_file_name(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                        }
                                    }
                                    write_node => {
                                        {
                                            self.print_write_whatsit(65915i32, p);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    close_node => {
                                        self.print_write_whatsit(66737i32, p);
                                    }
                                    special_node => {
                                        {
                                            self.print_esc(66738i32);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    latespecial_node => {
                                        {
                                            self.print_esc(66738i32);
                                            self.print(66753i32);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    language_node => {
                                        {
                                            self.print_esc(66740i32);
                                            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                            self.print(66754i32);
                                            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b0());
                                            self.print_char(44i32);
                                            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b1());
                                            self.print_char(41i32);
                                        }
                                    }
                                    pdf_save_pos_node => {
                                        self.print_esc(66748i32);
                                    }
                                    native_word_node | native_word_node_AT => {
                                        {
                                            self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1())) - 1179650) as usize)].rh());
                                            self.print_char(32i32);
                                            self.print_native_word(p);
                                        }
                                    }
                                    glyph_node => {
                                        {
                                            self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1())) - 1179650) as usize)].rh());
                                            self.print(66755i32);
                                            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                        }
                                    }
                                    pic_node | pdf_node => {
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == pic_node) {
                                                self.print_esc(66743i32);
                                            } else {
                                                self.print_esc(66744i32);
                                            }
                                            self.print(66756i32);
                                            {
                                                let __for_end_11 = (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b0()).wrapping_sub(1i32);
                                                i = 0i32;
                                                while i <= __for_end_11 {
                                                    { let __a64_0 = self.pic_path_byte(p, i); let __a64_1 = true; self.print_raw_char(__a64_0, __a64_1) };
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                            self.print(34i32);
                                        }
                                    }
                                    _ => {
                                        self.print(66757i32);
                                    }
                                }
                            }
                            glue_node => {
                                // §215
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                    // §216
                                    {
                                        self.print_esc(65626i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                            self.print_char(99i32);
                                        } else {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == x_leaders) {
                                                self.print_char(120i32);
                                            }
                                        }
                                        self.print(65627i32);
                                        self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 0i32);
                                        {
                                            {
                                                if (46i32 > 65535i32) {
                                                    {
                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65490i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((46i32 % 1024i32)).wrapping_add(56320i32);
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                } else {
                                                    {
                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                        }
                                    }
                                } else {
                                    // §215
                                    {
                                        self.print_esc(65622i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() != normal) {
                                            {
                                                self.print_char(40i32);
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() < cond_math_glue) {
                                                    self.print_skip_param((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(1i32));
                                                } else {
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() == cond_math_glue) {
                                                        self.print_esc(65623i32);
                                                    } else {
                                                        self.print_esc(65624i32);
                                                    }
                                                }
                                                self.print_char(41i32);
                                            }
                                        }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() != cond_math_glue) {
                                            {
                                                self.print_char(32i32);
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() < cond_math_glue) {
                                                    self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 0i32);
                                                } else {
                                                    self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 65625i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            kern_node => {
                                // §217
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() != mu_glue) {
                                    {
                                        self.print_esc(65603i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() != normal) {
                                            self.print_char(32i32);
                                        }
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == acc_kern) {
                                            self.print(65628i32);
                                        } else {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == space_adjustment) {
                                                self.print(65629i32);
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        self.print_esc(65630i32);
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        self.print(65625i32);
                                    }
                                }
                            }
                            margin_kern_node => {
                                // §209
                                {
                                    self.print_esc(65603i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() == left_side) {
                                        self.print(65604i32);
                                    } else {
                                        self.print(65605i32);
                                    }
                                }
                            }
                            math_node => {
                                // §218
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() > after) {
                                    {
                                        if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                            self.print_esc(65631i32);
                                        } else {
                                            self.print_esc(65632i32);
                                        }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() > R_code) {
                                            self.print_char(82i32);
                                        } else {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() > L_code) {
                                                self.print_char(76i32);
                                            } else {
                                                self.print_char(77i32);
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        self.print_esc(65633i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == before) {
                                            self.print(65634i32);
                                        } else {
                                            self.print(65635i32);
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() != 0i32) {
                                            {
                                                self.print(65636i32);
                                                self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                    }
                                }
                            }
                            ligature_node => {
                                // §219
                                {
                                    self.print_font_and_char((p).wrapping_add(1i32));
                                    self.print(65637i32);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() > 1i32) {
                                        self.print_char(124i32);
                                    }
                                    self.font_in_short_display = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b0();
                                    self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                        self.print_char(124i32);
                                    }
                                    self.print_char(41i32);
                                }
                            }
                            penalty_node => {
                                // §220
                                {
                                    self.print_esc(65638i32);
                                    self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                }
                            }
                            disc_node => {
                                // §221
                                {
                                    self.print_esc(65639i32);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() > 0i32) {
                                        {
                                            self.print(65640i32);
                                            self.print_int(self.mem[crate::ix::U((p) as usize)].hh().b1());
                                        }
                                    }
                                    {
                                        {
                                            if (46i32 > 65535i32) {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65490i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((46i32 % 1024i32)).wrapping_add(56320i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                    {
                                        if (124i32 > 65535i32) {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65412i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((124i32 % 1024i32)).wrapping_add(56320i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        } else {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 124i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            mark_node => {
                                // §222
                                {
                                    self.print_esc(65641i32);
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                        {
                                            self.print_char(115i32);
                                            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        }
                                    }
                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                }
                            }
                            adjust_node => {
                                // §223
                                {
                                    self.print_esc(65642i32);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() != 0i32) {
                                        self.print(65643i32);
                                    }
                                    {
                                        {
                                            if (46i32 > 65535i32) {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65490i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((46i32 % 1024i32)).wrapping_add(56320i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            style_node => {
                                // §732
                                self.print_style(self.mem[crate::ix::U((p) as usize)].hh().b1());
                            }
                            choice_node => {
                                // §737
                                {
                                    self.print_esc(65838i32);
                                    {
                                        if (68i32 > 65535i32) {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65468i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((68i32 % 1024i32)).wrapping_add(56320i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        } else {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 68i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        if (84i32 > 65535i32) {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65452i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((84i32 % 1024i32)).wrapping_add(56320i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        } else {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 84i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        if (83i32 > 65535i32) {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65453i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((83i32 % 1024i32)).wrapping_add(56320i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        } else {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 83i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        if (115i32 > 65535i32) {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65421i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((115i32 % 1024i32)).wrapping_add(56320i32);
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        } else {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 115i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad | inner_noad | radical_noad | over_noad | under_noad | vcenter_noad | accent_noad | left_noad | right_noad => {
                                // §738
                                {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        ord_noad => {
                                            self.print_esc(66269i32);
                                        }
                                        op_noad => {
                                            self.print_esc(66270i32);
                                        }
                                        bin_noad => {
                                            self.print_esc(66271i32);
                                        }
                                        rel_noad => {
                                            self.print_esc(66272i32);
                                        }
                                        open_noad => {
                                            self.print_esc(66273i32);
                                        }
                                        close_noad => {
                                            self.print_esc(66274i32);
                                        }
                                        punct_noad => {
                                            self.print_esc(66275i32);
                                        }
                                        inner_noad => {
                                            self.print_esc(66276i32);
                                        }
                                        over_noad => {
                                            self.print_esc(66277i32);
                                        }
                                        under_noad => {
                                            self.print_esc(66278i32);
                                        }
                                        vcenter_noad => {
                                            self.print_esc(65855i32);
                                        }
                                        radical_noad => {
                                            {
                                                self.print_esc(65847i32);
                                                self.print_delimiter((p).wrapping_add(4i32));
                                            }
                                        }
                                        accent_noad => {
                                            {
                                                self.print_esc(65813i32);
                                                self.print_fam_and_char((p).wrapping_add(4i32));
                                            }
                                        }
                                        left_noad => {
                                            {
                                                self.print_esc(66279i32);
                                                self.print_delimiter((p).wrapping_add(1i32));
                                            }
                                        }
                                        right_noad => {
                                            {
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal) {
                                                    self.print_esc(66280i32);
                                                } else {
                                                    self.print_esc(66281i32);
                                                }
                                                self.print_delimiter((p).wrapping_add(1i32));
                                            }
                                        }
                                        _ => {}
                                    }
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() < left_noad) {
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() != normal) {
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == limits) {
                                                    self.print_esc(66282i32);
                                                } else {
                                                    self.print_esc(66283i32);
                                                }
                                            }
                                            self.print_subsidiary_data((p).wrapping_add(1i32), 46i32);
                                        }
                                    }
                                    self.print_subsidiary_data((p).wrapping_add(2i32), 94i32);
                                    self.print_subsidiary_data((p).wrapping_add(3i32), 95i32);
                                }
                            }
                            fraction_noad => {
                                // §739
                                {
                                    self.print_esc(66284i32);
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == default_code) {
                                        self.print(66285i32);
                                    } else {
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    }
                                    if (((((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0() % 256i32) != 0i32) || ((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1()).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0() / 256i32)).wrapping_mul(65536i32)) != min_quarterword)) || ((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2() % 256i32) != 0i32)) || ((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b3()).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2() / 256i32)).wrapping_mul(65536i32)) != min_quarterword)) {
                                        {
                                            self.print(66286i32);
                                            self.print_delimiter((p).wrapping_add(4i32));
                                        }
                                    }
                                    if (((((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b0() % 256i32) != 0i32) || ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b1()).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b0() / 256i32)).wrapping_mul(65536i32)) != min_quarterword)) || ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b2() % 256i32) != 0i32)) || ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b3()).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b2() / 256i32)).wrapping_mul(65536i32)) != min_quarterword)) {
                                        {
                                            self.print(66287i32);
                                            self.print_delimiter((p).wrapping_add(5i32));
                                        }
                                    }
                                    self.print_subsidiary_data((p).wrapping_add(2i32), 92i32);
                                    self.print_subsidiary_data((p).wrapping_add(3i32), 47i32);
                                }
                            }
                            _ => {
                                // §209
                                self.print(65606i32);
                            }
                        }
                    }
                    // §208
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
        }
    }

    /// The recursive machinery is started by calling `show_box`.
    // §224
    pub fn show_box(&mut self, mut p: halfword) {
        // §262
        self.depth_threshold = self.eqtb[crate::ix::U(((7892289i32) - 1) as usize)].int();
        self.breadth_max = self.eqtb[crate::ix::U(((7892288i32) - 1) as usize)].int();
        // §224
        if (self.breadth_max <= 0i32) {
            self.breadth_max = 5i32;
        }
        if ((self.pool_ptr).wrapping_add(self.depth_threshold) >= pool_size) {
            self.depth_threshold = ((pool_size).wrapping_sub(self.pool_ptr)).wrapping_sub(1i32);
        }
        self.show_node_list(p);
        self.print_ln();
    }

    /// The recursive machinery is started by calling `show_box`.
    // §224
    pub fn short_display_n(&mut self, mut p: i32, mut m: i32) {
        self.breadth_max = m;
        self.depth_threshold = ((pool_size).wrapping_sub(self.pool_ptr)).wrapping_sub(1i32);
        self.show_node_list(p);
    }

    /// First, however, we shall consider two non-recursive procedures that do
    /// simpler tasks. The first of these, `delete_token_ref`, is called when
    /// a pointer to a token list's reference count is being removed. This means
    /// that the token list should disappear if the reference count was `null`,
    /// otherwise the count should be decreased by one.
    // §226
    pub fn delete_token_ref(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
            self.flush_list(p);
        } else {
            { let __v65 = (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v65); }
        }
    }

    /// Similarly, `delete_glue_ref` is called when a pointer to a glue
    /// specification is being withdrawn.
    // §227
    pub fn delete_glue_ref(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
            self.free_node(p, glue_spec_size);
        } else {
            { let __v66 = (self.mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v66); }
        }
    }

    /// Now we are ready to delete any node list, recursively.
    /// In practice, the nodes deleted are usually charnodes (about 2/3 of the time),
    /// and they are glue nodes in about half of the remaining cases.
    // §228
    pub fn flush_node_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §228
        while (p != (268435455i32).wrapping_neg()) {
            {
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                if (p >= self.hi_mem_min) {
                    {
                        { let __v67 = self.avail; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v67); }
                        self.avail = p;
                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    }
                } else {
                    {
                        'l_done_f: {
                            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                hlist_node | vlist_node | unset_node => {
                                    {
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                        self.free_node(p, box_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                rule_node => {
                                    {
                                        self.free_node(p, rule_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                ins_node => {
                                    {
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh());
                                        self.delete_glue_ref(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh());
                                        self.free_node(p, ins_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                whatsit_node => {
                                    // §1418
                                    {
                                        match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                            open_node => {
                                                self.free_node(p, open_node_size);
                                            }
                                            write_node | special_node | latespecial_node => {
                                                {
                                                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                                    self.free_node(p, write_node_size);
                                                    break 'l_done_f;
                                                }
                                            }
                                            close_node | language_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            native_word_node | native_word_node_AT => {
                                                {
                                                    {
                                                        if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].int() != null_ptr) {
                                                            {
                                                                self.free_glyph_info(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].int());
                                                                self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_int(null_ptr);
                                                                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b3(0i32);
                                                            }
                                                        }
                                                    }
                                                    self.free_node(p, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0());
                                                }
                                            }
                                            glyph_node => {
                                                self.free_node(p, glyph_node_size);
                                            }
                                            pic_node | pdf_node => {
                                                self.free_node(p, (pic_node_size).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b0()).wrapping_add(7i32) / 8i32)));
                                            }
                                            pdf_save_pos_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            _ => {
                                                self.confusion(66759i32);
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                                glue_node => {
                                    // §228
                                    {
                                        {
                                            if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                self.free_node(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), glue_spec_size);
                                            } else {
                                                { let __ix68 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v69 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix68) as usize)].set_hh_rh(__v69); }
                                            }
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                            self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                        self.free_node(p, medium_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                kern_node | math_node | penalty_node => {
                                    {
                                        self.free_node(p, medium_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                margin_kern_node => {
                                    {
                                        self.free_node(p, margin_kern_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                ligature_node => {
                                    self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                }
                                mark_node => {
                                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                }
                                disc_node => {
                                    {
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    }
                                }
                                adjust_node => {
                                    self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                }
                                style_node => {
                                    // §740
                                    {
                                        self.free_node(p, style_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                choice_node => {
                                    {
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                        self.free_node(p, style_node_size);
                                        break 'l_done_f;
                                    }
                                }
                                ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad | inner_noad | radical_noad | over_noad | under_noad | vcenter_noad | accent_noad => {
                                    {
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() >= sub_box) {
                                            self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh() >= sub_box) {
                                            self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh() >= sub_box) {
                                            self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().lh());
                                        }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == radical_noad) {
                                            self.free_node(p, radical_noad_size);
                                        } else {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == accent_noad) {
                                                self.free_node(p, accent_noad_size);
                                            } else {
                                                self.free_node(p, noad_size);
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                                left_noad | right_noad => {
                                    {
                                        self.free_node(p, noad_size);
                                        break 'l_done_f;
                                    }
                                }
                                fraction_noad => {
                                    {
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                        self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().lh());
                                        self.free_node(p, fraction_noad_size);
                                        break 'l_done_f;
                                    }
                                }
                                _ => {
                                    // §228
                                    self.confusion(65644i32);
                                }
                            }
                            self.free_node(p, small_node_size);
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
    // §230
    pub fn copy_node_list(&mut self, mut p: halfword) -> halfword {
        let mut copy_node_list: halfword = 0;
        let mut h: halfword = 0; // §230
        let mut q: halfword = 0; // §230
        let mut r: halfword = 0; // §230
        let mut words: i32 = 0; // §230
        h = self.get_avail();
        q = h;
        while (p != (268435455i32).wrapping_neg()) {
            {
                // §231
                words = 1i32;
                if (p >= self.hi_mem_min) {
                    r = self.get_avail();
                } else {
                    // §232
                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                        hlist_node | vlist_node | unset_node => {
                            {
                                r = self.get_node(box_node_size);
                                // §1711
                                { let __v70 = self.mem[crate::ix::U(((p).wrapping_add(7i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].set_hh_lh(__v70); }
                                { let __v71 = self.mem[crate::ix::U(((p).wrapping_add(7i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].set_hh_rh(__v71); }
                                // §232
                                { let __v72 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)] = __v72; }
                                { let __v73 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)] = __v73; }
                                { let __v74 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(__v74); }
                                words = 5i32;
                            }
                        }
                        rule_node => {
                            {
                                r = self.get_node(rule_node_size);
                                words = 4i32;
                            }
                        }
                        ins_node => {
                            {
                                r = self.get_node(ins_node_size);
                                { let __v75 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)] = __v75; }
                                { let __ix76 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(); let __v77 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix76) as usize)].set_hh_rh(__v77); }
                                { let __v78 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()); self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_hh_lh(__v78); }
                                words = 4i32;
                            }
                        }
                        whatsit_node => {
                            // §1417
                            match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                open_node => {
                                    {
                                        r = self.get_node(open_node_size);
                                        words = open_node_size;
                                    }
                                }
                                write_node | special_node | latespecial_node => {
                                    {
                                        r = self.get_node(write_node_size);
                                        { let __ix79 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v80 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix79) as usize)].set_hh_lh(__v80); }
                                        words = write_node_size;
                                    }
                                }
                                close_node | language_node => {
                                    {
                                        r = self.get_node(small_node_size);
                                        words = small_node_size;
                                    }
                                }
                                native_word_node | native_word_node_AT => {
                                    {
                                        words = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0();
                                        r = self.get_node(words);
                                        while (words > 0i32) {
                                            {
                                                words = (words).wrapping_sub(1i32);
                                                { let __v81 = self.mem[crate::ix::U(((p).wrapping_add(words)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(words)) as usize)] = __v81; }
                                            }
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_int(null_ptr);
                                        self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_qqqq_b3(0i32);
                                        self.copy_native_glyph_info(p, r);
                                    }
                                }
                                glyph_node => {
                                    {
                                        r = self.get_node(glyph_node_size);
                                        words = glyph_node_size;
                                    }
                                }
                                pic_node | pdf_node => {
                                    {
                                        words = (pic_node_size).wrapping_add(((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b0()).wrapping_add(7i32) / 8i32));
                                        r = self.get_node(words);
                                    }
                                }
                                pdf_save_pos_node => {
                                    r = self.get_node(small_node_size);
                                }
                                _ => {
                                    self.confusion(66758i32);
                                }
                            }
                        }
                        glue_node => {
                            // §232
                            {
                                r = self.get_node(medium_node_size);
                                { let __ix82 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v83 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix82) as usize)].set_hh_rh(__v83); }
                                // §1713
                                { let __v84 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(__v84); }
                                { let __v85 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_rh(__v85); }
                                // §232
                                { let __v86 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v86); }
                                { let __v87 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v87); }
                            }
                        }
                        kern_node | math_node | penalty_node => {
                            {
                                r = self.get_node(medium_node_size);
                                words = medium_node_size;
                            }
                        }
                        margin_kern_node => {
                            {
                                r = self.get_node(margin_kern_node_size);
                                words = margin_kern_node_size;
                            }
                        }
                        ligature_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v88 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)] = __v88; }
                                { let __v89 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v89); }
                            }
                        }
                        disc_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v90 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v90); }
                                { let __v91 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v91); }
                            }
                        }
                        mark_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __ix92 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v93 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix92) as usize)].set_hh_lh(__v93); }
                                words = small_node_size;
                            }
                        }
                        adjust_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v94 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v94); }
                            }
                        }
                        _ => {
                            self.confusion(65645i32);
                        }
                    }
                }
                // §231
                while (words > 0i32) {
                    {
                        words = (words).wrapping_sub(1i32);
                        { let __v95 = self.mem[crate::ix::U(((p).wrapping_add(words)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(words)) as usize)] = __v95; }
                    }
                }
                // §230
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                q = r;
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        q = self.mem[crate::ix::U((h) as usize)].hh().rh();
        {
            { let __v96 = self.avail; self.mem[crate::ix::U((h) as usize)].set_hh_rh(__v96); }
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
    // §237
    pub fn print_mode(&mut self, mut m: i32) {
        if (m > 0i32) {
            match (m / 104i32) {
                0 => {
                    self.print(65646i32);
                }
                1 => {
                    self.print(65647i32);
                }
                2 => {
                    self.print(65648i32);
                }
                _ => {}
            }
        } else {
            if (m == 0i32) {
                self.print(65649i32);
            } else {
                match ((m).wrapping_neg() / 104i32) {
                    0 => {
                        self.print(65650i32);
                    }
                    1 => {
                        self.print(65651i32);
                    }
                    2 => {
                        self.print(65633i32);
                    }
                    _ => {}
                }
            }
        }
        self.print(65652i32);
    }

    /// When \TeX's work on one level is interrupted, the state is saved by
    /// calling `push_nest`. This routine changes `head` and `tail` so that
    /// a new (empty) list is begun; it does not change `mode` or `aux`.
    // §242
    pub fn push_nest(&mut self) {
        if (self.nest_ptr > self.max_nest_stack) {
            {
                self.max_nest_stack = self.nest_ptr;
                if (self.nest_ptr == nest_size) {
                    self.overflow(65653i32, nest_size);
                }
            }
        }
        { let __ix97 = self.nest_ptr; let __v98 = self.cur_list; self.nest[crate::ix::U((__ix97) as usize)] = __v98; }
        self.nest_ptr = (self.nest_ptr).wrapping_add(1i32);
        self.cur_list.head_field = self.get_avail();
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.pg_field = 0i32;
        self.cur_list.ml_field = self.line;
        self.cur_list.eTeX_aux_field = (268435455i32).wrapping_neg();
    }

    /// Conversely, when \TeX\ is finished on the current level, the former
    /// state is restored by calling `pop_nest`. This routine will never be
    /// called at the lowest semantic level, nor will it be called unless `head`
    /// is a node that should be returned to free memory.
    // §243
    pub fn pop_nest(&mut self) {
        {
            { let __ix99 = self.cur_list.head_field; let __v100 = self.avail; self.mem[crate::ix::U((__ix99) as usize)].set_hh_rh(__v100); }
            self.avail = self.cur_list.head_field;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.nest_ptr = (self.nest_ptr).wrapping_sub(1i32);
        self.cur_list = self.nest[crate::ix::U((self.nest_ptr) as usize)];
    }

    /// Here is a procedure that displays what \TeX\ is working on, at all levels.
    // §244
    pub fn show_activities(&mut self) {
        let mut p: i32 = 0; // §244
        let mut m: i32 = 0; // §244
        let mut a: memory_word = memory_word::default(); // §244
        let mut q: halfword = 0; // §244
        let mut r: halfword = 0; // §244
        let mut t: i32 = 0; // §244
        { let __ix101 = self.nest_ptr; let __v102 = self.cur_list; self.nest[crate::ix::U((__ix101) as usize)] = __v102; }
        self.print_nl(65626i32);
        self.print_ln();
        {
            let __for_end_2 = 0i32;
            p = self.nest_ptr;
            while p >= __for_end_2 {
                {
                    m = self.nest[crate::ix::U((p) as usize)].mode_field;
                    a = self.nest[crate::ix::U((p) as usize)].aux_field;
                    self.print_nl(65654i32);
                    self.print_mode(m);
                    self.print(65655i32);
                    self.print_int((self.nest[crate::ix::U((p) as usize)].ml_field).wrapping_abs());
                    if (m == hmode) {
                        if (self.nest[crate::ix::U((p) as usize)].pg_field != 8585216i32) {
                            {
                                self.print(65656i32);
                                self.print_int((self.nest[crate::ix::U((p) as usize)].pg_field % 65536i32));
                                self.print(65657i32);
                                self.print_int((self.nest[crate::ix::U((p) as usize)].pg_field / 4194304i32));
                                self.print_char(44i32);
                                self.print_int(((self.nest[crate::ix::U((p) as usize)].pg_field / 65536i32) % 64i32));
                                self.print_char(41i32);
                            }
                        }
                    }
                    if (self.nest[crate::ix::U((p) as usize)].ml_field < 0i32) {
                        self.print(65658i32);
                    }
                    if (p == 0i32) {
                        {
                            // §1040
                            if (page_head != self.page_tail) {
                                {
                                    self.print_nl(66409i32);
                                    if self.output_active {
                                        self.print(66410i32);
                                    }
                                    self.show_box(self.mem[crate::ix::U((page_head) as usize)].hh().rh());
                                    if (self.page_contents > empty) {
                                        {
                                            self.print_nl(66411i32);
                                            self.print_totals();
                                            self.print_nl(66412i32);
                                            self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                                            r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                            while (r != page_ins_head) {
                                                {
                                                    self.print_ln();
                                                    self.print_esc(65618i32);
                                                    t = self.mem[crate::ix::U((r) as usize)].hh().b1();
                                                    self.print_int(t);
                                                    self.print(66413i32);
                                                    if (self.eqtb[crate::ix::U((((count_base).wrapping_add(t)) - 1) as usize)].int() == 1000i32) {
                                                        t = self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int();
                                                    } else {
                                                        t = (self.x_over_n(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int(), 1000i32)).wrapping_mul(self.eqtb[crate::ix::U((((count_base).wrapping_add(t)) - 1) as usize)].int());
                                                    }
                                                    self.print_scaled(t);
                                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == split_up) {
                                                        {
                                                            q = page_head;
                                                            t = 0i32;
                                                            loop {
                                                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                if ((self.mem[crate::ix::U((q) as usize)].hh().b0() == ins_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == self.mem[crate::ix::U((r) as usize)].hh().b1())) {
                                                                    t = (t).wrapping_add(1i32);
                                                                }
                                                                if (q == self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh()) { break; }
                                                            }
                                                            self.print(66414i32);
                                                            self.print_int(t);
                                                            self.print(66415i32);
                                                        }
                                                    }
                                                    r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §244
                            if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                self.print_nl(65659i32);
                            }
                        }
                    }
                    self.show_box(self.mem[crate::ix::U((self.nest[crate::ix::U((p) as usize)].head_field) as usize)].hh().rh());
                    // §245
                    match ((m).wrapping_abs() / 104i32) {
                        0 => {
                            {
                                self.print_nl(65660i32);
                                if (a.int() <= (65536000i32).wrapping_neg()) {
                                    self.print(65661i32);
                                } else {
                                    self.print_scaled(a.int());
                                }
                                if (self.nest[crate::ix::U((p) as usize)].pg_field != 0i32) {
                                    {
                                        self.print(65662i32);
                                        self.print_int(self.nest[crate::ix::U((p) as usize)].pg_field);
                                        self.print(65663i32);
                                        if (self.nest[crate::ix::U((p) as usize)].pg_field != 1i32) {
                                            self.print_char(115i32);
                                        }
                                    }
                                }
                            }
                        }
                        1 => {
                            {
                                self.print_nl(65664i32);
                                self.print_int(a.hh().lh());
                                if (m > 0i32) {
                                    if (a.hh().rh() > 0i32) {
                                        {
                                            self.print(65665i32);
                                            self.print_int(a.hh().rh());
                                        }
                                    }
                                }
                            }
                        }
                        2 => {
                            if (a.int() != (268435455i32).wrapping_neg()) {
                                {
                                    self.print(65666i32);
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
    // §263
    pub fn print_param(&mut self, mut n: i32) {
        match n {
            pretolerance_code => {
                self.print_esc(65712i32);
            }
            tolerance_code => {
                self.print_esc(65713i32);
            }
            line_penalty_code => {
                self.print_esc(65714i32);
            }
            hyphen_penalty_code => {
                self.print_esc(65715i32);
            }
            ex_hyphen_penalty_code => {
                self.print_esc(65716i32);
            }
            club_penalty_code => {
                self.print_esc(65717i32);
            }
            widow_penalty_code => {
                self.print_esc(65718i32);
            }
            display_widow_penalty_code => {
                self.print_esc(65719i32);
            }
            broken_penalty_code => {
                self.print_esc(65720i32);
            }
            bin_op_penalty_code => {
                self.print_esc(65721i32);
            }
            rel_penalty_code => {
                self.print_esc(65722i32);
            }
            pre_display_penalty_code => {
                self.print_esc(65723i32);
            }
            post_display_penalty_code => {
                self.print_esc(65724i32);
            }
            inter_line_penalty_code => {
                self.print_esc(65725i32);
            }
            double_hyphen_demerits_code => {
                self.print_esc(65726i32);
            }
            final_hyphen_demerits_code => {
                self.print_esc(65727i32);
            }
            adj_demerits_code => {
                self.print_esc(65728i32);
            }
            mag_code => {
                self.print_esc(65729i32);
            }
            delimiter_factor_code => {
                self.print_esc(65730i32);
            }
            looseness_code => {
                self.print_esc(65731i32);
            }
            time_code => {
                self.print_esc(65732i32);
            }
            day_code => {
                self.print_esc(65733i32);
            }
            month_code => {
                self.print_esc(65734i32);
            }
            year_code => {
                self.print_esc(65735i32);
            }
            show_box_breadth_code => {
                self.print_esc(65736i32);
            }
            show_box_depth_code => {
                self.print_esc(65737i32);
            }
            hbadness_code => {
                self.print_esc(65738i32);
            }
            vbadness_code => {
                self.print_esc(65739i32);
            }
            pausing_code => {
                self.print_esc(65740i32);
            }
            tracing_online_code => {
                self.print_esc(65741i32);
            }
            tracing_macros_code => {
                self.print_esc(65742i32);
            }
            tracing_stats_code => {
                self.print_esc(65743i32);
            }
            tracing_paragraphs_code => {
                self.print_esc(65744i32);
            }
            tracing_pages_code => {
                self.print_esc(65745i32);
            }
            tracing_output_code => {
                self.print_esc(65746i32);
            }
            tracing_lost_chars_code => {
                self.print_esc(65747i32);
            }
            tracing_commands_code => {
                self.print_esc(65748i32);
            }
            tracing_restores_code => {
                self.print_esc(65749i32);
            }
            uc_hyph_code => {
                self.print_esc(65750i32);
            }
            output_penalty_code => {
                self.print_esc(65751i32);
            }
            max_dead_cycles_code => {
                self.print_esc(65752i32);
            }
            hang_after_code => {
                self.print_esc(65753i32);
            }
            floating_penalty_code => {
                self.print_esc(65754i32);
            }
            global_defs_code => {
                self.print_esc(65755i32);
            }
            cur_fam_code => {
                self.print_esc(65756i32);
            }
            escape_char_code => {
                self.print_esc(65757i32);
            }
            default_hyphen_char_code => {
                self.print_esc(65758i32);
            }
            default_skew_char_code => {
                self.print_esc(65759i32);
            }
            end_line_char_code => {
                self.print_esc(65760i32);
            }
            new_line_char_code => {
                self.print_esc(65761i32);
            }
            language_code => {
                self.print_esc(65762i32);
            }
            left_hyphen_min_code => {
                self.print_esc(65763i32);
            }
            right_hyphen_min_code => {
                self.print_esc(65764i32);
            }
            holding_inserts_code => {
                self.print_esc(65765i32);
            }
            error_context_lines_code => {
                self.print_esc(65766i32);
            }
            char_sub_def_min_code => {
                self.print_esc(65767i32);
            }
            char_sub_def_max_code => {
                self.print_esc(65768i32);
            }
            tracing_char_sub_def_code => {
                self.print_esc(65769i32);
            }
            tracing_stack_levels_code => {
                self.print_esc(65770i32);
            }
            partoken_context_code => {
                self.print_esc(65771i32);
            }
            show_stream_code => {
                self.print_esc(65772i32);
            }
            XeTeX_linebreak_penalty_code => {
                self.print_esc(65773i32);
            }
            XeTeX_protrude_chars_code => {
                self.print_esc(65774i32);
            }
            tracing_assigns_code => {
                // §1469
                self.print_esc(66850i32);
            }
            tracing_groups_code => {
                self.print_esc(66851i32);
            }
            tracing_ifs_code => {
                self.print_esc(66852i32);
            }
            tracing_scan_tokens_code => {
                self.print_esc(66853i32);
            }
            tracing_nesting_code => {
                self.print_esc(66854i32);
            }
            pre_display_direction_code => {
                self.print_esc(66855i32);
            }
            last_line_fit_code => {
                self.print_esc(66856i32);
            }
            saving_vdiscards_code => {
                self.print_esc(66857i32);
            }
            saving_hyph_codes_code => {
                self.print_esc(66858i32);
            }
            ignore_primitive_error_code => {
                self.print_esc(66859i32);
            }
            suppress_fontnotfound_error_code => {
                // §1510
                self.print_esc(66898i32);
            }
            75 => {
                self.print_esc(66899i32);
            }
            77 => {
                self.print_esc(66900i32);
            }
            78 => {
                self.print_esc(66901i32);
            }
            79 => {
                self.print_esc(66902i32);
            }
            76 => {
                self.print_esc(66903i32);
            }
            80 => {
                self.print_esc(66904i32);
            }
            83 => {
                self.print_esc(66905i32);
            }
            84 => {
                self.print_esc(66906i32);
            }
            85 => {
                self.print_esc(66907i32);
            }
            86 => {
                self.print_esc(66908i32);
            }
            synctex_code => {
                // §1704
                self.print_esc(66962i32);
            }
            _ => {
                // §263
                self.print(65775i32);
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
    // §267
    pub fn fix_date_and_time(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.sys_time); let mut __f1 = ::core::mem::take(&mut self.sys_day); let mut __f2 = ::core::mem::take(&mut self.sys_month); let mut __f3 = ::core::mem::take(&mut self.sys_year); let __r = self.date_and_time(&mut __f0, &mut __f1, &mut __f2, &mut __f3); self.sys_time = __f0; self.sys_day = __f1; self.sys_month = __f2; self.sys_year = __f3; __r };
        { let __v103 = self.sys_time; self.eqtb[crate::ix::U(((7892284i32) - 1) as usize)].set_int(__v103); }
        { let __v104 = self.sys_day; self.eqtb[crate::ix::U(((7892285i32) - 1) as usize)].set_int(__v104); }
        { let __v105 = self.sys_month; self.eqtb[crate::ix::U(((7892286i32) - 1) as usize)].set_int(__v105); }
        { let __v106 = self.sys_year; self.eqtb[crate::ix::U(((7892287i32) - 1) as usize)].set_int(__v106); }
    }

    /// \TeX\ is occasionally supposed to print diagnostic information that
    /// goes only into the transcript file, unless `tracing_online` is positive.
    /// Here are two routines that adjust the destination of print commands:
    // §271
    pub fn begin_diagnostic(&mut self) {
        self.old_setting = self.selector;
        if ((self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int() <= 0i32) && (self.selector == term_and_log)) {
            {
                self.selector = (self.selector).wrapping_sub(1i32);
                if (self.history == spotless) {
                    self.history = warning_issued;
                }
            }
        }
    }

    /// \TeX\ is occasionally supposed to print diagnostic information that
    /// goes only into the transcript file, unless `tracing_online` is positive.
    /// Here are two routines that adjust the destination of print commands:
    // §271
    pub fn end_diagnostic(&mut self, mut blank_line: bool) {
        self.print_nl(65626i32);
        if blank_line {
            self.print_ln();
        }
        self.selector = self.old_setting;
    }

    /// The final region of `eqtb` contains the dimension parameters defined
    /// here, and the `number_regs` \.{\\dimen} registers.
    // §273
    pub fn print_length_param(&mut self, mut n: i32) {
        match n {
            par_indent_code => {
                self.print_esc(65779i32);
            }
            math_surround_code => {
                self.print_esc(65780i32);
            }
            line_skip_limit_code => {
                self.print_esc(65781i32);
            }
            hsize_code => {
                self.print_esc(65782i32);
            }
            vsize_code => {
                self.print_esc(65783i32);
            }
            max_depth_code => {
                self.print_esc(65784i32);
            }
            split_max_depth_code => {
                self.print_esc(65785i32);
            }
            box_max_depth_code => {
                self.print_esc(65786i32);
            }
            hfuzz_code => {
                self.print_esc(65787i32);
            }
            vfuzz_code => {
                self.print_esc(65788i32);
            }
            delimiter_shortfall_code => {
                self.print_esc(65789i32);
            }
            null_delimiter_space_code => {
                self.print_esc(65790i32);
            }
            script_space_code => {
                self.print_esc(65791i32);
            }
            pre_display_size_code => {
                self.print_esc(65792i32);
            }
            display_width_code => {
                self.print_esc(65793i32);
            }
            display_indent_code => {
                self.print_esc(65794i32);
            }
            overfull_rule_code => {
                self.print_esc(65795i32);
            }
            hang_indent_code => {
                self.print_esc(65796i32);
            }
            h_offset_code => {
                self.print_esc(65797i32);
            }
            v_offset_code => {
                self.print_esc(65798i32);
            }
            emergency_stretch_code => {
                self.print_esc(65799i32);
            }
            pdf_page_width_code => {
                self.print_esc(65800i32);
            }
            pdf_page_height_code => {
                self.print_esc(65801i32);
            }
            _ => {
                self.print(65802i32);
            }
        }
    }

    /// The `print_cmd_chr` routine prints a symbolic interpretation of a
    /// command code and its modifier. This is used in certain `\.{You can\'t}'
    /// error messages, and in the implementation of diagnostic routines like
    /// \.{\\show}.
    /// The body of `print_cmd_chr` is a rather tedious listing of print
    /// commands, and most of it is essentially an inverse to the `primitive`
    /// routine that enters a \TeX\ primitive into `eqtb`. Therefore much of
    /// this procedure appears elsewhere in the program,
    /// together with the corresponding `primitive` calls.
    // §328
    pub fn print_cmd_chr(&mut self, mut cmd: quarterword, mut chr_code: halfword) {
        let mut n: i32 = 0; // §328
        let mut font_name_str: str_number = 0; // §328
        let mut quote_char: UTF16_code = 0; // §328
        match cmd {
            left_brace => {
                {
                    self.print(65876i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            right_brace => {
                {
                    self.print(65877i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            math_shift => {
                {
                    self.print(65878i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            mac_param => {
                {
                    self.print(65879i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            sup_mark => {
                {
                    self.print(65880i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            sub_mark => {
                {
                    self.print(65881i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            endv => {
                self.print(65882i32);
            }
            spacer => {
                {
                    self.print(65883i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            letter => {
                {
                    self.print(65884i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            other_char => {
                {
                    self.print(65885i32);
                    if (chr_code < 65536i32) {
                        self.print(chr_code);
                    } else {
                        self.print_char(chr_code);
                    }
                }
            }
            assign_glue | assign_mu_glue => {
                // §253
                if (chr_code < skip_base) {
                    self.print_skip_param((chr_code).wrapping_sub(1205764i32));
                } else {
                    if (chr_code < mu_skip_base) {
                        {
                            self.print_esc(65687i32);
                            self.print_int((chr_code).wrapping_sub(1205783i32));
                        }
                    } else {
                        {
                            self.print_esc(65688i32);
                            self.print_int((chr_code).wrapping_sub(1206039i32));
                        }
                    }
                }
            }
            assign_toks => {
                // §257
                if (chr_code >= toks_base) {
                    {
                        self.print_esc(65699i32);
                        self.print_int((chr_code).wrapping_sub(1206307i32));
                    }
                } else {
                    match chr_code {
                        output_routine_loc => {
                            self.print_esc(65690i32);
                        }
                        every_par_loc => {
                            self.print_esc(65691i32);
                        }
                        every_math_loc => {
                            self.print_esc(65692i32);
                        }
                        every_display_loc => {
                            self.print_esc(65693i32);
                        }
                        every_hbox_loc => {
                            self.print_esc(65694i32);
                        }
                        every_vbox_loc => {
                            self.print_esc(65695i32);
                        }
                        every_job_loc => {
                            self.print_esc(65696i32);
                        }
                        every_cr_loc => {
                            self.print_esc(65697i32);
                        }
                        every_eof_loc => {
                            // §1468
                            self.print_esc(66849i32);
                        }
                        XeTeX_inter_char_loc => {
                            self.print_esc(66747i32);
                        }
                        _ => {
                            // §257
                            self.print_esc(65698i32);
                        }
                    }
                }
            }
            assign_int => {
                // §265
                if (chr_code < count_base) {
                    self.print_param((chr_code).wrapping_sub(7892264i32));
                } else {
                    {
                        self.print_esc(65777i32);
                        self.print_int((chr_code).wrapping_sub(7892352i32));
                    }
                }
            }
            assign_dimen => {
                // §275
                if (chr_code < scaled_base) {
                    self.print_length_param((chr_code).wrapping_sub(9006720i32));
                } else {
                    {
                        self.print_esc(65803i32);
                        self.print_int((chr_code).wrapping_sub(9006743i32));
                    }
                }
            }
            accent => {
                // §296
                self.print_esc(65813i32);
            }
            advance => {
                self.print_esc(65814i32);
            }
            after_assignment => {
                self.print_esc(65815i32);
            }
            after_group => {
                self.print_esc(65816i32);
            }
            assign_font_dimen => {
                self.print_esc(65826i32);
            }
            begin_group => {
                self.print_esc(65817i32);
            }
            break_penalty => {
                self.print_esc(65845i32);
            }
            char_num => {
                self.print_esc(65818i32);
            }
            cs_name => {
                self.print_esc(65809i32);
            }
            def_font => {
                self.print_esc(65825i32);
            }
            delim_num => {
                if (chr_code == 1i32) {
                    self.print_esc(65821i32);
                } else {
                    self.print_esc(65819i32);
                }
            }
            divide => {
                self.print_esc(65822i32);
            }
            end_cs_name => {
                self.print_esc(65810i32);
            }
            end_group => {
                self.print_esc(65823i32);
            }
            ex_space => {
                self.print_esc(32i32);
            }
            expand_after => {
                if (chr_code == 0i32) {
                    self.print_esc(65824i32);
                } else {
                    // §1574
                    self.print_esc(66155i32);
                }
            }
            halign => {
                // §296
                self.print_esc(65827i32);
            }
            hrule => {
                self.print_esc(65828i32);
            }
            ignore_spaces => {
                if (chr_code == 0i32) {
                    self.print_esc(65829i32);
                } else {
                    self.print_esc(65806i32);
                }
            }
            insert => {
                self.print_esc(65618i32);
            }
            ital_corr => {
                self.print_esc(47i32);
            }
            mark => {
                {
                    self.print_esc(65641i32);
                    if (chr_code > 0i32) {
                        self.print_char(115i32);
                    }
                }
            }
            math_accent => {
                if (chr_code == 1i32) {
                    self.print_esc(65832i32);
                } else {
                    self.print_esc(65830i32);
                }
            }
            math_char_num => {
                if (chr_code == 2i32) {
                    self.print_esc(65837i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(65835i32);
                    } else {
                        self.print_esc(65833i32);
                    }
                }
            }
            math_choice => {
                self.print_esc(65838i32);
            }
            multiply => {
                self.print_esc(65839i32);
            }
            no_align => {
                self.print_esc(65840i32);
            }
            no_boundary => {
                self.print_esc(65841i32);
            }
            no_expand => {
                if (chr_code == 0i32) {
                    self.print_esc(65842i32);
                } else {
                    self.print_esc(65806i32);
                }
            }
            non_script => {
                self.print_esc(65623i32);
            }
            omit => {
                self.print_esc(65843i32);
            }
            radical => {
                if (chr_code == 1i32) {
                    self.print_esc(65849i32);
                } else {
                    self.print_esc(65847i32);
                }
            }
            read_to_cs => {
                if (chr_code == 0i32) {
                    self.print_esc(65850i32);
                } else {
                    // §1571
                    self.print_esc(66923i32);
                }
            }
            relax => {
                // §296
                self.print_esc(65851i32);
            }
            set_box => {
                self.print_esc(65852i32);
            }
            set_prev_graf => {
                self.print_esc(65846i32);
            }
            set_shape => {
                match chr_code {
                    par_shape_loc => {
                        self.print_esc(65844i32);
                    }
                    inter_line_penalties_loc => {
                        // §1676
                        self.print_esc(66958i32);
                    }
                    club_penalties_loc => {
                        self.print_esc(66959i32);
                    }
                    widow_penalties_loc => {
                        self.print_esc(66960i32);
                    }
                    display_widow_penalties_loc => {
                        self.print_esc(66961i32);
                    }
                    _ => {}
                }
            }
            the => {
                // §296
                if (chr_code == 0i32) {
                    self.print_esc(65853i32);
                } else {
                    // §1497
                    if (chr_code == 1i32) {
                        self.print_esc(66888i32);
                    } else {
                        self.print_esc(66889i32);
                    }
                }
            }
            toks_register => {
                // §1644
                {
                    self.print_esc(65699i32);
                    if (chr_code != mem_bot) {
                        self.print_sa_num(chr_code);
                    }
                }
            }
            vadjust => {
                // §296
                self.print_esc(65642i32);
            }
            valign => {
                if (chr_code == 0i32) {
                    self.print_esc(65854i32);
                } else {
                    // §1512
                    match chr_code {
                        begin_L_code => {
                            self.print_esc(66909i32);
                        }
                        end_L_code => {
                            self.print_esc(66910i32);
                        }
                        begin_R_code => {
                            self.print_esc(66911i32);
                        }
                        _ => {
                            self.print_esc(66912i32);
                        }
                    }
                }
            }
            vcenter => {
                // §296
                self.print_esc(65855i32);
            }
            vrule => {
                self.print_esc(65856i32);
            }
            partoken_name => {
                self.print_esc(65776i32);
            }
            par_end => {
                // §365
                self.print_esc(65919i32);
            }
            input => {
                // §411
                if (chr_code == 0i32) {
                    self.print_esc(65953i32);
                } else {
                    // §1559
                    if (chr_code == 2i32) {
                        self.print_esc(66921i32);
                    } else {
                        // §411
                        self.print_esc(65954i32);
                    }
                }
            }
            top_bot_mark => {
                // §419
                {
                    match (chr_code % marks_code) {
                        first_mark_code => {
                            self.print_esc(65956i32);
                        }
                        bot_mark_code => {
                            self.print_esc(65957i32);
                        }
                        split_first_mark_code => {
                            self.print_esc(65958i32);
                        }
                        split_bot_mark_code => {
                            self.print_esc(65959i32);
                        }
                        _ => {
                            self.print_esc(65955i32);
                        }
                    }
                    if (chr_code >= marks_code) {
                        self.print_char(115i32);
                    }
                }
            }
            register => {
                // §1643
                {
                    if ((chr_code < mem_bot) || (chr_code > lo_mem_stat_max)) {
                        cmd = (self.mem[crate::ix::U((chr_code) as usize)].hh().b0() / 64i32);
                    } else {
                        {
                            cmd = (chr_code).wrapping_sub(0i32);
                            chr_code = (268435455i32).wrapping_neg();
                        }
                    }
                    if (cmd == int_val) {
                        self.print_esc(65777i32);
                    } else {
                        if (cmd == dimen_val) {
                            self.print_esc(65803i32);
                        } else {
                            if (cmd == glue_val) {
                                self.print_esc(65687i32);
                            } else {
                                self.print_esc(65688i32);
                            }
                        }
                    }
                    if (chr_code != (268435455i32).wrapping_neg()) {
                        self.print_sa_num(chr_code);
                    }
                }
            }
            set_aux => {
                // §451
                if (chr_code == vmode) {
                    self.print_esc(66004i32);
                } else {
                    self.print_esc(66003i32);
                }
            }
            set_page_int => {
                if (chr_code == 0i32) {
                    self.print_esc(66005i32);
                } else {
                    // §1503
                    if (chr_code == 2i32) {
                        self.print_esc(66894i32);
                    } else {
                        // §451
                        self.print_esc(66006i32);
                    }
                }
            }
            set_box_dimen => {
                if (chr_code == width_offset) {
                    self.print_esc(66007i32);
                } else {
                    if (chr_code == height_offset) {
                        self.print_esc(66008i32);
                    } else {
                        self.print_esc(66009i32);
                    }
                }
            }
            last_item => {
                match chr_code {
                    int_val => {
                        self.print_esc(66010i32);
                    }
                    dimen_val => {
                        self.print_esc(66011i32);
                    }
                    glue_val => {
                        self.print_esc(66012i32);
                    }
                    input_line_no_code => {
                        self.print_esc(66013i32);
                    }
                    last_node_type_code => {
                        // §1453
                        self.print_esc(66804i32);
                    }
                    eTeX_version_code => {
                        self.print_esc(66805i32);
                    }
                    XeTeX_version_code => {
                        self.print_esc(66806i32);
                    }
                    XeTeX_count_glyphs_code => {
                        self.print_esc(66808i32);
                    }
                    XeTeX_count_variations_code => {
                        self.print_esc(66809i32);
                    }
                    XeTeX_variation_code => {
                        self.print_esc(66810i32);
                    }
                    XeTeX_find_variation_by_name_code => {
                        self.print_esc(66811i32);
                    }
                    XeTeX_variation_min_code => {
                        self.print_esc(66812i32);
                    }
                    XeTeX_variation_max_code => {
                        self.print_esc(66813i32);
                    }
                    XeTeX_variation_default_code => {
                        self.print_esc(66814i32);
                    }
                    XeTeX_count_features_code => {
                        self.print_esc(66815i32);
                    }
                    XeTeX_feature_code_code => {
                        self.print_esc(66816i32);
                    }
                    XeTeX_find_feature_by_name_code => {
                        self.print_esc(66817i32);
                    }
                    XeTeX_is_exclusive_feature_code => {
                        self.print_esc(66818i32);
                    }
                    XeTeX_count_selectors_code => {
                        self.print_esc(66819i32);
                    }
                    XeTeX_selector_code_code => {
                        self.print_esc(66820i32);
                    }
                    XeTeX_find_selector_by_name_code => {
                        self.print_esc(66821i32);
                    }
                    XeTeX_is_default_selector_code => {
                        self.print_esc(66822i32);
                    }
                    XeTeX_OT_count_scripts_code => {
                        self.print_esc(66826i32);
                    }
                    XeTeX_OT_count_languages_code => {
                        self.print_esc(66827i32);
                    }
                    XeTeX_OT_count_features_code => {
                        self.print_esc(66828i32);
                    }
                    XeTeX_OT_script_code => {
                        self.print_esc(66829i32);
                    }
                    XeTeX_OT_language_code => {
                        self.print_esc(66830i32);
                    }
                    XeTeX_OT_feature_code => {
                        self.print_esc(66831i32);
                    }
                    XeTeX_map_char_to_glyph_code => {
                        self.print_esc(66832i32);
                    }
                    XeTeX_glyph_index_code => {
                        self.print_esc(66833i32);
                    }
                    XeTeX_glyph_bounds_code => {
                        self.print_esc(66834i32);
                    }
                    XeTeX_font_type_code => {
                        self.print_esc(66836i32);
                    }
                    XeTeX_first_char_code => {
                        self.print_esc(66837i32);
                    }
                    XeTeX_last_char_code => {
                        self.print_esc(66838i32);
                    }
                    XeTeX_pdf_page_count_code => {
                        self.print_esc(66839i32);
                    }
                    current_group_level_code => {
                        // §1474
                        self.print_esc(66873i32);
                    }
                    current_group_type_code => {
                        self.print_esc(66874i32);
                    }
                    current_if_level_code => {
                        // §1477
                        self.print_esc(66875i32);
                    }
                    current_if_type_code => {
                        self.print_esc(66876i32);
                    }
                    current_if_branch_code => {
                        self.print_esc(66877i32);
                    }
                    font_char_wd_code => {
                        // §1480
                        self.print_esc(66878i32);
                    }
                    font_char_ht_code => {
                        self.print_esc(66879i32);
                    }
                    font_char_dp_code => {
                        self.print_esc(66880i32);
                    }
                    font_char_ic_code => {
                        self.print_esc(66881i32);
                    }
                    par_shape_length_code => {
                        // §1483
                        self.print_esc(66882i32);
                    }
                    par_shape_indent_code => {
                        self.print_esc(66883i32);
                    }
                    par_shape_dimen_code => {
                        self.print_esc(66884i32);
                    }
                    67 => {
                        // §1590
                        self.print_esc(66933i32);
                    }
                    68 => {
                        self.print_esc(66934i32);
                    }
                    69 => {
                        self.print_esc(66935i32);
                    }
                    70 => {
                        self.print_esc(66936i32);
                    }
                    glue_stretch_order_code => {
                        // §1613
                        self.print_esc(66940i32);
                    }
                    glue_shrink_order_code => {
                        self.print_esc(66941i32);
                    }
                    glue_stretch_code => {
                        self.print_esc(66942i32);
                    }
                    glue_shrink_code => {
                        self.print_esc(66943i32);
                    }
                    mu_to_glue_code => {
                        // §1617
                        self.print_esc(66944i32);
                    }
                    glue_to_mu_code => {
                        self.print_esc(66945i32);
                    }
                    pdf_last_x_pos_code => {
                        // §451
                        self.print_esc(66015i32);
                    }
                    pdf_last_y_pos_code => {
                        self.print_esc(66016i32);
                    }
                    elapsed_time_code => {
                        self.print_esc(66017i32);
                    }
                    pdf_shell_escape_code => {
                        self.print_esc(66018i32);
                    }
                    random_seed_code => {
                        self.print_esc(66019i32);
                    }
                    _ => {
                        self.print_esc(66014i32);
                    }
                }
            }
            convert => {
                // §504
                match chr_code {
                    number_code => {
                        self.print_esc(66087i32);
                    }
                    roman_numeral_code => {
                        self.print_esc(66088i32);
                    }
                    string_code => {
                        self.print_esc(66089i32);
                    }
                    meaning_code => {
                        self.print_esc(66090i32);
                    }
                    font_name_code => {
                        self.print_esc(66091i32);
                    }
                    eTeX_revision_code => {
                        self.print_esc(66106i32);
                    }
                    expanded_code => {
                        self.print_esc(66092i32);
                    }
                    left_margin_kern_code => {
                        self.print_esc(66093i32);
                    }
                    right_margin_kern_code => {
                        self.print_esc(66094i32);
                    }
                    pdf_creation_date_code => {
                        self.print_esc(66095i32);
                    }
                    pdf_file_mod_date_code => {
                        self.print_esc(66096i32);
                    }
                    pdf_file_size_code => {
                        self.print_esc(66097i32);
                    }
                    pdf_mdfive_sum_code => {
                        self.print_esc(66098i32);
                    }
                    pdf_file_dump_code => {
                        self.print_esc(66099i32);
                    }
                    pdf_strcmp_code => {
                        self.print_esc(66100i32);
                    }
                    uniform_deviate_code => {
                        self.print_esc(66101i32);
                    }
                    normal_deviate_code => {
                        self.print_esc(66102i32);
                    }
                    XeTeX_revision_code => {
                        // §1459
                        self.print_esc(66807i32);
                    }
                    XeTeX_variation_name_code => {
                        self.print_esc(66823i32);
                    }
                    XeTeX_feature_name_code => {
                        self.print_esc(66824i32);
                    }
                    XeTeX_selector_name_code => {
                        self.print_esc(66825i32);
                    }
                    XeTeX_glyph_name_code => {
                        self.print_esc(66835i32);
                    }
                    XeTeX_Uchar_code => {
                        self.print_esc(66104i32);
                    }
                    XeTeX_Ucharcat_code => {
                        self.print_esc(66105i32);
                    }
                    _ => {
                        // §504
                        self.print_esc(66103i32);
                    }
                }
            }
            if_test => {
                // §523
                {
                    if (chr_code >= unless_code) {
                        self.print_esc(66155i32);
                    }
                    match (chr_code % unless_code) {
                        if_cat_code => {
                            self.print_esc(66138i32);
                        }
                        if_int_code => {
                            self.print_esc(66139i32);
                        }
                        if_dim_code => {
                            self.print_esc(66140i32);
                        }
                        if_odd_code => {
                            self.print_esc(66141i32);
                        }
                        if_vmode_code => {
                            self.print_esc(66142i32);
                        }
                        if_hmode_code => {
                            self.print_esc(66143i32);
                        }
                        if_mmode_code => {
                            self.print_esc(66144i32);
                        }
                        if_inner_code => {
                            self.print_esc(66145i32);
                        }
                        if_void_code => {
                            self.print_esc(66146i32);
                        }
                        if_hbox_code => {
                            self.print_esc(66147i32);
                        }
                        if_vbox_code => {
                            self.print_esc(66148i32);
                        }
                        ifx_code => {
                            self.print_esc(66149i32);
                        }
                        if_eof_code => {
                            self.print_esc(66150i32);
                        }
                        if_true_code => {
                            self.print_esc(66151i32);
                        }
                        if_false_code => {
                            self.print_esc(66152i32);
                        }
                        if_case_code => {
                            self.print_esc(66153i32);
                        }
                        if_primitive_code => {
                            self.print_esc(66154i32);
                        }
                        if_def_code => {
                            // §1575
                            self.print_esc(66924i32);
                        }
                        if_cs_code => {
                            self.print_esc(66925i32);
                        }
                        if_font_char_code => {
                            self.print_esc(66926i32);
                        }
                        if_in_csname_code => {
                            self.print_esc(66927i32);
                        }
                        _ => {
                            // §523
                            self.print_esc(66137i32);
                        }
                    }
                }
            }
            fi_or_else => {
                // §527
                if (chr_code == fi_code) {
                    self.print_esc(66156i32);
                } else {
                    if (chr_code == or_code) {
                        self.print_esc(66157i32);
                    } else {
                        self.print_esc(66158i32);
                    }
                }
            }
            tab_mark => {
                // §829
                if (chr_code == span_code) {
                    self.print_esc(66319i32);
                } else {
                    {
                        self.print(66323i32);
                        if (chr_code < 65536i32) {
                            self.print(chr_code);
                        } else {
                            self.print_char(chr_code);
                        }
                    }
                }
            }
            car_ret => {
                if (chr_code == cr_code) {
                    self.print_esc(66320i32);
                } else {
                    self.print_esc(66321i32);
                }
            }
            set_page_dimen => {
                // §1038
                match chr_code {
                    0 => {
                        self.print_esc(66399i32);
                    }
                    1 => {
                        self.print_esc(66400i32);
                    }
                    2 => {
                        self.print_esc(66401i32);
                    }
                    3 => {
                        self.print_esc(66402i32);
                    }
                    4 => {
                        self.print_esc(66403i32);
                    }
                    5 => {
                        self.print_esc(66404i32);
                    }
                    6 => {
                        self.print_esc(66405i32);
                    }
                    _ => {
                        self.print_esc(66406i32);
                    }
                }
            }
            stop => {
                // §1107
                if (chr_code == 1i32) {
                    self.print_esc(66453i32);
                } else {
                    self.print_esc(65631i32);
                }
            }
            hskip => {
                // §1113
                match chr_code {
                    skip_code => {
                        self.print_esc(66454i32);
                    }
                    fil_code => {
                        self.print_esc(66455i32);
                    }
                    fill_code => {
                        self.print_esc(66456i32);
                    }
                    ss_code => {
                        self.print_esc(66457i32);
                    }
                    _ => {
                        self.print_esc(66458i32);
                    }
                }
            }
            vskip => {
                match chr_code {
                    skip_code => {
                        self.print_esc(66459i32);
                    }
                    fil_code => {
                        self.print_esc(66460i32);
                    }
                    fill_code => {
                        self.print_esc(66461i32);
                    }
                    ss_code => {
                        self.print_esc(66462i32);
                    }
                    _ => {
                        self.print_esc(66463i32);
                    }
                }
            }
            mskip => {
                self.print_esc(65624i32);
            }
            kern => {
                self.print_esc(65603i32);
            }
            mkern => {
                self.print_esc(65630i32);
            }
            hmove => {
                // §1126
                if (chr_code == 1i32) {
                    self.print_esc(66481i32);
                } else {
                    self.print_esc(66482i32);
                }
            }
            vmove => {
                if (chr_code == 1i32) {
                    self.print_esc(66483i32);
                } else {
                    self.print_esc(66484i32);
                }
            }
            make_box => {
                match chr_code {
                    box_code => {
                        self.print_esc(65701i32);
                    }
                    copy_code => {
                        self.print_esc(66485i32);
                    }
                    last_box_code => {
                        self.print_esc(66486i32);
                    }
                    vsplit_code => {
                        self.print_esc(66394i32);
                    }
                    vtop_code => {
                        self.print_esc(66487i32);
                    }
                    5 => {
                        self.print_esc(66396i32);
                    }
                    _ => {
                        self.print_esc(66488i32);
                    }
                }
            }
            leader_ship => {
                if (chr_code == a_leaders) {
                    self.print_esc(66490i32);
                } else {
                    if (chr_code == c_leaders) {
                        self.print_esc(66491i32);
                    } else {
                        if (chr_code == x_leaders) {
                            self.print_esc(66492i32);
                        } else {
                            self.print_esc(66489i32);
                        }
                    }
                }
            }
            start_par => {
                // §1143
                if (chr_code == 0i32) {
                    self.print_esc(66509i32);
                } else {
                    self.print_esc(66508i32);
                }
            }
            remove_item => {
                // §1162
                if (chr_code == glue_node) {
                    self.print_esc(66521i32);
                } else {
                    if (chr_code == kern_node) {
                        self.print_esc(66520i32);
                    } else {
                        self.print_esc(66519i32);
                    }
                }
            }
            un_hbox => {
                if (chr_code == copy_code) {
                    self.print_esc(66523i32);
                } else {
                    self.print_esc(66522i32);
                }
            }
            un_vbox => {
                if (chr_code == copy_code) {
                    self.print_esc(66525i32);
                } else {
                    // §1673
                    if (chr_code == last_box_code) {
                        self.print_esc(66956i32);
                    } else {
                        if (chr_code == vsplit_code) {
                            self.print_esc(66957i32);
                        } else {
                            // §1162
                            self.print_esc(66524i32);
                        }
                    }
                }
            }
            discretionary => {
                // §1169
                if (chr_code == 1i32) {
                    self.print_esc(45i32);
                } else {
                    self.print_esc(65639i32);
                }
            }
            eq_no => {
                // §1197
                if (chr_code == 1i32) {
                    self.print_esc(66557i32);
                } else {
                    self.print_esc(66556i32);
                }
            }
            math_comp => {
                // §1211
                match chr_code {
                    ord_noad => {
                        self.print_esc(66269i32);
                    }
                    op_noad => {
                        self.print_esc(66270i32);
                    }
                    bin_noad => {
                        self.print_esc(66271i32);
                    }
                    rel_noad => {
                        self.print_esc(66272i32);
                    }
                    open_noad => {
                        self.print_esc(66273i32);
                    }
                    close_noad => {
                        self.print_esc(66274i32);
                    }
                    punct_noad => {
                        self.print_esc(66275i32);
                    }
                    inner_noad => {
                        self.print_esc(66276i32);
                    }
                    under_noad => {
                        self.print_esc(66278i32);
                    }
                    _ => {
                        self.print_esc(66277i32);
                    }
                }
            }
            limit_switch => {
                if (chr_code == limits) {
                    self.print_esc(66282i32);
                } else {
                    if (chr_code == no_limits) {
                        self.print_esc(66283i32);
                    } else {
                        self.print_esc(66558i32);
                    }
                }
            }
            math_style => {
                // §1224
                self.print_style(chr_code);
            }
            above => {
                // §1233
                match chr_code {
                    over_code => {
                        self.print_esc(66579i32);
                    }
                    atop_code => {
                        self.print_esc(66580i32);
                    }
                    3 => {
                        self.print_esc(66581i32);
                    }
                    4 => {
                        self.print_esc(66582i32);
                    }
                    5 => {
                        self.print_esc(66583i32);
                    }
                    _ => {
                        self.print_esc(66578i32);
                    }
                }
            }
            left_right => {
                // §1243
                if (chr_code == left_noad) {
                    self.print_esc(66279i32);
                } else {
                    // §1508
                    if (chr_code == middle_noad) {
                        self.print_esc(66281i32);
                    } else {
                        // §1243
                        self.print_esc(66280i32);
                    }
                }
            }
            prefix => {
                // §1263
                if (chr_code == 1i32) {
                    self.print_esc(66603i32);
                } else {
                    if (chr_code == 2i32) {
                        self.print_esc(66604i32);
                    } else {
                        // §1582
                        if (chr_code == 8i32) {
                            self.print_esc(66617i32);
                        } else {
                            // §1263
                            self.print_esc(66605i32);
                        }
                    }
                }
            }
            def => {
                if (chr_code == 0i32) {
                    self.print_esc(66606i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(66607i32);
                    } else {
                        if (chr_code == 2i32) {
                            self.print_esc(66608i32);
                        } else {
                            self.print_esc(66609i32);
                        }
                    }
                }
            }
            let_ => {
                // §1274
                if (chr_code != normal) {
                    self.print_esc(66627i32);
                } else {
                    self.print_esc(66626i32);
                }
            }
            shorthand_def => {
                // §1277
                match chr_code {
                    char_def_code => {
                        self.print_esc(66628i32);
                    }
                    math_char_def_code => {
                        self.print_esc(66629i32);
                    }
                    XeTeX_math_char_def_code => {
                        self.print_esc(66633i32);
                    }
                    XeTeX_math_char_num_def_code => {
                        self.print_esc(66631i32);
                    }
                    count_def_code => {
                        self.print_esc(66634i32);
                    }
                    dimen_def_code => {
                        self.print_esc(66635i32);
                    }
                    skip_def_code => {
                        self.print_esc(66636i32);
                    }
                    mu_skip_def_code => {
                        self.print_esc(66637i32);
                    }
                    _ => {
                        self.print_esc(66638i32);
                    }
                }
            }
            char_given => {
                {
                    self.print_esc(65818i32);
                    self.print_hex(chr_code);
                }
            }
            math_given => {
                {
                    self.print_esc(65833i32);
                    self.print_hex(chr_code);
                }
            }
            XeTeX_math_given => {
                {
                    self.print_esc(65837i32);
                    { let __a107_0 = self.math_class_field(chr_code); self.print_hex(__a107_0) };
                    { let __a108_0 = self.math_fam_field(chr_code); self.print_hex(__a108_0) };
                    { let __a109_0 = self.math_char_field(chr_code); self.print_hex(__a109_0) };
                }
            }
            def_code => {
                // §1285
                if (chr_code == cat_code_base) {
                    self.print_esc(65707i32);
                } else {
                    if (chr_code == math_code_base) {
                        self.print_esc(65711i32);
                    } else {
                        if (chr_code == lc_code_base) {
                            self.print_esc(65708i32);
                        } else {
                            if (chr_code == uc_code_base) {
                                self.print_esc(65709i32);
                            } else {
                                if (chr_code == sf_code_base) {
                                    self.print_esc(65710i32);
                                } else {
                                    self.print_esc(65778i32);
                                }
                            }
                        }
                    }
                }
            }
            XeTeX_def_code => {
                if (chr_code == sf_code_base) {
                    self.print_esc(66645i32);
                } else {
                    if (chr_code == math_code_base) {
                        self.print_esc(66642i32);
                    } else {
                        if (chr_code == 5664041i32) {
                            self.print_esc(66644i32);
                        } else {
                            if (chr_code == del_code_base) {
                                self.print_esc(66647i32);
                            } else {
                                self.print_esc(66649i32);
                            }
                        }
                    }
                }
            }
            def_family => {
                self.print_size((chr_code).wrapping_sub(1206824i32));
            }
            hyph_data => {
                // §1305
                if (chr_code == 1i32) {
                    self.print_esc(66382i32);
                } else {
                    self.print_esc(66370i32);
                }
            }
            assign_font_int => {
                // §1309
                match chr_code {
                    0 => {
                        self.print_esc(66664i32);
                    }
                    1 => {
                        self.print_esc(66665i32);
                    }
                    lp_code_base => {
                        self.print_esc(66666i32);
                    }
                    rp_code_base => {
                        self.print_esc(66667i32);
                    }
                    _ => {}
                }
            }
            set_font => {
                // §1315
                {
                    self.print(66675i32);
                    font_name_str = self.font_name[crate::ix::U((chr_code) as usize)];
                    if ((self.font_area[crate::ix::U((chr_code) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((chr_code) as usize)] == otgr_font_flag)) {
                        {
                            quote_char = 34i32;
                            {
                                let __for_end_7 = (self.length(font_name_str)).wrapping_sub(1i32);
                                n = 0i32;
                                while n <= __for_end_7 {
                                    if (self.str_pool[crate::ix::U(((self.str_start[crate::ix::U(((font_name_str).wrapping_sub(65536i32)) as usize)]).wrapping_add(n)) as usize)] == 34i32) {
                                        quote_char = 39i32;
                                    }
                                    n = n.wrapping_add(1);
                                }
                            }
                            self.print_char(quote_char);
                            self.print(font_name_str);
                            self.print_char(quote_char);
                        }
                    } else {
                        self.print(font_name_str);
                    }
                    if (self.font_size[crate::ix::U((chr_code) as usize)] != self.font_dsize[crate::ix::U((chr_code) as usize)]) {
                        {
                            self.print(66121i32);
                            self.print_scaled(self.font_size[crate::ix::U((chr_code) as usize)]);
                            self.print(65689i32);
                        }
                    }
                }
            }
            set_interaction => {
                // §1317
                match chr_code {
                    batch_mode => {
                        self.print_esc(65554i32);
                    }
                    nonstop_mode => {
                        self.print_esc(65555i32);
                    }
                    scroll_mode => {
                        self.print_esc(65556i32);
                    }
                    _ => {
                        self.print_esc(66676i32);
                    }
                }
            }
            in_stream => {
                // §1327
                if (chr_code == 0i32) {
                    self.print_esc(66678i32);
                } else {
                    self.print_esc(66677i32);
                }
            }
            message => {
                // §1332
                if (chr_code == 0i32) {
                    self.print_esc(66679i32);
                } else {
                    self.print_esc(66680i32);
                }
            }
            case_shift => {
                // §1341
                if (chr_code == lc_code_base) {
                    self.print_esc(66686i32);
                } else {
                    self.print_esc(66687i32);
                }
            }
            xray => {
                // §1346
                match chr_code {
                    show_box_code => {
                        self.print_esc(66689i32);
                    }
                    show_the_code => {
                        self.print_esc(66690i32);
                    }
                    show_lists_code => {
                        self.print_esc(66691i32);
                    }
                    show_groups => {
                        // §1486
                        self.print_esc(66885i32);
                    }
                    show_tokens => {
                        // §1495
                        self.print_esc(66887i32);
                    }
                    show_ifs => {
                        // §1500
                        self.print_esc(66890i32);
                    }
                    _ => {
                        // §1346
                        self.print_esc(66688i32);
                    }
                }
            }
            undefined_cs => {
                // §1349
                self.print(66698i32);
            }
            call | long_call | outer_call | long_outer_call => {
                {
                    n = (cmd).wrapping_sub(114i32);
                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((chr_code) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
                        n = (n).wrapping_add(4i32);
                    }
                    if ((((n / 4i32)) % 2) != 0) {
                        self.print_esc(66617i32);
                    }
                    if (((n) % 2) != 0) {
                        self.print_esc(66603i32);
                    }
                    if ((((n / 2i32)) % 2) != 0) {
                        self.print_esc(66604i32);
                    }
                    if (n > 0i32) {
                        self.print_char(32i32);
                    }
                    self.print(66699i32);
                }
            }
            end_template => {
                self.print_esc(66700i32);
            }
            extension => {
                // §1401
                match chr_code {
                    open_node => {
                        self.print_esc(66736i32);
                    }
                    write_node => {
                        self.print_esc(65915i32);
                    }
                    close_node => {
                        self.print_esc(66737i32);
                    }
                    special_node => {
                        self.print_esc(66738i32);
                    }
                    immediate_code => {
                        self.print_esc(66739i32);
                    }
                    set_language_code => {
                        self.print_esc(66740i32);
                    }
                    pdf_save_pos_node => {
                        self.print_esc(66748i32);
                    }
                    reset_timer_code => {
                        self.print_esc(66741i32);
                    }
                    set_random_seed_code => {
                        self.print_esc(66742i32);
                    }
                    pic_file_code => {
                        self.print_esc(66743i32);
                    }
                    pdf_file_code => {
                        self.print_esc(66744i32);
                    }
                    glyph_code => {
                        self.print_esc(66745i32);
                    }
                    XeTeX_linebreak_locale_extension_code => {
                        self.print_esc(66746i32);
                    }
                    XeTeX_input_encoding_extension_code => {
                        self.print_esc(66749i32);
                    }
                    XeTeX_default_encoding_extension_code => {
                        self.print_esc(66750i32);
                    }
                    _ => {
                        self.print(66751i32);
                    }
                }
            }
            _ => {
                // §328
                self.print(65886i32);
            }
        }
    }

    /// @<Declare the procedure called `print_cmd_chr`
    // §1457
    pub fn not_aat_font_error(&mut self, mut cmd: i32, mut c: i32, mut f: i32) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66840i32);
        }
        self.print_cmd_chr(cmd, c);
        self.print(66841i32);
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        self.print(66842i32);
        self.error();
    }

    /// @<Declare the procedure called `print_cmd_chr`
    // §1457
    pub fn not_aat_gr_font_error(&mut self, mut cmd: i32, mut c: i32, mut f: i32) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66840i32);
        }
        self.print_cmd_chr(cmd, c);
        self.print(66841i32);
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        self.print(66843i32);
        self.error();
    }

    /// @<Declare the procedure called `print_cmd_chr`
    // §1457
    pub fn not_ot_font_error(&mut self, mut cmd: i32, mut c: i32, mut f: i32) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66840i32);
        }
        self.print_cmd_chr(cmd, c);
        self.print(66841i32);
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        self.print(66844i32);
        self.error();
    }

}
