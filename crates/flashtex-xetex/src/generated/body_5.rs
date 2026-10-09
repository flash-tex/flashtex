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
    /// Before an mlist is converted to an hlist, \TeX\ makes sure that
    /// the fonts in family~2 have enough parameters to be math-symbol
    /// fonts, and that the fonts in family~3 have enough parameters to be
    /// math-extension fonts. The math-symbol parameters are referred to by using the
    /// following macros, which take a size code as their parameter; for example,
    /// `num1(cur_size)` gives the value of the `num1` parameter for the current size.
    /// NB: the access functions here must all put the font \# into /f/ for mathsy().
    /// The accessors are defined with
    /// `define_mathsy_accessor(NAME)(fontdimen-number)(NAME)`
    /// because I can't see how to only give the name once, with WEB's limited
    /// macro capabilities. This seems a bit ugly, but it works.
    // §742
    pub fn delim1(&mut self, mut size_code: i32) -> scaled {
        let mut delim1: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 20i32);
        } else {
            rval = self.font_info[crate::ix::U(((20i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        delim1 = rval;
        delim1
    }

    /// Before an mlist is converted to an hlist, \TeX\ makes sure that
    /// the fonts in family~2 have enough parameters to be math-symbol
    /// fonts, and that the fonts in family~3 have enough parameters to be
    /// math-extension fonts. The math-symbol parameters are referred to by using the
    /// following macros, which take a size code as their parameter; for example,
    /// `num1(cur_size)` gives the value of the `num1` parameter for the current size.
    /// NB: the access functions here must all put the font \# into /f/ for mathsy().
    /// The accessors are defined with
    /// `define_mathsy_accessor(NAME)(fontdimen-number)(NAME)`
    /// because I can't see how to only give the name once, with WEB's limited
    /// macro capabilities. This seems a bit ugly, but it works.
    // §742
    pub fn delim2(&mut self, mut size_code: i32) -> scaled {
        let mut delim2: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 21i32);
        } else {
            rval = self.font_info[crate::ix::U(((21i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        delim2 = rval;
        delim2
    }

    /// Before an mlist is converted to an hlist, \TeX\ makes sure that
    /// the fonts in family~2 have enough parameters to be math-symbol
    /// fonts, and that the fonts in family~3 have enough parameters to be
    /// math-extension fonts. The math-symbol parameters are referred to by using the
    /// following macros, which take a size code as their parameter; for example,
    /// `num1(cur_size)` gives the value of the `num1` parameter for the current size.
    /// NB: the access functions here must all put the font \# into /f/ for mathsy().
    /// The accessors are defined with
    /// `define_mathsy_accessor(NAME)(fontdimen-number)(NAME)`
    /// because I can't see how to only give the name once, with WEB's limited
    /// macro capabilities. This seems a bit ugly, but it works.
    // §742
    pub fn axis_height(&mut self, mut size_code: i32) -> scaled {
        let mut axis_height: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 22i32);
        } else {
            rval = self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        axis_height = rval;
        axis_height
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn default_rule_thickness(&mut self) -> scaled {
        let mut default_rule_thickness: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 8i32);
        } else {
            rval = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        default_rule_thickness = rval;
        default_rule_thickness
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn big_op_spacing1(&mut self) -> scaled {
        let mut big_op_spacing1: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 9i32);
        } else {
            rval = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        big_op_spacing1 = rval;
        big_op_spacing1
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn big_op_spacing2(&mut self) -> scaled {
        let mut big_op_spacing2: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 10i32);
        } else {
            rval = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        big_op_spacing2 = rval;
        big_op_spacing2
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn big_op_spacing3(&mut self) -> scaled {
        let mut big_op_spacing3: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 11i32);
        } else {
            rval = self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        big_op_spacing3 = rval;
        big_op_spacing3
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn big_op_spacing4(&mut self) -> scaled {
        let mut big_op_spacing4: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 12i32);
        } else {
            rval = self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        big_op_spacing4 = rval;
        big_op_spacing4
    }

    /// The math-extension parameters have similar macros, but the size code is
    /// omitted (since it is always `cur_size` when we refer to such parameters).
    // §743
    pub fn big_op_spacing5(&mut self) -> scaled {
        let mut big_op_spacing5: scaled = 0;
        let mut f: i32 = 0; // §743
        let mut rval: scaled = 0; // §743
        f = self.eqtb[crate::ix::U((((1206827i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathex_param(f, 13i32);
        } else {
            rval = self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        big_op_spacing5 = rval;
        big_op_spacing5
    }

    /// Here is a function that returns a pointer to a rule node having a given
    /// thickness `t`. The rule will extend horizontally to the boundary of the vlist
    /// that eventually contains it.
    // §747
    pub fn fraction_rule(&mut self, mut t: scaled) -> halfword {
        let mut fraction_rule: halfword = 0;
        let mut p: halfword = 0; // §747
        p = self.new_rule();
        self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(t);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        fraction_rule = p;
        fraction_rule
    }

    /// The `overbar` function returns a pointer to a vlist box that consists of
    /// a given box `b`, above which has been placed a kern of height `k` under a
    /// fraction rule of thickness `t` under additional space of height `t`.
    // §748
    pub fn overbar(&mut self, mut b: halfword, mut k: scaled, mut t: scaled) -> halfword {
        let mut overbar: halfword = 0;
        let mut p: halfword = 0; // §748
        let mut q: halfword = 0; // §748
        p = self.new_kern(k);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(b);
        q = self.fraction_rule(t);
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
        p = self.new_kern(t);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
        overbar = self.vpackage(p, 0i32, additional, max_dimen);
        overbar
    }

    /// Here is a subroutine that creates a new box, whose list contains a
    /// single character, and whose width includes the italic correction for
    /// that character. The height or depth of the box will be negative, if
    /// the height or depth of the character is negative; thus, this routine
    /// may deliver a slightly different result than `hpack` would produce.
    /// @<Declare subprocedures for `var_delimiter`
    // §752
    pub fn char_box(&mut self, mut f: internal_font_number, mut c: i32) -> halfword {
        let mut char_box: halfword = 0;
        let mut q: four_quarters = four_quarters::default(); // §752
        let mut hd: eight_bits = 0; // §752
        let mut b: halfword = 0; // §752
        let mut p: halfword = 0; // §752
        if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
            {
                b = self.new_null_box();
                p = self.new_native_character(f, c);
                self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                { let __v679 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v679); }
                { let __v680 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v680); }
                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() < 0i32) {
                    self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(0i32);
                } else {
                    { let __v681 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v681); }
                }
            }
        } else {
            {
                q = { let __s682 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s682)] }.qqqq();
                hd = q.b1();
                b = self.new_null_box();
                { let __v683 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add((q.b2() / 4i32))) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v683); }
                { let __v684 = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v684); }
                { let __v685 = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v685); }
                p = self.get_avail();
                self.mem[crate::ix::U((p) as usize)].set_hh_b1(c);
                self.mem[crate::ix::U((p) as usize)].set_hh_b0(f);
            }
        }
        self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        char_box = b;
        char_box
    }

    /// When we build an extensible character, it's handy to have the
    /// following subroutine, which puts a given character on top
    /// of the characters already in box `b`:
    /// @<Declare subprocedures for `var_delimiter`
    // §754
    pub fn stack_into_box(&mut self, mut b: halfword, mut f: internal_font_number, mut c: quarterword) {
        let mut p: halfword = 0; // §754
        p = self.char_box(f, c);
        { let __v686 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v686); }
        self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        { let __v687 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v687); }
    }

    /// Another handy subroutine computes the height plus depth of
    /// a given character:
    /// @<Declare subprocedures for `var_delimiter`
    // §755
    pub fn height_plus_depth(&mut self, mut f: internal_font_number, mut c: quarterword) -> scaled {
        let mut height_plus_depth: scaled = 0;
        let mut q: four_quarters = four_quarters::default(); // §755
        let mut hd: eight_bits = 0; // §755
        q = { let __s688 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s688)] }.qqqq();
        hd = q.b1();
        height_plus_depth = (self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int());
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
    // §749
    pub fn stack_glyph_into_box(&mut self, mut b: halfword, mut f: internal_font_number, mut g: i32) {
        let mut p: halfword = 0; // §749
        let mut q: halfword = 0; // §749
        p = self.get_node(glyph_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(glyph_node);
        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b1(f);
        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(g);
        self.set_native_glyph_metrics(p, ((1i32) != 0));
        if (self.mem[crate::ix::U((b) as usize)].hh().b0() == hlist_node) {
            {
                q = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                if (q == (268435455i32).wrapping_neg()) {
                    self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                } else {
                    {
                        while (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        if (self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int() < self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()) {
                            { let __v689 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v689); }
                        }
                        if (self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int() < self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()) {
                            { let __v690 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v690); }
                        }
                    }
                }
            }
        } else {
            {
                { let __v691 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v691); }
                self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                { let __v692 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v692); }
                if (self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int() < self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()) {
                    { let __v693 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v693); }
                }
            }
        }
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
    // §749
    pub fn stack_glue_into_box(&mut self, mut b: halfword, mut min: scaled, mut max: scaled) {
        let mut p: halfword = 0; // §749
        let mut q: halfword = 0; // §749
        q = self.new_spec(zero_glue);
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(min);
        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int((max).wrapping_sub(min));
        p = self.new_glue(q);
        if (self.mem[crate::ix::U((b) as usize)].hh().b0() == hlist_node) {
            {
                q = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                if (q == (268435455i32).wrapping_neg()) {
                    self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                } else {
                    {
                        while (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                    }
                }
            }
        } else {
            {
                { let __v694 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v694); }
                self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                { let __v695 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v695); }
                { let __v696 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v696); }
            }
        }
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
    // §749
    pub fn build_opentype_assembly(&mut self, mut f: internal_font_number, mut a: void_pointer, mut s: scaled, mut horiz: bool) -> halfword {
        let mut build_opentype_assembly: halfword = 0;
        let mut b: halfword = 0; // §749
        let mut n: i32 = 0; // §749
        let mut i: i32 = 0; // §749
        let mut j: i32 = 0; // §749
        let mut g: i32 = 0; // §749
        let mut p: halfword = 0; // §749
        let mut s_max: scaled = 0; // §749
        let mut o: scaled = 0; // §749
        let mut oo: scaled = 0; // §749
        let mut prev_o: scaled = 0; // §749
        let mut min_o: scaled = 0; // §749
        let mut no_extenders: bool = false; // §749
        let mut nat: scaled = 0; // §749
        let mut str: scaled = 0; // §749
        b = self.new_null_box();
        if horiz {
            self.mem[crate::ix::U((b) as usize)].set_hh_b0(hlist_node);
        } else {
            self.mem[crate::ix::U((b) as usize)].set_hh_b0(vlist_node);
        }
        n = (1i32).wrapping_neg();
        no_extenders = true;
        min_o = self.ot_min_connector_overlap(f);
        loop {
            n = (n).wrapping_add(1i32);
            s_max = 0i32;
            prev_o = 0i32;
            {
                let __for_end_3 = (self.ot_part_count(a)).wrapping_sub(1i32);
                i = 0i32;
                while i <= __for_end_3 {
                    {
                        if self.ot_part_is_extender(a, i) {
                            {
                                no_extenders = false;
                                {
                                    let __for_end_8 = n;
                                    j = 1i32;
                                    while j <= __for_end_8 {
                                        {
                                            o = self.ot_part_start_connector(f, a, i);
                                            if (min_o < o) {
                                                o = min_o;
                                            }
                                            if (prev_o < o) {
                                                o = prev_o;
                                            }
                                            s_max = ((s_max).wrapping_sub(o)).wrapping_add(self.ot_part_full_advance(f, a, i));
                                            prev_o = self.ot_part_end_connector(f, a, i);
                                        }
                                        j = j.wrapping_add(1);
                                    }
                                }
                            }
                        } else {
                            {
                                o = self.ot_part_start_connector(f, a, i);
                                if (min_o < o) {
                                    o = min_o;
                                }
                                if (prev_o < o) {
                                    o = prev_o;
                                }
                                s_max = ((s_max).wrapping_sub(o)).wrapping_add(self.ot_part_full_advance(f, a, i));
                                prev_o = self.ot_part_end_connector(f, a, i);
                            }
                        }
                    }
                    i = i.wrapping_add(1);
                }
            }
            if ((s_max >= s) || no_extenders) { break; }
        }
        prev_o = 0i32;
        {
            let __for_end_2 = (self.ot_part_count(a)).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                {
                    if self.ot_part_is_extender(a, i) {
                        {
                            {
                                let __for_end_7 = n;
                                j = 1i32;
                                while j <= __for_end_7 {
                                    {
                                        o = self.ot_part_start_connector(f, a, i);
                                        if (prev_o < o) {
                                            o = prev_o;
                                        }
                                        oo = o;
                                        if (min_o < o) {
                                            o = min_o;
                                        }
                                        if (oo > 0i32) {
                                            self.stack_glue_into_box(b, (oo).wrapping_neg(), (o).wrapping_neg());
                                        }
                                        g = self.ot_part_glyph(a, i);
                                        self.stack_glyph_into_box(b, f, g);
                                        prev_o = self.ot_part_end_connector(f, a, i);
                                    }
                                    j = j.wrapping_add(1);
                                }
                            }
                        }
                    } else {
                        {
                            o = self.ot_part_start_connector(f, a, i);
                            if (prev_o < o) {
                                o = prev_o;
                            }
                            oo = o;
                            if (min_o < o) {
                                o = min_o;
                            }
                            if (oo > 0i32) {
                                self.stack_glue_into_box(b, (oo).wrapping_neg(), (o).wrapping_neg());
                            }
                            g = self.ot_part_glyph(a, i);
                            self.stack_glyph_into_box(b, f, g);
                            prev_o = self.ot_part_end_connector(f, a, i);
                        }
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        p = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
        nat = 0i32;
        str = 0i32;
        while (p != (268435455i32).wrapping_neg()) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) {
                    {
                        if horiz {
                            nat = (nat).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                        } else {
                            nat = ((nat).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                        }
                    }
                } else {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) {
                        {
                            nat = (nat).wrapping_add(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int());
                            str = (str).wrapping_add(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(2i32)) as usize)].int());
                        }
                    }
                }
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        o = 0i32;
        if ((s > nat) && (str > 0i32)) {
            {
                o = (s).wrapping_sub(nat);
                if (o > str) {
                    o = str;
                }
                self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                self.mem[crate::ix::U(((b).wrapping_add(6i32)) as usize)].set_gr((((o) as f64) / ((str) as f64)));
                if horiz {
                    { let __v697 = (nat).wrapping_add(crate::system::pas_round((((str) as f64) * self.mem[crate::ix::U(((b).wrapping_add(6i32)) as usize)].gr()))); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v697); }
                } else {
                    { let __v698 = (nat).wrapping_add(crate::system::pas_round((((str) as f64) * self.mem[crate::ix::U(((b).wrapping_add(6i32)) as usize)].gr()))); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v698); }
                }
            }
        } else {
            if horiz {
                self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(nat);
            } else {
                self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(nat);
            }
        }
        build_opentype_assembly = b;
        build_opentype_assembly
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
    // §749
    pub fn var_delimiter(&mut self, mut d: halfword, mut s: i32, mut v: scaled) -> halfword {
        let mut var_delimiter: halfword = 0;
        let mut b: halfword = 0; // §749
        let mut ot_assembly_ptr: void_pointer = 0; // §749
        let mut f: internal_font_number = 0; // §749
        let mut g: internal_font_number = 0; // §749
        let mut c: quarterword = 0; // §749
        let mut x: quarterword = 0; // §749
        let mut y: quarterword = 0; // §749
        let mut m: i32 = 0; // §749
        let mut n: i32 = 0; // §749
        let mut u: scaled = 0; // §749
        let mut w: scaled = 0; // §749
        let mut q: four_quarters = four_quarters::default(); // §749
        let mut hd: eight_bits = 0; // §749
        let mut r: four_quarters = four_quarters::default(); // §749
        let mut z: i32 = 0; // §749
        let mut large_attempt: bool = false; // §749
        'l_found_f: {
            f = null_font;
            w = 0i32;
            large_attempt = false;
            z = (self.mem[crate::ix::U((d) as usize)].qqqq().b0() % 256i32);
            x = (self.mem[crate::ix::U((d) as usize)].qqqq().b1()).wrapping_add(((self.mem[crate::ix::U((d) as usize)].qqqq().b0() / 256i32)).wrapping_mul(65536i32));
            ot_assembly_ptr = nil;
            while true {
                {
                    // §750
                    if ((z != 0i32) || (x != min_quarterword)) {
                        {
                            z = ((z).wrapping_add(s)).wrapping_add(256i32);
                            loop {
                                z = (z).wrapping_sub(256i32);
                                g = self.eqtb[crate::ix::U((((math_font_base).wrapping_add(z)) - 1) as usize)].hh().rh();
                                if (g != null_font) {
                                    // §751
                                    if ((self.font_area[crate::ix::U((g) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((g) as usize)])) {
                                        {
                                            x = self.map_char_to_glyph(g, x);
                                            f = g;
                                            c = x;
                                            w = 0i32;
                                            n = 0i32;
                                            loop {
                                                y = { let mut __f3 = ::core::mem::take(&mut u); let __r = self.get_ot_math_variant(g, x, n, &mut __f3, 0i32); u = __f3; __r };
                                                if (u > w) {
                                                    {
                                                        c = y;
                                                        w = u;
                                                        if (u >= v) {
                                                            break 'l_found_f;
                                                        }
                                                    }
                                                }
                                                n = (n).wrapping_add(1i32);
                                                if (u < 0i32) { break; }
                                            }
                                            ot_assembly_ptr = self.get_ot_assembly_ptr(g, x, 0i32);
                                            if (ot_assembly_ptr != nil) {
                                                break 'l_found_f;
                                            }
                                        }
                                    } else {
                                        {
                                            y = x;
                                            if ((y >= self.font_bc[crate::ix::U((g) as usize)]) && (y <= self.font_ec[crate::ix::U((g) as usize)])) {
                                                {
                                                    'l_continue_b: loop {
                                                        q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((g) as usize)]).wrapping_add(y)) as usize)].qqqq();
                                                        if (q.b0() > min_quarterword) {
                                                            {
                                                                if ((q.b2() % 4i32) == ext_tag) {
                                                                    {
                                                                        f = g;
                                                                        c = y;
                                                                        break 'l_found_f;
                                                                    }
                                                                }
                                                                hd = q.b1();
                                                                u = (self.font_info[crate::ix::U(((self.height_base[crate::ix::U((g) as usize)]).wrapping_add((hd / 16i32))) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((g) as usize)]).wrapping_add((hd % 16i32))) as usize)].int());
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
                                                                if ((q.b2() % 4i32) == list_tag) {
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
                                }
                                if (z < script_size) { break; }
                            }
                        }
                    }
                    // §749
                    if large_attempt {
                        break 'l_found_f;
                    }
                    large_attempt = true;
                    z = (self.mem[crate::ix::U((d) as usize)].qqqq().b2() % 256i32);
                    x = (self.mem[crate::ix::U((d) as usize)].qqqq().b3()).wrapping_add(((self.mem[crate::ix::U((d) as usize)].qqqq().b2() / 256i32)).wrapping_mul(65536i32));
                }
            }
        }
        if (f != null_font) {
            {
                if (!((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((f) as usize)]))) {
                    // §753
                    if ((q.b2() % 4i32) == ext_tag) {
                        // §756
                        {
                            b = self.new_null_box();
                            self.mem[crate::ix::U((b) as usize)].set_hh_b0(vlist_node);
                            r = self.font_info[crate::ix::U(((self.exten_base[crate::ix::U((f) as usize)]).wrapping_add(q.b3())) as usize)].qqqq();
                            // §757
                            c = r.b3();
                            u = self.height_plus_depth(f, c);
                            w = 0i32;
                            q = { let __s699 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s699)] }.qqqq();
                            { let __v700 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add((q.b2() / 4i32))) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v700); }
                            c = r.b2();
                            if (c != min_quarterword) {
                                w = (w).wrapping_add(self.height_plus_depth(f, c));
                            }
                            c = r.b1();
                            if (c != min_quarterword) {
                                w = (w).wrapping_add(self.height_plus_depth(f, c));
                            }
                            c = r.b0();
                            if (c != min_quarterword) {
                                w = (w).wrapping_add(self.height_plus_depth(f, c));
                            }
                            n = 0i32;
                            if (u > 0i32) {
                                while (w < v) {
                                    {
                                        w = (w).wrapping_add(u);
                                        n = (n).wrapping_add(1i32);
                                        if (r.b1() != min_quarterword) {
                                            w = (w).wrapping_add(u);
                                        }
                                    }
                                }
                            }
                            // §756
                            c = r.b2();
                            if (c != min_quarterword) {
                                self.stack_into_box(b, f, c);
                            }
                            c = r.b3();
                            {
                                let __for_end_7 = n;
                                m = 1i32;
                                while m <= __for_end_7 {
                                    self.stack_into_box(b, f, c);
                                    m = m.wrapping_add(1);
                                }
                            }
                            c = r.b1();
                            if (c != min_quarterword) {
                                {
                                    self.stack_into_box(b, f, c);
                                    c = r.b3();
                                    {
                                        let __for_end_9 = n;
                                        m = 1i32;
                                        while m <= __for_end_9 {
                                            self.stack_into_box(b, f, c);
                                            m = m.wrapping_add(1);
                                        }
                                    }
                                }
                            }
                            c = r.b0();
                            if (c != min_quarterword) {
                                self.stack_into_box(b, f, c);
                            }
                            { let __v701 = (w).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v701); }
                        }
                    } else {
                        // §753
                        b = self.char_box(f, c);
                    }
                } else {
                    // §749
                    {
                        if (ot_assembly_ptr != nil) {
                            b = self.build_opentype_assembly(f, ot_assembly_ptr, v, ((0i32) != 0));
                        } else {
                            {
                                b = self.new_null_box();
                                self.mem[crate::ix::U((b) as usize)].set_hh_b0(vlist_node);
                                { let __v702 = self.get_node(glyph_node_size); self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(__v702); }
                                { let __ix703 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix703) as usize)].set_hh_b0(whatsit_node); }
                                { let __ix704 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((__ix704) as usize)].set_hh_b1(glyph_node); }
                                { let __ix705 = (self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(4i32); self.mem[crate::ix::U((__ix705) as usize)].set_qqqq_b1(f); }
                                { let __ix706 = (self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(4i32); self.mem[crate::ix::U((__ix706) as usize)].set_qqqq_b2(c); }
                                self.set_native_glyph_metrics(self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(), ((1i32) != 0));
                                { let __v707 = self.mem[crate::ix::U(((self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v707); }
                                { let __v708 = self.mem[crate::ix::U(((self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v708); }
                                { let __v709 = self.mem[crate::ix::U(((self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v709); }
                            }
                        }
                    }
                }
            }
        } else {
            {
                b = self.new_null_box();
                { let __v710 = self.eqtb[crate::ix::U(((9006731i32) - 1) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v710); }
            }
        }
        { let __v711 = (self.half((self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.axis_height(s)); self.mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int(__v711); }
        self.free_ot_assembly(ot_assembly_ptr);
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
    // §758
    pub fn rebox(&mut self, mut b: halfword, mut w: scaled) -> halfword {
        let mut rebox: halfword = 0;
        let mut p: halfword = 0; // §758
        let mut f: internal_font_number = 0; // §758
        let mut v: scaled = 0; // §758
        if ((self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int() != w) && (self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
            {
                if (self.mem[crate::ix::U((b) as usize)].hh().b0() == vlist_node) {
                    b = self.hpack(b, 0i32, additional);
                }
                p = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                if ((p >= self.hi_mem_min) && (self.mem[crate::ix::U((p) as usize)].hh().rh() == (268435455i32).wrapping_neg())) {
                    {
                        f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                        v = { let __s713 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s712 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s712)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s713)] }.int();
                        if (v != self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()) {
                            { let __v714 = self.new_kern((self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()).wrapping_sub(v)); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v714); }
                        }
                    }
                }
                self.free_node(b, box_node_size);
                b = self.new_glue(ss_glue);
                self.mem[crate::ix::U((b) as usize)].set_hh_rh(p);
                while (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
                { let __v715 = self.new_glue(ss_glue); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v715); }
                rebox = self.hpack(b, w, exactly);
            }
        } else {
            {
                self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(w);
                rebox = b;
            }
        }
        rebox
    }

    /// Here is a subroutine that creates a new glue specification from another
    /// one that is expressed in `\.{mu}', given the value of the math unit.
    // §759
    pub fn math_glue(&mut self, mut g: halfword, mut m: scaled) -> halfword {
        let mut math_glue: halfword = 0;
        let mut p: halfword = 0; // §759
        let mut n: i32 = 0; // §759
        let mut f: scaled = 0; // §759
        n = self.x_over_n(m, 65536i32);
        f = self.remainder;
        if (f < 0i32) {
            {
                n = (n).wrapping_sub(1i32);
                f = (f).wrapping_add(65536i32);
            }
        }
        p = self.get_node(glue_spec_size);
        { let __v716 = { let __a717_0 = n; let __a717_1 = self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(); let __a717_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a717_3 = 1073741823i32; self.mult_and_add(__a717_0, __a717_1, __a717_2, __a717_3) }; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v716); }
        { let __v718 = self.mem[crate::ix::U((g) as usize)].hh().b0(); self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v718); }
        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == normal) {
            { let __v719 = { let __a720_0 = n; let __a720_1 = self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); let __a720_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(), f, 65536i32); let __a720_3 = 1073741823i32; self.mult_and_add(__a720_0, __a720_1, __a720_2, __a720_3) }; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v719); }
        } else {
            { let __v721 = self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v721); }
        }
        { let __v722 = self.mem[crate::ix::U((g) as usize)].hh().b1(); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v722); }
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal) {
            { let __v723 = { let __a724_0 = n; let __a724_1 = self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); let __a724_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(), f, 65536i32); let __a724_3 = 1073741823i32; self.mult_and_add(__a724_0, __a724_1, __a724_2, __a724_3) }; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v723); }
        } else {
            { let __v725 = self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v725); }
        }
        math_glue = p;
        math_glue
    }

    /// The `math_kern` subroutine removes `mu_glue` from a kern node, given
    /// the value of the math unit.
    // §760
    pub fn math_kern(&mut self, mut p: halfword, mut m: scaled) {
        let mut n: i32 = 0; // §760
        let mut f: scaled = 0; // §760
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == mu_glue) {
            {
                n = self.x_over_n(m, 65536i32);
                f = self.remainder;
                if (f < 0i32) {
                    {
                        n = (n).wrapping_sub(1i32);
                        f = (f).wrapping_add(65536i32);
                    }
                }
                { let __v726 = { let __a727_0 = n; let __a727_1 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); let __a727_2 = self.xn_over_d(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a727_3 = 1073741823i32; self.mult_and_add(__a727_0, __a727_1, __a727_2, __a727_3) }; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v726); }
                self.mem[crate::ix::U((p) as usize)].set_hh_b1(explicit);
            }
        }
    }

    /// Sometimes it is necessary to destroy an mlist. The following
    /// subroutine empties the current list, assuming that `abs(mode)=mmode`.
    // §761
    pub fn flush_math(&mut self) {
        self.flush_node_list(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh());
        self.flush_node_list(self.cur_list.aux_field.int());
        { let __ix728 = self.cur_list.head_field; self.mem[crate::ix::U((__ix728) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.aux_field.set_int((268435455i32).wrapping_neg());
    }

    /// The recursion in `mlist_to_hlist` is due primarily to a subroutine
    /// called `clean_box` that puts a given noad field into a box using a given
    /// math style; `mlist_to_hlist` can call `clean_box`, which can call
    /// `mlist_to_hlist`.
    /// The box returned by `clean_box` is ``clean'' in the
    /// sense that its `shift_amount` is zero.
    // §763
    pub fn clean_box(&mut self, mut p: halfword, mut s: small_number) -> halfword {
        let mut clean_box: halfword = 0;
        let mut q: halfword = 0; // §763
        let mut save_style: small_number = 0; // §763
        let mut x: halfword = 0; // §763
        let mut r: halfword = 0; // §763
        'l_found_f: {
            match self.mem[crate::ix::U((p) as usize)].hh().rh() {
                math_char => {
                    {
                        self.cur_mlist = self.new_noad();
                        { let __ix729 = (self.cur_mlist).wrapping_add(1i32); let __v730 = self.mem[crate::ix::U((p) as usize)]; self.mem[crate::ix::U((__ix729) as usize)] = __v730; }
                    }
                }
                sub_box => {
                    {
                        q = self.mem[crate::ix::U((p) as usize)].hh().lh();
                        break 'l_found_f;
                    }
                }
                sub_mlist => {
                    self.cur_mlist = self.mem[crate::ix::U((p) as usize)].hh().lh();
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
            q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
            self.cur_style = save_style;
            // §746
            {
                if (self.cur_style < script_style) {
                    self.cur_size = text_size;
                } else {
                    self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                }
                self.cur_mu = { let __a731_0 = self.math_quad(self.cur_size); let __a731_1 = 18i32; self.x_over_n(__a731_0, __a731_1) };
            }
        }
        // §763
        if ((q >= self.hi_mem_min) || (q == (268435455i32).wrapping_neg())) {
            x = self.hpack(q, 0i32, additional);
        } else {
            if (((self.mem[crate::ix::U((q) as usize)].hh().rh() == (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((q) as usize)].hh().b0() <= vlist_node)) && (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].int() == 0i32)) {
                x = q;
            } else {
                x = self.hpack(q, 0i32, additional);
            }
        }
        // §764
        q = self.mem[crate::ix::U(((x).wrapping_add(5i32)) as usize)].hh().rh();
        if (q >= self.hi_mem_min) {
            {
                r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                if (r != (268435455i32).wrapping_neg()) {
                    if (self.mem[crate::ix::U((r) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                        if (!(r >= self.hi_mem_min)) {
                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == kern_node) {
                                {
                                    self.free_node(r, medium_node_size);
                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                }
                            }
                        }
                    }
                }
            }
        }
        // §763
        clean_box = x;
        clean_box
    }

    /// It is convenient to have a procedure that converts a `math_char`
    /// field to an ``unpacked'' form. The `fetch` routine sets `cur_f`, `cur_c`,
    /// and `cur_i` to the font code, character code, and character information bytes of
    /// a given noad field. It also takes care of issuing error messages for
    /// nonexistent characters; in such cases, `char_exists(cur_i)` will be `false`
    /// after `fetch` has acted, and the field will also have been reset to `empty`.
    // §765
    pub fn fetch(&mut self, mut a: halfword) {
        self.cur_c = self.cast_to_ushort(self.mem[crate::ix::U((a) as usize)].hh().b1());
        self.cur_f = self.eqtb[crate::ix::U(((((math_font_base).wrapping_add((self.mem[crate::ix::U((a) as usize)].hh().b0() % 256i32))).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        self.cur_c = (self.cur_c).wrapping_add(((self.mem[crate::ix::U((a) as usize)].hh().b0() / 256i32)).wrapping_mul(65536i32));
        if (self.cur_f == null_font) {
            // §766
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
                self.print_size(self.cur_size);
                self.print_char(32i32);
                self.print_int((self.mem[crate::ix::U((a) as usize)].hh().b0() % 256i32));
                self.print(66305i32);
                self.print(self.cur_c);
                self.print_char(41i32);
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 66306i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66307i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66308i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66309i32;
                }
                self.error();
                self.cur_i = self.null_character;
                self.mem[crate::ix::U((a) as usize)].set_hh_rh(empty);
            }
        } else {
            // §765
            if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag)) {
                {
                    self.cur_i = self.null_character;
                }
            } else {
                {
                    if ((self.cur_c >= self.font_bc[crate::ix::U((self.cur_f) as usize)]) && (self.cur_c <= self.font_ec[crate::ix::U((self.cur_f) as usize)])) {
                        self.cur_i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_c)) as usize)].qqqq();
                    } else {
                        self.cur_i = self.null_character;
                    }
                    if (!(self.cur_i.b0() > min_quarterword)) {
                        {
                            self.char_warning(self.cur_f, self.cur_c);
                            self.mem[crate::ix::U((a) as usize)].set_hh_rh(empty);
                            self.cur_i = self.null_character;
                        }
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
    // §777
    pub fn make_over(&mut self, mut q: halfword) {
        { let __v732 = { let __a733_0 = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32)); let __a733_1 = (3i32).wrapping_mul(self.default_rule_thickness()); let __a733_2 = self.default_rule_thickness(); self.overbar(__a733_0, __a733_1, __a733_2) }; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v732); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// @<Declare math...
    // §778
    pub fn make_under(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §778
        let mut x: halfword = 0; // §778
        let mut y: halfword = 0; // §778
        let mut delta: scaled = 0; // §778
        x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
        p = { let __a734_0 = (3i32).wrapping_mul(self.default_rule_thickness()); self.new_kern(__a734_0) };
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        { let __v735 = { let __a736_0 = self.default_rule_thickness(); self.fraction_rule(__a736_0) }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v735); }
        y = self.vpackage(x, 0i32, additional, max_dimen);
        delta = ((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.default_rule_thickness());
        { let __v737 = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(__v737); }
        { let __v738 = (delta).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(__v738); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// @<Declare math...
    // §779
    pub fn make_vcenter(&mut self, mut q: halfword) {
        let mut v: halfword = 0; // §779
        let mut delta: scaled = 0; // §779
        v = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
        if (self.mem[crate::ix::U((v) as usize)].hh().b0() != vlist_node) {
            self.confusion(65855i32);
        }
        delta = (self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int());
        { let __v739 = (self.axis_height(self.cur_size)).wrapping_add(self.half(delta)); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v739); }
        { let __v740 = (delta).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v740); }
    }

    /// According to the rules in the \.{DVI} file specifications, we ensure alignment
    /// between a square root sign and the rule above its nucleus by assuming that the
    /// baseline of the square-root symbol is the same as the bottom of the rule. The
    /// height of the square-root symbol will be the thickness of the rule, and the
    /// depth of the square-root symbol should exceed or equal the height-plus-depth
    /// of the nucleus plus a certain minimum clearance~`clr`. The symbol will be
    /// placed so that the actual clearance is `clr` plus half the excess.
    /// @<Declare math...
    // §780
    pub fn make_radical(&mut self, mut q: halfword) {
        let mut x: halfword = 0; // §780
        let mut y: halfword = 0; // §780
        let mut f: internal_font_number = 0; // §780
        let mut rule_thickness: scaled = 0; // §780
        let mut delta: scaled = 0; // §780
        let mut clr: scaled = 0; // §780
        f = self.eqtb[crate::ix::U(((((math_font_base).wrapping_add((self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b0() % 256i32))).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rule_thickness = self.get_ot_math_constant(f, radicalRuleThickness);
        } else {
            rule_thickness = self.default_rule_thickness();
        }
        x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            {
                if (self.cur_style < text_style) {
                    clr = self.get_ot_math_constant(f, radicalDisplayStyleVerticalGap);
                } else {
                    clr = self.get_ot_math_constant(f, radicalVerticalGap);
                }
            }
        } else {
            {
                if (self.cur_style < text_style) {
                    clr = (rule_thickness).wrapping_add(((self.math_x_height(self.cur_size)).wrapping_abs() / 4i32));
                } else {
                    {
                        clr = rule_thickness;
                        clr = (clr).wrapping_add(((clr).wrapping_abs() / 4i32));
                    }
                }
            }
        }
        y = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, (((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr)).wrapping_add(rule_thickness));
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            {
                { let __v741 = ((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int())).wrapping_sub(rule_thickness); self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(__v741); }
                self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(rule_thickness);
            }
        }
        delta = (self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int()).wrapping_sub(((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr));
        if (delta > 0i32) {
            clr = (clr).wrapping_add(self.half(delta));
        }
        { let __v742 = ((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(clr)).wrapping_neg(); self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v742); }
        { let __v743 = self.overbar(x, clr, self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((y) as usize)].set_hh_rh(__v743); }
        { let __v744 = self.hpack(y, 0i32, additional); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v744); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// Slants are not considered when placing accents in math mode. The accenter is
    /// centered over the accentee, and the accent width is treated as zero with
    /// respect to the size of the final box.
    /// @<Declare math...
    // §781
    pub fn compute_ot_math_accent_pos(&mut self, mut p: halfword) -> scaled {
        let mut compute_ot_math_accent_pos: scaled = 0;
        let mut q: halfword = 0; // §781
        let mut r: halfword = 0; // §781
        let mut s: scaled = 0; // §781
        let mut g: scaled = 0; // §781
        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
            {
                self.fetch((p).wrapping_add(1i32));
                q = self.new_native_character(self.cur_f, self.cur_c);
                g = self.get_native_glyph(q, 0i32);
                s = self.get_ot_math_accent_pos(self.cur_f, g);
            }
        } else {
            {
                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == sub_mlist) {
                    {
                        r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                        if ((r != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((r) as usize)].hh().b0() == accent_noad)) {
                            s = self.compute_ot_math_accent_pos(r);
                        } else {
                            s = 2147483647i32;
                        }
                    }
                } else {
                    s = 2147483647i32;
                }
            }
        }
        compute_ot_math_accent_pos = s;
        compute_ot_math_accent_pos
    }

    /// Slants are not considered when placing accents in math mode. The accenter is
    /// centered over the accentee, and the accent width is treated as zero with
    /// respect to the size of the final box.
    /// @<Declare math...
    // §781
    pub fn make_math_accent(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §781
        let mut x: halfword = 0; // §781
        let mut y: halfword = 0; // §781
        let mut a: i32 = 0; // §781
        let mut c: i32 = 0; // §781
        let mut g: i32 = 0; // §781
        let mut f: internal_font_number = 0; // §781
        let mut i: four_quarters = four_quarters::default(); // §781
        let mut s: scaled = 0; // §781
        let mut sa: scaled = 0; // §781
        let mut h: scaled = 0; // §781
        let mut delta: scaled = 0; // §781
        let mut w: scaled = 0; // §781
        let mut w2: scaled = 0; // §781
        let mut ot_assembly_ptr: void_pointer = 0; // §781
        self.fetch((q).wrapping_add(4i32));
        x = (268435455i32).wrapping_neg();
        ot_assembly_ptr = nil;
        if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag)) {
            {
                c = self.cur_c;
                f = self.cur_f;
                if (!((self.mem[crate::ix::U((q) as usize)].hh().b1() == bottom_acc) || (self.mem[crate::ix::U((q) as usize)].hh().b1() == 3i32))) {
                    s = self.compute_ot_math_accent_pos(q);
                } else {
                    s = 0i32;
                }
                x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
                w = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int();
                h = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
            }
        } else {
            if (self.cur_i.b0() > min_quarterword) {
                {
                    'l_done_f: {
                        'l_done1_f: {
                            i = self.cur_i;
                            c = self.cur_c;
                            f = self.cur_f;
                            // §785
                            s = 0i32;
                            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
                                {
                                    self.fetch((q).wrapping_add(1i32));
                                    if ((self.cur_i.b2() % 4i32) == lig_tag) {
                                        {
                                            a = (self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_i.b3());
                                            self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                            if (self.cur_i.b0() > stop_flag) {
                                                {
                                                    a = ((((self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                                    self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                                }
                                            }
                                            while true {
                                                {
                                                    if (self.cur_i.b1() == self.skew_char[crate::ix::U((self.cur_f) as usize)]) {
                                                        {
                                                            if (self.cur_i.b2() >= kern_flag) {
                                                                if (self.cur_i.b0() <= stop_flag) {
                                                                    s = self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())) as usize)].int();
                                                                }
                                                            }
                                                            break 'l_done1_f;
                                                        }
                                                    }
                                                    if (self.cur_i.b0() >= stop_flag) {
                                                        break 'l_done1_f;
                                                    }
                                                    a = ((a).wrapping_add(self.cur_i.b0())).wrapping_add(1i32);
                                                    self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §781
                        x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
                        w = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int();
                        h = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                        // §784
                        while true {
                            {
                                if ((i.b2() % 4i32) != list_tag) {
                                    break 'l_done_f;
                                }
                                y = i.b3();
                                i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(y)) as usize)].qqqq();
                                if (!(i.b0() > min_quarterword)) {
                                    break 'l_done_f;
                                }
                                if (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int() > w) {
                                    break 'l_done_f;
                                }
                                c = y;
                            }
                        }
                    }
                }
            }
        }
        // §781
        if (x != (268435455i32).wrapping_neg()) {
            {
                if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
                    if ((self.mem[crate::ix::U((q) as usize)].hh().b1() == bottom_acc) || (self.mem[crate::ix::U((q) as usize)].hh().b1() == 3i32)) {
                        delta = 0i32;
                    } else {
                        if (h < self.get_ot_math_constant(f, accentBaseHeight)) {
                            delta = h;
                        } else {
                            delta = self.get_ot_math_constant(f, accentBaseHeight);
                        }
                    }
                } else {
                    if (h < self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) {
                        delta = h;
                    } else {
                        delta = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
                    }
                }
                if ((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() != empty) || (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != empty)) {
                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
                        // §786
                        {
                            self.flush_node_list(x);
                            x = self.new_noad();
                            { let __v745 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)] = __v745; }
                            { let __v746 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)] = __v746; }
                            { let __v747 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)] = __v747; }
                            { let __v748 = self.empty_field; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh(__v748); }
                            { let __v749 = self.empty_field; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_hh(__v749); }
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_mlist);
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
                            x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                            delta = ((delta).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_sub(h);
                            h = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                        }
                    }
                }
                // §781
                y = self.char_box(f, c);
                if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
                    {
                        'l_found_f: {
                            p = self.get_node(glyph_node_size);
                            self.mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
                            self.mem[crate::ix::U((p) as usize)].set_hh_b1(glyph_node);
                            self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b1(f);
                            { let __v750 = self.get_native_glyph(self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh(), 0i32); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(__v750); }
                            self.set_native_glyph_metrics(p, ((1i32) != 0));
                            self.free_node(self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh(), self.mem[crate::ix::U(((self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(4i32)) as usize)].qqqq().b0());
                            self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                            // §783
                            if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                self.set_native_glyph_metrics(p, ((1i32) != 0));
                            } else {
                                {
                                    c = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2();
                                    a = 0i32;
                                    loop {
                                        g = { let mut __f3 = ::core::mem::take(&mut w2); let __r = self.get_ot_math_variant(f, c, a, &mut __f3, 1i32); w2 = __f3; __r };
                                        if ((w2 > 0i32) && (w2 <= w)) {
                                            {
                                                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(g);
                                                self.set_native_glyph_metrics(p, ((1i32) != 0));
                                                a = (a).wrapping_add(1i32);
                                            }
                                        }
                                        if ((w2 < 0i32) || (w2 >= w)) { break; }
                                    }
                                    if (w2 < 0i32) {
                                        {
                                            ot_assembly_ptr = self.get_ot_assembly_ptr(f, c, 1i32);
                                            if (ot_assembly_ptr != nil) {
                                                {
                                                    self.free_node(p, glyph_node_size);
                                                    p = self.build_opentype_assembly(f, ot_assembly_ptr, w, ((1i32) != 0));
                                                    self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                                                    break 'l_found_f;
                                                }
                                            }
                                        }
                                    } else {
                                        self.set_native_glyph_metrics(p, ((1i32) != 0));
                                    }
                                }
                            }
                        }
                        { let __v751 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(__v751); }
                        { let __v752 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(__v752); }
                        { let __v753 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(__v753); }
                        if ((self.mem[crate::ix::U((q) as usize)].hh().b1() == bottom_acc) || (self.mem[crate::ix::U((q) as usize)].hh().b1() == 3i32)) {
                            {
                                if (self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int() < 0i32) {
                                    self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(0i32);
                                }
                            }
                        } else {
                            if (self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int() < 0i32) {
                                self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(0i32);
                            }
                        }
                        // §781
                        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                            {
                                sa = self.get_ot_math_accent_pos(f, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                if (sa == 2147483647i32) {
                                    sa = self.half(self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int());
                                }
                            }
                        } else {
                            sa = self.half(self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int());
                        }
                        if (((self.mem[crate::ix::U((q) as usize)].hh().b1() == bottom_acc) || (self.mem[crate::ix::U((q) as usize)].hh().b1() == 3i32)) || (s == 2147483647i32)) {
                            s = self.half(w);
                        }
                        self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int((s).wrapping_sub(sa));
                    }
                } else {
                    { let __v754 = (s).wrapping_add(self.half((w).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int()))); self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v754); }
                }
                self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(0i32);
                if ((self.mem[crate::ix::U((q) as usize)].hh().b1() == bottom_acc) || (self.mem[crate::ix::U((q) as usize)].hh().b1() == 3i32)) {
                    {
                        self.mem[crate::ix::U((x) as usize)].set_hh_rh(y);
                        y = self.vpackage(x, 0i32, additional, max_dimen);
                        { let __v755 = ((h).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int())).wrapping_neg(); self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v755); }
                    }
                } else {
                    {
                        p = self.new_kern((delta).wrapping_neg());
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                        self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                        y = self.vpackage(y, 0i32, additional, max_dimen);
                        if (self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int() < h) {
                            // §782
                            {
                                p = self.new_kern((h).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()));
                                { let __v756 = self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v756); }
                                self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                                self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(h);
                            }
                        }
                    }
                }
                // §781
                { let __v757 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(__v757); }
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
            }
        }
        self.free_ot_assembly(ot_assembly_ptr);
    }

    /// The `make_fraction` procedure is a bit different because it sets
    /// `new_hlist(q)` directly rather than making a sub-box.
    /// @<Declare math...
    // §787
    pub fn make_fraction(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §787
        let mut v: halfword = 0; // §787
        let mut x: halfword = 0; // §787
        let mut y: halfword = 0; // §787
        let mut z: halfword = 0; // §787
        let mut delta: scaled = 0; // §787
        let mut delta1: scaled = 0; // §787
        let mut delta2: scaled = 0; // §787
        let mut shift_up: scaled = 0; // §787
        let mut shift_down: scaled = 0; // §787
        let mut clr: scaled = 0; // §787
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == default_code) {
            { let __v758 = self.default_rule_thickness(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v758); }
        }
        // §788
        x = self.clean_box((q).wrapping_add(2i32), ((self.cur_style).wrapping_add(2i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        z = self.clean_box((q).wrapping_add(3i32), (((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(3i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        if (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() < self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int()) {
            x = self.rebox(x, self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int());
        } else {
            z = self.rebox(z, self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int());
        }
        if (self.cur_style < text_style) {
            {
                shift_up = self.num1(self.cur_size);
                shift_down = self.denom1(self.cur_size);
            }
        } else {
            {
                shift_down = self.denom2(self.cur_size);
                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() != 0i32) {
                    shift_up = self.num2(self.cur_size);
                } else {
                    shift_up = self.num3(self.cur_size);
                }
            }
        }
        // §787
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            // §789
            {
                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                    {
                        if (self.cur_style < text_style) {
                            clr = self.get_ot_math_constant(self.cur_f, stackDisplayStyleGapMin);
                        } else {
                            clr = self.get_ot_math_constant(self.cur_f, stackGapMin);
                        }
                    }
                } else {
                    {
                        if (self.cur_style < text_style) {
                            clr = (7i32).wrapping_mul(self.default_rule_thickness());
                        } else {
                            clr = (3i32).wrapping_mul(self.default_rule_thickness());
                        }
                    }
                }
                delta = self.half((clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down))));
                if (delta > 0i32) {
                    {
                        shift_up = (shift_up).wrapping_add(delta);
                        shift_down = (shift_down).wrapping_add(delta);
                    }
                }
            }
        } else {
            // §790
            {
                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                    {
                        delta = self.half(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                        if (self.cur_style < text_style) {
                            clr = self.get_ot_math_constant(self.cur_f, fractionNumDisplayStyleGapMin);
                        } else {
                            clr = self.get_ot_math_constant(self.cur_f, fractionNumeratorGapMin);
                        }
                        delta1 = (clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.axis_height(self.cur_size)).wrapping_add(delta)));
                        if (self.cur_style < text_style) {
                            clr = self.get_ot_math_constant(self.cur_f, fractionDenomDisplayStyleGapMin);
                        } else {
                            clr = self.get_ot_math_constant(self.cur_f, fractionDenominatorGapMin);
                        }
                        delta2 = (clr).wrapping_sub(((self.axis_height(self.cur_size)).wrapping_sub(delta)).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                    }
                } else {
                    {
                        if (self.cur_style < text_style) {
                            clr = (3i32).wrapping_mul(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                        } else {
                            clr = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                        }
                        delta = self.half(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                        delta1 = (clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.axis_height(self.cur_size)).wrapping_add(delta)));
                        delta2 = (clr).wrapping_sub(((self.axis_height(self.cur_size)).wrapping_sub(delta)).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                    }
                }
                if (delta1 > 0i32) {
                    shift_up = (shift_up).wrapping_add(delta1);
                }
                if (delta2 > 0i32) {
                    shift_down = (shift_down).wrapping_add(delta2);
                }
            }
        }
        // §791
        v = self.new_null_box();
        self.mem[crate::ix::U((v) as usize)].set_hh_b0(vlist_node);
        { let __v759 = (shift_up).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v759); }
        { let __v760 = (self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int()).wrapping_add(shift_down); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v760); }
        { let __v761 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v761); }
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            {
                p = self.new_kern(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
            }
        } else {
            {
                y = self.fraction_rule(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                p = { let __a762_0 = ((self.axis_height(self.cur_size)).wrapping_sub(delta)).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)); self.new_kern(__a762_0) };
                self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                p = { let __a763_0 = ((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.axis_height(self.cur_size)).wrapping_add(delta)); self.new_kern(__a763_0) };
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(y);
            }
        }
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(x);
        // §792
        if (self.cur_style < text_style) {
            delta = self.delim1(self.cur_size);
        } else {
            delta = self.delim2(self.cur_size);
        }
        x = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, delta);
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(v);
        z = self.var_delimiter((q).wrapping_add(5i32), self.cur_size, delta);
        self.mem[crate::ix::U((v) as usize)].set_hh_rh(z);
        { let __v764 = self.hpack(x, 0i32, additional); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v764); }
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
    // §793
    pub fn make_op(&mut self, mut q: halfword) -> scaled {
        let mut make_op: scaled = 0;
        let mut delta: scaled = 0; // §793
        let mut p: halfword = 0; // §793
        let mut v: halfword = 0; // §793
        let mut x: halfword = 0; // §793
        let mut y: halfword = 0; // §793
        let mut z: halfword = 0; // §793
        let mut c: quarterword = 0; // §793
        let mut i: four_quarters = four_quarters::default(); // §793
        let mut shift_up: scaled = 0; // §793
        let mut shift_down: scaled = 0; // §793
        let mut h1: scaled = 0; // §793
        let mut h2: scaled = 0; // §793
        let mut n: i32 = 0; // §793
        let mut g: i32 = 0; // §793
        let mut ot_assembly_ptr: void_pointer = 0; // §793
        let mut save_f: internal_font_number = 0; // §793
        if ((self.mem[crate::ix::U((q) as usize)].hh().b1() == normal) && (self.cur_style < text_style)) {
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(limits);
        }
        delta = 0i32;
        ot_assembly_ptr = nil;
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
            {
                self.fetch((q).wrapping_add(1i32));
                if (!((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)]))) {
                    {
                        if ((self.cur_style < text_style) && ((self.cur_i.b2() % 4i32) == list_tag)) {
                            {
                                c = self.cur_i.b3();
                                i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(c)) as usize)].qqqq();
                                if (i.b0() > min_quarterword) {
                                    {
                                        self.cur_c = c;
                                        self.cur_i = i;
                                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_b1(c);
                                    }
                                }
                            }
                        }
                        delta = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((self.cur_i.b2() / 4i32))) as usize)].int();
                    }
                }
                x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                    {
                        p = self.mem[crate::ix::U(((x).wrapping_add(5i32)) as usize)].hh().rh();
                        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                            {
                                'l_found_f: {
                                    if (self.cur_style < text_style) {
                                        {
                                            h1 = self.get_ot_math_constant(self.cur_f, displayOperatorMinHeight);
                                            if (((h1) as f64) < (((((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_mul(5i32)) as f64) / ((4i32) as f64))) {
                                                h1 = (((((((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_mul(5i32)) as f64) / ((4i32) as f64))) as i32);
                                            }
                                            c = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2();
                                            n = 0i32;
                                            loop {
                                                g = { let mut __f3 = ::core::mem::take(&mut h2); let __r = self.get_ot_math_variant(self.cur_f, c, n, &mut __f3, 0i32); h2 = __f3; __r };
                                                if (h2 > 0i32) {
                                                    {
                                                        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(g);
                                                        self.set_native_glyph_metrics(p, ((1i32) != 0));
                                                    }
                                                }
                                                n = (n).wrapping_add(1i32);
                                                if ((h2 < 0i32) || (h2 >= h1)) { break; }
                                            }
                                            if (h2 < 0i32) {
                                                {
                                                    ot_assembly_ptr = self.get_ot_assembly_ptr(self.cur_f, c, 0i32);
                                                    if (ot_assembly_ptr != nil) {
                                                        {
                                                            self.free_node(p, glyph_node_size);
                                                            p = self.build_opentype_assembly(self.cur_f, ot_assembly_ptr, h1, ((0i32) != 0));
                                                            self.mem[crate::ix::U(((x).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                                                            delta = 0i32;
                                                            break 'l_found_f;
                                                        }
                                                    }
                                                }
                                            } else {
                                                self.set_native_glyph_metrics(p, ((1i32) != 0));
                                            }
                                        }
                                    }
                                    delta = self.get_ot_math_ital_corr(self.cur_f, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                }
                                { let __v765 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v765); }
                                { let __v766 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].set_int(__v766); }
                                { let __v767 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].set_int(__v767); }
                            }
                        }
                    }
                }
                if ((self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != empty) && (self.mem[crate::ix::U((q) as usize)].hh().b1() != limits)) {
                    { let __v768 = (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int()).wrapping_sub(delta); self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v768); }
                }
                { let __v769 = (self.half((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.axis_height(self.cur_size)); self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v769); }
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
            }
        }
        save_f = self.cur_f;
        if (self.mem[crate::ix::U((q) as usize)].hh().b1() == limits) {
            // §794
            {
                x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                y = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                z = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                v = self.new_null_box();
                self.mem[crate::ix::U((v) as usize)].set_hh_b0(vlist_node);
                { let __v770 = self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v770); }
                if (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v771 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v771); }
                }
                if (self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v772 = self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v772); }
                }
                x = self.rebox(x, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                y = self.rebox(y, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                z = self.rebox(z, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                { let __v773 = self.half(delta); self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v773); }
                { let __v774 = (self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U(((z).wrapping_add(4i32)) as usize)].set_int(__v774); }
                { let __v775 = self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v775); }
                { let __v776 = self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v776); }
                // §795
                self.cur_f = save_f;
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == empty) {
                    {
                        self.free_node(x, box_node_size);
                        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(y);
                    }
                } else {
                    {
                        shift_up = (self.big_op_spacing3()).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int());
                        if (shift_up < self.big_op_spacing1()) {
                            shift_up = self.big_op_spacing1();
                        }
                        p = self.new_kern(shift_up);
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(y);
                        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
                        p = { let __a777_0 = self.big_op_spacing5(); self.new_kern(__a777_0) };
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                        { let __v778 = ((((self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.big_op_spacing5())).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_up); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v778); }
                    }
                }
                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                    self.free_node(z, box_node_size);
                } else {
                    {
                        shift_down = (self.big_op_spacing4()).wrapping_sub(self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int());
                        if (shift_down < self.big_op_spacing2()) {
                            shift_down = self.big_op_spacing2();
                        }
                        p = self.new_kern(shift_down);
                        self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                        p = { let __a779_0 = self.big_op_spacing5(); self.new_kern(__a779_0) };
                        self.mem[crate::ix::U((z) as usize)].set_hh_rh(p);
                        { let __v780 = ((((self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.big_op_spacing5())).wrapping_add(self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_down); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v780); }
                    }
                }
                // §794
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(v);
            }
        }
        // §793
        self.free_ot_assembly(ot_assembly_ptr);
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
    // §796
    pub fn make_ord(&mut self, mut q: halfword) {
        let mut a: i32 = 0; // §796
        let mut p: halfword = 0; // §796
        let mut r: halfword = 0; // §796
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                    if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == empty) {
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
                            {
                                p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                if (p != (268435455i32).wrapping_neg()) {
                                    if ((self.mem[crate::ix::U((p) as usize)].hh().b0() >= ord_noad) && (self.mem[crate::ix::U((p) as usize)].hh().b0() <= punct_noad)) {
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
                                            if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b0() % 256i32) == (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().b0() % 256i32)) {
                                                {
                                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(math_text_char);
                                                    self.fetch((q).wrapping_add(1i32));
                                                    if ((self.cur_i.b2() % 4i32) == lig_tag) {
                                                        {
                                                            a = (self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_i.b3());
                                                            self.cur_c = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b1();
                                                            self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                                            if (self.cur_i.b0() > stop_flag) {
                                                                {
                                                                    a = ((((self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                                                    self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                                                }
                                                            }
                                                            while true {
                                                                {
                                                                    // §797
                                                                    if (self.cur_i.b1() == self.cur_c) {
                                                                        if (self.cur_i.b0() <= stop_flag) {
                                                                            if (self.cur_i.b2() >= kern_flag) {
                                                                                {
                                                                                    p = self.new_kern(self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())) as usize)].int());
                                                                                    { let __v781 = self.mem[crate::ix::U((q) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v781); }
                                                                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
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
                                                                                            { let __v782 = self.cur_i.b3(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_b1(__v782); }
                                                                                        }
                                                                                        2 | 6 => {
                                                                                            { let __v783 = self.cur_i.b3(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b1(__v783); }
                                                                                        }
                                                                                        3 | 7 | 11 => {
                                                                                            {
                                                                                                r = self.new_noad();
                                                                                                { let __v784 = self.cur_i.b3(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_b1(__v784); }
                                                                                                { let __v785 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().b0() % 256i32); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_b0(__v785); }
                                                                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                                                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                                                                                                if (self.cur_i.b2() < 11i32) {
                                                                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(math_char);
                                                                                                } else {
                                                                                                    self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(math_text_char);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        _ => {
                                                                                            {
                                                                                                { let __v786 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v786); }
                                                                                                { let __v787 = self.cur_i.b3(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_b1(__v787); }
                                                                                                { let __v788 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)] = __v788; }
                                                                                                { let __v789 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)] = __v789; }
                                                                                                self.free_node(p, noad_size);
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    if (self.cur_i.b2() > 3i32) {
                                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                                    }
                                                                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(math_char);
                                                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §796
                                                                    if (self.cur_i.b0() >= stop_flag) {
                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                    }
                                                                    a = ((a).wrapping_add(self.cur_i.b0())).wrapping_add(1i32);
                                                                    self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
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
    // §800
    pub fn attach_hkern_to_new_hlist(&mut self, mut q: halfword, mut delta: scaled) -> halfword {
        let mut attach_hkern_to_new_hlist: halfword = 0;
        let mut y: halfword = 0; // §800
        let mut z: halfword = 0; // §800
        z = self.new_kern(delta);
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (268435455i32).wrapping_neg()) {
            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(z);
        } else {
            {
                y = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                while (self.mem[crate::ix::U((y) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                    y = self.mem[crate::ix::U((y) as usize)].hh().rh();
                }
                self.mem[crate::ix::U((y) as usize)].set_hh_rh(z);
            }
        }
        attach_hkern_to_new_hlist = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
        attach_hkern_to_new_hlist
    }

    /// The purpose of `make_scripts(q,delta)` is to attach the subscript and/or
    /// superscript of noad `q` to the list that starts at `new_hlist(q)`,
    /// given that the subscript and superscript aren't both empty. The superscript
    /// will appear to the right of the subscript by a given distance `delta`.
    /// We set `shift_down` and `shift_up` to the minimum amounts to shift the
    /// baseline of subscripts and superscripts based on the given nucleus.
    /// @<Declare math...
    // §800
    pub fn make_scripts(&mut self, mut q: halfword, mut delta: scaled) {
        let mut p: halfword = 0; // §800
        let mut x: halfword = 0; // §800
        let mut y: halfword = 0; // §800
        let mut z: halfword = 0; // §800
        let mut shift_up: scaled = 0; // §800
        let mut shift_down: scaled = 0; // §800
        let mut clr: scaled = 0; // §800
        let mut sub_kern: scaled = 0; // §800
        let mut sup_kern: scaled = 0; // §800
        let mut script_c: halfword = 0; // §800
        let mut script_g: quarterword = 0; // §800
        let mut script_f: internal_font_number = 0; // §800
        let mut sup_g: quarterword = 0; // §800
        let mut sup_f: internal_font_number = 0; // §800
        let mut sub_g: quarterword = 0; // §800
        let mut sub_f: internal_font_number = 0; // §800
        let mut t: i32 = 0; // §800
        let mut save_f: internal_font_number = 0; // §800
        let mut script_head: halfword = 0; // §800
        let mut script_ptr: halfword = 0; // §800
        let mut saved_math_style: small_number = 0; // §800
        let mut this_math_style: small_number = 0; // §800
        p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
        script_c = (268435455i32).wrapping_neg();
        script_g = 0i32;
        script_f = 0i32;
        sup_kern = 0i32;
        sub_kern = 0i32;
        if ((p >= self.hi_mem_min) || ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node))) {
            {
                shift_up = 0i32;
                shift_down = 0i32;
            }
        } else {
            {
                z = self.hpack(p, 0i32, additional);
                if (self.cur_style < script_style) {
                    t = script_size;
                } else {
                    t = script_script_size;
                }
                shift_up = (self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.sup_drop(t));
                shift_down = (self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.sub_drop(t));
                self.free_node(z, box_node_size);
            }
        }
        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == empty) {
            // §801
            {
                script_head = (q).wrapping_add(3i32);
                // §805
                script_c = (268435455i32).wrapping_neg();
                script_g = 0i32;
                script_f = null_font;
                this_math_style = ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32);
                if (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == sub_mlist) {
                    {
                        script_ptr = self.mem[crate::ix::U((script_head) as usize)].hh().lh();
                        script_head = (268435455i32).wrapping_neg();
                        while ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                            {
                                match self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() {
                                    kern_node | glue_node => {
                                    }
                                    style_node => {
                                        {
                                            this_math_style = self.mem[crate::ix::U((script_ptr) as usize)].hh().b1();
                                        }
                                    }
                                    choice_node => {
                                    }
                                    ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad => {
                                        {
                                            script_head = (script_ptr).wrapping_add(1i32);
                                            script_ptr = (268435455i32).wrapping_neg();
                                        }
                                    }
                                    _ => {
                                        script_ptr = (268435455i32).wrapping_neg();
                                    }
                                }
                                if ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                                    if (self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() == choice_node) {
                                        match (this_math_style / 2i32) {
                                            0 => {
                                                script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().lh();
                                            }
                                            1 => {
                                                script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                            }
                                            2 => {
                                                script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().lh();
                                            }
                                            3 => {
                                                script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().rh();
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        script_ptr = self.mem[crate::ix::U((script_ptr) as usize)].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                }
                if (((script_head >= mem_min) && (script_head <= self.mem_end)) && (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == math_char)) {
                    {
                        save_f = self.cur_f;
                        saved_math_style = self.cur_style;
                        self.cur_style = this_math_style;
                        // §746
                        {
                            if (self.cur_style < script_style) {
                                self.cur_size = text_size;
                            } else {
                                self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                            }
                            self.cur_mu = { let __a790_0 = self.math_quad(self.cur_size); let __a790_1 = 18i32; self.x_over_n(__a790_0, __a790_1) };
                        }
                        // §805
                        self.fetch(script_head);
                        if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                            {
                                script_c = self.new_native_character(self.cur_f, self.cur_c);
                                script_g = self.get_native_glyph(script_c, 0i32);
                                script_f = self.cur_f;
                            }
                        }
                        self.cur_f = save_f;
                        self.cur_style = saved_math_style;
                        // §746
                        {
                            if (self.cur_style < script_style) {
                                self.cur_size = text_size;
                            } else {
                                self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                            }
                            self.cur_mu = { let __a791_0 = self.math_quad(self.cur_size); let __a791_1 = 18i32; self.x_over_n(__a791_0, __a791_1) };
                        }
                    }
                }
                // §801
                sub_g = script_g;
                sub_f = script_f;
                save_f = self.cur_f;
                x = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                self.cur_f = save_f;
                { let __v792 = (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006732i32) - 1) as usize)].int()); self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v792); }
                if (shift_down < self.sub1(self.cur_size)) {
                    shift_down = self.sub1(self.cur_size);
                }
                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                    clr = (self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.get_ot_math_constant(self.cur_f, subscriptTopMax));
                } else {
                    clr = (self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_sub((((self.math_x_height(self.cur_size)).wrapping_mul(4i32)).wrapping_abs() / 5i32));
                }
                if (shift_down < clr) {
                    shift_down = clr;
                }
                self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(shift_down);
                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                    // §806
                    {
                        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                            {
                                sub_kern = self.get_ot_math_kern(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1(), self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2(), sub_f, sub_g, sub_cmd, shift_down);
                                if (sub_kern != 0i32) {
                                    p = self.attach_hkern_to_new_hlist(q, sub_kern);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // §800
            {
                // §802
                {
                    script_head = (q).wrapping_add(2i32);
                    // §805
                    script_c = (268435455i32).wrapping_neg();
                    script_g = 0i32;
                    script_f = null_font;
                    this_math_style = ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32);
                    if (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == sub_mlist) {
                        {
                            script_ptr = self.mem[crate::ix::U((script_head) as usize)].hh().lh();
                            script_head = (268435455i32).wrapping_neg();
                            while ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                                {
                                    match self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() {
                                        kern_node | glue_node => {
                                        }
                                        style_node => {
                                            {
                                                this_math_style = self.mem[crate::ix::U((script_ptr) as usize)].hh().b1();
                                            }
                                        }
                                        choice_node => {
                                        }
                                        ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad => {
                                            {
                                                script_head = (script_ptr).wrapping_add(1i32);
                                                script_ptr = (268435455i32).wrapping_neg();
                                            }
                                        }
                                        _ => {
                                            script_ptr = (268435455i32).wrapping_neg();
                                        }
                                    }
                                    if ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                                        if (self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() == choice_node) {
                                            match (this_math_style / 2i32) {
                                                0 => {
                                                    script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().lh();
                                                }
                                                1 => {
                                                    script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                }
                                                2 => {
                                                    script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().lh();
                                                }
                                                3 => {
                                                    script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().rh();
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            script_ptr = self.mem[crate::ix::U((script_ptr) as usize)].hh().rh();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if (((script_head >= mem_min) && (script_head <= self.mem_end)) && (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == math_char)) {
                        {
                            save_f = self.cur_f;
                            saved_math_style = self.cur_style;
                            self.cur_style = this_math_style;
                            // §746
                            {
                                if (self.cur_style < script_style) {
                                    self.cur_size = text_size;
                                } else {
                                    self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                }
                                self.cur_mu = { let __a793_0 = self.math_quad(self.cur_size); let __a793_1 = 18i32; self.x_over_n(__a793_0, __a793_1) };
                            }
                            // §805
                            self.fetch(script_head);
                            if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                                {
                                    script_c = self.new_native_character(self.cur_f, self.cur_c);
                                    script_g = self.get_native_glyph(script_c, 0i32);
                                    script_f = self.cur_f;
                                }
                            }
                            self.cur_f = save_f;
                            self.cur_style = saved_math_style;
                            // §746
                            {
                                if (self.cur_style < script_style) {
                                    self.cur_size = text_size;
                                } else {
                                    self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                }
                                self.cur_mu = { let __a794_0 = self.math_quad(self.cur_size); let __a794_1 = 18i32; self.x_over_n(__a794_0, __a794_1) };
                            }
                        }
                    }
                    // §802
                    sup_g = script_g;
                    sup_f = script_f;
                    save_f = self.cur_f;
                    x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                    self.cur_f = save_f;
                    { let __v795 = (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006732i32) - 1) as usize)].int()); self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v795); }
                    if (((self.cur_style) % 2) != 0) {
                        clr = self.sup3(self.cur_size);
                    } else {
                        if (self.cur_style < text_style) {
                            clr = self.sup1(self.cur_size);
                        } else {
                            clr = self.sup2(self.cur_size);
                        }
                    }
                    if (shift_up < clr) {
                        shift_up = clr;
                    }
                    if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                        clr = (self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.get_ot_math_constant(self.cur_f, superscriptBottomMin));
                    } else {
                        clr = (self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()).wrapping_add(((self.math_x_height(self.cur_size)).wrapping_abs() / 4i32));
                    }
                    if (shift_up < clr) {
                        shift_up = clr;
                    }
                    if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                        // §807
                        {
                            if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                                if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                                    {
                                        sup_kern = self.get_ot_math_kern(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1(), self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2(), sup_f, sup_g, sup_cmd, shift_up);
                                        if (sup_kern != 0i32) {
                                            p = self.attach_hkern_to_new_hlist(q, sup_kern);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // §800
                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                    self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int((shift_up).wrapping_neg());
                } else {
                    // §803
                    {
                        save_f = self.cur_f;
                        script_head = (q).wrapping_add(3i32);
                        // §805
                        script_c = (268435455i32).wrapping_neg();
                        script_g = 0i32;
                        script_f = null_font;
                        this_math_style = ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32);
                        if (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == sub_mlist) {
                            {
                                script_ptr = self.mem[crate::ix::U((script_head) as usize)].hh().lh();
                                script_head = (268435455i32).wrapping_neg();
                                while ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                                    {
                                        match self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() {
                                            kern_node | glue_node => {
                                            }
                                            style_node => {
                                                {
                                                    this_math_style = self.mem[crate::ix::U((script_ptr) as usize)].hh().b1();
                                                }
                                            }
                                            choice_node => {
                                            }
                                            ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad => {
                                                {
                                                    script_head = (script_ptr).wrapping_add(1i32);
                                                    script_ptr = (268435455i32).wrapping_neg();
                                                }
                                            }
                                            _ => {
                                                script_ptr = (268435455i32).wrapping_neg();
                                            }
                                        }
                                        if ((script_ptr >= mem_min) && (script_ptr <= self.mem_end)) {
                                            if (self.mem[crate::ix::U((script_ptr) as usize)].hh().b0() == choice_node) {
                                                match (this_math_style / 2i32) {
                                                    0 => {
                                                        script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().lh();
                                                    }
                                                    1 => {
                                                        script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                    }
                                                    2 => {
                                                        script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().lh();
                                                    }
                                                    3 => {
                                                        script_ptr = self.mem[crate::ix::U(((script_ptr).wrapping_add(2i32)) as usize)].hh().rh();
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                script_ptr = self.mem[crate::ix::U((script_ptr) as usize)].hh().rh();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if (((script_head >= mem_min) && (script_head <= self.mem_end)) && (self.mem[crate::ix::U((script_head) as usize)].hh().rh() == math_char)) {
                            {
                                save_f = self.cur_f;
                                saved_math_style = self.cur_style;
                                self.cur_style = this_math_style;
                                // §746
                                {
                                    if (self.cur_style < script_style) {
                                        self.cur_size = text_size;
                                    } else {
                                        self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                    }
                                    self.cur_mu = { let __a796_0 = self.math_quad(self.cur_size); let __a796_1 = 18i32; self.x_over_n(__a796_0, __a796_1) };
                                }
                                // §805
                                self.fetch(script_head);
                                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                                    {
                                        script_c = self.new_native_character(self.cur_f, self.cur_c);
                                        script_g = self.get_native_glyph(script_c, 0i32);
                                        script_f = self.cur_f;
                                    }
                                }
                                self.cur_f = save_f;
                                self.cur_style = saved_math_style;
                                // §746
                                {
                                    if (self.cur_style < script_style) {
                                        self.cur_size = text_size;
                                    } else {
                                        self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                    }
                                    self.cur_mu = { let __a797_0 = self.math_quad(self.cur_size); let __a797_1 = 18i32; self.x_over_n(__a797_0, __a797_1) };
                                }
                            }
                        }
                        // §803
                        sub_g = script_g;
                        sub_f = script_f;
                        y = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                        self.cur_f = save_f;
                        { let __v798 = (self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006732i32) - 1) as usize)].int()); self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(__v798); }
                        if (shift_down < self.sub2(self.cur_size)) {
                            shift_down = self.sub2(self.cur_size);
                        }
                        if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                            clr = (self.get_ot_math_constant(self.cur_f, subSuperscriptGapMin)).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                        } else {
                            clr = ((4i32).wrapping_mul(self.default_rule_thickness())).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                        }
                        if (clr > 0i32) {
                            {
                                shift_down = (shift_down).wrapping_add(clr);
                                if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                                    clr = (self.get_ot_math_constant(self.cur_f, superscriptBottomMaxWithSubscript)).wrapping_sub((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()));
                                } else {
                                    clr = ((((self.math_x_height(self.cur_size)).wrapping_mul(4i32)).wrapping_abs() / 5i32)).wrapping_sub((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()));
                                }
                                if (clr > 0i32) {
                                    {
                                        shift_up = (shift_up).wrapping_add(clr);
                                        shift_down = (shift_down).wrapping_sub(clr);
                                    }
                                }
                            }
                        }
                        if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])) {
                            {
                                // §806
                                {
                                    if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                                        {
                                            sub_kern = self.get_ot_math_kern(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1(), self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2(), sub_f, sub_g, sub_cmd, shift_down);
                                            if (sub_kern != 0i32) {
                                                p = self.attach_hkern_to_new_hlist(q, sub_kern);
                                            }
                                        }
                                    }
                                }
                                // §807
                                {
                                    if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                                        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
                                            {
                                                sup_kern = self.get_ot_math_kern(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1(), self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2(), sup_f, sup_g, sup_cmd, shift_up);
                                                if (sup_kern != 0i32) {
                                                    p = self.attach_hkern_to_new_hlist(q, sup_kern);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // §803
                            {
                                sup_kern = 0i32;
                                sub_kern = 0i32;
                            }
                        }
                        self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(((sup_kern).wrapping_add(delta)).wrapping_sub(sub_kern));
                        p = self.new_kern(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(y);
                        x = self.vpackage(x, 0i32, additional, max_dimen);
                        self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(shift_down);
                    }
                }
            }
        }
        // §800
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (268435455i32).wrapping_neg()) {
            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(x);
        } else {
            {
                p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                while (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(x);
            }
        }
    }

    /// The `make_left_right` function constructs a left or right delimiter of
    /// the required size and returns the value `open_noad` or `close_noad`. The
    /// `right_noad` and `left_noad` will both be based on the original `style`,
    /// so they will have consistent sizes.
    /// We use the fact that `right_noad-left_noad=close_noad-open_noad`.
    /// @<Declare math...
    // §810
    pub fn make_left_right(&mut self, mut q: halfword, mut style: small_number, mut max_d: scaled, mut max_h: scaled) -> small_number {
        let mut make_left_right: small_number = 0;
        let mut delta: scaled = 0; // §810
        let mut delta1: scaled = 0; // §810
        let mut delta2: scaled = 0; // §810
        self.cur_style = style;
        // §746
        {
            if (self.cur_style < script_style) {
                self.cur_size = text_size;
            } else {
                self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
            }
            self.cur_mu = { let __a799_0 = self.math_quad(self.cur_size); let __a799_1 = 18i32; self.x_over_n(__a799_0, __a799_1) };
        }
        // §810
        delta2 = (max_d).wrapping_add(self.axis_height(self.cur_size));
        delta1 = ((max_h).wrapping_add(max_d)).wrapping_sub(delta2);
        if (delta2 > delta1) {
            delta1 = delta2;
        }
        delta = ((delta1 / 500i32)).wrapping_mul(self.eqtb[crate::ix::U(((7892282i32) - 1) as usize)].int());
        delta2 = ((delta1).wrapping_add(delta1)).wrapping_sub(self.eqtb[crate::ix::U(((9006730i32) - 1) as usize)].int());
        if (delta < delta2) {
            delta = delta2;
        }
        { let __v800 = self.var_delimiter((q).wrapping_add(1i32), self.cur_size, delta); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v800); }
        make_left_right = (self.mem[crate::ix::U((q) as usize)].hh().b0()).wrapping_sub(10i32);
        make_left_right
    }

    /// Here is the overall plan of `mlist_to_hlist`, and the list of its
    /// local variables.
    // §769
    pub fn mlist_to_hlist(&mut self) {
        let mut mlist: halfword = 0; // §769
        let mut penalties: bool = false; // §769
        let mut style: small_number = 0; // §769
        let mut save_style: small_number = 0; // §769
        let mut q: halfword = 0; // §769
        let mut r: halfword = 0; // §769
        let mut r_type: small_number = 0; // §769
        let mut t: small_number = 0; // §769
        let mut p: halfword = 0; // §769
        let mut x: halfword = 0; // §769
        let mut y: halfword = 0; // §769
        let mut z: halfword = 0; // §769
        let mut pen: i32 = 0; // §769
        let mut s: small_number = 0; // §769
        let mut max_h: scaled = 0; // §769
        let mut max_d: scaled = 0; // §769
        let mut delta: scaled = 0; // §769
        mlist = self.cur_mlist;
        penalties = self.mlist_penalties;
        style = self.cur_style;
        q = mlist;
        r = (268435455i32).wrapping_neg();
        r_type = op_noad;
        max_h = 0i32;
        max_d = 0i32;
        // §746
        {
            if (self.cur_style < script_style) {
                self.cur_size = text_size;
            } else {
                self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
            }
            self.cur_mu = { let __a801_0 = self.math_quad(self.cur_size); let __a801_1 = 18i32; self.x_over_n(__a801_0, __a801_1) };
        }
        // §769
        while (q != (268435455i32).wrapping_neg()) {
            // §770
            {
                // goto labels: reswitch, L82, L80, L81
                let mut __goto_1: i32 = 0;
                'l_dispatch_1: loop {
                    if __goto_1 <= 0 {
                        // §771
                        delta = 0i32;
                        match self.mem[crate::ix::U((q) as usize)].hh().b0() {
                            bin_noad => {
                                match r_type {
                                    bin_noad | op_noad | rel_noad | open_noad | punct_noad | left_noad => {
                                        {
                                            self.mem[crate::ix::U((q) as usize)].set_hh_b0(ord_noad);
                                            { __goto_1 = 0; continue 'l_dispatch_1; }
                                        }
                                    }
                                    _ => {
                                    }
                                }
                            }
                            rel_noad | close_noad | punct_noad | right_noad => {
                                {
                                    // §772
                                    if (r_type == bin_noad) {
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(ord_noad);
                                    }
                                    // §771
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == right_noad) {
                                        { __goto_1 = 2; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            left_noad => {
                                // §776
                                { __goto_1 = 2; continue 'l_dispatch_1; }
                            }
                            fraction_noad => {
                                {
                                    self.make_fraction(q);
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                            op_noad => {
                                {
                                    delta = self.make_op(q);
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b1() == limits) {
                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                    }
                                }
                            }
                            ord_noad => {
                                self.make_ord(q);
                            }
                            open_noad | inner_noad => {
                            }
                            radical_noad => {
                                self.make_radical(q);
                            }
                            over_noad => {
                                self.make_over(q);
                            }
                            under_noad => {
                                self.make_under(q);
                            }
                            accent_noad => {
                                self.make_math_accent(q);
                            }
                            vcenter_noad => {
                                self.make_vcenter(q);
                            }
                            style_node => {
                                // §773
                                {
                                    self.cur_style = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                    // §746
                                    {
                                        if (self.cur_style < script_style) {
                                            self.cur_size = text_size;
                                        } else {
                                            self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = { let __a802_0 = self.math_quad(self.cur_size); let __a802_1 = 18i32; self.x_over_n(__a802_0, __a802_1) };
                                    }
                                    // §773
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            choice_node => {
                                // §774
                                {
                                    match (self.cur_style / 2i32) {
                                        0 => {
                                            {
                                                p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                            }
                                        }
                                        1 => {
                                            {
                                                p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                                                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                            }
                                        }
                                        2 => {
                                            {
                                                p = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh();
                                                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                            }
                                        }
                                        3 => {
                                            {
                                                p = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh();
                                                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                            }
                                        }
                                        _ => {}
                                    }
                                    self.flush_node_list(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh());
                                    self.flush_node_list(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh());
                                    self.flush_node_list(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().lh());
                                    self.flush_node_list(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh());
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(style_node);
                                    { let __v803 = self.cur_style; self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v803); }
                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                                    self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
                                    if (p != (268435455i32).wrapping_neg()) {
                                        {
                                            z = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                            while (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                            }
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                                        }
                                    }
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            ins_node | mark_node | adjust_node | whatsit_node | penalty_node | disc_node => {
                                // §773
                                { __goto_1 = 3; continue 'l_dispatch_1; }
                            }
                            rule_node => {
                                {
                                    if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() > max_h) {
                                        max_h = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                                    }
                                    if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() > max_d) {
                                        max_d = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                                    }
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            glue_node => {
                                {
                                    // §775
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b1() == mu_glue) {
                                        {
                                            x = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                            y = self.math_glue(x, self.cur_mu);
                                            self.delete_glue_ref(x);
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
                                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(normal);
                                        }
                                    } else {
                                        if ((self.cur_size != text_size) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == cond_math_glue)) {
                                            {
                                                p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                if (p != (268435455i32).wrapping_neg()) {
                                                    if ((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node)) {
                                                        {
                                                            { let __v804 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v804); }
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                            self.flush_node_list(p);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // §773
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            kern_node => {
                                {
                                    self.math_kern(q, self.cur_mu);
                                    { __goto_1 = 3; continue 'l_dispatch_1; }
                                }
                            }
                            _ => {
                                // §771
                                self.confusion(66310i32);
                            }
                        }
                        // §798
                        match self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() {
                            math_char | math_text_char => {
                                // §799
                                {
                                    self.fetch((q).wrapping_add(1i32));
                                    if ((self.font_area[crate::ix::U((self.cur_f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag)) {
                                        {
                                            z = self.new_native_character(self.cur_f, self.cur_c);
                                            p = self.get_node(glyph_node_size);
                                            self.mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
                                            self.mem[crate::ix::U((p) as usize)].set_hh_b1(glyph_node);
                                            { let __v805 = self.cur_f; self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b1(__v805); }
                                            { let __v806 = self.get_native_glyph(z, 0i32); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(__v806); }
                                            self.set_native_glyph_metrics(p, ((1i32) != 0));
                                            self.free_node(z, self.mem[crate::ix::U(((z).wrapping_add(4i32)) as usize)].qqqq().b0());
                                            delta = self.get_ot_math_ital_corr(self.cur_f, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                            if ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_text_char) && ((((!((self.font_area[crate::ix::U((self.cur_f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((self.cur_f) as usize)])))) as i32) != 0i32)) {
                                                delta = 0i32;
                                            }
                                            if ((self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) && (delta != 0i32)) {
                                                {
                                                    { let __v807 = self.new_kern(delta); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v807); }
                                                    delta = 0i32;
                                                }
                                            }
                                        }
                                    } else {
                                        if (self.cur_i.b0() > min_quarterword) {
                                            {
                                                delta = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((self.cur_i.b2() / 4i32))) as usize)].int();
                                                p = self.new_character(self.cur_f, self.cur_c);
                                                if ((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_text_char) && (self.font_info[crate::ix::U(((space_code).wrapping_add(self.param_base[crate::ix::U((self.cur_f) as usize)])) as usize)].int() != 0i32)) {
                                                    delta = 0i32;
                                                }
                                                if ((self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) && (delta != 0i32)) {
                                                    {
                                                        { let __v808 = self.new_kern(delta); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v808); }
                                                        delta = 0i32;
                                                    }
                                                }
                                            }
                                        } else {
                                            p = (268435455i32).wrapping_neg();
                                        }
                                    }
                                }
                            }
                            empty => {
                                // §798
                                p = (268435455i32).wrapping_neg();
                            }
                            sub_box => {
                                p = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                            }
                            sub_mlist => {
                                {
                                    self.cur_mlist = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                    save_style = self.cur_style;
                                    self.mlist_penalties = false;
                                    self.mlist_to_hlist();
                                    self.cur_style = save_style;
                                    // §746
                                    {
                                        if (self.cur_style < script_style) {
                                            self.cur_size = text_size;
                                        } else {
                                            self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = { let __a809_0 = self.math_quad(self.cur_size); let __a809_1 = 18i32; self.x_over_n(__a809_0, __a809_1) };
                                    }
                                    // §798
                                    p = self.hpack(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), 0i32, additional);
                                }
                            }
                            _ => {
                                self.confusion(66311i32);
                            }
                        }
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(p);
                        if ((self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) && (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == empty)) {
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                        self.make_scripts(q, delta);
                    }
                    if __goto_1 <= 1 { // L82
                        // §770
                        z = self.hpack(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(), 0i32, additional);
                        if (self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int() > max_h) {
                            max_h = self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int();
                        }
                        if (self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int() > max_d) {
                            max_d = self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int();
                        }
                        self.free_node(z, box_node_size);
                    }
                    if __goto_1 <= 2 { // L80
                        r = q;
                        r_type = self.mem[crate::ix::U((r) as usize)].hh().b0();
                        if (r_type == right_noad) {
                            {
                                r_type = left_noad;
                                self.cur_style = style;
                                // §746
                                {
                                    if (self.cur_style < script_style) {
                                        self.cur_size = text_size;
                                    } else {
                                        self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                    }
                                    self.cur_mu = { let __a810_0 = self.math_quad(self.cur_size); let __a810_1 = 18i32; self.x_over_n(__a810_0, __a810_1) };
                                }
                            }
                        }
                    }
                    if __goto_1 <= 3 { // L81
                        // §770
                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                    }
                    break 'l_dispatch_1;
                }
            }
        }
        // §772
        if (r_type == bin_noad) {
            self.mem[crate::ix::U((r) as usize)].set_hh_b0(ord_noad);
        }
        // §808
        p = temp_head;
        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        q = mlist;
        r_type = 0i32;
        self.cur_style = style;
        // §746
        {
            if (self.cur_style < script_style) {
                self.cur_size = text_size;
            } else {
                self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
            }
            self.cur_mu = { let __a811_0 = self.math_quad(self.cur_size); let __a811_1 = 18i32; self.x_over_n(__a811_0, __a811_1) };
        }
        // §808
        while (q != (268435455i32).wrapping_neg()) {
            {
                'l_done_f: {
                    'l_L83_f: {
                        // §809
                        t = ord_noad;
                        s = noad_size;
                        pen = inf_penalty;
                        match self.mem[crate::ix::U((q) as usize)].hh().b0() {
                            op_noad | open_noad | close_noad | punct_noad | inner_noad => {
                                t = self.mem[crate::ix::U((q) as usize)].hh().b0();
                            }
                            bin_noad => {
                                {
                                    t = bin_noad;
                                    pen = self.eqtb[crate::ix::U(((7892273i32) - 1) as usize)].int();
                                }
                            }
                            rel_noad => {
                                {
                                    t = rel_noad;
                                    pen = self.eqtb[crate::ix::U(((7892274i32) - 1) as usize)].int();
                                }
                            }
                            ord_noad | vcenter_noad | over_noad | under_noad => {
                            }
                            radical_noad => {
                                s = radical_noad_size;
                            }
                            accent_noad => {
                                s = accent_noad_size;
                            }
                            fraction_noad => {
                                s = fraction_noad_size;
                            }
                            left_noad | right_noad => {
                                t = self.make_left_right(q, style, max_d, max_h);
                            }
                            style_node => {
                                // §811
                                {
                                    self.cur_style = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                    s = style_node_size;
                                    // §746
                                    {
                                        if (self.cur_style < script_style) {
                                            self.cur_size = text_size;
                                        } else {
                                            self.cur_size = (script_size).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                                        }
                                        self.cur_mu = { let __a812_0 = self.math_quad(self.cur_size); let __a812_1 = 18i32; self.x_over_n(__a812_0, __a812_1) };
                                    }
                                    // §811
                                    break 'l_L83_f;
                                }
                            }
                            whatsit_node | penalty_node | rule_node | disc_node | adjust_node | ins_node | mark_node | glue_node | kern_node => {
                                // §809
                                {
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                    p = q;
                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                    break 'l_done_f;
                                }
                            }
                            _ => {
                                self.confusion(66312i32);
                            }
                        }
                        // §814
                        if (r_type > 0i32) {
                            {
                                match self.str_pool[crate::ix::U(((((r_type).wrapping_mul(8i32)).wrapping_add(t)).wrapping_add(self.magic_offset)) as usize)] {
                                    48 => {
                                        x = 0i32;
                                    }
                                    49 => {
                                        if (self.cur_style < script_style) {
                                            x = thin_mu_skip_code;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    50 => {
                                        x = thin_mu_skip_code;
                                    }
                                    51 => {
                                        if (self.cur_style < script_style) {
                                            x = med_mu_skip_code;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    52 => {
                                        if (self.cur_style < script_style) {
                                            x = thick_mu_skip_code;
                                        } else {
                                            x = 0i32;
                                        }
                                    }
                                    _ => {
                                        self.confusion(66314i32);
                                    }
                                }
                                if (x != 0i32) {
                                    {
                                        y = self.math_glue(self.eqtb[crate::ix::U((((glue_base).wrapping_add(x)) - 1) as usize)].hh().rh(), self.cur_mu);
                                        z = self.new_glue(y);
                                        self.mem[crate::ix::U((y) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                                        p = z;
                                        self.mem[crate::ix::U((z) as usize)].set_hh_b1((x).wrapping_add(1i32));
                                    }
                                }
                            }
                        }
                        // §815
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() != (268435455i32).wrapping_neg()) {
                            {
                                { let __v813 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v813); }
                                loop {
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (self.mem[crate::ix::U((p) as usize)].hh().rh() == (268435455i32).wrapping_neg()) { break; }
                                }
                            }
                        }
                        if penalties {
                            if (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                if (pen < inf_penalty) {
                                    {
                                        r_type = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().b0();
                                        if (r_type != penalty_node) {
                                            if (r_type != rel_noad) {
                                                {
                                                    z = self.new_penalty(pen);
                                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                                                    p = z;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §808
                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == right_noad) {
                            t = open_noad;
                        }
                        r_type = t;
                    }
                    r = q;
                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                    self.free_node(r, s);
                }
            }
        }
    }

    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §820
    pub fn push_alignment(&mut self) {
        let mut p: halfword = 0; // §820
        p = self.get_node(align_stack_node_size);
        { let __v814 = self.align_ptr; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v814); }
        { let __v815 = self.cur_align; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v815); }
        { let __v816 = self.mem[crate::ix::U((align_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v816); }
        { let __v817 = self.cur_span; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(__v817); }
        { let __v818 = self.cur_loop; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v818); }
        { let __v819 = self.align_state; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v819); }
        { let __v820 = self.cur_head; self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_lh(__v820); }
        { let __v821 = self.cur_tail; self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_hh_rh(__v821); }
        { let __v822 = self.cur_pre_head; self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_lh(__v822); }
        { let __v823 = self.cur_pre_tail; self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_rh(__v823); }
        self.align_ptr = p;
        self.cur_head = self.get_avail();
        self.cur_pre_head = self.get_avail();
    }

    /// Alignment stack maintenance is handled by a pair of trivial routines
    /// called `push_alignment` and `pop_alignment`.
    // §820
    pub fn pop_alignment(&mut self) {
        let mut p: halfword = 0; // §820
        {
            { let __ix824 = self.cur_head; let __v825 = self.avail; self.mem[crate::ix::U((__ix824) as usize)].set_hh_rh(__v825); }
            self.avail = self.cur_head;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        {
            { let __ix826 = self.cur_pre_head; let __v827 = self.avail; self.mem[crate::ix::U((__ix826) as usize)].set_hh_rh(__v827); }
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
        { let __v828 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U((align_head) as usize)].set_hh_rh(__v828); }
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
    // §830
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
                self.fatal_error(65917i32);
            }
            if ((self.cur_cmd == assign_glue) && (self.cur_chr == 1205775i32)) {
                {
                    self.scan_optional_equals();
                    self.scan_glue(glue_val);
                    if (self.eqtb[crate::ix::U(((7892307i32) - 1) as usize)].int() > 0i32) {
                        self.geq_define(1205775i32, glue_ref, self.cur_val);
                    } else {
                        self.eq_define(1205775i32, glue_ref, self.cur_val);
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
    // §822
    pub fn init_align(&mut self) {
        let mut save_cs_ptr: halfword = 0; // §822
        let mut p: halfword = 0; // §822
        'l_done_f: {
            save_cs_ptr = self.cur_cs;
            self.push_alignment();
            self.align_state = (1000000i32).wrapping_neg();
            // §824
            if ((self.cur_list.mode_field == mmode) && ((self.cur_list.tail_field != self.cur_list.head_field) || (self.cur_list.aux_field.int() != (268435455i32).wrapping_neg()))) {
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
                    self.print_esc(65827i32);
                    self.print(66315i32);
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66316i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66317i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66318i32;
                    }
                    self.error();
                    self.flush_math();
                }
            }
            // §822
            self.push_nest();
            // §823
            if (self.cur_list.mode_field == mmode) {
                {
                    self.cur_list.mode_field = (1i32).wrapping_neg();
                    { let __v829 = self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(2i32)) as usize)].aux_field.int(); self.cur_list.aux_field.set_int(__v829); }
                }
            } else {
                if (self.cur_list.mode_field > 0i32) {
                    self.cur_list.mode_field = (self.cur_list.mode_field).wrapping_neg();
                }
            }
            // §822
            self.scan_spec(align_group, false);
            // §825
            self.mem[crate::ix::U((align_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
            self.cur_align = align_head;
            self.cur_loop = (268435455i32).wrapping_neg();
            self.scanner_status = aligning;
            self.warning_index = save_cs_ptr;
            self.align_state = (1000000i32).wrapping_neg();
            while true {
                {
                    'l_done2_f: {
                        'l_done1_f: {
                            // §826
                            { let __ix830 = self.cur_align; let __v831 = self.new_param_glue(tab_skip_code); self.mem[crate::ix::U((__ix830) as usize)].set_hh_rh(__v831); }
                            self.cur_align = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
                            // §825
                            if (self.cur_cmd == car_ret) {
                                break 'l_done_f;
                            }
                            // §831
                            p = hold_head;
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                            while true {
                                {
                                    self.get_preamble_token();
                                    if (self.cur_cmd == mac_param) {
                                        break 'l_done1_f;
                                    }
                                    if (((self.cur_cmd <= car_ret) && (self.cur_cmd >= tab_mark)) && (self.align_state == (1000000i32).wrapping_neg())) {
                                        if (((p == hold_head) && (self.cur_loop == (268435455i32).wrapping_neg())) && (self.cur_cmd == tab_mark)) {
                                            self.cur_loop = self.cur_align;
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
                                                    self.print(66324i32);
                                                }
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 66325i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 66326i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66327i32;
                                                }
                                                self.back_error();
                                                break 'l_done1_f;
                                            }
                                        }
                                    } else {
                                        if ((self.cur_cmd != spacer) || (p != hold_head)) {
                                            {
                                                { let __v832 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v832); }
                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                { let __v833 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v833); }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §827
                        { let __ix834 = self.cur_align; let __v835 = self.new_null_box(); self.mem[crate::ix::U((__ix834) as usize)].set_hh_rh(__v835); }
                        self.cur_align = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
                        { let __ix836 = self.cur_align; self.mem[crate::ix::U((__ix836) as usize)].set_hh_lh(end_span); }
                        { let __ix837 = (self.cur_align).wrapping_add(1i32); self.mem[crate::ix::U((__ix837) as usize)].set_int((1073741824i32).wrapping_neg()); }
                        { let __ix838 = (self.cur_align).wrapping_add(3i32); let __v839 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix838) as usize)].set_int(__v839); }
                        // §832
                        p = hold_head;
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
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
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(66328i32);
                                            }
                                            {
                                                self.help_ptr = 3i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 66325i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 66326i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 66329i32;
                                            }
                                            self.error();
                                            continue 'l_continue_b;
                                        }
                                    }
                                    { let __v840 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v840); }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    { let __v841 = self.cur_tok; self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v841); }
                                    break 'l_continue_b;
                                }
                            }
                        }
                    }
                    { let __v842 = self.get_avail(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v842); }
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(end_template_token);
                    // §827
                    { let __ix843 = (self.cur_align).wrapping_add(2i32); let __v844 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix843) as usize)].set_int(__v844); }
                }
            }
        }
        // §825
        self.scanner_status = normal;
        // §822
        self.new_save_level(align_group);
        if (self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh(), every_cr_text);
        }
        self.align_peek();
    }

    /// The parameter to `init_span` is a pointer to the alignrecord where the
    /// next column or group of columns will begin. A new semantic level is
    /// entered, so that the columns will generate a list for subsequent packaging.
    /// @<Declare the procedure called `init_span`
    // §835
    pub fn init_span(&mut self, mut p: halfword) {
        self.push_nest();
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
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
    // §834
    pub fn init_row(&mut self) {
        self.push_nest();
        self.cur_list.mode_field = ((106i32).wrapping_neg()).wrapping_sub(self.cur_list.mode_field);
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            self.cur_list.aux_field.set_hh_lh(0i32);
        } else {
            self.cur_list.aux_field.set_int(0i32);
        }
        {
            { let __ix845 = self.cur_list.tail_field; let __v846 = self.new_glue(self.mem[crate::ix::U(((self.mem[crate::ix::U((align_head) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix845) as usize)].set_hh_rh(__v846); }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix847 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix847) as usize)].set_hh_b1(12i32); }
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
    // §836
    pub fn init_col(&mut self) {
        { let __ix848 = (self.cur_align).wrapping_add(5i32); let __v849 = self.cur_cmd; self.mem[crate::ix::U((__ix848) as usize)].set_hh_lh(__v849); }
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
    // §839
    pub fn fin_col(&mut self) -> bool {
        let mut fin_col: bool = false;
        let mut p: halfword = 0; // §839
        let mut q: halfword = 0; // §839
        let mut r: halfword = 0; // §839
        let mut s: halfword = 0; // §839
        let mut u: halfword = 0; // §839
        let mut w: scaled = 0; // §839
        let mut o: glue_ord = 0; // §839
        let mut n: halfword = 0; // §839
        'l_exit_f: {
            if (self.cur_align == (268435455i32).wrapping_neg()) {
                self.confusion(66330i32);
            }
            q = self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh();
            if (q == (268435455i32).wrapping_neg()) {
                self.confusion(66330i32);
            }
            if (self.align_state < 500000i32) {
                self.fatal_error(65917i32);
            }
            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
            // §840
            if ((p == (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh() < cr_code)) {
                if (self.cur_loop != (268435455i32).wrapping_neg()) {
                    // §841
                    {
                        { let __v850 = self.new_null_box(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v850); }
                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(end_span);
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int((1073741824i32).wrapping_neg());
                        self.cur_loop = self.mem[crate::ix::U((self.cur_loop) as usize)].hh().rh();
                        // §842
                        q = hold_head;
                        r = self.mem[crate::ix::U(((self.cur_loop).wrapping_add(3i32)) as usize)].int();
                        while (r != (268435455i32).wrapping_neg()) {
                            {
                                { let __v851 = self.get_avail(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v851); }
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                { let __v852 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v852); }
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        { let __v853 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v853); }
                        q = hold_head;
                        r = self.mem[crate::ix::U(((self.cur_loop).wrapping_add(2i32)) as usize)].int();
                        while (r != (268435455i32).wrapping_neg()) {
                            {
                                { let __v854 = self.get_avail(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v854); }
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                { let __v855 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v855); }
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                        { let __v856 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v856); }
                        // §841
                        self.cur_loop = self.mem[crate::ix::U((self.cur_loop) as usize)].hh().rh();
                        { let __v857 = self.new_glue(self.mem[crate::ix::U(((self.cur_loop).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v857); }
                        { let __ix858 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((__ix858) as usize)].set_hh_b1(12i32); }
                    }
                } else {
                    // §840
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66331i32);
                        }
                        self.print_esc(66320i32);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66332i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66333i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66334i32;
                        }
                        { let __ix859 = (self.cur_align).wrapping_add(5i32); self.mem[crate::ix::U((__ix859) as usize)].set_hh_lh(cr_code); }
                        self.error();
                    }
                }
            }
            // §839
            if (self.mem[crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)].hh().lh() != span_code) {
                {
                    self.unsave();
                    self.new_save_level(align_group);
                    // §844
                    {
                        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
                            {
                                self.adjust_tail = self.cur_tail;
                                self.pre_adjust_tail = self.cur_pre_tail;
                                u = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional);
                                w = self.mem[crate::ix::U(((u).wrapping_add(1i32)) as usize)].int();
                                self.cur_tail = self.adjust_tail;
                                self.adjust_tail = (268435455i32).wrapping_neg();
                                self.cur_pre_tail = self.pre_adjust_tail;
                                self.pre_adjust_tail = (268435455i32).wrapping_neg();
                            }
                        } else {
                            {
                                u = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional, 0i32);
                                w = self.mem[crate::ix::U(((u).wrapping_add(3i32)) as usize)].int();
                            }
                        }
                        n = min_quarterword;
                        if (self.cur_span != self.cur_align) {
                            // §846
                            {
                                q = self.cur_span;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                                    if (q == self.cur_align) { break; }
                                }
                                if (n > max_quarterword) {
                                    self.confusion(66335i32);
                                }
                                q = self.cur_span;
                                while (self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().lh()) as usize)].hh().rh() < n) {
                                    q = self.mem[crate::ix::U((q) as usize)].hh().lh();
                                }
                                if (self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().lh()) as usize)].hh().rh() > n) {
                                    {
                                        s = self.get_node(span_node_size);
                                        { let __v860 = self.mem[crate::ix::U((q) as usize)].hh().lh(); self.mem[crate::ix::U((s) as usize)].set_hh_lh(__v860); }
                                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(n);
                                        self.mem[crate::ix::U((q) as usize)].set_hh_lh(s);
                                        self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].set_int(w);
                                    }
                                } else {
                                    if (self.mem[crate::ix::U(((self.mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int() < w) {
                                        { let __ix861 = (self.mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix861) as usize)].set_int(w); }
                                    }
                                }
                            }
                        } else {
                            // §844
                            if (w > self.mem[crate::ix::U(((self.cur_align).wrapping_add(1i32)) as usize)].int()) {
                                { let __ix862 = (self.cur_align).wrapping_add(1i32); self.mem[crate::ix::U((__ix862) as usize)].set_int(w); }
                            }
                        }
                        self.mem[crate::ix::U((u) as usize)].set_hh_b0(unset_node);
                        self.mem[crate::ix::U((u) as usize)].set_hh_b1(n);
                        // §701
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
                        // §844
                        self.mem[crate::ix::U(((u).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                        { let __v863 = self.total_stretch[crate::ix::U((o) as usize)]; self.mem[crate::ix::U(((u).wrapping_add(6i32)) as usize)].set_int(__v863); }
                        // §707
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
                        // §844
                        self.mem[crate::ix::U(((u).wrapping_add(5i32)) as usize)].set_hh_b0(o);
                        { let __v864 = self.total_shrink[crate::ix::U((o) as usize)]; self.mem[crate::ix::U(((u).wrapping_add(4i32)) as usize)].set_int(__v864); }
                        self.pop_nest();
                        { let __ix865 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix865) as usize)].set_hh_rh(u); }
                        self.cur_list.tail_field = u;
                    }
                    // §843
                    {
                        { let __ix866 = self.cur_list.tail_field; let __v867 = self.new_glue(self.mem[crate::ix::U(((self.mem[crate::ix::U((self.cur_align) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U((__ix866) as usize)].set_hh_rh(__v867); }
                        self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                    { let __ix868 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix868) as usize)].set_hh_b1(12i32); }
                    // §839
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
    // §847
    pub fn fin_row(&mut self) {
        let mut p: halfword = 0; // §847
        if (self.cur_list.mode_field == (105i32).wrapping_neg()) {
            {
                p = self.hpack(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional);
                self.pop_nest();
                if (self.cur_pre_head != self.cur_pre_tail) {
                    {
                        { let __ix869 = self.cur_list.tail_field; let __v870 = self.mem[crate::ix::U((self.cur_pre_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix869) as usize)].set_hh_rh(__v870); }
                        self.cur_list.tail_field = self.cur_pre_tail;
                    }
                }
                self.append_to_vlist(p);
                if (self.cur_head != self.cur_tail) {
                    {
                        { let __ix871 = self.cur_list.tail_field; let __v872 = self.mem[crate::ix::U((self.cur_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix871) as usize)].set_hh_rh(__v872); }
                        self.cur_list.tail_field = self.cur_tail;
                    }
                }
            }
        } else {
            {
                p = self.vpackage(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional, max_dimen);
                self.pop_nest();
                { let __ix873 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix873) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
                self.cur_list.aux_field.set_hh_lh(1000i32);
            }
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(unset_node);
        self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(0i32);
        if (self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
            self.begin_token_list(self.eqtb[crate::ix::U(((every_cr_loc) - 1) as usize)].hh().rh(), every_cr_text);
        }
        self.align_peek();
    }

    /// Finally, we will reach the end of the alignment, and we can breathe a
    /// sigh of relief that memory hasn't overflowed. All the unset boxes will now be
    /// set so that the columns line up, taking due account of spanned columns.
    // §848
    pub fn fin_align(&mut self) {
        let mut p: halfword = 0; // §848
        let mut q: halfword = 0; // §848
        let mut r: halfword = 0; // §848
        let mut s: halfword = 0; // §848
        let mut u: halfword = 0; // §848
        let mut v: halfword = 0; // §848
        let mut t: scaled = 0; // §848
        let mut w: scaled = 0; // §848
        let mut o: scaled = 0; // §848
        let mut n: halfword = 0; // §848
        let mut rule_save: scaled = 0; // §848
        let mut aux_save: memory_word = memory_word::default(); // §848
        if (self.cur_group != align_group) {
            self.confusion(66336i32);
        }
        self.unsave();
        if (self.cur_group != align_group) {
            self.confusion(66337i32);
        }
        self.unsave();
        if (self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(1i32)) as usize)].mode_field == mmode) {
            o = self.eqtb[crate::ix::U(((9006735i32) - 1) as usize)].int();
        } else {
            o = 0i32;
        }
        // §849
        q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
        loop {
            self.flush_list(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int());
            self.flush_list(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int());
            p = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                // §850
                {
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                    r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                    s = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh();
                    if (s != zero_glue) {
                        {
                            { let __v874 = (self.mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v874); }
                            self.delete_glue_ref(s);
                            self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(zero_glue);
                        }
                    }
                }
            }
            // §849
            if (self.mem[crate::ix::U((q) as usize)].hh().lh() != end_span) {
                // §851
                {
                    t = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((self.mem[crate::ix::U(((self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int());
                    r = self.mem[crate::ix::U((q) as usize)].hh().lh();
                    s = end_span;
                    self.mem[crate::ix::U((s) as usize)].set_hh_lh(p);
                    n = 1i32;
                    loop {
                        { let __v875 = (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v875); }
                        u = self.mem[crate::ix::U((r) as usize)].hh().lh();
                        while (self.mem[crate::ix::U((r) as usize)].hh().rh() > n) {
                            {
                                s = self.mem[crate::ix::U((s) as usize)].hh().lh();
                                n = (self.mem[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32);
                            }
                        }
                        if (self.mem[crate::ix::U((r) as usize)].hh().rh() < n) {
                            {
                                { let __v876 = self.mem[crate::ix::U((s) as usize)].hh().lh(); self.mem[crate::ix::U((r) as usize)].set_hh_lh(__v876); }
                                self.mem[crate::ix::U((s) as usize)].set_hh_lh(r);
                                { let __v877 = (self.mem[crate::ix::U((r) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v877); }
                                s = r;
                            }
                        } else {
                            {
                                if (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((self.mem[crate::ix::U((s) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int()) {
                                    { let __ix878 = (self.mem[crate::ix::U((s) as usize)].hh().lh()).wrapping_add(1i32); let __v879 = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix878) as usize)].set_int(__v879); }
                                }
                                self.free_node(r, span_node_size);
                            }
                        }
                        r = u;
                        if (r == end_span) { break; }
                    }
                }
            }
            // §849
            self.mem[crate::ix::U((q) as usize)].set_hh_b0(unset_node);
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(min_quarterword);
            self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
            self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
            self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(0i32);
            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(0i32);
            q = p;
            if (q == (268435455i32).wrapping_neg()) { break; }
        }
        // §852
        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
        self.pack_begin_line = (self.cur_list.ml_field).wrapping_neg();
        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
            {
                rule_save = self.eqtb[crate::ix::U(((9006736i32) - 1) as usize)].int();
                self.eqtb[crate::ix::U(((9006736i32) - 1) as usize)].set_int(0i32);
                p = self.hpack(self.mem[crate::ix::U((align_head) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int());
                self.eqtb[crate::ix::U(((9006736i32) - 1) as usize)].set_int(rule_save);
            }
        } else {
            {
                q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
                loop {
                    { let __v880 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v880); }
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                    if (q == (268435455i32).wrapping_neg()) { break; }
                }
                p = self.vpackage(self.mem[crate::ix::U((align_head) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(), max_dimen);
                q = self.mem[crate::ix::U((self.mem[crate::ix::U((align_head) as usize)].hh().rh()) as usize)].hh().rh();
                loop {
                    { let __v881 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v881); }
                    self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(0i32);
                    q = self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().rh();
                    if (q == (268435455i32).wrapping_neg()) { break; }
                }
            }
        }
        self.pack_begin_line = 0i32;
        // §853
        q = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
        s = self.cur_list.head_field;
        while (q != (268435455i32).wrapping_neg()) {
            {
                if (!(q >= self.hi_mem_min)) {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == unset_node) {
                        // §855
                        {
                            if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(hlist_node);
                                    { let __v882 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v882); }
                                    if (self.nest[crate::ix::U(((self.nest_ptr).wrapping_sub(1i32)) as usize)].mode_field == mmode) {
                                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(dlist);
                                    }
                                }
                            } else {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(vlist_node);
                                    { let __v883 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v883); }
                                }
                            }
                            { let __v884 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b1(__v884); }
                            { let __v885 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_hh_b0(__v885); }
                            { let __v886 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr(); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_gr(__v886); }
                            self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(o);
                            r = self.mem[crate::ix::U((self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh();
                            s = self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh();
                            loop {
                                // §856
                                n = self.mem[crate::ix::U((r) as usize)].hh().b1();
                                t = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int();
                                w = t;
                                u = hold_head;
                                self.mem[crate::ix::U((r) as usize)].set_hh_b1(0i32);
                                while (n > min_quarterword) {
                                    {
                                        n = (n).wrapping_sub(1i32);
                                        // §857
                                        s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                        v = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().lh();
                                        { let __v887 = self.new_glue(v); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v887); }
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
                                        { let __v888 = self.new_null_box(); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v888); }
                                        u = self.mem[crate::ix::U((u) as usize)].hh().rh();
                                        t = (t).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int());
                                        if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                            { let __v889 = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((u).wrapping_add(1i32)) as usize)].set_int(__v889); }
                                        } else {
                                            {
                                                self.mem[crate::ix::U((u) as usize)].set_hh_b0(vlist_node);
                                                { let __v890 = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((u).wrapping_add(3i32)) as usize)].set_int(__v890); }
                                            }
                                        }
                                    }
                                }
                                // §856
                                if (self.cur_list.mode_field == (1i32).wrapping_neg()) {
                                    // §858
                                    {
                                        { let __v891 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v891); }
                                        { let __v892 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v892); }
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
                                                        { let __v893 = ((((t).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int())) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v893); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v894 = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(__v894); }
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b1() == normal) && ((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t) > self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int())) {
                                                            self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                                        } else {
                                                            { let __v895 = ((((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(t)) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v895); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(hlist_node);
                                    }
                                } else {
                                    // §859
                                    {
                                        { let __v896 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v896); }
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
                                                        { let __v897 = ((((t).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int())) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v897); }
                                                    }
                                                }
                                            } else {
                                                {
                                                    { let __v898 = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b0(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(__v898); }
                                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                                                    if (self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int() == 0i32) {
                                                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                                    } else {
                                                        if ((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().b1() == normal) && ((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(t) > self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int())) {
                                                            self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                                        } else {
                                                            { let __v899 = ((((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(t)) as f64) / ((self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v899); }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(w);
                                        self.mem[crate::ix::U((r) as usize)].set_hh_b0(vlist_node);
                                    }
                                }
                                // §856
                                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                                if (u != hold_head) {
                                    {
                                        { let __v900 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((u) as usize)].set_hh_rh(__v900); }
                                        { let __v901 = self.mem[crate::ix::U((hold_head) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v901); }
                                        r = u;
                                    }
                                }
                                // §855
                                r = self.mem[crate::ix::U((self.mem[crate::ix::U((r) as usize)].hh().rh()) as usize)].hh().rh();
                                s = self.mem[crate::ix::U((self.mem[crate::ix::U((s) as usize)].hh().rh()) as usize)].hh().rh();
                                if (r == (268435455i32).wrapping_neg()) { break; }
                            }
                        }
                    } else {
                        // §853
                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == rule_node) {
                            // §854
                            {
                                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v902 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v902); }
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v903 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v903); }
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
                                    { let __v904 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v904); }
                                }
                                if (o != 0i32) {
                                    {
                                        r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
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
                // §853
                s = q;
                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
            }
        }
        // §848
        self.flush_node_list(p);
        self.pop_alignment();
        // §860
        aux_save = self.cur_list.aux_field;
        p = self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
        q = self.cur_list.tail_field;
        self.pop_nest();
        if (self.cur_list.mode_field == mmode) {
            // §1260
            {
                self.do_assignments();
                if (self.cur_cmd != math_shift) {
                    // §1261
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66602i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66316i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66317i32;
                        }
                        self.back_error();
                    }
                } else {
                    // §1251
                    {
                        self.get_x_token();
                        if (self.cur_cmd != math_shift) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66598i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66599i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66600i32;
                                }
                                self.back_error();
                            }
                        }
                    }
                }
                // §1260
                self.flush_node_list(self.cur_list.eTeX_aux_field);
                self.pop_nest();
                {
                    { let __ix905 = self.cur_list.tail_field; let __v906 = self.new_penalty(self.eqtb[crate::ix::U(((7892275i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix905) as usize)].set_hh_rh(__v906); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                {
                    { let __ix907 = self.cur_list.tail_field; let __v908 = self.new_param_glue(above_display_skip_code); self.mem[crate::ix::U((__ix907) as usize)].set_hh_rh(__v908); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                { let __ix909 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix909) as usize)].set_hh_rh(p); }
                if (p != (268435455i32).wrapping_neg()) {
                    self.cur_list.tail_field = q;
                }
                {
                    { let __ix910 = self.cur_list.tail_field; let __v911 = self.new_penalty(self.eqtb[crate::ix::U(((7892276i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix910) as usize)].set_hh_rh(__v911); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                {
                    { let __ix912 = self.cur_list.tail_field; let __v913 = self.new_param_glue(below_display_skip_code); self.mem[crate::ix::U((__ix912) as usize)].set_hh_rh(__v913); }
                    self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                self.cur_list.aux_field.set_int(aux_save.int());
                self.resume_after_display();
            }
        } else {
            // §860
            {
                self.cur_list.aux_field = aux_save;
                { let __ix914 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix914) as usize)].set_hh_rh(p); }
                if (p != (268435455i32).wrapping_neg()) {
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
    // §833
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
    // §874
    pub fn finite_shrink(&mut self, mut p: halfword) -> halfword {
        let mut finite_shrink: halfword = 0;
        let mut q: halfword = 0; // §874
        if self.no_shrink_error_yet {
            {
                self.no_shrink_error_yet = false;
                if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                    self.end_diagnostic(true);
                }
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66338i32);
                }
                {
                    self.help_ptr = 5i32;
                    self.help_line[crate::ix::U((4i32) as usize)] = 66339i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 66340i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66341i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66342i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66343i32;
                }
                self.error();
                if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
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
    // §877
    pub fn push_node(&mut self, mut p: halfword) {
        if (self.hlist_stack_level > max_hlist_stack) {
            self.pdf_error(66344i32, 66345i32);
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
    // §877
    pub fn pop_node(&mut self) -> halfword {
        let mut pop_node: halfword = 0;
        self.hlist_stack_level = (self.hlist_stack_level).wrapping_sub(1i32);
        if (self.hlist_stack_level < 0i32) {
            self.pdf_error(66346i32, 66347i32);
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
    // §877
    pub fn find_protchar_left(&mut self, mut l: halfword, mut d: bool) -> halfword {
        let mut find_protchar_left: halfword = 0;
        let mut t: halfword = 0; // §877
        let mut run: bool = false; // §877
        if ((((((self.mem[crate::ix::U((l) as usize)].hh().rh() != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) {
            l = self.mem[crate::ix::U((l) as usize)].hh().rh();
        } else {
            if d {
                while ((self.mem[crate::ix::U((l) as usize)].hh().rh() != (268435455i32).wrapping_neg()) && (!((l >= self.hi_mem_min) || (self.mem[crate::ix::U((l) as usize)].hh().b0() < math_node)))) {
                    l = self.mem[crate::ix::U((l) as usize)].hh().rh();
                }
            }
        }
        self.hlist_stack_level = 0i32;
        run = true;
        loop {
            t = l;
            while ((run && (self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
                {
                    self.push_node(l);
                    l = self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh();
                }
            }
            while (run && ((!(l >= self.hi_mem_min)) && (((((((((self.mem[crate::ix::U((l) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((l) as usize)].hh().b0() == penalty_node)) || ((((self.mem[crate::ix::U((l) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U((l) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == kern_node) && ((self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((l) as usize)].hh().b1() == normal)))) || ((self.mem[crate::ix::U((l) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((l) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((l).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()))))) {
                {
                    while ((self.mem[crate::ix::U((l) as usize)].hh().rh() == (268435455i32).wrapping_neg()) && (self.hlist_stack_level > 0i32)) {
                        {
                            l = self.pop_node();
                        }
                    }
                    if (self.mem[crate::ix::U((l) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
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
    // §877
    pub fn find_protchar_right(&mut self, mut l: halfword, mut r: halfword) -> halfword {
        let mut find_protchar_right: halfword = 0;
        let mut t: halfword = 0; // §877
        let mut run: bool = false; // §877
        find_protchar_right = (268435455i32).wrapping_neg();
        if (r == (268435455i32).wrapping_neg()) {
            return find_protchar_right;
        }
        self.hlist_stack_level = 0i32;
        run = true;
        loop {
            t = r;
            while ((run && (self.mem[crate::ix::U((r) as usize)].hh().b0() == hlist_node)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
                {
                    self.push_node(l);
                    self.push_node(r);
                    l = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh();
                    r = l;
                    while (self.mem[crate::ix::U((r) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
            }
            while (run && ((!(r >= self.hi_mem_min)) && (((((((((self.mem[crate::ix::U((r) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == penalty_node)) || ((((self.mem[crate::ix::U((r) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U((r) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == kern_node) && ((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((r) as usize)].hh().b1() == normal)))) || ((self.mem[crate::ix::U((r) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((r) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()))))) {
                {
                    while ((r == l) && (self.hlist_stack_level > 0i32)) {
                        {
                            r = self.pop_node();
                            l = self.pop_node();
                        }
                    }
                    if ((r != l) && (r != (268435455i32).wrapping_neg())) {
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
    // §877
    pub fn total_pw(&mut self, mut q: halfword, mut p: halfword) -> scaled {
        let mut total_pw: scaled = 0;
        let mut l: halfword = 0; // §877
        let mut r: halfword = 0; // §877
        let mut n: i32 = 0; // §877
        'l_done_f: {
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                l = self.first_p;
            } else {
                l = self.mem[crate::ix::U(((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().rh();
            }
            r = self.prev_rightmost(self.global_prev_p, p);
            if (((p != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node)) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg())) {
                {
                    r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                    while (self.mem[crate::ix::U((r) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
            } else {
                r = self.find_protchar_right(l, r);
            }
            if ((l != (268435455i32).wrapping_neg()) && (self.mem[crate::ix::U((l) as usize)].hh().b0() == disc_node)) {
                {
                    if (self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
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
                                    if (self.mem[crate::ix::U((l) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
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
    // §877
    pub fn try_break(&mut self, mut pi: i32, mut break_type: small_number) {
        let mut r: halfword = 0; // §877
        let mut prev_r: halfword = 0; // §877
        let mut old_l: halfword = 0; // §877
        let mut no_break_yet: bool = false; // §877
        let mut prev_prev_r: halfword = 0; // §878
        let mut s: halfword = 0; // §878
        let mut q: halfword = 0; // §878
        let mut v: halfword = 0; // §878
        let mut t: i32 = 0; // §878
        let mut f: internal_font_number = 0; // §878
        let mut l: halfword = 0; // §878
        let mut node_r_stays_active: bool = false; // §878
        let mut line_width: scaled = 0; // §878
        let mut fit_class: i32 = 0; // §878
        let mut b: halfword = 0; // §878
        let mut d: i32 = 0; // §878
        let mut artificial_demerits: bool = false; // §878
        let mut save_link: halfword = 0; // §878
        let mut shortfall: scaled = 0; // §878
        let mut g: scaled = 0; // §1655
        'l_exit_f: {
            // §879
            if ((pi).wrapping_abs() >= inf_penalty) {
                if (pi > 0i32) {
                    break 'l_exit_f;
                } else {
                    pi = (10000i32).wrapping_neg();
                }
            }
            // §877
            no_break_yet = true;
            prev_r = active;
            old_l = 0i32;
            { let __v915 = self.active_width[crate::ix::U(((1i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v915; }
            { let __v916 = self.active_width[crate::ix::U(((2i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v916; }
            { let __v917 = self.active_width[crate::ix::U(((3i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v917; }
            { let __v918 = self.active_width[crate::ix::U(((4i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v918; }
            { let __v919 = self.active_width[crate::ix::U(((5i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v919; }
            { let __v920 = self.active_width[crate::ix::U(((6i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v920; }
            while true {
                {
                    'l_continue_b: loop {
                        r = self.mem[crate::ix::U((prev_r) as usize)].hh().rh();
                        // §880
                        if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                            {
                                { let __v921 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v921; }
                                { let __v922 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v922; }
                                { let __v923 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v923; }
                                { let __v924 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v924; }
                                { let __v925 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v925; }
                                { let __v926 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v926; }
                                prev_prev_r = prev_r;
                                prev_r = r;
                                continue 'l_continue_b;
                            }
                        }
                        // §883
                        {
                            l = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh();
                            if (l > old_l) {
                                {
                                    if ((self.minimum_demerits < awful_bad) && ((old_l != self.easy_line) || (r == last_active))) {
                                        // §884
                                        {
                                            if no_break_yet {
                                                // §885
                                                {
                                                    'l_done_f: {
                                                        no_break_yet = false;
                                                        { let __v927 = self.background[crate::ix::U(((1i32) - 1) as usize)]; self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v927; }
                                                        { let __v928 = self.background[crate::ix::U(((2i32) - 1) as usize)]; self.break_width[crate::ix::U(((2i32) - 1) as usize)] = __v928; }
                                                        { let __v929 = self.background[crate::ix::U(((3i32) - 1) as usize)]; self.break_width[crate::ix::U(((3i32) - 1) as usize)] = __v929; }
                                                        { let __v930 = self.background[crate::ix::U(((4i32) - 1) as usize)]; self.break_width[crate::ix::U(((4i32) - 1) as usize)] = __v930; }
                                                        { let __v931 = self.background[crate::ix::U(((5i32) - 1) as usize)]; self.break_width[crate::ix::U(((5i32) - 1) as usize)] = __v931; }
                                                        { let __v932 = self.background[crate::ix::U(((6i32) - 1) as usize)]; self.break_width[crate::ix::U(((6i32) - 1) as usize)] = __v932; }
                                                        s = self.cur_p;
                                                        if (break_type > unhyphenated) {
                                                            if (self.cur_p != (268435455i32).wrapping_neg()) {
                                                                // §888
                                                                {
                                                                    t = self.mem[crate::ix::U((self.cur_p) as usize)].hh().b1();
                                                                    v = self.cur_p;
                                                                    s = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                                                                    while (t > 0i32) {
                                                                        {
                                                                            t = (t).wrapping_sub(1i32);
                                                                            v = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                                            // §889
                                                                            if (v >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[crate::ix::U((v) as usize)].hh().b0();
                                                                                    { let __v933 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub({ let __s935 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s934 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((v) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s934)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s935)] }.int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v933; }
                                                                                }
                                                                            } else {
                                                                                match self.mem[crate::ix::U((v) as usize)].hh().b0() {
                                                                                    ligature_node => {
                                                                                        {
                                                                                            f = self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                            self.xtx_ligature_present = true;
                                                                                            { let __v936 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub({ let __s938 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s937 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s937)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s938)] }.int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v936; }
                                                                                        }
                                                                                    }
                                                                                    hlist_node | vlist_node | rule_node | kern_node => {
                                                                                        { let __v939 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v939; }
                                                                                    }
                                                                                    whatsit_node => {
                                                                                        if (((((self.mem[crate::ix::U((v) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((v) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((v) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((v) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((v) as usize)].hh().b1() == pdf_node)) {
                                                                                            { let __v940 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v940; }
                                                                                        } else {
                                                                                            self.confusion(66348i32);
                                                                                        }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(66349i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §888
                                                                    while (s != (268435455i32).wrapping_neg()) {
                                                                        {
                                                                            // §890
                                                                            if (s >= self.hi_mem_min) {
                                                                                {
                                                                                    f = self.mem[crate::ix::U((s) as usize)].hh().b0();
                                                                                    { let __v941 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s943 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s942 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((s) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s942)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s943)] }.int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v941; }
                                                                                }
                                                                            } else {
                                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                                    ligature_node => {
                                                                                        {
                                                                                            f = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b0();
                                                                                            self.xtx_ligature_present = true;
                                                                                            { let __v944 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add({ let __s946 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s945 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s945)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s946)] }.int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v944; }
                                                                                        }
                                                                                    }
                                                                                    hlist_node | vlist_node | rule_node | kern_node => {
                                                                                        { let __v947 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v947; }
                                                                                    }
                                                                                    whatsit_node => {
                                                                                        if (((((self.mem[crate::ix::U((s) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((s) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((s) as usize)].hh().b1() == pdf_node)) {
                                                                                            { let __v948 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v948; }
                                                                                        } else {
                                                                                            self.confusion(66350i32);
                                                                                        }
                                                                                    }
                                                                                    _ => {
                                                                                        self.confusion(66351i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                            // §888
                                                                            s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                                                        }
                                                                    }
                                                                    { let __v949 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.disc_width); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v949; }
                                                                    if (self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                                        s = self.mem[crate::ix::U((v) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §885
                                                        while (s != (268435455i32).wrapping_neg()) {
                                                            {
                                                                if (s >= self.hi_mem_min) {
                                                                    break 'l_done_f;
                                                                }
                                                                match self.mem[crate::ix::U((s) as usize)].hh().b0() {
                                                                    glue_node => {
                                                                        // §886
                                                                        {
                                                                            v = self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].hh().lh();
                                                                            { let __v950 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v950; }
                                                                            { let __ix951 = (2i32).wrapping_add(self.mem[crate::ix::U((v) as usize)].hh().b0()); let __v952 = (self.break_width[crate::ix::U((((2i32).wrapping_add(self.mem[crate::ix::U((v) as usize)].hh().b0())) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()); self.break_width[crate::ix::U(((__ix951) - 1) as usize)] = __v952; }
                                                                            { let __v953 = (self.break_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()); self.break_width[crate::ix::U(((6i32) - 1) as usize)] = __v953; }
                                                                        }
                                                                    }
                                                                    penalty_node => {
                                                                        // §885
                                                                    }
                                                                    math_node => {
                                                                        { let __v954 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v954; }
                                                                    }
                                                                    kern_node => {
                                                                        if (self.mem[crate::ix::U((s) as usize)].hh().b1() != explicit) {
                                                                            break 'l_done_f;
                                                                        } else {
                                                                            { let __v955 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int()); self.break_width[crate::ix::U(((1i32) - 1) as usize)] = __v955; }
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
                                            // §891
                                            if (self.mem[crate::ix::U((prev_r) as usize)].hh().b0() == delta_node) {
                                                {
                                                    { let __v956 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].set_int(__v956); }
                                                    { let __v957 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].set_int(__v957); }
                                                    { let __v958 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].set_int(__v958); }
                                                    { let __v959 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].set_int(__v959); }
                                                    { let __v960 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].set_int(__v960); }
                                                    { let __v961 = ((self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()).wrapping_sub(self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)])).wrapping_add(self.break_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].set_int(__v961); }
                                                }
                                            } else {
                                                if (prev_r == active) {
                                                    {
                                                        { let __v962 = self.break_width[crate::ix::U(((1i32) - 1) as usize)]; self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v962; }
                                                        { let __v963 = self.break_width[crate::ix::U(((2i32) - 1) as usize)]; self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v963; }
                                                        { let __v964 = self.break_width[crate::ix::U(((3i32) - 1) as usize)]; self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v964; }
                                                        { let __v965 = self.break_width[crate::ix::U(((4i32) - 1) as usize)]; self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v965; }
                                                        { let __v966 = self.break_width[crate::ix::U(((5i32) - 1) as usize)]; self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v966; }
                                                        { let __v967 = self.break_width[crate::ix::U(((6i32) - 1) as usize)]; self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v967; }
                                                    }
                                                } else {
                                                    {
                                                        q = self.get_node(delta_node_size);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_b0(delta_node);
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                                        { let __v968 = (self.break_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v968); }
                                                        { let __v969 = (self.break_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v969); }
                                                        { let __v970 = (self.break_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v970); }
                                                        { let __v971 = (self.break_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v971); }
                                                        { let __v972 = (self.break_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_int(__v972); }
                                                        { let __v973 = (self.break_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(__v973); }
                                                        self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                        prev_prev_r = prev_r;
                                                        prev_r = q;
                                                    }
                                                }
                                            }
                                            // §884
                                            if ((self.eqtb[crate::ix::U(((7892280i32) - 1) as usize)].int()).wrapping_abs() >= (awful_bad).wrapping_sub(self.minimum_demerits)) {
                                                self.minimum_demerits = 1073741822i32;
                                            } else {
                                                self.minimum_demerits = (self.minimum_demerits).wrapping_add((self.eqtb[crate::ix::U(((7892280i32) - 1) as usize)].int()).wrapping_abs());
                                            }
                                            {
                                                let __for_end_11 = tight_fit;
                                                fit_class = very_loose_fit;
                                                while fit_class <= __for_end_11 {
                                                    {
                                                        if (self.minimal_demerits[crate::ix::U((fit_class) as usize)] <= self.minimum_demerits) {
                                                            // §893
                                                            {
                                                                q = self.get_node(passive_node_size);
                                                                { let __v974 = self.passive; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v974); }
                                                                self.passive = q;
                                                                { let __v975 = self.cur_p; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v975); }
                                                                self.pass_number = (self.pass_number).wrapping_add(1i32);
                                                                { let __v976 = self.pass_number; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v976); }
                                                                { let __v977 = self.best_place[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v977); }
                                                                q = self.get_node(self.active_node_size);
                                                                { let __v978 = self.passive; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v978); }
                                                                { let __v979 = (self.best_pl_line[crate::ix::U((fit_class) as usize)]).wrapping_add(1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v979); }
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_b1(fit_class);
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_b0(break_type);
                                                                { let __v980 = self.minimal_demerits[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v980); }
                                                                if self.do_last_line_fit {
                                                                    // §1662
                                                                    {
                                                                        { let __v981 = self.best_pl_short[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v981); }
                                                                        { let __v982 = self.best_pl_glue[crate::ix::U((fit_class) as usize)]; self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v982); }
                                                                    }
                                                                }
                                                                // §893
                                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                                self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                                prev_r = q;
                                                                if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                                                                    // §894
                                                                    {
                                                                        self.print_nl(66352i32);
                                                                        self.print_int(self.mem[crate::ix::U((self.passive) as usize)].hh().lh());
                                                                        self.print(66353i32);
                                                                        self.print_int((self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_sub(1i32));
                                                                        self.print_char(46i32);
                                                                        self.print_int(fit_class);
                                                                        if (break_type == hyphenated) {
                                                                            self.print_char(45i32);
                                                                        }
                                                                        self.print(66354i32);
                                                                        self.print_int(self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int());
                                                                        if self.do_last_line_fit {
                                                                            // §1663
                                                                            {
                                                                                self.print(66954i32);
                                                                                self.print_scaled(self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int());
                                                                                if (self.cur_p == (268435455i32).wrapping_neg()) {
                                                                                    self.print(66955i32);
                                                                                } else {
                                                                                    self.print(66425i32);
                                                                                }
                                                                                self.print_scaled(self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].int());
                                                                            }
                                                                        }
                                                                        // §894
                                                                        self.print(66355i32);
                                                                        if (self.mem[crate::ix::U(((self.passive).wrapping_add(1i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg()) {
                                                                            self.print_char(48i32);
                                                                        } else {
                                                                            self.print_int(self.mem[crate::ix::U((self.mem[crate::ix::U(((self.passive).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().lh());
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §884
                                                        self.minimal_demerits[crate::ix::U((fit_class) as usize)] = awful_bad;
                                                    }
                                                    fit_class = fit_class.wrapping_add(1);
                                                }
                                            }
                                            self.minimum_demerits = awful_bad;
                                            // §892
                                            if (r != last_active) {
                                                {
                                                    q = self.get_node(delta_node_size);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(delta_node);
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                                    { let __v983 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((1i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v983); }
                                                    { let __v984 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((2i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v984); }
                                                    { let __v985 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((3i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v985); }
                                                    { let __v986 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((4i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_int(__v986); }
                                                    { let __v987 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((5i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_int(__v987); }
                                                    { let __v988 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.break_width[crate::ix::U(((6i32) - 1) as usize)]); self.mem[crate::ix::U(((q).wrapping_add(6i32)) as usize)].set_int(__v988); }
                                                    self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(q);
                                                    prev_prev_r = prev_r;
                                                    prev_r = q;
                                                }
                                            }
                                        }
                                    }
                                    // §883
                                    if (r == last_active) {
                                        break 'l_exit_f;
                                    }
                                    // §898
                                    if (l > self.easy_line) {
                                        {
                                            line_width = self.second_width;
                                            old_l = 1073741822i32;
                                        }
                                    } else {
                                        {
                                            old_l = l;
                                            if (l > self.last_special_line) {
                                                line_width = self.second_width;
                                            } else {
                                                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
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
                        // §899
                        {
                            'l_L60_f: {
                                'l_found_f: {
                                    artificial_demerits = false;
                                    shortfall = (line_width).wrapping_sub(self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]);
                                    if (self.eqtb[crate::ix::U(((7892338i32) - 1) as usize)].int() > 1i32) {
                                        shortfall = (shortfall).wrapping_add(self.total_pw(r, self.cur_p));
                                    }
                                    if (shortfall > 0i32) {
                                        // §900
                                        if (((self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] != 0i32) || (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] != 0i32)) || (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] != 0i32)) {
                                            {
                                                if self.do_last_line_fit {
                                                    {
                                                        if (self.cur_p == (268435455i32).wrapping_neg()) {
                                                            // §1657
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
                                                                    if (self.eqtb[crate::ix::U(((7892331i32) - 1) as usize)].int() < 1000i32) {
                                                                        g = self.fract(g, self.eqtb[crate::ix::U(((7892331i32) - 1) as usize)].int(), 1000i32, max_dimen);
                                                                    }
                                                                    if self.arith_error {
                                                                        if (self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() > 0i32) {
                                                                            g = max_dimen;
                                                                        } else {
                                                                            g = (1073741823i32).wrapping_neg();
                                                                        }
                                                                    }
                                                                    if (g > 0i32) {
                                                                        // §1658
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
                                                                        // §1657
                                                                        if (g < 0i32) {
                                                                            // §1659
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
                                                                // §1657
                                                            }
                                                        }
                                                        // §900
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
                                        // §901
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
                                    // §899
                                    if self.do_last_line_fit {
                                        // §1660
                                        {
                                            if (self.cur_p == (268435455i32).wrapping_neg()) {
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
                                // §899
                                if ((b > inf_bad) || (pi == (10000i32).wrapping_neg())) {
                                    // §902
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
                                    // §899
                                    {
                                        prev_r = r;
                                        if (b > self.threshold) {
                                            continue 'l_continue_b;
                                        }
                                        node_r_stays_active = true;
                                    }
                                }
                                // §903
                                if artificial_demerits {
                                    d = 0i32;
                                } else {
                                    // §907
                                    {
                                        d = (self.eqtb[crate::ix::U(((7892266i32) - 1) as usize)].int()).wrapping_add(b);
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
                                            if (self.cur_p != (268435455i32).wrapping_neg()) {
                                                d = (d).wrapping_add(self.eqtb[crate::ix::U(((7892278i32) - 1) as usize)].int());
                                            } else {
                                                d = (d).wrapping_add(self.eqtb[crate::ix::U(((7892279i32) - 1) as usize)].int());
                                            }
                                        }
                                        if (((fit_class).wrapping_sub(self.mem[crate::ix::U((r) as usize)].hh().b1())).wrapping_abs() > 1i32) {
                                            d = (d).wrapping_add(self.eqtb[crate::ix::U(((7892280i32) - 1) as usize)].int());
                                        }
                                    }
                                }
                                // §903
                                if (self.eqtb[crate::ix::U(((7892296i32) - 1) as usize)].int() > 0i32) {
                                    // §904
                                    {
                                        if (self.printed_node != self.cur_p) {
                                            // §905
                                            {
                                                self.print_nl(65626i32);
                                                if (self.cur_p == (268435455i32).wrapping_neg()) {
                                                    self.short_display(self.mem[crate::ix::U((self.printed_node) as usize)].hh().rh());
                                                } else {
                                                    {
                                                        save_link = self.mem[crate::ix::U((self.cur_p) as usize)].hh().rh();
                                                        { let __ix989 = self.cur_p; self.mem[crate::ix::U((__ix989) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                                                        self.print_nl(65626i32);
                                                        self.short_display(self.mem[crate::ix::U((self.printed_node) as usize)].hh().rh());
                                                        { let __ix990 = self.cur_p; self.mem[crate::ix::U((__ix990) as usize)].set_hh_rh(save_link); }
                                                    }
                                                }
                                                self.printed_node = self.cur_p;
                                            }
                                        }
                                        // §904
                                        self.print_nl(64i32);
                                        if (self.cur_p == (268435455i32).wrapping_neg()) {
                                            self.print_esc(65919i32);
                                        } else {
                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() != glue_node) {
                                                {
                                                    if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == penalty_node) {
                                                        self.print_esc(65845i32);
                                                    } else {
                                                        if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == disc_node) {
                                                            self.print_esc(65639i32);
                                                        } else {
                                                            if (self.mem[crate::ix::U((self.cur_p) as usize)].hh().b0() == kern_node) {
                                                                self.print_esc(65603i32);
                                                            } else {
                                                                self.print_esc(65633i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.print(66356i32);
                                        if (self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                            self.print_char(48i32);
                                        } else {
                                            self.print_int(self.mem[crate::ix::U((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh());
                                        }
                                        self.print(66357i32);
                                        if (b > inf_bad) {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(b);
                                        }
                                        self.print(66358i32);
                                        self.print_int(pi);
                                        self.print(66359i32);
                                        if artificial_demerits {
                                            self.print_char(42i32);
                                        } else {
                                            self.print_int(d);
                                        }
                                    }
                                }
                                // §903
                                d = (d).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int());
                                if (d <= self.minimal_demerits[crate::ix::U((fit_class) as usize)]) {
                                    {
                                        self.minimal_demerits[crate::ix::U((fit_class) as usize)] = d;
                                        { let __v991 = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().rh(); self.best_place[crate::ix::U((fit_class) as usize)] = __v991; }
                                        self.best_pl_line[crate::ix::U((fit_class) as usize)] = l;
                                        if self.do_last_line_fit {
                                            // §1661
                                            {
                                                self.best_pl_short[crate::ix::U((fit_class) as usize)] = shortfall;
                                                self.best_pl_glue[crate::ix::U((fit_class) as usize)] = g;
                                            }
                                        }
                                        // §903
                                        if (d < self.minimum_demerits) {
                                            self.minimum_demerits = d;
                                        }
                                    }
                                }
                                // §899
                                if node_r_stays_active {
                                    continue 'l_continue_b;
                                }
                            }
                            { let __v992 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(__v992); }
                            // §908
                            self.free_node(r, self.active_node_size);
                            if (prev_r == active) {
                                // §909
                                {
                                    r = self.mem[crate::ix::U((active) as usize)].hh().rh();
                                    if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                                        {
                                            { let __v993 = (self.active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.active_width[crate::ix::U(((1i32) - 1) as usize)] = __v993; }
                                            { let __v994 = (self.active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.active_width[crate::ix::U(((2i32) - 1) as usize)] = __v994; }
                                            { let __v995 = (self.active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.active_width[crate::ix::U(((3i32) - 1) as usize)] = __v995; }
                                            { let __v996 = (self.active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.active_width[crate::ix::U(((4i32) - 1) as usize)] = __v996; }
                                            { let __v997 = (self.active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.active_width[crate::ix::U(((5i32) - 1) as usize)] = __v997; }
                                            { let __v998 = (self.active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.active_width[crate::ix::U(((6i32) - 1) as usize)] = __v998; }
                                            { let __v999 = self.active_width[crate::ix::U(((1i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v999; }
                                            { let __v1000 = self.active_width[crate::ix::U(((2i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1000; }
                                            { let __v1001 = self.active_width[crate::ix::U(((3i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1001; }
                                            { let __v1002 = self.active_width[crate::ix::U(((4i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1002; }
                                            { let __v1003 = self.active_width[crate::ix::U(((5i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1003; }
                                            { let __v1004 = self.active_width[crate::ix::U(((6i32) - 1) as usize)]; self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1004; }
                                            { let __v1005 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((active) as usize)].set_hh_rh(__v1005); }
                                            self.free_node(r, delta_node_size);
                                        }
                                    }
                                }
                            } else {
                                // §908
                                if (self.mem[crate::ix::U((prev_r) as usize)].hh().b0() == delta_node) {
                                    {
                                        r = self.mem[crate::ix::U((prev_r) as usize)].hh().rh();
                                        if (r == last_active) {
                                            {
                                                { let __v1006 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1006; }
                                                { let __v1007 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1007; }
                                                { let __v1008 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1008; }
                                                { let __v1009 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1009; }
                                                { let __v1010 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1010; }
                                                { let __v1011 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_sub(self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1011; }
                                                self.mem[crate::ix::U((prev_prev_r) as usize)].set_hh_rh(last_active);
                                                self.free_node(prev_r, delta_node_size);
                                                prev_r = prev_prev_r;
                                            }
                                        } else {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == delta_node) {
                                                {
                                                    { let __v1012 = (self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((1i32) - 1) as usize)] = __v1012; }
                                                    { let __v1013 = (self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((2i32) - 1) as usize)] = __v1013; }
                                                    { let __v1014 = (self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((3i32) - 1) as usize)] = __v1014; }
                                                    { let __v1015 = (self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((4i32) - 1) as usize)] = __v1015; }
                                                    { let __v1016 = (self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((5i32) - 1) as usize)] = __v1016; }
                                                    { let __v1017 = (self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)]).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.cur_active_width[crate::ix::U(((6i32) - 1) as usize)] = __v1017; }
                                                    { let __v1018 = (self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(1i32)) as usize)].set_int(__v1018); }
                                                    { let __v1019 = (self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(2i32)) as usize)].set_int(__v1019); }
                                                    { let __v1020 = (self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(3i32)) as usize)].set_int(__v1020); }
                                                    { let __v1021 = (self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(4i32)) as usize)].set_int(__v1021); }
                                                    { let __v1022 = (self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(5i32)) as usize)].set_int(__v1022); }
                                                    { let __v1023 = (self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].int()); self.mem[crate::ix::U(((prev_r).wrapping_add(6i32)) as usize)].set_int(__v1023); }
                                                    { let __v1024 = self.mem[crate::ix::U((r) as usize)].hh().rh(); self.mem[crate::ix::U((prev_r) as usize)].set_hh_rh(__v1024); }
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
        // §877
        if (self.cur_p == self.printed_node) {
            // §906
            if (self.cur_p != (268435455i32).wrapping_neg()) {
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
    // §925
    pub fn post_line_break(&mut self, mut d: bool) {
        let mut q: halfword = 0; // §925
        let mut r: halfword = 0; // §925
        let mut s: halfword = 0; // §925
        let mut p: halfword = 0; // §925
        let mut k: halfword = 0; // §925
        let mut w: scaled = 0; // §925
        let mut glue_break: bool = false; // §925
        let mut ptmp: halfword = 0; // §925
        let mut disc_break: bool = false; // §925
        let mut post_disc_break: bool = false; // §925
        let mut cur_width: scaled = 0; // §925
        let mut cur_indent: scaled = 0; // §925
        let mut t: quarterword = 0; // §925
        let mut pen: i32 = 0; // §925
        let mut cur_line: halfword = 0; // §925
        let mut LR_ptr: halfword = 0; // §925
        LR_ptr = self.cur_list.eTeX_aux_field;
        // §926
        q = self.mem[crate::ix::U(((self.best_bet).wrapping_add(1i32)) as usize)].hh().rh();
        self.cur_p = (268435455i32).wrapping_neg();
        loop {
            r = q;
            q = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
            { let __v1025 = self.cur_p; self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v1025); }
            self.cur_p = r;
            if (q == (268435455i32).wrapping_neg()) { break; }
        }
        // §925
        cur_line = (self.cur_list.pg_field).wrapping_add(1i32);
        loop {
            'l_done_f: {
                // §928
                if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                    // §1517
                    {
                        q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                        if (LR_ptr != (268435455i32).wrapping_neg()) {
                            {
                                self.temp_ptr = LR_ptr;
                                r = q;
                                loop {
                                    s = self.new_math(0i32, (self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().lh()).wrapping_sub(1i32));
                                    self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                    r = s;
                                    self.temp_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                    if (self.temp_ptr == (268435455i32).wrapping_neg()) { break; }
                                }
                                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(r);
                            }
                        }
                        while (q != self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh()) {
                            {
                                if (!(q >= self.hi_mem_min)) {
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        // §1518
                                        if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                            {
                                                if (LR_ptr != (268435455i32).wrapping_neg()) {
                                                    if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                        {
                                                            self.temp_ptr = LR_ptr;
                                                            LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                            {
                                                                { let __ix1026 = self.temp_ptr; let __v1027 = self.avail; self.mem[crate::ix::U((__ix1026) as usize)].set_hh_rh(__v1027); }
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
                                                { let __ix1028 = self.temp_ptr; let __v1029 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1028) as usize)].set_hh_lh(__v1029); }
                                                { let __ix1030 = self.temp_ptr; self.mem[crate::ix::U((__ix1030) as usize)].set_hh_rh(LR_ptr); }
                                                LR_ptr = self.temp_ptr;
                                            }
                                        }
                                    }
                                }
                                // §1517
                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            }
                        }
                    }
                }
                // §929
                q = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().rh();
                disc_break = false;
                post_disc_break = false;
                glue_break = false;
                if (q != (268435455i32).wrapping_neg()) {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == glue_node) {
                        {
                            self.delete_glue_ref(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh());
                            { let __v1031 = self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1031); }
                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(9i32);
                            { let __ix1032 = self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh(); let __v1033 = (self.mem[crate::ix::U((self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)].hh().rh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix1032) as usize)].set_hh_rh(__v1033); }
                            glue_break = true;
                            break 'l_done_f;
                        }
                    } else {
                        {
                            if (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node) {
                                // §930
                                {
                                    t = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                    // §931
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
                                            self.mem[crate::ix::U((s) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                            self.flush_node_list(self.mem[crate::ix::U((q) as usize)].hh().rh());
                                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(0i32);
                                        }
                                    }
                                    // §930
                                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                        // §932
                                        {
                                            s = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                                            while (self.mem[crate::ix::U((s) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                            }
                                            self.mem[crate::ix::U((s) as usize)].set_hh_rh(r);
                                            r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                            post_disc_break = true;
                                        }
                                    }
                                    // §930
                                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() != (268435455i32).wrapping_neg()) {
                                        // §933
                                        {
                                            s = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(s);
                                            while (self.mem[crate::ix::U((s) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                s = self.mem[crate::ix::U((s) as usize)].hh().rh();
                                            }
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
                                            q = s;
                                        }
                                    }
                                    // §930
                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                                    disc_break = true;
                                }
                            } else {
                                // §929
                                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == kern_node) {
                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                                } else {
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        {
                                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(0i32);
                                            if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                                                // §1518
                                                if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                                    {
                                                        if (LR_ptr != (268435455i32).wrapping_neg()) {
                                                            if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = LR_ptr;
                                                                    LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix1034 = self.temp_ptr; let __v1035 = self.avail; self.mem[crate::ix::U((__ix1034) as usize)].set_hh_rh(__v1035); }
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
                                                        { let __ix1036 = self.temp_ptr; let __v1037 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1036) as usize)].set_hh_lh(__v1037); }
                                                        { let __ix1038 = self.temp_ptr; self.mem[crate::ix::U((__ix1038) as usize)].set_hh_rh(LR_ptr); }
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
                    // §929
                    {
                        q = temp_head;
                        while (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                    }
                }
            }
            if (self.eqtb[crate::ix::U(((7892338i32) - 1) as usize)].int() > 0i32) {
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
                            { let __v1039 = self.mem[crate::ix::U((ptmp) as usize)].hh().rh(); self.mem[crate::ix::U((k) as usize)].set_hh_rh(__v1039); }
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
                    // §934
                    r = self.new_param_glue(right_skip_code);
                    { let __v1040 = self.mem[crate::ix::U((q) as usize)].hh().rh(); self.mem[crate::ix::U((r) as usize)].set_hh_rh(__v1040); }
                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                    q = r;
                }
            }
            // §928
            if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                // §1519
                if (LR_ptr != (268435455i32).wrapping_neg()) {
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
                        while (r != (268435455i32).wrapping_neg()) {
                            {
                                self.temp_ptr = self.new_math(0i32, self.mem[crate::ix::U((r) as usize)].hh().lh());
                                { let __v1041 = self.temp_ptr; self.mem[crate::ix::U((s) as usize)].set_hh_rh(__v1041); }
                                s = self.temp_ptr;
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                        self.mem[crate::ix::U((s) as usize)].set_hh_rh(q);
                    }
                }
            }
            // §935
            r = self.mem[crate::ix::U((q) as usize)].hh().rh();
            self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
            q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
            self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(r);
            if (self.eqtb[crate::ix::U(((7892338i32) - 1) as usize)].int() > 0i32) {
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
            if (self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)].hh().rh() != zero_glue) {
                {
                    r = self.new_param_glue(left_skip_code);
                    self.mem[crate::ix::U((r) as usize)].set_hh_rh(q);
                    q = r;
                }
            }
            // §937
            if (cur_line > self.last_special_line) {
                {
                    cur_width = self.second_width;
                    cur_indent = self.second_indent;
                }
            } else {
                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
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
            self.just_box = self.hpack(q, cur_width, exactly);
            { let __ix1042 = (self.just_box).wrapping_add(4i32); self.mem[crate::ix::U((__ix1042) as usize)].set_int(cur_indent); }
            // §936
            if (pre_adjust_head != self.pre_adjust_tail) {
                {
                    { let __ix1043 = self.cur_list.tail_field; let __v1044 = self.mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1043) as usize)].set_hh_rh(__v1044); }
                    self.cur_list.tail_field = self.pre_adjust_tail;
                }
            }
            self.pre_adjust_tail = (268435455i32).wrapping_neg();
            self.append_to_vlist(self.just_box);
            if (adjust_head != self.adjust_tail) {
                {
                    { let __ix1045 = self.cur_list.tail_field; let __v1046 = self.mem[crate::ix::U((adjust_head) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1045) as usize)].set_hh_rh(__v1046); }
                    self.cur_list.tail_field = self.adjust_tail;
                }
            }
            self.adjust_tail = (268435455i32).wrapping_neg();
            // §938
            if ((cur_line).wrapping_add(1i32) != self.best_line) {
                {
                    q = self.eqtb[crate::ix::U(((inter_line_penalties_loc) - 1) as usize)].hh().rh();
                    if (q != (268435455i32).wrapping_neg()) {
                        {
                            r = cur_line;
                            if (r > self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) {
                                r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            }
                            pen = self.mem[crate::ix::U((((q).wrapping_add(r)).wrapping_add(1i32)) as usize)].int();
                        }
                    } else {
                        pen = self.eqtb[crate::ix::U(((7892277i32) - 1) as usize)].int();
                    }
                    q = self.eqtb[crate::ix::U(((club_penalties_loc) - 1) as usize)].hh().rh();
                    if (q != (268435455i32).wrapping_neg()) {
                        {
                            r = (cur_line).wrapping_sub(self.cur_list.pg_field);
                            if (r > self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()) {
                                r = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            }
                            pen = (pen).wrapping_add(self.mem[crate::ix::U((((q).wrapping_add(r)).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        if (cur_line == (self.cur_list.pg_field).wrapping_add(1i32)) {
                            pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((7892269i32) - 1) as usize)].int());
                        }
                    }
                    if d {
                        q = self.eqtb[crate::ix::U(((display_widow_penalties_loc) - 1) as usize)].hh().rh();
                    } else {
                        q = self.eqtb[crate::ix::U(((widow_penalties_loc) - 1) as usize)].hh().rh();
                    }
                    if (q != (268435455i32).wrapping_neg()) {
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
                                pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((7892271i32) - 1) as usize)].int());
                            } else {
                                pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((7892270i32) - 1) as usize)].int());
                            }
                        }
                    }
                    if disc_break {
                        pen = (pen).wrapping_add(self.eqtb[crate::ix::U(((7892272i32) - 1) as usize)].int());
                    }
                    if (pen != 0i32) {
                        {
                            r = self.new_penalty(pen);
                            { let __ix1047 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix1047) as usize)].set_hh_rh(r); }
                            self.cur_list.tail_field = r;
                        }
                    }
                }
            }
            // §925
            cur_line = (cur_line).wrapping_add(1i32);
            self.cur_p = self.mem[crate::ix::U(((self.cur_p).wrapping_add(1i32)) as usize)].hh().lh();
            if (self.cur_p != (268435455i32).wrapping_neg()) {
                if (!post_disc_break) {
                    // §927
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
                                        if ((self.mem[crate::ix::U((q) as usize)].hh().b1() != explicit) && (self.mem[crate::ix::U((q) as usize)].hh().b1() != space_adjustment)) {
                                            break 'l_done1_f;
                                        }
                                    }
                                    r = q;
                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == math_node) {
                                        if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                                            // §1518
                                            if (((self.mem[crate::ix::U((q) as usize)].hh().b1()) % 2) != 0) {
                                                {
                                                    if (LR_ptr != (268435455i32).wrapping_neg()) {
                                                        if (self.mem[crate::ix::U((LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                            {
                                                                self.temp_ptr = LR_ptr;
                                                                LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                {
                                                                    { let __ix1048 = self.temp_ptr; let __v1049 = self.avail; self.mem[crate::ix::U((__ix1048) as usize)].set_hh_rh(__v1049); }
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
                                                    { let __ix1050 = self.temp_ptr; let __v1051 = ((L_code).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix1050) as usize)].set_hh_lh(__v1051); }
                                                    { let __ix1052 = self.temp_ptr; self.mem[crate::ix::U((__ix1052) as usize)].set_hh_rh(LR_ptr); }
                                                    LR_ptr = self.temp_ptr;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §927
                        if (r != temp_head) {
                            {
                                self.mem[crate::ix::U((r) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                self.flush_node_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
                                self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(q);
                            }
                        }
                    }
                }
            }
            if (self.cur_p == (268435455i32).wrapping_neg()) { break; }
        }
        // §925
        if ((cur_line != self.best_line) || (self.mem[crate::ix::U((temp_head) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
            self.confusion(66368i32);
        }
        self.cur_list.pg_field = (self.best_line).wrapping_sub(1i32);
        self.cur_list.eTeX_aux_field = LR_ptr;
    }

    /// @<Declare the function called `reconstitute`
    // §960
    pub fn reconstitute(&mut self, mut j: small_number, mut n: small_number, mut bchar: halfword, mut hchar: halfword) -> small_number {
        let mut reconstitute: small_number = 0;
        let mut p: halfword = 0; // §960
        let mut t: halfword = 0; // §960
        let mut q: four_quarters = four_quarters::default(); // §960
        let mut cur_rh: halfword = 0; // §960
        let mut test_char: halfword = 0; // §960
        let mut w: scaled = 0; // §960
        let mut k: font_index = 0; // §960
        // goto labels: continue, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.hyphen_passed = 0i32;
                t = hold_head;
                w = 0i32;
                self.mem[crate::ix::U((hold_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                // §962
                self.cur_l = self.hu[crate::ix::U((j) as usize)];
                self.cur_q = t;
                if (j == 0i32) {
                    {
                        self.ligature_present = self.init_lig;
                        p = self.init_list;
                        if self.ligature_present {
                            self.lft_hit = self.init_lft;
                        }
                        while (p > (268435455i32).wrapping_neg()) {
                            {
                                {
                                    { let __v1053 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1053); }
                                    t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                    { let __v1054 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1054); }
                                    { let __v1055 = self.mem[crate::ix::U((p) as usize)].hh().b1(); self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1055); }
                                }
                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            }
                        }
                    }
                } else {
                    if (self.cur_l < non_char) {
                        {
                            { let __v1056 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1056); }
                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                            { let __v1057 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1057); }
                            { let __v1058 = self.cur_l; self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1058); }
                        }
                    }
                }
                self.lig_stack = (268435455i32).wrapping_neg();
                {
                    if (j < n) {
                        self.cur_r = self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)];
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
                // §960
                if (self.cur_l == non_char) {
                    // §963
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
                        q = { let __s1059 = ((self.char_base[crate::ix::U((self.hf) as usize)]).wrapping_add(self.effective_char(true, self.hf, self.cur_l))) as usize; self.font_info[crate::ix::U(__s1059)] }.qqqq();
                        if ((q.b2() % 4i32) != lig_tag) {
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
                                            // §965
                                            {
                                                if (self.cur_l == non_char) {
                                                    self.lft_hit = true;
                                                }
                                                if (j == n) {
                                                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
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
                                                            if (self.lig_stack > (268435455i32).wrapping_neg()) {
                                                                { let __ix1060 = self.lig_stack; let __v1061 = self.cur_r; self.mem[crate::ix::U((__ix1060) as usize)].set_hh_b1(__v1061); }
                                                            } else {
                                                                {
                                                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                                                    if (j == n) {
                                                                        bchar = non_char;
                                                                    } else {
                                                                        {
                                                                            p = self.get_avail();
                                                                            { let __ix1062 = (self.lig_stack).wrapping_add(1i32); self.mem[crate::ix::U((__ix1062) as usize)].set_hh_rh(p); }
                                                                            { let __v1063 = self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1063); }
                                                                            { let __v1064 = self.hf; self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1064); }
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
                                                            { let __ix1065 = self.lig_stack; self.mem[crate::ix::U((__ix1065) as usize)].set_hh_rh(p); }
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
                                                                        if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                                                            {
                                                                                { let __v1066 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1066); }
                                                                                self.rt_hit = false;
                                                                            }
                                                                        }
                                                                    }
                                                                    { let __ix1067 = self.cur_q; self.mem[crate::ix::U((__ix1067) as usize)].set_hh_rh(p); }
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
                                                            if (self.lig_stack > (268435455i32).wrapping_neg()) {
                                                                {
                                                                    if (self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                                                        {
                                                                            { let __v1068 = self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1068); }
                                                                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                                            j = (j).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                    p = self.lig_stack;
                                                                    self.lig_stack = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                    self.free_node(p, small_node_size);
                                                                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)];
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
                                                                            { let __v1069 = self.get_avail(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1069); }
                                                                            t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                                            { let __v1070 = self.hf; self.mem[crate::ix::U((t) as usize)].set_hh_b0(__v1070); }
                                                                            { let __v1071 = self.cur_r; self.mem[crate::ix::U((t) as usize)].set_hh_b1(__v1071); }
                                                                        }
                                                                        j = (j).wrapping_add(1i32);
                                                                        {
                                                                            if (j < n) {
                                                                                self.cur_r = self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)];
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
                                        // §963
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
                // §964
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
                            if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                {
                                    { let __v1072 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v1072); }
                                    self.rt_hit = false;
                                }
                            }
                        }
                        { let __ix1073 = self.cur_q; self.mem[crate::ix::U((__ix1073) as usize)].set_hh_rh(p); }
                        t = p;
                        self.ligature_present = false;
                    }
                }
                if (w != 0i32) {
                    {
                        { let __v1074 = self.new_kern(w); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1074); }
                        t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                        w = 0i32;
                        self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_hh_lh(0i32);
                    }
                }
                if (self.lig_stack > (268435455i32).wrapping_neg()) {
                    {
                        self.cur_q = t;
                        self.cur_l = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                        self.ligature_present = true;
                        {
                            if (self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh() > (268435455i32).wrapping_neg()) {
                                {
                                    { let __v1075 = self.mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh(); self.mem[crate::ix::U((t) as usize)].set_hh_rh(__v1075); }
                                    t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                    j = (j).wrapping_add(1i32);
                                }
                            }
                            p = self.lig_stack;
                            self.lig_stack = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            self.free_node(p, small_node_size);
                            if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                {
                                    if (j < n) {
                                        self.cur_r = self.hu[crate::ix::U(((j).wrapping_add(1i32)) as usize)];
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
                // §960
                reconstitute = j;
            }
            break 'l_dispatch_1;
        }
        reconstitute
    }

}
