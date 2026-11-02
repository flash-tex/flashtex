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
    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1308
    pub fn omit_error(&mut self) {
        {
            self.dg_mark();
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1542i32);
        }
        self.print_esc(618i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 1552i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 1551i32;
        }
        self.error();
    }

    /// An `align_group` code is supposed to remain on the `save_stack`
    /// during an entire alignment, until `fin_align` removes it.
    /// A devious user might force an `endv` command to occur just about anywhere;
    /// we must defeat such hacks.
    /// @<Declare act...
    // §1310
    pub fn do_endv(&mut self) {
        self.base_ptr = self.input_ptr;
        { let __ix1737 = self.base_ptr; let __v1738 = self.cur_input; self.input_stack[crate::ix::U((__ix1737) as usize)] = __v1738; }
        while (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field == null)) && (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field == token_list)) {
            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
        }
        if (((self.input_stack[crate::ix::U((self.base_ptr) as usize)].index_field != v_template) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].loc_field != null)) || (self.input_stack[crate::ix::U((self.base_ptr) as usize)].state_field != token_list)) {
            self.fatal_error(689i32);
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

    /// @<Declare act...
    // §1314
    pub fn cs_error(&mut self) {
        if (self.cur_chr == 10i32) {
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
                    self.print(944i32);
                }
                self.print_esc(601i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1554i32;
                }
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
                    self.print(944i32);
                }
                self.print_esc(590i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1555i32;
                }
            }
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
    // §1315
    pub fn push_math(&mut self, mut c: group_code) {
        self.push_nest();
        self.cur_list.mode_field = (209i32).wrapping_neg();
        self.cur_list.aux_field.set_int(null);
        self.new_save_level(c);
    }

    /// When calculating the natural width, `w`, of the final line preceding
    /// the display, we may have to copy all or part of its hlist.  We copy,
    /// however, only those parts of the original list that are relevant for the
    /// computation of `pre_display_size`.
    /// @<Declare subprocedures for `init_math`
    // §1734
    pub fn just_copy(&mut self, mut p: halfword, mut h: halfword, mut t: halfword) {
        let mut r: halfword = 0; // §1734
        let mut words: i32 = 0; // §1734
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        while (p != null) {
            {
                'l_not_found_f: {
                    'l_found_f: {
                        words = 1i32;
                        if (p >= self.hi_mem_min) {
                            r = self.get_avail();
                        } else {
                            match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                hlist_node | vlist_node => {
                                    {
                                        r = self.get_node(box_node_size);
                                        // §1919
                                        { let __v1739 = __av_mem[crate::ix::U(((p).wrapping_add(7i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(7i32)) as usize)].set_int(__v1739); }
                                        { let __v1740 = __av_mem[crate::ix::U(((p).wrapping_add(8i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(8i32)) as usize)].set_int(__v1740); }
                                        // §1734
                                        { let __v1741 = __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)]; __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)] = __v1741; }
                                        { let __v1742 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)]; __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)] = __v1742; }
                                        words = 5i32;
                                        __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(null);
                                    }
                                }
                                rule_node => {
                                    {
                                        r = self.get_node(rule_node_size);
                                        words = rule_node_size;
                                    }
                                }
                                ligature_node => {
                                    {
                                        r = self.get_avail();
                                        { let __v1743 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((r) as usize)] = __v1743; }
                                        break 'l_found_f;
                                    }
                                }
                                kern_node | math_node => {
                                    {
                                        words = medium_node_size;
                                        r = self.get_node(words);
                                    }
                                }
                                glue_node => {
                                    {
                                        r = self.get_node(medium_node_size);
                                        { let __ix1744 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v1745 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1744) as usize)].set_hh_rh(__v1745); }
                                        // §1921
                                        { let __v1746 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v1746); }
                                        { let __v1747 = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1747); }
                                        // §1734
                                        { let __v1748 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v1748); }
                                        __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(null);
                                    }
                                }
                                whatsit_node => {
                                    // §1605
                                    match __av_mem[crate::ix::U((p) as usize)].hh().b1() {
                                        open_node => {
                                            {
                                                r = self.get_node(open_node_size);
                                                words = open_node_size;
                                            }
                                        }
                                        write_node | special_node | latespecial_node => {
                                            {
                                                r = self.get_node(write_node_size);
                                                { let __ix1749 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1750 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1749) as usize)].set_hh_lh(__v1750); }
                                                words = write_node_size;
                                            }
                                        }
                                        close_node | language_node => {
                                            {
                                                r = self.get_node(small_node_size);
                                                words = small_node_size;
                                            }
                                        }
                                        pdf_literal_node | pdf_lateliteral_node => {
                                            {
                                                r = self.get_node(write_node_size);
                                                { let __ix1751 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1752 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1751) as usize)].set_hh_lh(__v1752); }
                                                words = write_node_size;
                                            }
                                        }
                                        pdf_colorstack_node => {
                                            {
                                                if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() <= colorstack_data) {
                                                    {
                                                        r = self.get_node(pdf_colorstack_setter_node_size);
                                                        { let __ix1753 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh(); let __v1754 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1753) as usize)].set_hh_lh(__v1754); }
                                                        words = pdf_colorstack_setter_node_size;
                                                    }
                                                } else {
                                                    {
                                                        r = self.get_node(pdf_colorstack_getter_node_size);
                                                        words = pdf_colorstack_getter_node_size;
                                                    }
                                                }
                                            }
                                        }
                                        pdf_setmatrix_node => {
                                            {
                                                r = self.get_node(pdf_setmatrix_node_size);
                                                { let __ix1755 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v1756 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1755) as usize)].set_hh_lh(__v1756); }
                                                words = pdf_setmatrix_node_size;
                                            }
                                        }
                                        pdf_save_node => {
                                            {
                                                r = self.get_node(pdf_save_node_size);
                                                words = pdf_save_node_size;
                                            }
                                        }
                                        pdf_restore_node => {
                                            {
                                                r = self.get_node(pdf_restore_node_size);
                                                words = pdf_restore_node_size;
                                            }
                                        }
                                        pdf_refobj_node => {
                                            {
                                                r = self.get_node(pdf_refobj_node_size);
                                                words = pdf_refobj_node_size;
                                            }
                                        }
                                        pdf_refxform_node => {
                                            {
                                                r = self.get_node(pdf_refxform_node_size);
                                                words = pdf_refxform_node_size;
                                            }
                                        }
                                        pdf_refximage_node => {
                                            {
                                                r = self.get_node(pdf_refximage_node_size);
                                                words = pdf_refximage_node_size;
                                            }
                                        }
                                        pdf_annot_node => {
                                            {
                                                r = self.get_node(pdf_annot_node_size);
                                                { let __ix1757 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh(); let __v1758 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1757) as usize)].set_hh_lh(__v1758); }
                                                words = pdf_annot_node_size;
                                            }
                                        }
                                        pdf_start_link_node => {
                                            {
                                                r = self.get_node(pdf_annot_node_size);
                                                { let __v1759 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v1759); }
                                                { let __v1760 = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1760); }
                                                { let __v1761 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v1761); }
                                                { let __v1762 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh(); __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_lh(__v1762); }
                                                if (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh() != null) {
                                                    { let __ix1763 = __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh(); let __v1764 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1763) as usize)].set_hh_lh(__v1764); }
                                                }
                                                { let __v1765 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(__v1765); }
                                                { let __ix1766 = (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32); let __v1767 = (__av_mem[crate::ix::U(((__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1766) as usize)].set_hh_rh(__v1767); }
                                                { let __v1768 = __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int(); __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_int(__v1768); }
                                            }
                                        }
                                        pdf_end_link_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_dest_node => {
                                            {
                                                r = self.get_node(pdf_dest_node_size);
                                                if (__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                    { let __ix1769 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v1770 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1769) as usize)].set_hh_lh(__v1770); }
                                                }
                                                words = pdf_dest_node_size;
                                            }
                                        }
                                        pdf_thread_node | pdf_start_thread_node => {
                                            {
                                                r = self.get_node(pdf_thread_node_size);
                                                if (__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                    { let __ix1771 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v1772 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1771) as usize)].set_hh_lh(__v1772); }
                                                }
                                                if (__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
                                                    { let __ix1773 = __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh(); let __v1774 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1773) as usize)].set_hh_lh(__v1774); }
                                                }
                                                words = pdf_thread_node_size;
                                            }
                                        }
                                        pdf_end_thread_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_save_pos_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_snap_ref_point_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_snapy_node => {
                                            {
                                                { let __ix1775 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v1776 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1775) as usize)].set_hh_rh(__v1776); }
                                                r = self.get_node(snap_node_size);
                                                words = snap_node_size;
                                            }
                                        }
                                        pdf_snapy_comp_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_interword_space_on_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_interword_space_off_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_fake_space_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_running_link_off_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        pdf_running_link_on_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        _ => {
                                            self.confusion(1931i32);
                                        }
                                    }
                                }
                                _ => {
                                    // §1734
                                    break 'l_not_found_f;
                                }
                            }
                        }
                        while (words > 0i32) {
                            {
                                words = (words).wrapping_sub(1i32);
                                { let __v1777 = __av_mem[crate::ix::U(((p).wrapping_add(words)) as usize)]; __av_mem[crate::ix::U(((r).wrapping_add(words)) as usize)] = __v1777; }
                            }
                        }
                    }
                    __av_mem[crate::ix::U((h) as usize)].set_hh_rh(r);
                    h = r;
                }
                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        __av_mem[crate::ix::U((h) as usize)].set_hh_rh(t);
    }

    /// @<Declare subprocedures for `init_math`
    // §1739
    pub fn just_reverse(&mut self, mut p: halfword) {
        let mut l: halfword = 0; // §1739
        let mut t: halfword = 0; // §1739
        let mut q: halfword = 0; // §1739
        let mut m: halfword = 0; // §1739
        let mut n: halfword = 0; // §1739
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_done_f: {
            m = min_halfword;
            n = min_halfword;
            if (__av_mem[crate::ix::U((temp_head) as usize)].hh().rh() == null) {
                {
                    self.just_copy(__av_mem[crate::ix::U((p) as usize)].hh().rh(), temp_head, null);
                    q = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh();
                }
            } else {
                {
                    q = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                    __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                    self.flush_node_list(__av_mem[crate::ix::U((temp_head) as usize)].hh().rh());
                }
            }
            t = self.new_edge(self.cur_dir, 0i32);
            l = t;
            self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
            while (q != null) {
                if (q >= self.hi_mem_min) {
                    loop {
                        p = q;
                        q = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                        l = p;
                        if (!(q >= self.hi_mem_min)) { break; }
                    }
                } else {
                    {
                        p = q;
                        q = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == math_node) {
                            // §1740
                            if (((__av_mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                    {
                                        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                        self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                    }
                                } else {
                                    {
                                        {
                                            self.temp_ptr = self.LR_ptr;
                                            self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                            {
                                                { let __ix1778 = self.temp_ptr; let __v1779 = self.avail; __av_mem[crate::ix::U((__ix1778) as usize)].set_hh_rh(__v1779); }
                                                self.avail = self.temp_ptr;
                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                            }
                                        }
                                        if (n > min_halfword) {
                                            {
                                                n = (n).wrapping_sub(1i32);
                                                { let __v1780 = (__av_mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(1i32); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v1780); }
                                            }
                                        } else {
                                            {
                                                if (m > min_halfword) {
                                                    m = (m).wrapping_sub(1i32);
                                                } else {
                                                    {
                                                        { let __v1781 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v1781); }
                                                        __av_mem[crate::ix::U((t) as usize)].set_hh_rh(q);
                                                        self.free_node(p, medium_node_size);
                                                        break 'l_done_f;
                                                    }
                                                }
                                                __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    {
                                        self.temp_ptr = self.get_avail();
                                        { let __ix1782 = self.temp_ptr; let __v1783 = ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); __av_mem[crate::ix::U((__ix1782) as usize)].set_hh_lh(__v1783); }
                                        { let __ix1784 = self.temp_ptr; let __v1785 = self.LR_ptr; __av_mem[crate::ix::U((__ix1784) as usize)].set_hh_rh(__v1785); }
                                        self.LR_ptr = self.temp_ptr;
                                    }
                                    if ((n > min_halfword) || ((__av_mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir)) {
                                        {
                                            n = (n).wrapping_add(1i32);
                                            { let __v1786 = (__av_mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v1786); }
                                        }
                                    } else {
                                        {
                                            __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                            m = (m).wrapping_add(1i32);
                                        }
                                    }
                                }
                            }
                        }
                        // §1739
                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                        l = p;
                    }
                }
            }
            break 'l_done_f;
            { let __v1787 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v1787); }
            __av_mem[crate::ix::U((t) as usize)].set_hh_rh(q);
            self.free_node(p, small_node_size);
        }
        __av_mem[crate::ix::U((temp_head) as usize)].set_hh_rh(l);
    }

    /// @<Declare act...
    // §1317
    pub fn init_math(&mut self) {
        let mut w: scaled = 0; // §1317
        let mut j: halfword = 0; // §1317
        let mut x: i32 = 0; // §1317
        let mut l: scaled = 0; // §1317
        let mut s: scaled = 0; // §1317
        let mut p: halfword = 0; // §1317
        let mut q: halfword = 0; // §1317
        let mut f: internal_font_number = 0; // §1317
        let mut n: i32 = 0; // §1317
        let mut v: scaled = 0; // §1317
        let mut d: scaled = 0; // §1317
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.get_token();
        if ((self.cur_cmd == math_shift) && (self.cur_list.mode_field > 0i32)) {
            // §1324
            {
                j = null;
                w = (1073741823i32).wrapping_neg();
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    // §1733
                    {
                        self.pop_nest();
                        // §1732
                        if (self.cur_list.eTeX_aux_field == null) {
                            x = 0i32;
                        } else {
                            if (__av_mem[crate::ix::U((self.cur_list.eTeX_aux_field) as usize)].hh().lh() >= R_code) {
                                x = (1i32).wrapping_neg();
                            } else {
                                x = 1i32;
                            }
                        }
                    }
                } else {
                    // §1324
                    {
                        'l_done_f: {
                            self.line_break(true);
                            // §1735
                            if (self.eTeX_mode == 1i32) {
                                // §1741
                                {
                                    if (__av_eqtb[crate::ix::U(((26636i32) - 1) as usize)].hh().rh() == zero_glue) {
                                        j = self.new_kern(0i32);
                                    } else {
                                        j = self.new_param_glue(right_skip_code);
                                    }
                                    if (__av_eqtb[crate::ix::U(((26635i32) - 1) as usize)].hh().rh() == zero_glue) {
                                        p = self.new_kern(0i32);
                                    } else {
                                        p = self.new_param_glue(left_skip_code);
                                    }
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_rh(j);
                                    j = self.new_null_box();
                                    { let __v1788 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)].set_int(__v1788); }
                                    { let __v1789 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(4i32)) as usize)].int(); __av_mem[crate::ix::U(((j).wrapping_add(4i32)) as usize)].set_int(__v1789); }
                                    __av_mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                                    { let __v1790 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b1(); __av_mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)].set_hh_b1(__v1790); }
                                    { let __v1791 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b0(); __av_mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)].set_hh_b0(__v1791); }
                                    { let __v1792 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(6i32)) as usize)].gr(); __av_mem[crate::ix::U(((j).wrapping_add(6i32)) as usize)].set_gr(__v1792); }
                                }
                            }
                            // §1735
                            v = __av_mem[crate::ix::U(((self.just_box).wrapping_add(4i32)) as usize)].int();
                            // §1732
                            if (self.cur_list.eTeX_aux_field == null) {
                                x = 0i32;
                            } else {
                                if (__av_mem[crate::ix::U((self.cur_list.eTeX_aux_field) as usize)].hh().lh() >= R_code) {
                                    x = (1i32).wrapping_neg();
                                } else {
                                    x = 1i32;
                                }
                            }
                            // §1735
                            if (x >= 0i32) {
                                {
                                    p = __av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().rh();
                                    __av_mem[crate::ix::U((temp_head) as usize)].set_hh_rh(null);
                                }
                            } else {
                                {
                                    v = ((v).wrapping_neg()).wrapping_sub(__av_mem[crate::ix::U(((self.just_box).wrapping_add(1i32)) as usize)].int());
                                    p = self.new_math(0i32, begin_L_code);
                                    __av_mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
                                    { let __a1793_0 = __av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().rh(); let __a1793_1 = p; let __a1793_2 = self.new_math(0i32, end_L_code); self.just_copy(__a1793_0, __a1793_1, __a1793_2) };
                                    self.cur_dir = right_to_left;
                                }
                            }
                            v = (v).wrapping_add((2i32).wrapping_mul(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()));
                            if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                                // §1711
                                {
                                    self.temp_ptr = self.get_avail();
                                    { let __ix1794 = self.temp_ptr; __av_mem[crate::ix::U((__ix1794) as usize)].set_hh_lh(before); }
                                    { let __ix1795 = self.temp_ptr; let __v1796 = self.LR_ptr; __av_mem[crate::ix::U((__ix1795) as usize)].set_hh_rh(__v1796); }
                                    self.LR_ptr = self.temp_ptr;
                                }
                            }
                            // §1325
                            while (p != null) {
                                {
                                    // goto labels: reswitch, found, not_found
                                    let mut __goto_1: i32 = 0;
                                    'l_dispatch_1: loop {
                                        if __goto_1 <= 0 {
                                            // §1326
                                            if (p >= self.hi_mem_min) {
                                                {
                                                    f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                                    d = { let __s1798 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s1797 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, __av_mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s1797)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s1798)] }.int();
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                            match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                                hlist_node | vlist_node | rule_node => {
                                                    {
                                                        d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                ligature_node => {
                                                    // §827
                                                    {
                                                        { let __v1799 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((lig_trick) as usize)] = __v1799; }
                                                        { let __v1800 = __av_mem[crate::ix::U((p) as usize)].hh().rh(); __av_mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v1800); }
                                                        p = lig_trick;
                                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                margin_kern_node => {
                                                    // §1326
                                                    d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                }
                                                kern_node => {
                                                    d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                }
                                                math_node => {
                                                    // §1737
                                                    {
                                                        d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                        if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                                                            // §1738
                                                            if (((__av_mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                                {
                                                                    if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                        {
                                                                            self.temp_ptr = self.LR_ptr;
                                                                            self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                            {
                                                                                { let __ix1801 = self.temp_ptr; let __v1802 = self.avail; __av_mem[crate::ix::U((__ix1801) as usize)].set_hh_rh(__v1802); }
                                                                                self.avail = self.temp_ptr;
                                                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                            }
                                                                        }
                                                                    } else {
                                                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() > L_code) {
                                                                            {
                                                                                w = max_dimen;
                                                                                break 'l_done_f;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    {
                                                                        self.temp_ptr = self.get_avail();
                                                                        { let __ix1803 = self.temp_ptr; let __v1804 = ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); __av_mem[crate::ix::U((__ix1803) as usize)].set_hh_lh(__v1804); }
                                                                        { let __ix1805 = self.temp_ptr; let __v1806 = self.LR_ptr; __av_mem[crate::ix::U((__ix1805) as usize)].set_hh_rh(__v1806); }
                                                                        self.LR_ptr = self.temp_ptr;
                                                                    }
                                                                    if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir) {
                                                                        {
                                                                            self.just_reverse(p);
                                                                            p = temp_head;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            // §1737
                                                            if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= L_code) {
                                                                {
                                                                    w = max_dimen;
                                                                    break 'l_done_f;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                edge_node => {
                                                    {
                                                        d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                        self.cur_dir = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                                                    }
                                                }
                                                glue_node => {
                                                    // §1327
                                                    {
                                                        q = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                        d = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                                                        if (__av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b0() == stretching) {
                                                            {
                                                                if ((__av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b1() == __av_mem[crate::ix::U((q) as usize)].hh().b0()) && (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() != 0i32)) {
                                                                    v = max_dimen;
                                                                }
                                                            }
                                                        } else {
                                                            if (__av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b0() == shrinking) {
                                                                {
                                                                    if ((__av_mem[crate::ix::U(((self.just_box).wrapping_add(5i32)) as usize)].hh().b1() == __av_mem[crate::ix::U((q) as usize)].hh().b1()) && (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                                        v = max_dimen;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                                        }
                                                    }
                                                }
                                                whatsit_node => {
                                                    // §1609
                                                    if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                                        d = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                    } else {
                                                        d = 0i32;
                                                    }
                                                }
                                                _ => {
                                                    // §1326
                                                    d = 0i32;
                                                }
                                            }
                                            // §1325
                                            if (v < max_dimen) {
                                                v = (v).wrapping_add(d);
                                            }
                                            { __goto_1 = 2; continue 'l_dispatch_1; }
                                        }
                                        if __goto_1 <= 1 { // found
                                            if (v < max_dimen) {
                                                {
                                                    v = (v).wrapping_add(d);
                                                    w = v;
                                                }
                                            } else {
                                                {
                                                    w = max_dimen;
                                                    break 'l_done_f;
                                                }
                                            }
                                        }
                                        if __goto_1 <= 2 { // not_found
                                            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                        }
                                        break 'l_dispatch_1;
                                    }
                                }
                            }
                        }
                        if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                            // §1736
                            {
                                while (self.LR_ptr != null) {
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                        {
                                            { let __ix1807 = self.temp_ptr; let __v1808 = self.avail; __av_mem[crate::ix::U((__ix1807) as usize)].set_hh_rh(__v1808); }
                                            self.avail = self.temp_ptr;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                }
                                if (self.LR_problems != 0i32) {
                                    {
                                        w = max_dimen;
                                        self.LR_problems = 0i32;
                                    }
                                }
                            }
                        }
                        self.cur_dir = left_to_right;
                        self.flush_node_list(__av_mem[crate::ix::U((temp_head) as usize)].hh().rh());
                    }
                }
                // §1328
                if (__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == null) {
                    if ((__av_eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() != 0i32) && (((__av_eqtb[crate::ix::U(((29318i32) - 1) as usize)].int() >= 0i32) && ((self.cur_list.pg_field).wrapping_add(2i32) > __av_eqtb[crate::ix::U(((29318i32) - 1) as usize)].int())) || ((self.cur_list.pg_field).wrapping_add(1i32) < (__av_eqtb[crate::ix::U(((29318i32) - 1) as usize)].int()).wrapping_neg()))) {
                        {
                            l = (__av_eqtb[crate::ix::U(((29906i32) - 1) as usize)].int()).wrapping_sub((__av_eqtb[crate::ix::U(((29920i32) - 1) as usize)].int()).wrapping_abs());
                            if (__av_eqtb[crate::ix::U(((29920i32) - 1) as usize)].int() > 0i32) {
                                s = __av_eqtb[crate::ix::U(((29920i32) - 1) as usize)].int();
                            } else {
                                s = 0i32;
                            }
                        }
                    } else {
                        {
                            l = __av_eqtb[crate::ix::U(((29906i32) - 1) as usize)].int();
                            s = 0i32;
                        }
                    }
                } else {
                    {
                        n = __av_mem[crate::ix::U((__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                        if ((self.cur_list.pg_field).wrapping_add(2i32) >= n) {
                            p = (__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(n));
                        } else {
                            p = (__av_eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul((self.cur_list.pg_field).wrapping_add(2i32)));
                        }
                        s = __av_mem[crate::ix::U(((p).wrapping_sub(1i32)) as usize)].int();
                        l = __av_mem[crate::ix::U((p) as usize)].int();
                    }
                }
                // §1324
                self.push_math(math_shift_group);
                self.cur_list.mode_field = mmode;
                self.eq_word_define(29321i32, (1i32).wrapping_neg());
                self.eq_word_define(29916i32, w);
                self.cur_list.eTeX_aux_field = j;
                if (self.eTeX_mode == 1i32) {
                    self.eq_word_define(29384i32, x);
                }
                self.eq_word_define(29917i32, l);
                self.eq_word_define(29918i32, s);
                if (__av_eqtb[crate::ix::U(((every_display_loc) - 1) as usize)].hh().rh() != null) {
                    self.begin_token_list(__av_eqtb[crate::ix::U(((every_display_loc) - 1) as usize)].hh().rh(), every_display_text);
                }
                if (self.nest_ptr == 1i32) {
                    self.build_page();
                }
            }
        } else {
            // §1317
            {
                self.back_input();
                // §1318
                {
                    self.push_math(math_shift_group);
                    self.eq_word_define(29321i32, (1i32).wrapping_neg());
                    if (__av_eqtb[crate::ix::U(((every_math_loc) - 1) as usize)].hh().rh() != null) {
                        self.begin_token_list(__av_eqtb[crate::ix::U(((every_math_loc) - 1) as usize)].hh().rh(), every_math_text);
                    }
                }
            }
        }
    }

    /// When \TeX\ is in display math mode, `cur_group=math_shift_group`,
    /// so it is not necessary for the `start_eq_no` procedure to test for
    /// this condition.
    /// @<Declare act...
    // §1321
    pub fn start_eq_no(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        { let __ix1809 = (self.save_ptr).wrapping_add(0i32); let __v1810 = self.cur_chr; self.save_stack[crate::ix::U((__ix1809) as usize)].set_int(__v1810); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        // §1318
        {
            self.push_math(math_shift_group);
            self.eq_word_define(29321i32, (1i32).wrapping_neg());
            if (__av_eqtb[crate::ix::U(((every_math_loc) - 1) as usize)].hh().rh() != null) {
                self.begin_token_list(__av_eqtb[crate::ix::U(((every_math_loc) - 1) as usize)].hh().rh(), every_math_text);
            }
        }
    }

    /// Recall that the `nucleus`, `subscr`, and `supscr` fields in a noad are
    /// broken down into subfields called `math_type` and either `info` or
    /// `(fam,character)`. The job of `scan_math` is to figure out what to place
    /// in one of these principal fields; it looks at the subformula that
    /// comes next in the input, and places an encoding of that subformula
    /// into a given word of `mem`.
    // §1330
    pub fn scan_math(&mut self, mut p: halfword) {
        let mut c: i32 = 0; // §1330
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // goto labels: restart, reswitch, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                loop {
                    // §431
                    self.get_x_token();
                    if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                }
            }
            if __goto_1 <= 1 { // reswitch
                // §1330
                match self.cur_cmd {
                    letter | other_char | char_given => {
                        {
                            c = __av_eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                            if (c == 32768i32) {
                                {
                                    // §1331
                                    {
                                        self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                        self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                                        self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                                        self.x_token();
                                        self.back_input();
                                    }
                                    // §1330
                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                }
                            }
                        }
                    }
                    char_num => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.cur_cmd = char_given;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    math_char_num => {
                        {
                            self.scan_fifteen_bit_int();
                            c = self.cur_val;
                        }
                    }
                    math_given => {
                        c = self.cur_chr;
                    }
                    delim_num => {
                        {
                            self.scan_twenty_seven_bit_int();
                            c = (self.cur_val / 4096i32);
                        }
                    }
                    _ => {
                        // §1332
                        {
                            self.back_input();
                            self.scan_left_brace();
                            { let __ix1811 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix1811) as usize)].set_int(p); }
                            self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                            self.push_math(math_group);
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §1330
                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(math_char);
                __av_mem[crate::ix::U((p) as usize)].set_hh_b1((c % 256i32));
                if ((c >= var_code) && ((__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() < 16i32))) {
                    { let __v1812 = __av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int(); __av_mem[crate::ix::U((p) as usize)].set_hh_b0(__v1812); }
                } else {
                    __av_mem[crate::ix::U((p) as usize)].set_hh_b0(((c / 256i32) % 16i32));
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
    // §1334
    pub fn set_math_char(&mut self, mut c: i32) {
        let mut p: halfword = 0; // §1334
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (c >= 32768i32) {
            // §1331
            {
                self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                self.cur_cmd = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0();
                self.cur_chr = __av_eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().rh();
                self.x_token();
                self.back_input();
            }
        } else {
            // §1334
            {
                p = self.new_noad();
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(math_char);
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b1((c % 256i32));
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b0(((c / 256i32) % 16i32));
                if (c >= var_code) {
                    {
                        if ((__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() < 16i32)) {
                            { let __v1813 = __av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int(); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b0(__v1813); }
                        }
                        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(ord_noad);
                    }
                } else {
                    __av_mem[crate::ix::U((p) as usize)].set_hh_b0((ord_noad).wrapping_add((c / 4096i32)));
                }
                { let __ix1814 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1814) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
    }

    /// @<Declare act...
    // §1338
    pub fn math_limit_switch(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            if (self.cur_list.head_field != self.cur_list.tail_field) {
                if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == op_noad) {
                    {
                        { let __ix1815 = self.cur_list.tail_field; let __v1816 = self.cur_chr; __av_mem[crate::ix::U((__ix1815) as usize)].set_hh_b1(__v1816); }
                        break 'l_exit_f;
                    }
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
                self.print(1559i32);
            }
            {
                self.help_ptr = 1i32;
                self.help_line[crate::ix::U((0i32) as usize)] = 1560i32;
            }
            self.error();
        }
    }

    /// Delimiter fields of noads are filled in by the `scan_delimiter` routine.
    /// The first parameter of this procedure is the `mem` address where the
    /// delimiter is to be placed; the second tells if this delimiter follows
    /// \.{\\radical} or not.
    /// @<Declare act...
    // §1339
    pub fn scan_delimiter(&mut self, mut p: halfword, mut r: bool) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if r {
            self.scan_twenty_seven_bit_int();
        } else {
            {
                // §431
                loop {
                    self.get_x_token();
                    if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                }
                // §1339
                match self.cur_cmd {
                    letter | other_char => {
                        self.cur_val = __av_eqtb[crate::ix::U((((del_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].int();
                    }
                    delim_num => {
                        self.scan_twenty_seven_bit_int();
                    }
                    _ => {
                        self.cur_val = (1i32).wrapping_neg();
                    }
                }
            }
        }
        if (self.cur_val < 0i32) {
            // §1340
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
                    self.print(1561i32);
                }
                {
                    self.help_ptr = 6i32;
                    self.help_line[crate::ix::U((5i32) as usize)] = 1562i32;
                    self.help_line[crate::ix::U((4i32) as usize)] = 1563i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 1564i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1565i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1566i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1567i32;
                }
                self.back_error();
                self.cur_val = 0i32;
            }
        }
        // §1339
        { let __v1817 = ((self.cur_val / 1048576i32) % 16i32); __av_mem[crate::ix::U((p) as usize)].set_qqqq_b0(__v1817); }
        { let __v1818 = ((self.cur_val / 4096i32) % 256i32); __av_mem[crate::ix::U((p) as usize)].set_qqqq_b1(__v1818); }
        { let __v1819 = ((self.cur_val / 256i32) % 16i32); __av_mem[crate::ix::U((p) as usize)].set_qqqq_b2(__v1819); }
        { let __v1820 = (self.cur_val % 256i32); __av_mem[crate::ix::U((p) as usize)].set_qqqq_b3(__v1820); }
    }

    /// @<Declare act...
    // §1342
    pub fn math_radical(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1821 = self.cur_list.tail_field; let __v1822 = self.get_node(radical_noad_size); __av_mem[crate::ix::U((__ix1821) as usize)].set_hh_rh(__v1822); }
            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix1823 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1823) as usize)].set_hh_b0(radical_noad); }
        { let __ix1824 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1824) as usize)].set_hh_b1(normal); }
        { let __ix1825 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1826 = self.empty_field; __av_mem[crate::ix::U((__ix1825) as usize)].set_hh(__v1826); }
        { let __ix1827 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1828 = self.empty_field; __av_mem[crate::ix::U((__ix1827) as usize)].set_hh(__v1828); }
        { let __ix1829 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1830 = self.empty_field; __av_mem[crate::ix::U((__ix1829) as usize)].set_hh(__v1830); }
        self.scan_delimiter((self.cur_list.tail_field).wrapping_add(4i32), true);
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// @<Declare act...
    // §1344
    pub fn math_ac(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.cur_cmd == accent) {
            // §1345
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
                    self.print(1568i32);
                }
                self.print_esc(611i32);
                self.print(1569i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1570i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1571i32;
                }
                self.error();
            }
        }
        // §1344
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1831 = self.cur_list.tail_field; let __v1832 = self.get_node(accent_noad_size); __av_mem[crate::ix::U((__ix1831) as usize)].set_hh_rh(__v1832); }
            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        { let __ix1833 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1833) as usize)].set_hh_b0(accent_noad); }
        { let __ix1834 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1834) as usize)].set_hh_b1(normal); }
        { let __ix1835 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1836 = self.empty_field; __av_mem[crate::ix::U((__ix1835) as usize)].set_hh(__v1836); }
        { let __ix1837 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1838 = self.empty_field; __av_mem[crate::ix::U((__ix1837) as usize)].set_hh(__v1838); }
        { let __ix1839 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1840 = self.empty_field; __av_mem[crate::ix::U((__ix1839) as usize)].set_hh(__v1840); }
        { let __ix1841 = (self.cur_list.tail_field).wrapping_add(4i32); __av_mem[crate::ix::U((__ix1841) as usize)].set_hh_rh(math_char); }
        self.scan_fifteen_bit_int();
        { let __ix1842 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1843 = (self.cur_val % 256i32); __av_mem[crate::ix::U((__ix1842) as usize)].set_hh_b1(__v1843); }
        if ((self.cur_val >= var_code) && ((__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int() < 16i32))) {
            { let __ix1844 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1845 = __av_eqtb[crate::ix::U(((29321i32) - 1) as usize)].int(); __av_mem[crate::ix::U((__ix1844) as usize)].set_hh_b0(__v1845); }
        } else {
            { let __ix1846 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1847 = ((self.cur_val / 256i32) % 16i32); __av_mem[crate::ix::U((__ix1846) as usize)].set_hh_b0(__v1847); }
        }
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// The routine that scans the four mlists of a \.{\\mathchoice} is very
    /// much like the routine that builds discretionary nodes.
    /// @<Declare act...
    // §1351
    pub fn append_choices(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1848 = self.cur_list.tail_field; let __v1849 = self.new_choice(); __av_mem[crate::ix::U((__ix1848) as usize)].set_hh_rh(__v1849); }
            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        { let __ix1850 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[crate::ix::U((__ix1850) as usize)].set_int(0i32); }
        self.push_math(math_choice_group);
        self.scan_left_brace();
    }

    /// At the end of a math formula or subformula, the `fin_mlist` routine is
    /// called upon to return a pointer to the newly completed mlist, and to
    /// pop the nest back to the enclosing semantic level. The parameter to
    /// `fin_mlist`, if not null, points to a `right_noad` that ends the
    /// current mlist; this `right_noad` has not yet been appended.
    /// @<Declare the function called `fin_mlist`
    // §1363
    pub fn fin_mlist(&mut self, mut p: halfword) -> halfword {
        let mut fin_mlist: halfword = 0;
        let mut q: halfword = 0; // §1363
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.cur_list.aux_field.int() != null) {
            // §1364
            {
                { let __ix1851 = (self.cur_list.aux_field.int()).wrapping_add(3i32); __av_mem[crate::ix::U((__ix1851) as usize)].set_hh_rh(sub_mlist); }
                { let __ix1852 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v1853 = __av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1852) as usize)].set_hh_lh(__v1853); }
                if (p == null) {
                    q = self.cur_list.aux_field.int();
                } else {
                    {
                        q = __av_mem[crate::ix::U(((self.cur_list.aux_field.int()).wrapping_add(2i32)) as usize)].hh().lh();
                        if ((__av_mem[crate::ix::U((q) as usize)].hh().b0() != left_noad) || (self.cur_list.eTeX_aux_field == null)) {
                            self.confusion(1299i32);
                        }
                        { let __ix1854 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v1855 = __av_mem[crate::ix::U((self.cur_list.eTeX_aux_field) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1854) as usize)].set_hh_lh(__v1855); }
                        { let __ix1856 = self.cur_list.eTeX_aux_field; let __v1857 = self.cur_list.aux_field.int(); __av_mem[crate::ix::U((__ix1856) as usize)].set_hh_rh(__v1857); }
                        { let __ix1858 = self.cur_list.aux_field.int(); __av_mem[crate::ix::U((__ix1858) as usize)].set_hh_rh(p); }
                    }
                }
            }
        } else {
            // §1363
            {
                { let __ix1859 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1859) as usize)].set_hh_rh(p); }
                q = __av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh();
            }
        }
        self.pop_nest();
        fin_mlist = q;
        fin_mlist
    }

    /// @<Declare act...
    // §1353
    pub fn build_choices(&mut self) {
        let mut p: halfword = 0; // §1353
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            self.unsave();
            p = self.fin_mlist(null);
            match self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int() {
                0 => {
                    { let __ix1860 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1860) as usize)].set_hh_lh(p); }
                }
                1 => {
                    { let __ix1861 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1861) as usize)].set_hh_rh(p); }
                }
                2 => {
                    { let __ix1862 = (self.cur_list.tail_field).wrapping_add(2i32); __av_mem[crate::ix::U((__ix1862) as usize)].set_hh_lh(p); }
                }
                3 => {
                    {
                        { let __ix1863 = (self.cur_list.tail_field).wrapping_add(2i32); __av_mem[crate::ix::U((__ix1863) as usize)].set_hh_rh(p); }
                        self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                        break 'l_exit_f;
                    }
                }
                _ => {}
            }
            { let __ix1864 = (self.save_ptr).wrapping_sub(1i32); let __v1865 = (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()).wrapping_add(1i32); self.save_stack[crate::ix::U((__ix1864) as usize)].set_int(__v1865); }
            self.push_math(math_choice_group);
            self.scan_left_brace();
        }
    }

    /// @<Declare act...
    // §1355
    pub fn sub_sup(&mut self) {
        let mut t: small_number = 0; // §1355
        let mut p: halfword = 0; // §1355
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        t = empty;
        p = null;
        if (self.cur_list.tail_field != self.cur_list.head_field) {
            if ((__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() >= ord_noad) && (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() < left_noad)) {
                {
                    p = (((self.cur_list.tail_field).wrapping_add(2i32)).wrapping_add(self.cur_cmd)).wrapping_sub(7i32);
                    t = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
        }
        if ((p == null) || (t != empty)) {
            // §1356
            {
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1866 = self.cur_list.tail_field; let __v1867 = self.new_noad(); __av_mem[crate::ix::U((__ix1866) as usize)].set_hh_rh(__v1867); }
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                p = (((self.cur_list.tail_field).wrapping_add(2i32)).wrapping_add(self.cur_cmd)).wrapping_sub(7i32);
                if (t != empty) {
                    {
                        if (self.cur_cmd == sup_mark) {
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
                                    self.print(1572i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1573i32;
                                }
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
                                    self.print(1574i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1575i32;
                                }
                            }
                        }
                        self.error();
                    }
                }
            }
        }
        // §1355
        self.scan_math(p);
    }

    /// @<Declare act...
    // §1360
    pub fn math_fraction(&mut self) {
        let mut c: small_number = 0; // §1360
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        c = self.cur_chr;
        if (self.cur_list.aux_field.int() != null) {
            // §1362
            {
                if (c >= delimited_code) {
                    {
                        self.scan_delimiter(garbage, false);
                        self.scan_delimiter(garbage, false);
                    }
                }
                if ((c % delimited_code) == above_code) {
                    self.scan_dimen(false, false, false);
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
                    self.print(1582i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1583i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1584i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1585i32;
                }
                self.error();
            }
        } else {
            // §1360
            {
                { let __v1868 = self.get_node(fraction_noad_size); self.cur_list.aux_field.set_int(__v1868); }
                { let __ix1869 = self.cur_list.aux_field.int(); __av_mem[crate::ix::U((__ix1869) as usize)].set_hh_b0(fraction_noad); }
                { let __ix1870 = self.cur_list.aux_field.int(); __av_mem[crate::ix::U((__ix1870) as usize)].set_hh_b1(normal); }
                { let __ix1871 = (self.cur_list.aux_field.int()).wrapping_add(2i32); __av_mem[crate::ix::U((__ix1871) as usize)].set_hh_rh(sub_mlist); }
                { let __ix1872 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v1873 = __av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1872) as usize)].set_hh_lh(__v1873); }
                { let __ix1874 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v1875 = self.empty_field; __av_mem[crate::ix::U((__ix1874) as usize)].set_hh(__v1875); }
                { let __ix1876 = (self.cur_list.aux_field.int()).wrapping_add(4i32); let __v1877 = self.null_delimiter; __av_mem[crate::ix::U((__ix1876) as usize)].set_qqqq(__v1877); }
                { let __ix1878 = (self.cur_list.aux_field.int()).wrapping_add(5i32); let __v1879 = self.null_delimiter; __av_mem[crate::ix::U((__ix1878) as usize)].set_qqqq(__v1879); }
                { let __ix1880 = self.cur_list.head_field; __av_mem[crate::ix::U((__ix1880) as usize)].set_hh_rh(null); }
                self.cur_list.tail_field = self.cur_list.head_field;
                // §1361
                if (c >= delimited_code) {
                    {
                        self.scan_delimiter((self.cur_list.aux_field.int()).wrapping_add(4i32), false);
                        self.scan_delimiter((self.cur_list.aux_field.int()).wrapping_add(5i32), false);
                    }
                }
                match (c % delimited_code) {
                    above_code => {
                        {
                            self.scan_dimen(false, false, false);
                            { let __ix1881 = (self.cur_list.aux_field.int()).wrapping_add(1i32); let __v1882 = self.cur_val; __av_mem[crate::ix::U((__ix1881) as usize)].set_int(__v1882); }
                        }
                    }
                    over_code => {
                        { let __ix1883 = (self.cur_list.aux_field.int()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1883) as usize)].set_int(default_code); }
                    }
                    atop_code => {
                        { let __ix1884 = (self.cur_list.aux_field.int()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1884) as usize)].set_int(0i32); }
                    }
                    _ => {}
                }
            }
        }
    }

    /// @<Declare act...
    // §1370
    pub fn math_left_right(&mut self) {
        let mut t: small_number = 0; // §1370
        let mut p: halfword = 0; // §1370
        let mut q: halfword = 0; // §1370
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        t = self.cur_chr;
        if ((t != left_noad) && (self.cur_group != math_left_group)) {
            // §1371
            {
                if (self.cur_group == math_shift_group) {
                    {
                        self.scan_delimiter(garbage, false);
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(944i32);
                        }
                        if (t == middle_noad) {
                            {
                                self.print_esc(1300i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1586i32;
                                }
                            }
                        } else {
                            {
                                self.print_esc(1299i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1587i32;
                                }
                            }
                        }
                        self.error();
                    }
                } else {
                    self.off_save();
                }
            }
        } else {
            // §1370
            {
                p = self.new_noad();
                __av_mem[crate::ix::U((p) as usize)].set_hh_b0(t);
                self.scan_delimiter((p).wrapping_add(1i32), false);
                if (t == middle_noad) {
                    {
                        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(right_noad);
                        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(middle_noad);
                    }
                }
                if (t == left_noad) {
                    q = p;
                } else {
                    {
                        q = self.fin_mlist(p);
                        self.unsave();
                    }
                }
                if (t != right_noad) {
                    {
                        self.push_math(math_left_group);
                        { let __ix1885 = self.cur_list.head_field; __av_mem[crate::ix::U((__ix1885) as usize)].set_hh_rh(q); }
                        self.cur_list.tail_field = p;
                        self.cur_list.eTeX_aux_field = p;
                    }
                } else {
                    {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1886 = self.cur_list.tail_field; let __v1887 = self.new_noad(); __av_mem[crate::ix::U((__ix1886) as usize)].set_hh_rh(__v1887); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                        { let __ix1888 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix1888) as usize)].set_hh_b0(inner_noad); }
                        { let __ix1889 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1889) as usize)].set_hh_rh(sub_mlist); }
                        { let __ix1890 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1890) as usize)].set_hh_lh(q); }
                    }
                }
            }
        }
    }

    /// The `app_display` procedure used to append the displayed equation
    /// and\slash or equation number to the current vertical list has three
    /// parameters:  the prototype box, the hbox to be appended, and the
    /// displacement of the hbox in the display line.
    /// @<Declare subprocedures for `after_math`
    // §1745
    pub fn app_display(&mut self, mut j: halfword, mut b: halfword, mut d: scaled) {
        let mut z: scaled = 0; // §1745
        let mut s: scaled = 0; // §1745
        let mut e: scaled = 0; // §1745
        let mut x: i32 = 0; // §1745
        let mut p: halfword = 0; // §1745
        let mut q: halfword = 0; // §1745
        let mut r: halfword = 0; // §1745
        let mut t: halfword = 0; // §1745
        let mut u: halfword = 0; // §1745
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        s = __av_eqtb[crate::ix::U(((29918i32) - 1) as usize)].int();
        x = __av_eqtb[crate::ix::U(((29384i32) - 1) as usize)].int();
        if (x == 0i32) {
            __av_mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int((s).wrapping_add(d));
        } else {
            {
                z = __av_eqtb[crate::ix::U(((29917i32) - 1) as usize)].int();
                p = b;
                // §1746
                if (x > 0i32) {
                    e = ((z).wrapping_sub(d)).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                } else {
                    {
                        e = d;
                        d = ((z).wrapping_sub(e)).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                    }
                }
                if (j != null) {
                    {
                        b = self.copy_node_list(j);
                        { let __v1891 = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v1891); }
                        { let __v1892 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v1892); }
                        s = (s).wrapping_sub(__av_mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].int());
                        d = (d).wrapping_add(s);
                        e = (((e).wrapping_add(__av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int())).wrapping_sub(z)).wrapping_sub(s);
                    }
                }
                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == dlist) {
                    q = p;
                } else {
                    {
                        r = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                        self.free_node(p, box_node_size);
                        if (r == null) {
                            self.confusion(2058i32);
                        }
                        if (x > 0i32) {
                            {
                                p = r;
                                loop {
                                    q = r;
                                    r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                    if (r == null) { break; }
                                }
                            }
                        } else {
                            {
                                p = null;
                                q = r;
                                loop {
                                    t = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                    __av_mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                                    p = r;
                                    r = t;
                                    if (r == null) { break; }
                                }
                            }
                        }
                    }
                }
                // §1747
                if (j == null) {
                    {
                        r = self.new_kern(0i32);
                        t = self.new_kern(0i32);
                    }
                } else {
                    {
                        r = __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                        t = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                u = self.new_math(0i32, end_M_code);
                if (__av_mem[crate::ix::U((t) as usize)].hh().b0() == glue_node) {
                    {
                        j = self.new_skip_param(right_skip_code);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(j);
                        __av_mem[crate::ix::U((j) as usize)].set_hh_rh(u);
                        j = __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].hh().lh();
                        { let __ix1893 = self.temp_ptr; let __v1894 = __av_mem[crate::ix::U((j) as usize)].hh().b0(); __av_mem[crate::ix::U((__ix1893) as usize)].set_hh_b0(__v1894); }
                        { let __ix1895 = self.temp_ptr; let __v1896 = __av_mem[crate::ix::U((j) as usize)].hh().b1(); __av_mem[crate::ix::U((__ix1895) as usize)].set_hh_b1(__v1896); }
                        { let __ix1897 = (self.temp_ptr).wrapping_add(1i32); let __v1898 = (e).wrapping_sub(__av_mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)].int()); __av_mem[crate::ix::U((__ix1897) as usize)].set_int(__v1898); }
                        { let __ix1899 = (self.temp_ptr).wrapping_add(2i32); let __v1900 = (__av_mem[crate::ix::U(((j).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix1899) as usize)].set_int(__v1900); }
                        { let __ix1901 = (self.temp_ptr).wrapping_add(3i32); let __v1902 = (__av_mem[crate::ix::U(((j).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix1901) as usize)].set_int(__v1902); }
                        __av_mem[crate::ix::U((u) as usize)].set_hh_rh(t);
                    }
                } else {
                    {
                        __av_mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(e);
                        __av_mem[crate::ix::U((t) as usize)].set_hh_rh(u);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(t);
                    }
                }
                u = self.new_math(0i32, begin_M_code);
                if (__av_mem[crate::ix::U((r) as usize)].hh().b0() == glue_node) {
                    {
                        j = self.new_skip_param(left_skip_code);
                        __av_mem[crate::ix::U((u) as usize)].set_hh_rh(j);
                        __av_mem[crate::ix::U((j) as usize)].set_hh_rh(p);
                        j = __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().lh();
                        { let __ix1903 = self.temp_ptr; let __v1904 = __av_mem[crate::ix::U((j) as usize)].hh().b0(); __av_mem[crate::ix::U((__ix1903) as usize)].set_hh_b0(__v1904); }
                        { let __ix1905 = self.temp_ptr; let __v1906 = __av_mem[crate::ix::U((j) as usize)].hh().b1(); __av_mem[crate::ix::U((__ix1905) as usize)].set_hh_b1(__v1906); }
                        { let __ix1907 = (self.temp_ptr).wrapping_add(1i32); let __v1908 = (d).wrapping_sub(__av_mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)].int()); __av_mem[crate::ix::U((__ix1907) as usize)].set_int(__v1908); }
                        { let __ix1909 = (self.temp_ptr).wrapping_add(2i32); let __v1910 = (__av_mem[crate::ix::U(((j).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix1909) as usize)].set_int(__v1910); }
                        { let __ix1911 = (self.temp_ptr).wrapping_add(3i32); let __v1912 = (__av_mem[crate::ix::U(((j).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U((__ix1911) as usize)].set_int(__v1912); }
                        __av_mem[crate::ix::U((r) as usize)].set_hh_rh(u);
                    }
                } else {
                    {
                        __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(d);
                        __av_mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                        __av_mem[crate::ix::U((u) as usize)].set_hh_rh(r);
                        if (j == null) {
                            {
                                b = self.hpack(u, 0i32, additional);
                                __av_mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int(s);
                            }
                        } else {
                            __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(u);
                        }
                    }
                }
            }
        }
        // §1745
        self.append_to_vlist(b);
    }

    /// @<Declare act...
    // §1373
    pub fn after_math(&mut self) {
        let mut l: bool = false; // §1373
        let mut danger: bool = false; // §1373
        let mut m: i32 = 0; // §1373
        let mut p: halfword = 0; // §1373
        let mut a: halfword = 0; // §1373
        let mut b: halfword = 0; // §1377
        let mut w: scaled = 0; // §1377
        let mut z: scaled = 0; // §1377
        let mut e: scaled = 0; // §1377
        let mut q: scaled = 0; // §1377
        let mut d: scaled = 0; // §1377
        let mut s: scaled = 0; // §1377
        let mut g1: small_number = 0; // §1377
        let mut g2: small_number = 0; // §1377
        let mut r: halfword = 0; // §1377
        let mut t: halfword = 0; // §1377
        let mut pre_t: halfword = 0; // §1377
        let mut j: halfword = 0; // §1742
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        danger = false;
        // §1743
        if (self.cur_list.mode_field == mmode) {
            j = self.cur_list.eTeX_aux_field;
        }
        // §1374
        if (((self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27695i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27711i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params)) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27727i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params)) {
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
                    self.print(1588i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1589i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1590i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1591i32;
                }
                self.error();
                self.flush_math();
                danger = true;
            }
        } else {
            if (((self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27696i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27712i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params)) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27728i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params)) {
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
                        self.print(1592i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 1593i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1594i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1595i32;
                    }
                    self.error();
                    self.flush_math();
                    danger = true;
                }
            }
        }
        // §1373
        m = self.cur_list.mode_field;
        l = false;
        p = self.fin_mlist(null);
        if (self.cur_list.mode_field == (m).wrapping_neg()) {
            {
                // §1376
                {
                    self.get_x_token();
                    if (self.cur_cmd != math_shift) {
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
                                self.print(1596i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1597i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1598i32;
                            }
                            self.back_error();
                        }
                    }
                }
                // §1373
                self.cur_mlist = p;
                self.cur_style = text_style;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                a = self.hpack(__av_mem[crate::ix::U((temp_head) as usize)].hh().rh(), 0i32, additional);
                __av_mem[crate::ix::U((a) as usize)].set_hh_b1(dlist);
                self.unsave();
                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int() == 1i32) {
                    l = true;
                }
                danger = false;
                // §1743
                if (self.cur_list.mode_field == mmode) {
                    j = self.cur_list.eTeX_aux_field;
                }
                // §1374
                if (((self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27695i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27711i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params)) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27727i32) - 1) as usize)].hh().rh()) as usize)] < total_mathsy_params)) {
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
                            self.print(1588i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 1589i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1590i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1591i32;
                        }
                        self.error();
                        self.flush_math();
                        danger = true;
                    }
                } else {
                    if (((self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27696i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27712i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params)) || (self.font_params[crate::ix::U((__av_eqtb[crate::ix::U(((27728i32) - 1) as usize)].hh().rh()) as usize)] < total_mathex_params)) {
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
                                self.print(1592i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1593i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1594i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1595i32;
                            }
                            self.error();
                            self.flush_math();
                            danger = true;
                        }
                    }
                }
                // §1373
                m = self.cur_list.mode_field;
                p = self.fin_mlist(null);
            }
        } else {
            a = null;
        }
        if (m < 0i32) {
            // §1375
            {
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1913 = self.cur_list.tail_field; let __v1914 = self.new_math(__av_eqtb[crate::ix::U(((29904i32) - 1) as usize)].int(), before); __av_mem[crate::ix::U((__ix1913) as usize)].set_hh_rh(__v1914); }
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                self.cur_mlist = p;
                self.cur_style = text_style;
                self.mlist_penalties = (self.cur_list.mode_field > 0i32);
                self.mlist_to_hlist();
                { let __ix1915 = self.cur_list.tail_field; let __v1916 = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1915) as usize)].set_hh_rh(__v1916); }
                while (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh() != null) {
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1917 = self.cur_list.tail_field; let __v1918 = self.new_math(__av_eqtb[crate::ix::U(((29904i32) - 1) as usize)].int(), after); __av_mem[crate::ix::U((__ix1917) as usize)].set_hh_rh(__v1918); }
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                self.cur_list.aux_field.set_hh_lh(1000i32);
                self.unsave();
            }
        } else {
            // §1373
            {
                if (a == null) {
                    // §1376
                    {
                        self.get_x_token();
                        if (self.cur_cmd != math_shift) {
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
                                    self.print(1596i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1597i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1598i32;
                                }
                                self.back_error();
                            }
                        }
                    }
                }
                // §1378
                self.cur_mlist = p;
                self.cur_style = display_style;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                p = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh();
                self.adjust_tail = adjust_head;
                self.pre_adjust_tail = pre_adjust_head;
                b = self.hpack(p, 0i32, additional);
                p = __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                t = self.adjust_tail;
                self.adjust_tail = null;
                pre_t = self.pre_adjust_tail;
                self.pre_adjust_tail = null;
                w = __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int();
                z = __av_eqtb[crate::ix::U(((29917i32) - 1) as usize)].int();
                s = __av_eqtb[crate::ix::U(((29918i32) - 1) as usize)].int();
                if (__av_eqtb[crate::ix::U(((29384i32) - 1) as usize)].int() < 0i32) {
                    s = ((s).wrapping_neg()).wrapping_sub(z);
                }
                if ((a == null) || danger) {
                    {
                        e = 0i32;
                        q = 0i32;
                    }
                } else {
                    {
                        e = __av_mem[crate::ix::U(((a).wrapping_add(1i32)) as usize)].int();
                        q = (e).wrapping_add(self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U(((27695i32) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                    }
                }
                if ((w).wrapping_add(q) > z) {
                    // §1380
                    {
                        if ((e != 0i32) && ((((((w).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)])).wrapping_add(q) <= z) || (self.total_shrink[crate::ix::U((fil) as usize)] != 0i32)) || (self.total_shrink[crate::ix::U((fill) as usize)] != 0i32)) || (self.total_shrink[crate::ix::U((filll) as usize)] != 0i32))) {
                            {
                                self.free_node(b, box_node_size);
                                b = self.hpack(p, (z).wrapping_sub(q), exactly);
                            }
                        } else {
                            {
                                e = 0i32;
                                if (w > z) {
                                    {
                                        self.free_node(b, box_node_size);
                                        b = self.hpack(p, z, exactly);
                                    }
                                }
                            }
                        }
                        w = __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int();
                    }
                }
                // §1381
                __av_mem[crate::ix::U((b) as usize)].set_hh_b1(dlist);
                d = self.half((z).wrapping_sub(w));
                if ((e > 0i32) && (d < (2i32).wrapping_mul(e))) {
                    {
                        d = self.half(((z).wrapping_sub(w)).wrapping_sub(e));
                        if (p != null) {
                            if (!(p >= self.hi_mem_min)) {
                                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) {
                                    d = 0i32;
                                }
                            }
                        }
                    }
                }
                // §1382
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1919 = self.cur_list.tail_field; let __v1920 = self.new_penalty(__av_eqtb[crate::ix::U(((29288i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix1919) as usize)].set_hh_rh(__v1920); }
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                if (((d).wrapping_add(s) <= __av_eqtb[crate::ix::U(((29916i32) - 1) as usize)].int()) || l) {
                    {
                        g1 = above_display_skip_code;
                        g2 = below_display_skip_code;
                    }
                } else {
                    {
                        g1 = above_display_short_skip_code;
                        g2 = below_display_short_skip_code;
                    }
                }
                if (l && (e == 0i32)) {
                    {
                        self.app_display(j, a, 0i32);
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1921 = self.cur_list.tail_field; let __v1922 = self.new_penalty(inf_penalty); __av_mem[crate::ix::U((__ix1921) as usize)].set_hh_rh(__v1922); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                    }
                } else {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1923 = self.cur_list.tail_field; let __v1924 = self.new_param_glue(g1); __av_mem[crate::ix::U((__ix1923) as usize)].set_hh_rh(__v1924); }
                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                }
                // §1383
                if (e != 0i32) {
                    {
                        r = self.new_kern((((z).wrapping_sub(w)).wrapping_sub(e)).wrapping_sub(d));
                        if l {
                            {
                                __av_mem[crate::ix::U((a) as usize)].set_hh_rh(r);
                                __av_mem[crate::ix::U((r) as usize)].set_hh_rh(b);
                                b = a;
                                d = 0i32;
                            }
                        } else {
                            {
                                __av_mem[crate::ix::U((b) as usize)].set_hh_rh(r);
                                __av_mem[crate::ix::U((r) as usize)].set_hh_rh(a);
                            }
                        }
                        b = self.hpack(b, 0i32, additional);
                    }
                }
                self.app_display(j, b, d);
                // §1384
                if (((a != null) && (e == 0i32)) && (!l)) {
                    {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1925 = self.cur_list.tail_field; let __v1926 = self.new_penalty(inf_penalty); __av_mem[crate::ix::U((__ix1925) as usize)].set_hh_rh(__v1926); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                        self.app_display(j, a, (z).wrapping_sub(__av_mem[crate::ix::U(((a).wrapping_add(1i32)) as usize)].int()));
                        g2 = 0i32;
                    }
                }
                if (t != adjust_head) {
                    {
                        { let __ix1927 = self.cur_list.tail_field; let __v1928 = __av_mem[crate::ix::U((adjust_head) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1927) as usize)].set_hh_rh(__v1928); }
                        self.cur_list.tail_field = t;
                    }
                }
                if (pre_t != pre_adjust_head) {
                    {
                        { let __ix1929 = self.cur_list.tail_field; let __v1930 = __av_mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix1929) as usize)].set_hh_rh(__v1930); }
                        self.cur_list.tail_field = pre_t;
                    }
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1931 = self.cur_list.tail_field; let __v1932 = self.new_penalty(__av_eqtb[crate::ix::U(((29289i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix1931) as usize)].set_hh_rh(__v1932); }
                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                }
                if (g2 > 0i32) {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1933 = self.cur_list.tail_field; let __v1934 = self.new_param_glue(g2); __av_mem[crate::ix::U((__ix1933) as usize)].set_hh_rh(__v1934); }
                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                }
                // §1744
                self.flush_node_list(j);
                // §1378
                self.resume_after_display();
            }
        }
    }

    /// @<Declare act...
    // §1379
    pub fn resume_after_display(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (self.cur_group != math_shift_group) {
            self.confusion(1599i32);
        }
        self.unsave();
        self.cur_list.pg_field = (self.cur_list.pg_field).wrapping_add(3i32);
        self.push_nest();
        self.cur_list.mode_field = hmode;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (__av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (__av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() > 255i32) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = __av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int();
            }
        }
        { let __v1935 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v1935); }
        self.cur_list.pg_field = ((((self.norm_min(__av_eqtb[crate::ix::U(((29328i32) - 1) as usize)].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(__av_eqtb[crate::ix::U(((29329i32) - 1) as usize)].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        // §470
        {
            self.get_x_token();
            if (self.cur_cmd != spacer) {
                self.back_input();
            }
        }
        // §1379
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// When a control sequence is to be defined, by \.{\\def} or \.{\\let} or
    /// something similar, the `get_r_token` routine will substitute a special
    /// control sequence for a token that is not redefinable.
    /// @<Declare subprocedures for `prefixed_command`
    // §1394
    pub fn get_r_token(&mut self) {
        'l_restart_b: loop {
            loop {
                self.intr_weak = true;
                self.get_token();
                self.intr_weak = false;
                if (self.cur_tok != space_token) { break; }
            }
            if (((self.cur_cs == 0i32) || (self.cur_cs > eqtb_top)) || ((self.cur_cs > frozen_control_sequence) && (self.cur_cs <= eqtb_size))) {
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
                        self.print(1617i32);
                    }
                    {
                        self.help_ptr = 5i32;
                        self.help_line[crate::ix::U((4i32) as usize)] = 1618i32;
                        self.help_line[crate::ix::U((3i32) as usize)] = 1619i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 1620i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1621i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1622i32;
                    }
                    if (self.cur_cs == 0i32) {
                        self.back_input();
                    }
                    self.cur_tok = 19609i32;
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
    // §1408
    pub fn trap_zero_glue(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (((__av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int() == 0i32) && (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int() == 0i32)) && (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int() == 0i32)) {
            {
                { let __v1936 = (__av_mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v1936); }
                self.delete_glue_ref(self.cur_val);
                self.cur_val = zero_glue;
            }
        }
    }

    /// We use the fact that `register<advance<multiply<divide`.
    /// @<Declare subprocedures for `prefixed_command`
    // §1415
    pub fn do_register_command(&mut self, mut a: small_number) {
        let mut l: halfword = 0; // §1415
        let mut q: halfword = 0; // §1415
        let mut r: halfword = 0; // §1415
        let mut s: halfword = 0; // §1415
        let mut p: i32 = 0; // §1415
        let mut e: bool = false; // §1415
        let mut w: i32 = 0; // §1415
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            'l_found_f: {
                q = self.cur_cmd;
                e = false;
                // §1416
                {
                    if (q != register) {
                        {
                            self.get_x_token();
                            if ((self.cur_cmd >= assign_int) && (self.cur_cmd <= assign_mu_glue)) {
                                {
                                    l = self.cur_chr;
                                    p = (self.cur_cmd).wrapping_sub(73i32);
                                    break 'l_found_f;
                                }
                            }
                            if (self.cur_cmd != register) {
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
                                        self.print(797i32);
                                    }
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print(798i32);
                                    self.print_cmd_chr(q, 0i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1656i32;
                                    }
                                    self.error();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    if ((self.cur_chr < mem_bot) || (self.cur_chr > lo_mem_stat_max)) {
                        {
                            l = self.cur_chr;
                            p = (__av_mem[crate::ix::U((l) as usize)].hh().b0() / 16i32);
                            e = true;
                        }
                    } else {
                        {
                            p = (self.cur_chr).wrapping_sub(0i32);
                            self.scan_register_num();
                            if (self.cur_val > 255i32) {
                                {
                                    self.find_sa_element(p, self.cur_val, true);
                                    l = self.cur_ptr;
                                    e = true;
                                }
                            } else {
                                match p {
                                    int_val => {
                                        l = (self.cur_val).wrapping_add(29391i32);
                                    }
                                    dimen_val => {
                                        l = (self.cur_val).wrapping_add(29937i32);
                                    }
                                    glue_val => {
                                        l = (self.cur_val).wrapping_add(26646i32);
                                    }
                                    mu_val => {
                                        l = (self.cur_val).wrapping_add(26902i32);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            if (p < glue_val) {
                if e {
                    w = __av_mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int();
                } else {
                    w = __av_eqtb[crate::ix::U(((l) - 1) as usize)].int();
                }
            } else {
                if e {
                    s = __av_mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)].hh().rh();
                } else {
                    s = __av_eqtb[crate::ix::U(((l) - 1) as usize)].hh().rh();
                }
            }
            // §1415
            if self.intr_rec_on {
                if (!e) {
                    if (q != register) {
                        self.flashtex_intr_read(l);
                    }
                }
            }
            if self.bm_rec_on {
                if (q != register) {
                    self.flashtex_bm_reg_loc(l, e);
                }
            }
            if (q == register) {
                self.scan_optional_equals();
            } else {
                if self.scan_keyword(1652i32) {
                }
            }
            self.arith_error = false;
            if (q < multiply) {
                // §1417
                if (p < glue_val) {
                    {
                        if (p == int_val) {
                            self.scan_int();
                        } else {
                            self.scan_dimen(false, false, false);
                        }
                        if (q == advance) {
                            self.cur_val = (self.cur_val).wrapping_add(w);
                        }
                    }
                } else {
                    {
                        self.scan_glue(p);
                        if (q == advance) {
                            // §1418
                            {
                                q = self.new_spec(self.cur_val);
                                r = s;
                                self.delete_glue_ref(self.cur_val);
                                { let __v1937 = (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1937); }
                                if (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                    __av_mem[crate::ix::U((q) as usize)].set_hh_b0(normal);
                                }
                                if (__av_mem[crate::ix::U((q) as usize)].hh().b0() == __av_mem[crate::ix::U((r) as usize)].hh().b0()) {
                                    { let __v1938 = (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()); __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1938); }
                                } else {
                                    if ((__av_mem[crate::ix::U((q) as usize)].hh().b0() < __av_mem[crate::ix::U((r) as usize)].hh().b0()) && (__av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int() != 0i32)) {
                                        {
                                            { let __v1939 = __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v1939); }
                                            { let __v1940 = __av_mem[crate::ix::U((r) as usize)].hh().b0(); __av_mem[crate::ix::U((q) as usize)].set_hh_b0(__v1940); }
                                        }
                                    }
                                }
                                if (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                    __av_mem[crate::ix::U((q) as usize)].set_hh_b1(normal);
                                }
                                if (__av_mem[crate::ix::U((q) as usize)].hh().b1() == __av_mem[crate::ix::U((r) as usize)].hh().b1()) {
                                    { let __v1941 = (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1941); }
                                } else {
                                    if ((__av_mem[crate::ix::U((q) as usize)].hh().b1() < __av_mem[crate::ix::U((r) as usize)].hh().b1()) && (__av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                        {
                                            { let __v1942 = __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v1942); }
                                            { let __v1943 = __av_mem[crate::ix::U((r) as usize)].hh().b1(); __av_mem[crate::ix::U((q) as usize)].set_hh_b1(__v1943); }
                                        }
                                    }
                                }
                                self.cur_val = q;
                            }
                        }
                    }
                }
            } else {
                // §1419
                {
                    self.scan_int();
                    if (p < glue_val) {
                        if (q == multiply) {
                            if (p == int_val) {
                                self.cur_val = self.mult_and_add(w, self.cur_val, 0i32, 2147483647i32);
                            } else {
                                self.cur_val = self.mult_and_add(w, self.cur_val, 0i32, 1073741823i32);
                            }
                        } else {
                            self.cur_val = self.x_over_n(w, self.cur_val);
                        }
                    } else {
                        {
                            r = self.new_spec(s);
                            if (q == multiply) {
                                {
                                    { let __v1944 = self.mult_and_add(__av_mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(), self.cur_val, 0i32, 1073741823i32); __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v1944); }
                                    { let __v1945 = self.mult_and_add(__av_mem[crate::ix::U(((s).wrapping_add(2i32)) as usize)].int(), self.cur_val, 0i32, 1073741823i32); __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v1945); }
                                    { let __v1946 = self.mult_and_add(__av_mem[crate::ix::U(((s).wrapping_add(3i32)) as usize)].int(), self.cur_val, 0i32, 1073741823i32); __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1946); }
                                }
                            } else {
                                {
                                    { let __v1947 = self.x_over_n(__av_mem[crate::ix::U(((s).wrapping_add(1i32)) as usize)].int(), self.cur_val); __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v1947); }
                                    { let __v1948 = self.x_over_n(__av_mem[crate::ix::U(((s).wrapping_add(2i32)) as usize)].int(), self.cur_val); __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v1948); }
                                    { let __v1949 = self.x_over_n(__av_mem[crate::ix::U(((s).wrapping_add(3i32)) as usize)].int(), self.cur_val); __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v1949); }
                                }
                            }
                            self.cur_val = r;
                        }
                    }
                }
            }
            // §1415
            if self.arith_error {
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
                        self.print(1653i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1654i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1655i32;
                    }
                    if (p >= glue_val) {
                        self.delete_glue_ref(self.cur_val);
                    }
                    self.error();
                    break 'l_exit_f;
                }
            }
            if (p < glue_val) {
                if e {
                    if (a >= 4i32) {
                        self.gsa_w_def(l, self.cur_val);
                    } else {
                        self.sa_w_def(l, self.cur_val);
                    }
                } else {
                    if (a >= 4i32) {
                        self.geq_word_define(l, self.cur_val);
                    } else {
                        self.eq_word_define(l, self.cur_val);
                    }
                }
            } else {
                {
                    self.trap_zero_glue();
                    if e {
                        if (a >= 4i32) {
                            self.gsa_def(l, self.cur_val);
                        } else {
                            self.sa_def(l, self.cur_val);
                        }
                    } else {
                        if (a >= 4i32) {
                            self.geq_define(l, glue_ref, self.cur_val);
                        } else {
                            self.eq_define(l, glue_ref, self.cur_val);
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1422
    pub fn alter_aux(&mut self) {
        let mut c: halfword = 0; // §1422
        if (self.cur_chr != (self.cur_list.mode_field).wrapping_abs()) {
            self.report_illegal_case();
        } else {
            {
                c = self.cur_chr;
                self.scan_optional_equals();
                if (c == vmode) {
                    {
                        self.scan_dimen(false, false, false);
                        { let __v1950 = self.cur_val; self.cur_list.aux_field.set_int(__v1950); }
                    }
                } else {
                    {
                        self.scan_int();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32767i32)) {
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
                                    self.print(1659i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1660i32;
                                }
                                self.int_error(self.cur_val);
                            }
                        } else {
                            { let __v1951 = self.cur_val; self.cur_list.aux_field.set_hh_lh(__v1951); }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1423
    pub fn alter_prev_graf(&mut self) {
        let mut p: i32 = 0; // §1423
        { let __ix1952 = self.nest_ptr; let __v1953 = self.cur_list; self.nest[crate::ix::U((__ix1952) as usize)] = __v1953; }
        p = self.nest_ptr;
        while ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() != vmode) {
            p = (p).wrapping_sub(1i32);
        }
        self.scan_optional_equals();
        self.scan_int();
        if (self.cur_val < 0i32) {
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
                    self.print(1382i32);
                }
                self.print_esc(621i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1661i32;
                }
                self.int_error(self.cur_val);
            }
        } else {
            {
                self.nest[crate::ix::U((p) as usize)].pg_field = self.cur_val;
                self.cur_list = self.nest[crate::ix::U((self.nest_ptr) as usize)];
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1424
    pub fn alter_page_so_far(&mut self) {
        let mut c: i32 = 0; // §1424
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        { let __v1954 = self.cur_val; self.page_so_far[crate::ix::U((c) as usize)] = __v1954; }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1425
    pub fn alter_integer(&mut self) {
        let mut c: small_number = 0; // §1425
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_int();
        if (c == 0i32) {
            self.dead_cycles = self.cur_val;
        } else {
            // §1697
            if (c == 2i32) {
                {
                    if ((self.cur_val < batch_mode) || (self.cur_val > error_stop_mode)) {
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
                                self.print(2043i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 2044i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 2045i32;
                            }
                            self.int_error(self.cur_val);
                        }
                    } else {
                        {
                            self.cur_chr = self.cur_val;
                            self.new_interaction();
                        }
                    }
                }
            } else {
                // §1425
                self.insert_penalties = self.cur_val;
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1426
    pub fn alter_box_dimen(&mut self) {
        let mut c: small_number = 0; // §1426
        let mut b: halfword = 0; // §1426
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        c = self.cur_chr;
        self.scan_register_num();
        {
            if self.bm_rec_on {
                self.flashtex_bm_box(self.cur_val);
            }
            if (self.cur_val < 256i32) {
                b = __av_eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
            } else {
                {
                    self.find_sa_element(box_val, self.cur_val, false);
                    if (self.cur_ptr == null) {
                        b = null;
                    } else {
                        b = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                    }
                }
            }
        }
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        if (b != null) {
            { let __v1955 = self.cur_val; __av_mem[crate::ix::U(((b).wrapping_add(c)) as usize)].set_int(__v1955); }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1436
    pub fn new_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §1436
        let mut s: scaled = 0; // §1436
        let mut f: internal_font_number = 0; // §1436
        let mut t: str_number = 0; // §1436
        let mut old_setting: i32 = 0; // §1436
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        'l_common_ending_f: {
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            self.get_r_token();
            u = self.cur_cs;
            if (u >= hash_base) {
                t = self.hash[crate::ix::U(((u) - 514) as usize)].rh();
            } else {
                if (u >= single_base) {
                    if (u == null_cs) {
                        t = 1109i32;
                    } else {
                        t = (u).wrapping_sub(257i32);
                    }
                } else {
                    {
                        old_setting = self.selector;
                        self.selector = new_string;
                        self.print(1109i32);
                        self.print((u).wrapping_sub(1i32));
                        self.selector = old_setting;
                        {
                            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                            }
                        }
                        t = self.make_string();
                    }
                }
            }
            if (a >= 4i32) {
                self.geq_define(u, set_font, null_font);
            } else {
                self.eq_define(u, set_font, null_font);
            }
            self.scan_optional_equals();
            self.scan_file_name();
            // §1437
            self.name_in_progress = true;
            if self.scan_keyword(1675i32) {
                // §1438
                {
                    self.scan_dimen(false, false, false);
                    s = self.cur_val;
                    if ((s <= 0i32) || (s >= 134217728i32)) {
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
                                self.print(1677i32);
                            }
                            self.print_scaled(s);
                            self.print(1678i32);
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1679i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1680i32;
                            }
                            self.error();
                            s = (10i32).wrapping_mul(unity);
                        }
                    }
                }
            } else {
                // §1437
                if self.scan_keyword(1676i32) {
                    {
                        self.scan_int();
                        s = (self.cur_val).wrapping_neg();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32768i32)) {
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
                                    self.print(645i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 646i32;
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
            // §1439
            {
                let __for_end_3 = self.font_ptr;
                f = 1i32;
                while f <= __for_end_3 {
                    if (self.str_eq_str(self.font_name[crate::ix::U((f) as usize)], self.cur_name) && self.str_eq_str(self.font_area[crate::ix::U((f) as usize)], self.cur_area)) {
                        {
                            if (self.pdf_font_step[crate::ix::U((f) as usize)] == 0i32) {
                                {
                                    if (s > 0i32) {
                                        {
                                            if (s == self.font_size[crate::ix::U((f) as usize)]) {
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    } else {
                                        {
                                            self.arith_error = false;
                                            if (self.font_size[crate::ix::U((f) as usize)] == self.xn_over_d(self.font_dsize[crate::ix::U((f) as usize)], (s).wrapping_neg(), 1000i32)) {
                                                if (!self.arith_error) {
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    f = f.wrapping_add(1);
                }
            }
            // §1436
            f = self.read_font_info(u, self.cur_name, self.cur_area, s);
        }
        if (a >= 4i32) {
            self.geq_define(u, set_font, f);
        } else {
            self.eq_define(u, set_font, f);
        }
        { let __v1956 = __av_eqtb[crate::ix::U(((u) - 1) as usize)]; __av_eqtb[crate::ix::U((((font_id_base).wrapping_add(f)) - 1) as usize)] = __v1956; }
        self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].set_rh(t);
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1444
    pub fn new_interaction(&mut self) {
        self.print_ln();
        self.interaction = self.cur_chr;
        if (self.interaction == batch_mode) {
            self.kpse_make_tex_discard_errors = true;
        } else {
            self.kpse_make_tex_discard_errors = false;
        }
        self.kpse_set_make_tex_discard_errors(self.kpse_make_tex_discard_errors);
        // §75
        if (self.interaction == batch_mode) {
            self.selector = no_print;
        } else {
            self.selector = term_only;
        }
        // §1444
        if self.log_opened {
            self.selector = (self.selector).wrapping_add(2i32);
        }
    }

    /// If the user says, e.g., `\.{\\global\\global}', the redundancy is
    /// silently accepted.
    /// @<Declare act...
    // §1390
    pub fn prefixed_command(&mut self) {
        let mut a: small_number = 0; // §1390
        let mut f: internal_font_number = 0; // §1390
        let mut j: halfword = 0; // §1390
        let mut k: font_index = 0; // §1390
        let mut p: halfword = 0; // §1390
        let mut q: halfword = 0; // §1390
        let mut r: halfword = 0; // §1390
        let mut n: i32 = 0; // §1390
        let mut e: bool = false; // §1390
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            'l_done_f: {
                a = 0i32;
                while (self.cur_cmd == prefix) {
                    {
                        if (!((((a / self.cur_chr)) % 2) != 0)) {
                            a = (a).wrapping_add(self.cur_chr);
                        }
                        // §431
                        loop {
                            self.get_x_token();
                            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                        }
                        // §1390
                        if (self.cur_cmd <= max_non_prefixed_command) {
                            // §1391
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
                                    self.print(1609i32);
                                }
                                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                self.print_char(39i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1610i32;
                                }
                                if (self.eTeX_mode == 1i32) {
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1611i32;
                                }
                                self.back_error();
                                break 'l_exit_f;
                            }
                        }
                        // §1390
                        if (__av_eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() > 2i32) {
                            if (self.eTeX_mode == 1i32) {
                                self.show_cur_cmd_chr();
                            }
                        }
                    }
                }
                // §1392
                if (a >= 8i32) {
                    {
                        j = protected_token;
                        a = (a).wrapping_sub(8i32);
                    }
                } else {
                    j = 0i32;
                }
                if ((self.cur_cmd != def) && (((a % 4i32) != 0i32) || (j != 0i32))) {
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
                            self.print(797i32);
                        }
                        self.print_esc(1601i32);
                        self.print(1612i32);
                        self.print_esc(1602i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1613i32;
                        }
                        if (self.eTeX_mode == 1i32) {
                            {
                                self.help_line[crate::ix::U((0i32) as usize)] = 1614i32;
                                self.print(1612i32);
                                self.print_esc(1615i32);
                            }
                        }
                        self.print(1616i32);
                        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                        self.print_char(39i32);
                        self.error();
                    }
                }
                // §1393
                if (__av_eqtb[crate::ix::U(((29320i32) - 1) as usize)].int() != 0i32) {
                    if (__av_eqtb[crate::ix::U(((29320i32) - 1) as usize)].int() < 0i32) {
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
                // §1390
                if self.intr_rec_on {
                    self.flashtex_intr_command();
                }
                if self.bm_rec_on {
                    self.flashtex_bm_command();
                }
                match self.cur_cmd {
                    set_font => {
                        // §1396
                        if (a >= 4i32) {
                            self.geq_define(cur_font_loc, data, self.cur_chr);
                        } else {
                            self.eq_define(cur_font_loc, data, self.cur_chr);
                        }
                    }
                    def => {
                        // §1397
                        {
                            if (((((self.cur_chr) % 2) != 0) && (!(a >= 4i32))) && (__av_eqtb[crate::ix::U(((29320i32) - 1) as usize)].int() >= 0i32)) {
                                a = (a).wrapping_add(4i32);
                            }
                            e = (self.cur_chr >= 2i32);
                            self.get_r_token();
                            p = self.cur_cs;
                            self.dg_def_begin();
                            q = self.scan_toks(true, e);
                            if (j != 0i32) {
                                {
                                    q = self.get_avail();
                                    __av_mem[crate::ix::U((q) as usize)].set_hh_lh(j);
                                    { let __v1957 = __av_mem[crate::ix::U((self.def_ref) as usize)].hh().rh(); __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v1957); }
                                    { let __ix1958 = self.def_ref; __av_mem[crate::ix::U((__ix1958) as usize)].set_hh_rh(q); }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(p, (call).wrapping_add((a % 4i32)), self.def_ref);
                            } else {
                                self.eq_define(p, (call).wrapping_add((a % 4i32)), self.def_ref);
                            }
                            self.dg_def(self.def_ref);
                        }
                    }
                    let_ => {
                        // §1400
                        if (self.cur_chr == 11i32) {
                        } else {
                            if (self.cur_chr == 10i32) {
                                {
                                    self.selector = term_and_log;
                                    self.get_token();
                                    self.mubyte_stoken = self.cur_tok;
                                    if (self.cur_tok <= cs_token_flag) {
                                        self.mubyte_stoken = (self.cur_tok % 256i32);
                                    }
                                    self.mubyte_prefix = 60i32;
                                    self.mubyte_relax = false;
                                    self.mubyte_tablein = true;
                                    self.mubyte_tableout = true;
                                    self.get_x_token();
                                    if (self.cur_cmd == spacer) {
                                        self.get_x_token();
                                    }
                                    if (self.cur_cmd == sub_mark) {
                                        {
                                            self.mubyte_tableout = false;
                                            self.get_x_token();
                                            if (self.cur_cmd == sub_mark) {
                                                {
                                                    self.mubyte_tableout = true;
                                                    self.mubyte_tablein = false;
                                                    self.get_x_token();
                                                }
                                            }
                                        }
                                    } else {
                                        if ((self.mubyte_stoken > cs_token_flag) && (self.cur_cmd == mac_param)) {
                                            {
                                                self.mubyte_tableout = false;
                                                self.scan_int();
                                                self.mubyte_prefix = self.cur_val;
                                                self.get_x_token();
                                                if (self.mubyte_prefix > 50i32) {
                                                    self.mubyte_prefix = 52i32;
                                                }
                                                if (self.mubyte_prefix <= 0i32) {
                                                    self.mubyte_prefix = 51i32;
                                                }
                                            }
                                        } else {
                                            if ((self.mubyte_stoken > cs_token_flag) && (self.cur_cmd == relax)) {
                                                {
                                                    self.mubyte_tableout = true;
                                                    self.mubyte_tablein = false;
                                                    self.mubyte_relax = true;
                                                    self.get_x_token();
                                                }
                                            }
                                        }
                                    }
                                    r = self.get_avail();
                                    p = r;
                                    while (self.cur_cs == 0i32) {
                                        {
                                            {
                                                self.dl_list_token_begin();
                                                q = self.get_avail();
                                                self.dl_token_end();
                                                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                { let __v1959 = self.cur_tok; __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v1959); }
                                                p = q;
                                            }
                                            self.get_x_token();
                                        }
                                    }
                                    if ((self.cur_cmd != end_cs_name) || (self.cur_chr != 10i32)) {
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
                                                self.print(723i32);
                                            }
                                            self.print_esc(601i32);
                                            self.print(724i32);
                                            {
                                                self.help_ptr = 2i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 725i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 1628i32;
                                            }
                                            self.back_error();
                                        }
                                    }
                                    p = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                                    if ((p == null) && self.mubyte_tablein) {
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
                                                self.print(1629i32);
                                            }
                                            self.print_esc(1626i32);
                                            self.print(1630i32);
                                            {
                                                self.help_ptr = 2i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 1631i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 1632i32;
                                            }
                                            self.error();
                                        }
                                    } else {
                                        {
                                            while (p != null) {
                                                {
                                                    {
                                                        { let __ix1960 = self.pool_ptr; let __v1961 = (__av_mem[crate::ix::U((p) as usize)].hh().lh() % 256i32); self.str_pool[crate::ix::U((__ix1960) as usize)] = __v1961; }
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                }
                                            }
                                            self.flush_list(r);
                                            if (((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(1i32) == self.pool_ptr) && (self.str_pool[crate::ix::U(((self.pool_ptr).wrapping_sub(1i32)) as usize)] == self.mubyte_stoken)) {
                                                {
                                                    if ((self.mubyte_read[crate::ix::U((self.mubyte_stoken) as usize)] != null) && self.mubyte_tablein) {
                                                        self.dispose_munode(self.mubyte_read[crate::ix::U((self.mubyte_stoken) as usize)]);
                                                    }
                                                    if self.mubyte_tablein {
                                                        self.mubyte_read[crate::ix::U((self.mubyte_stoken) as usize)] = null;
                                                    }
                                                    if self.mubyte_tableout {
                                                        self.mubyte_write[crate::ix::U((self.mubyte_stoken) as usize)] = 0i32;
                                                    }
                                                    self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                                                }
                                            } else {
                                                {
                                                    if self.mubyte_tablein {
                                                        self.mubyte_update();
                                                    }
                                                    if self.mubyte_tableout {
                                                        {
                                                            if (self.mubyte_stoken > cs_token_flag) {
                                                                {
                                                                    self.dispose_mutableout((self.mubyte_stoken).wrapping_sub(4095i32));
                                                                    if ((self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) || self.mubyte_relax) {
                                                                        {
                                                                            r = self.mubyte_cswrite[crate::ix::U((((self.mubyte_stoken).wrapping_sub(4095i32) % 128i32)) as usize)];
                                                                            p = self.get_avail();
                                                                            self.mubyte_cswrite[crate::ix::U((((self.mubyte_stoken).wrapping_sub(4095i32) % 128i32)) as usize)] = p;
                                                                            { let __v1962 = (self.mubyte_stoken).wrapping_sub(4095i32); __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v1962); }
                                                                            { let __v1963 = self.get_avail(); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v1963); }
                                                                            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                                            if self.mubyte_relax {
                                                                                {
                                                                                    __av_mem[crate::ix::U((p) as usize)].set_hh_lh(0i32);
                                                                                    self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                                                                                }
                                                                            } else {
                                                                                { let __v1964 = self.slow_make_string(); __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v1964); }
                                                                            }
                                                                            __av_mem[crate::ix::U((p) as usize)].set_hh_rh(r);
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.str_start[crate::ix::U((self.str_ptr) as usize)] == self.pool_ptr) {
                                                                        self.mubyte_write[crate::ix::U((self.mubyte_stoken) as usize)] = 0i32;
                                                                    } else {
                                                                        { let __ix1965 = self.mubyte_stoken; let __v1966 = self.slow_make_string(); self.mubyte_write[crate::ix::U((__ix1965) as usize)] = __v1966; }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    n = self.cur_chr;
                                    self.get_r_token();
                                    p = self.cur_cs;
                                    if (n == normal) {
                                        {
                                            loop {
                                                self.get_token();
                                                if (self.cur_cmd != spacer) { break; }
                                            }
                                            if (self.cur_tok == 3133i32) {
                                                {
                                                    self.get_token();
                                                    if (self.cur_cmd == spacer) {
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
                                    if (self.cur_cmd >= call) {
                                        { let __ix1967 = self.cur_chr; let __v1968 = (__av_mem[crate::ix::U((self.cur_chr) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1967) as usize)].set_hh_lh(__v1968); }
                                    } else {
                                        if ((self.cur_cmd == register) || (self.cur_cmd == toks_register)) {
                                            if ((self.cur_chr < mem_bot) || (self.cur_chr > lo_mem_stat_max)) {
                                                { let __ix1969 = (self.cur_chr).wrapping_add(1i32); let __v1970 = (__av_mem[crate::ix::U(((self.cur_chr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1969) as usize)].set_hh_lh(__v1970); }
                                            }
                                        }
                                    }
                                    if (a >= 4i32) {
                                        self.geq_define(p, self.cur_cmd, self.cur_chr);
                                    } else {
                                        self.eq_define(p, self.cur_cmd, self.cur_chr);
                                    }
                                }
                            }
                        }
                    }
                    shorthand_def => {
                        // §1403
                        if (self.cur_chr == char_sub_def_code) {
                            {
                                self.scan_char_num();
                                p = (char_sub_code_base).wrapping_add(self.cur_val);
                                self.scan_optional_equals();
                                self.scan_char_num();
                                n = self.cur_val;
                                self.scan_char_num();
                                if (__av_eqtb[crate::ix::U(((29334i32) - 1) as usize)].int() > 0i32) {
                                    {
                                        self.begin_diagnostic();
                                        self.print_nl(1641i32);
                                        self.print((p).wrapping_sub(29021i32));
                                        self.print(1642i32);
                                        self.print(n);
                                        self.print_char(32i32);
                                        self.print(self.cur_val);
                                        self.end_diagnostic(false);
                                    }
                                }
                                n = ((n).wrapping_mul(256i32)).wrapping_add(self.cur_val);
                                if (a >= 4i32) {
                                    self.geq_define(p, data, n);
                                } else {
                                    self.eq_define(p, data, n);
                                }
                                if ((p).wrapping_sub(29021i32) < __av_eqtb[crate::ix::U(((29332i32) - 1) as usize)].int()) {
                                    if (a >= 4i32) {
                                        self.geq_word_define(29332i32, (p).wrapping_sub(29021i32));
                                    } else {
                                        self.eq_word_define(29332i32, (p).wrapping_sub(29021i32));
                                    }
                                }
                                if ((p).wrapping_sub(29021i32) > __av_eqtb[crate::ix::U(((29333i32) - 1) as usize)].int()) {
                                    if (a >= 4i32) {
                                        self.geq_word_define(29333i32, (p).wrapping_sub(29021i32));
                                    } else {
                                        self.eq_word_define(29333i32, (p).wrapping_sub(29021i32));
                                    }
                                }
                            }
                        } else {
                            {
                                n = self.cur_chr;
                                self.get_r_token();
                                p = self.cur_cs;
                                if (a >= 4i32) {
                                    self.geq_define(p, relax, 256i32);
                                } else {
                                    self.eq_define(p, relax, 256i32);
                                }
                                self.scan_optional_equals();
                                match n {
                                    char_def_code => {
                                        {
                                            self.scan_char_num();
                                            if (a >= 4i32) {
                                                self.geq_define(p, char_given, self.cur_val);
                                            } else {
                                                self.eq_define(p, char_given, self.cur_val);
                                            }
                                        }
                                    }
                                    math_char_def_code => {
                                        {
                                            self.scan_fifteen_bit_int();
                                            if (a >= 4i32) {
                                                self.geq_define(p, math_given, self.cur_val);
                                            } else {
                                                self.eq_define(p, math_given, self.cur_val);
                                            }
                                        }
                                    }
                                    _ => {
                                        {
                                            self.scan_register_num();
                                            if (self.cur_val > 255i32) {
                                                {
                                                    j = (n).wrapping_sub(2i32);
                                                    if (j > mu_val) {
                                                        j = tok_val;
                                                    }
                                                    self.find_sa_element(j, self.cur_val, true);
                                                    { let __ix1971 = (self.cur_ptr).wrapping_add(1i32); let __v1972 = (__av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix1971) as usize)].set_hh_lh(__v1972); }
                                                    if (j == tok_val) {
                                                        j = toks_register;
                                                    } else {
                                                        j = register;
                                                    }
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, j, self.cur_ptr);
                                                    } else {
                                                        self.eq_define(p, j, self.cur_ptr);
                                                    }
                                                }
                                            } else {
                                                match n {
                                                    count_def_code => {
                                                        if (a >= 4i32) {
                                                            self.geq_define(p, assign_int, (count_base).wrapping_add(self.cur_val));
                                                        } else {
                                                            self.eq_define(p, assign_int, (count_base).wrapping_add(self.cur_val));
                                                        }
                                                    }
                                                    dimen_def_code => {
                                                        if (a >= 4i32) {
                                                            self.geq_define(p, assign_dimen, (scaled_base).wrapping_add(self.cur_val));
                                                        } else {
                                                            self.eq_define(p, assign_dimen, (scaled_base).wrapping_add(self.cur_val));
                                                        }
                                                    }
                                                    skip_def_code => {
                                                        if (a >= 4i32) {
                                                            self.geq_define(p, assign_glue, (skip_base).wrapping_add(self.cur_val));
                                                        } else {
                                                            self.eq_define(p, assign_glue, (skip_base).wrapping_add(self.cur_val));
                                                        }
                                                    }
                                                    mu_skip_def_code => {
                                                        if (a >= 4i32) {
                                                            self.geq_define(p, assign_mu_glue, (mu_skip_base).wrapping_add(self.cur_val));
                                                        } else {
                                                            self.eq_define(p, assign_mu_glue, (mu_skip_base).wrapping_add(self.cur_val));
                                                        }
                                                    }
                                                    toks_def_code => {
                                                        if (a >= 4i32) {
                                                            self.geq_define(p, assign_toks, (toks_base).wrapping_add(self.cur_val));
                                                        } else {
                                                            self.eq_define(p, assign_toks, (toks_base).wrapping_add(self.cur_val));
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
                    }
                    read_to_cs => {
                        // §1404
                        {
                            j = self.cur_chr;
                            self.scan_int();
                            n = self.cur_val;
                            if (!self.scan_keyword(1258i32)) {
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
                                        self.print(1499i32);
                                    }
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 1643i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1644i32;
                                    }
                                    self.error();
                                }
                            }
                            self.get_r_token();
                            p = self.cur_cs;
                            self.read_toks(n, p, j);
                            if (a >= 4i32) {
                                self.geq_define(p, call, self.cur_val);
                            } else {
                                self.eq_define(p, call, self.cur_val);
                            }
                        }
                    }
                    toks_register | assign_toks => {
                        // §1405
                        {
                            q = self.cur_cs;
                            e = false;
                            if (self.cur_cmd == toks_register) {
                                if (self.cur_chr == mem_bot) {
                                    {
                                        self.scan_register_num();
                                        if (self.cur_val > 255i32) {
                                            {
                                                self.find_sa_element(tok_val, self.cur_val, true);
                                                self.cur_chr = self.cur_ptr;
                                                e = true;
                                            }
                                        } else {
                                            self.cur_chr = (toks_base).wrapping_add(self.cur_val);
                                        }
                                    }
                                } else {
                                    e = true;
                                }
                            }
                            p = self.cur_chr;
                            self.scan_optional_equals();
                            // §431
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                            }
                            // §1405
                            if (self.cur_cmd != left_brace) {
                                // §1406
                                if ((self.cur_cmd == toks_register) || (self.cur_cmd == assign_toks)) {
                                    {
                                        if (self.cur_cmd == toks_register) {
                                            if (self.cur_chr == mem_bot) {
                                                {
                                                    self.scan_register_num();
                                                    if (self.cur_val < 256i32) {
                                                        q = __av_eqtb[crate::ix::U((((toks_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                                    } else {
                                                        {
                                                            self.find_sa_element(tok_val, self.cur_val, false);
                                                            if (self.cur_ptr == null) {
                                                                q = null;
                                                            } else {
                                                                q = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                q = __av_mem[crate::ix::U(((self.cur_chr).wrapping_add(1i32)) as usize)].hh().rh();
                                            }
                                        } else {
                                            q = __av_eqtb[crate::ix::U(((self.cur_chr) - 1) as usize)].hh().rh();
                                        }
                                        if self.intr_rec_on {
                                            self.flashtex_intr_abort(2i32);
                                        }
                                        if (q == null) {
                                            if e {
                                                if (a >= 4i32) {
                                                    self.gsa_def(p, null);
                                                } else {
                                                    self.sa_def(p, null);
                                                }
                                            } else {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, undefined_cs, null);
                                                } else {
                                                    self.eq_define(p, undefined_cs, null);
                                                }
                                            }
                                        } else {
                                            {
                                                { let __v1973 = (__av_mem[crate::ix::U((q) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_lh(__v1973); }
                                                if e {
                                                    if (a >= 4i32) {
                                                        self.gsa_def(p, q);
                                                    } else {
                                                        self.sa_def(p, q);
                                                    }
                                                } else {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, call, q);
                                                    } else {
                                                        self.eq_define(p, call, q);
                                                    }
                                                }
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                            }
                            // §1405
                            self.back_input();
                            self.cur_cs = q;
                            q = self.scan_toks(false, false);
                            if (__av_mem[crate::ix::U((self.def_ref) as usize)].hh().rh() == null) {
                                {
                                    if e {
                                        if (a >= 4i32) {
                                            self.gsa_def(p, null);
                                        } else {
                                            self.sa_def(p, null);
                                        }
                                    } else {
                                        if (a >= 4i32) {
                                            self.geq_define(p, undefined_cs, null);
                                        } else {
                                            self.eq_define(p, undefined_cs, null);
                                        }
                                    }
                                    {
                                        { let __ix1974 = self.def_ref; let __v1975 = self.avail; __av_mem[crate::ix::U((__ix1974) as usize)].set_hh_rh(__v1975); }
                                        self.avail = self.def_ref;
                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                    }
                                }
                            } else {
                                {
                                    if ((p == output_routine_loc) && (!e)) {
                                        {
                                            { let __v1976 = self.get_avail(); __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v1976); }
                                            q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                            __av_mem[crate::ix::U((q) as usize)].set_hh_lh(637i32);
                                            q = self.get_avail();
                                            __av_mem[crate::ix::U((q) as usize)].set_hh_lh(379i32);
                                            { let __v1977 = __av_mem[crate::ix::U((self.def_ref) as usize)].hh().rh(); __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v1977); }
                                            { let __ix1978 = self.def_ref; __av_mem[crate::ix::U((__ix1978) as usize)].set_hh_rh(q); }
                                        }
                                    }
                                    if e {
                                        if (a >= 4i32) {
                                            self.gsa_def(p, self.def_ref);
                                        } else {
                                            self.sa_def(p, self.def_ref);
                                        }
                                    } else {
                                        if (a >= 4i32) {
                                            self.geq_define(p, call, self.def_ref);
                                        } else {
                                            self.eq_define(p, call, self.def_ref);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    assign_int => {
                        // §1407
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
                    assign_dimen => {
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
                    assign_glue | assign_mu_glue => {
                        {
                            p = self.cur_chr;
                            n = self.cur_cmd;
                            self.scan_optional_equals();
                            if (n == assign_mu_glue) {
                                self.scan_glue(mu_val);
                            } else {
                                self.scan_glue(glue_val);
                            }
                            self.trap_zero_glue();
                            if (a >= 4i32) {
                                self.geq_define(p, glue_ref, self.cur_val);
                            } else {
                                self.eq_define(p, glue_ref, self.cur_val);
                            }
                        }
                    }
                    def_code => {
                        // §1411
                        {
                            // §1412
                            if (self.cur_chr == cat_code_base) {
                                n = max_char_code;
                            } else {
                                if (self.cur_chr == math_code_base) {
                                    n = 32768i32;
                                } else {
                                    if (self.cur_chr == sf_code_base) {
                                        n = 32767i32;
                                    } else {
                                        if (self.cur_chr == del_code_base) {
                                            n = 16777215i32;
                                        } else {
                                            n = 255i32;
                                        }
                                    }
                                }
                            }
                            // §1411
                            p = self.cur_chr;
                            self.scan_char_num();
                            if (p == xord_code_base) {
                                p = self.cur_val;
                            } else {
                                if (p == xchr_code_base) {
                                    p = (self.cur_val).wrapping_add(256i32);
                                } else {
                                    if (p == xprn_code_base) {
                                        p = (self.cur_val).wrapping_add(512i32);
                                    } else {
                                        p = (p).wrapping_add(self.cur_val);
                                    }
                                }
                            }
                            self.scan_optional_equals();
                            self.scan_int();
                            if (((self.cur_val < 0i32) && (p < del_code_base)) || (self.cur_val > n)) {
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
                                        self.print(1648i32);
                                    }
                                    self.print_int(((self.cur_val) as i64));
                                    if (p < del_code_base) {
                                        self.print(1649i32);
                                    } else {
                                        self.print(1650i32);
                                    }
                                    self.print_int(((n) as i64));
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1651i32;
                                    }
                                    self.error();
                                    self.cur_val = 0i32;
                                }
                            }
                            if (p < 256i32) {
                                { let __v1979 = self.cur_val; self.xord[crate::ix::U((p) as usize)] = __v1979; }
                            } else {
                                if (p < 512i32) {
                                    { let __v1980 = ((self.cur_val) as u8); self.xchr[crate::ix::U(((p).wrapping_sub(256i32)) as usize)] = __v1980; }
                                } else {
                                    if (p < 768i32) {
                                        { let __v1981 = self.cur_val; self.xprn[crate::ix::U(((p).wrapping_sub(512i32)) as usize)] = __v1981; }
                                    } else {
                                        if (p < math_code_base) {
                                            if (a >= 4i32) {
                                                self.geq_define(p, data, self.cur_val);
                                            } else {
                                                self.eq_define(p, data, self.cur_val);
                                            }
                                        } else {
                                            if (p < del_code_base) {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, data, self.cur_val);
                                                } else {
                                                    self.eq_define(p, data, self.cur_val);
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
                            }
                        }
                    }
                    def_family => {
                        // §1413
                        {
                            p = self.cur_chr;
                            self.scan_four_bit_int();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_font_ident();
                            if (a >= 4i32) {
                                self.geq_define(p, data, self.cur_val);
                            } else {
                                self.eq_define(p, data, self.cur_val);
                            }
                        }
                    }
                    register | advance | multiply | divide => {
                        // §1414
                        self.do_register_command(a);
                    }
                    set_box => {
                        // §1420
                        {
                            self.scan_register_num();
                            if (a >= 4i32) {
                                n = (global_box_flag).wrapping_add(self.cur_val);
                            } else {
                                n = (box_flag).wrapping_add(self.cur_val);
                            }
                            self.scan_optional_equals();
                            if self.set_box_allowed {
                                self.scan_box(n);
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
                                        self.print(792i32);
                                    }
                                    self.print_esc(625i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 1657i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 1658i32;
                                    }
                                    self.error();
                                }
                            }
                        }
                    }
                    set_aux => {
                        // §1421
                        self.alter_aux();
                    }
                    set_prev_graf => {
                        self.alter_prev_graf();
                    }
                    set_page_dimen => {
                        self.alter_page_so_far();
                    }
                    set_page_int => {
                        self.alter_integer();
                    }
                    set_box_dimen => {
                        self.alter_box_dimen();
                    }
                    set_shape => {
                        // §1427
                        {
                            q = self.cur_chr;
                            self.scan_optional_equals();
                            self.scan_int();
                            n = self.cur_val;
                            if (n <= 0i32) {
                                p = null;
                            } else {
                                if (q > par_shape_loc) {
                                    {
                                        n = ((self.cur_val / 2i32)).wrapping_add(1i32);
                                        p = self.get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        __av_mem[crate::ix::U((p) as usize)].set_hh_lh(n);
                                        n = self.cur_val;
                                        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(n);
                                        {
                                            let __for_end_10 = ((p).wrapping_add(n)).wrapping_add(1i32);
                                            j = (p).wrapping_add(2i32);
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_int();
                                                    { let __v1982 = self.cur_val; __av_mem[crate::ix::U((j) as usize)].set_int(__v1982); }
                                                }
                                                j = j.wrapping_add(1);
                                            }
                                        }
                                        if (!(((n) % 2) != 0)) {
                                            __av_mem[crate::ix::U((((p).wrapping_add(n)).wrapping_add(2i32)) as usize)].set_int(0i32);
                                        }
                                    }
                                } else {
                                    {
                                        p = self.get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        __av_mem[crate::ix::U((p) as usize)].set_hh_lh(n);
                                        {
                                            let __for_end_10 = n;
                                            j = 1i32;
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_dimen(false, false, false);
                                                    { let __v1983 = self.cur_val; __av_mem[crate::ix::U((((p).wrapping_add((2i32).wrapping_mul(j))).wrapping_sub(1i32)) as usize)].set_int(__v1983); }
                                                    self.scan_dimen(false, false, false);
                                                    { let __v1984 = self.cur_val; __av_mem[crate::ix::U(((p).wrapping_add((2i32).wrapping_mul(j))) as usize)].set_int(__v1984); }
                                                }
                                                j = j.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(q, shape_ref, p);
                            } else {
                                self.eq_define(q, shape_ref, p);
                            }
                        }
                    }
                    hyph_data => {
                        // §1431
                        if (self.cur_chr == 1i32) {
                            {
                                self.new_patterns();
                                break 'l_done_f;
                                {
                                    self.dg_mark();
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1662i32);
                                }
                                self.help_ptr = 0i32;
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_cmd == right_brace) { break; }
                                }
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                if self.bm_on {
                                    self.flashtex_bm_hyph_changed();
                                }
                                self.new_hyph_exceptions();
                                break 'l_done_f;
                            }
                        }
                    }
                    assign_font_dimen => {
                        // §1432
                        {
                            self.find_font_dimen(true);
                            k = self.cur_val;
                            self.scan_optional_equals();
                            self.scan_dimen(false, false, false);
                            { let __v1985 = self.cur_val; self.font_info[crate::ix::U((k) as usize)].set_int(__v1985); }
                            if self.bm_on {
                                self.flashtex_bm_font_set((1i32).wrapping_neg(), k, 0i32, self.cur_val);
                            }
                        }
                    }
                    assign_font_int => {
                        {
                            n = self.cur_chr;
                            self.scan_font_ident();
                            f = self.cur_val;
                            if (n == no_lig_code) {
                                {
                                    self.set_no_ligatures(f);
                                    if self.bm_on {
                                        self.flashtex_bm_font_set(n, f, 0i32, 0i32);
                                    }
                                }
                            } else {
                                if (n < lp_code_base) {
                                    {
                                        self.scan_optional_equals();
                                        self.scan_int();
                                        if (n == 0i32) {
                                            { let __v1986 = self.cur_val; self.hyphen_char[crate::ix::U((f) as usize)] = __v1986; }
                                        } else {
                                            { let __v1987 = self.cur_val; self.skew_char[crate::ix::U((f) as usize)] = __v1987; }
                                        }
                                        if self.bm_on {
                                            self.flashtex_bm_font_set(n, f, 0i32, self.cur_val);
                                        }
                                    }
                                } else {
                                    {
                                        self.scan_char_num();
                                        p = self.cur_val;
                                        self.scan_optional_equals();
                                        self.scan_int();
                                        match n {
                                            lp_code_base => {
                                                self.set_lp_code(f, p, self.cur_val);
                                            }
                                            rp_code_base => {
                                                self.set_rp_code(f, p, self.cur_val);
                                            }
                                            ef_code_base => {
                                                self.set_ef_code(f, p, self.cur_val);
                                            }
                                            tag_code => {
                                                self.set_tag_code(f, p, self.cur_val);
                                            }
                                            kn_bs_code_base => {
                                                self.set_kn_bs_code(f, p, self.cur_val);
                                            }
                                            st_bs_code_base => {
                                                self.set_st_bs_code(f, p, self.cur_val);
                                            }
                                            sh_bs_code_base => {
                                                self.set_sh_bs_code(f, p, self.cur_val);
                                            }
                                            kn_bc_code_base => {
                                                self.set_kn_bc_code(f, p, self.cur_val);
                                            }
                                            kn_ac_code_base => {
                                                self.set_kn_ac_code(f, p, self.cur_val);
                                            }
                                            _ => {}
                                        }
                                        if self.bm_on {
                                            self.flashtex_bm_font_set(n, f, p, self.cur_val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    def_font => {
                        // §1435
                        self.new_font(a);
                    }
                    letterspace_font => {
                        self.new_letterspaced_font(a);
                    }
                    pdf_copy_font => {
                        self.make_font_copy(a);
                    }
                    set_interaction => {
                        // §1443
                        self.new_interaction();
                    }
                    _ => {
                        // §1390
                        self.confusion(1608i32);
                    }
                }
            }
            if (self.after_token != 0i32) {
                // §1448
                {
                    self.cur_tok = self.after_token;
                    self.back_input();
                    self.after_token = 0i32;
                }
            }
        }
        // §1390
    }

    /// Here is a procedure that might be called `Get the next non-blank non-relax
    /// non-call non-assignment token'.
    /// @<Declare act...
    // §1449
    pub fn do_assignments(&mut self) {
        'l_exit_f: {
            while true {
                {
                    // §431
                    loop {
                        self.get_x_token();
                        if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
                    }
                    // §1449
                    if (self.cur_cmd <= max_non_prefixed_command) {
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
    // §1454
    pub fn open_or_close_in(&mut self) {
        let mut c: i32 = 0; // §1454
        let mut n: i32 = 0; // §1454
        c = self.cur_chr;
        self.scan_four_bit_int();
        n = self.cur_val;
        if (self.read_open[crate::ix::U((n) as usize)] != closed) {
            {
                { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((n) as usize)]); let __r = self.a_close(&mut __f0); self.read_file[crate::ix::U((n) as usize)] = __f0; __r };
                self.read_open[crate::ix::U((n) as usize)] = closed;
            }
        }
        if (c != 0i32) {
            {
                self.scan_optional_equals();
                self.scan_file_name();
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                self.set_tex_input_type(false);
                if (self.kpse_in_name_ok() && { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((n) as usize)]); let __r = self.a_open_in(&mut __f0); self.read_file[crate::ix::U((n) as usize)] = __f0; __r }) {
                    self.read_open[crate::ix::U((n) as usize)] = just_open;
                }
            }
        }
    }

    /// @<Declare act...
    // §1458
    pub fn issue_message(&mut self) {
        let mut old_setting: i32 = 0; // §1458
        let mut c: i32 = 0; // §1458
        let mut s: str_number = 0; // §1458
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        c = self.cur_chr;
        { let __v1988 = self.scan_toks(false, true); __av_mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v1988); }
        old_setting = self.selector;
        self.selector = new_string;
        self.message_printing = true;
        self.active_noconvert = true;
        self.token_show(self.def_ref);
        self.message_printing = false;
        self.active_noconvert = false;
        self.selector = old_setting;
        self.flush_list(self.def_ref);
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        if (c == 0i32) {
            // §1459
            {
                if ((self.term_offset).wrapping_add((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)])) > (self.max_print_line).wrapping_sub(2i32)) {
                    self.print_ln();
                } else {
                    if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                        self.print_char(32i32);
                    }
                }
                self.print(s);
                crate::system::break_out(&mut self.term_out);
            }
        } else {
            // §1462
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
                    self.print(345i32);
                }
                self.print(s);
                if (__av_eqtb[crate::ix::U(((err_help_loc) - 1) as usize)].hh().rh() != null) {
                    self.use_err_help = true;
                } else {
                    if self.long_help_seen {
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1687i32;
                        }
                    } else {
                        {
                            if (self.interaction < error_stop_mode) {
                                self.long_help_seen = true;
                            }
                            {
                                self.help_ptr = 4i32;
                                self.help_line[crate::ix::U((3i32) as usize)] = 1688i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1689i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1690i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1691i32;
                            }
                        }
                    }
                }
                self.error();
                self.use_err_help = false;
            }
        }
        // §1458
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
        }
    }

    /// @<Declare act...
    // §1467
    pub fn shift_case(&mut self) {
        let mut b: halfword = 0; // §1467
        let mut p: halfword = 0; // §1467
        let mut t: halfword = 0; // §1467
        let mut c: eight_bits = 0; // §1467
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        b = self.cur_chr;
        p = self.scan_toks(false, false);
        p = __av_mem[crate::ix::U((self.def_ref) as usize)].hh().rh();
        while (p != null) {
            {
                // §1468
                t = __av_mem[crate::ix::U((p) as usize)].hh().lh();
                if (t < 4352i32) {
                    {
                        c = (t % 256i32);
                        if (__av_eqtb[crate::ix::U((((b).wrapping_add(c)) - 1) as usize)].hh().rh() != 0i32) {
                            { let __v1989 = ((t).wrapping_sub(c)).wrapping_add(__av_eqtb[crate::ix::U((((b).wrapping_add(c)) - 1) as usize)].hh().rh()); __av_mem[crate::ix::U((p) as usize)].set_hh_lh(__v1989); }
                        }
                    }
                }
                // §1467
                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        self.begin_token_list(__av_mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), backed_up);
        {
            { let __ix1990 = self.def_ref; let __v1991 = self.avail; __av_mem[crate::ix::U((__ix1990) as usize)].set_hh_rh(__v1991); }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
    }

    /// @<Declare act...
    // §1472
    pub fn show_whatever(&mut self) {
        let mut p: halfword = 0; // §1472
        let mut t: small_number = 0; // §1472
        let mut m: i32 = 0; // §1472
        let mut l: i32 = 0; // §1472
        let mut n: i32 = 0; // §1472
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_common_ending_f: {
            match self.cur_chr {
                show_lists_code => {
                    {
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1472
                        self.begin_diagnostic();
                        self.show_activities();
                    }
                }
                show_box_code => {
                    // §1475
                    {
                        self.scan_register_num();
                        {
                            if self.bm_rec_on {
                                self.flashtex_bm_box(self.cur_val);
                            }
                            if (self.cur_val < 256i32) {
                                p = __av_eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(box_val, self.cur_val, false);
                                    if (self.cur_ptr == null) {
                                        p = null;
                                    } else {
                                        p = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                        }
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1475
                        self.begin_diagnostic();
                        self.print_nl(1707i32);
                        self.print_int(((self.cur_val) as i64));
                        self.print_char(61i32);
                        if (p == null) {
                            self.print(431i32);
                        } else {
                            self.show_box(p);
                        }
                    }
                }
                show_code => {
                    // §1473
                    {
                        self.get_token();
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1473
                        if (self.interaction == error_stop_mode) {
                        }
                        self.print_nl(1703i32);
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
                show_groups => {
                    // §1678
                    {
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1678
                        self.begin_diagnostic();
                        self.show_save_groups();
                    }
                }
                show_ifs => {
                    // §1692
                    {
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1692
                        self.begin_diagnostic();
                        self.print_nl(345i32);
                        self.print_ln();
                        if (self.cond_ptr == null) {
                            {
                                self.print_nl(380i32);
                                self.print(2040i32);
                            }
                        } else {
                            {
                                p = self.cond_ptr;
                                n = 0i32;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (p == null) { break; }
                                }
                                p = self.cond_ptr;
                                t = self.cur_if;
                                l = self.if_line;
                                m = self.if_limit;
                                loop {
                                    self.print_nl(2041i32);
                                    self.print_int(((n) as i64));
                                    self.print(660i32);
                                    self.print_cmd_chr(if_test, t);
                                    if (m == fi_code) {
                                        self.print_esc(943i32);
                                    }
                                    if (l != 0i32) {
                                        {
                                            self.ls_print_unknown(l);
                                            self.print(2039i32);
                                            self.print_int(((l) as i64));
                                        }
                                    }
                                    n = (n).wrapping_sub(1i32);
                                    t = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                                    l = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                    m = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (p == null) { break; }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // §1476
                    {
                        p = self.the_toks();
                        // §1910
                        if (((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() >= 0i32) && (__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int() < no_print)) && self.write_open[crate::ix::U((__av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int()) as usize)]) {
                            self.selector = __av_eqtb[crate::ix::U(((29337i32) - 1) as usize)].int();
                        }
                        // §1476
                        if (self.interaction == error_stop_mode) {
                        }
                        self.print_nl(1703i32);
                        self.token_show(temp_head);
                        self.flush_list(__av_mem[crate::ix::U((temp_head) as usize)].hh().rh());
                        break 'l_common_ending_f;
                    }
                }
            }
            // §1477
            self.end_diagnostic(true);
            {
                self.dg_mark();
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(264i32);
                }
                self.print(1708i32);
            }
            if (self.selector == term_and_log) {
                if (__av_eqtb[crate::ix::U(((29306i32) - 1) as usize)].int() <= 0i32) {
                    {
                        self.selector = term_only;
                        self.print(1709i32);
                        self.selector = term_and_log;
                    }
                }
            }
        }
        // §1472
        if (self.selector < no_print) {
            {
                self.print_ln();
                // §75
                if (self.interaction == batch_mode) {
                    self.selector = no_print;
                } else {
                    self.selector = term_only;
                }
                // §1472
                if self.log_opened {
                    self.selector = (self.selector).wrapping_add(2i32);
                }
            }
        } else {
            {
                if (self.interaction < error_stop_mode) {
                    {
                        self.help_ptr = 0i32;
                        self.error_count = (self.error_count).wrapping_sub(1i32);
                    }
                } else {
                    if (__av_eqtb[crate::ix::U(((29306i32) - 1) as usize)].int() > 0i32) {
                        {
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1698i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1699i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1700i32;
                            }
                        }
                    } else {
                        {
                            {
                                self.help_ptr = 5i32;
                                self.help_line[crate::ix::U((4i32) as usize)] = 1698i32;
                                self.help_line[crate::ix::U((3i32) as usize)] = 1699i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1700i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1701i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1702i32;
                            }
                        }
                    }
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1481
    pub fn store_fmt_file(&mut self) {
        let mut j: i32 = 0; // §1481
        let mut k: i32 = 0; // §1481
        let mut l: i32 = 0; // §1481
        let mut p: halfword = 0; // §1481
        let mut q: halfword = 0; // §1481
        let mut x: i32 = 0; // §1481
        let mut w: four_quarters = four_quarters::default(); // §1481
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // §1483
        if (self.save_ptr != 0i32) {
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
                    self.print(1711i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1712i32;
                }
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
        }
        // §1509
        self.selector = new_string;
        self.print(1730i32);
        self.print(self.job_name);
        self.print_char(32i32);
        self.print_int(((__av_eqtb[crate::ix::U(((29300i32) - 1) as usize)].int()) as i64));
        self.print_char(46i32);
        self.print_int(((__av_eqtb[crate::ix::U(((29299i32) - 1) as usize)].int()) as i64));
        self.print_char(46i32);
        self.print_int(((__av_eqtb[crate::ix::U(((29298i32) - 1) as usize)].int()) as i64));
        self.print_char(41i32);
        if (self.interaction == batch_mode) {
            self.selector = log_only;
        } else {
            self.selector = term_and_log;
        }
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.format_ident = self.make_string();
        self.pack_job_name(format_extension);
        while (!{ let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_out(&mut __f0); self.fmt_file = __f0; __r }) {
            self.prompt_file_name(1731i32, format_extension);
        }
        self.print_nl(1732i32);
        { let __a1992_0 = { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_make_name_string(&mut __f0); self.fmt_file = __f0; __r }; self.slow_print(__a1992_0) };
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
        }
        self.print_nl(345i32);
        self.slow_print(self.format_ident);
        // §1486
        {
            self.fmt_file.buf.set_int(312141437i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1928
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1993 = self.xord[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v1993); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1994 = ((self.xchr[crate::ix::U((k) as usize)]) as i32); self.fmt_file.buf.set_int(__v1994); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1995 = self.xprn[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v1995); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        // §1655
        {
            { let __v1996 = self.eTeX_mode; self.fmt_file.buf.set_int(__v1996); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = (0i32).wrapping_neg();
            j = 0i32;
            while j <= __for_end_2 {
                __av_eqtb[crate::ix::U((((eTeX_state_base).wrapping_add(j)) - 1) as usize)].set_int(0i32);
                j = j.wrapping_add(1);
            }
        }
        // §1759
        while (self.pseudo_files != null) {
            self.pseudo_close();
        }
        // §1486
        {
            self.fmt_file.buf.set_int(mem_bot);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(mem_top);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(eqtb_size);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1997 = self.hash_high; self.fmt_file.buf.set_int(__v1997); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(hash_prime);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(hyph_size);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1908
        {
            self.fmt_file.buf.set_int(1296847960i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        if self.mltex_p {
            {
                self.fmt_file.buf.set_int(1i32);
                crate::system::put_word(&mut self.fmt_file);
            }
        } else {
            {
                self.fmt_file.buf.set_int(0i32);
                crate::system::put_word(&mut self.fmt_file);
            }
        }
        // §1875
        {
            self.fmt_file.buf.set_int(1162040408i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        if (!self.enctex_p) {
            {
                self.fmt_file.buf.set_int(0i32);
                crate::system::put_word(&mut self.fmt_file);
            }
        } else {
            {
                {
                    self.fmt_file.buf.set_int(1i32);
                    crate::system::put_word(&mut self.fmt_file);
                }
                {
                    let __for_end_4 = 255i32;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            { let __v1998 = self.mubyte_read[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v1998); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    let __for_end_4 = 255i32;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            { let __v1999 = self.mubyte_write[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v1999); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    let __for_end_4 = 127i32;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            { let __v2000 = self.mubyte_cswrite[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2000); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        k = k.wrapping_add(1);
                    }
                }
            }
        }
        // §1488
        {
            { let __v2001 = self.pool_ptr; self.fmt_file.buf.set_int(__v2001); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2002 = self.str_ptr; self.fmt_file.buf.set_int(__v2002); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.str_ptr;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v2003 = self.str_start[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2003); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        k = 0i32;
        while ((k).wrapping_add(4i32) < self.pool_ptr) {
            {
                { let __v2004 = self.str_pool[crate::ix::U((k) as usize)]; w.set_b0(__v2004); }
                { let __v2005 = self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)]; w.set_b1(__v2005); }
                { let __v2006 = self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)]; w.set_b2(__v2006); }
                { let __v2007 = self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)]; w.set_b3(__v2007); }
                {
                    self.fmt_file.buf.set_qqqq(w);
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = (k).wrapping_add(4i32);
            }
        }
        k = (self.pool_ptr).wrapping_sub(4i32);
        { let __v2008 = self.str_pool[crate::ix::U((k) as usize)]; w.set_b0(__v2008); }
        { let __v2009 = self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)]; w.set_b1(__v2009); }
        { let __v2010 = self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)]; w.set_b2(__v2010); }
        { let __v2011 = self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)]; w.set_b3(__v2011); }
        {
            self.fmt_file.buf.set_qqqq(w);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((self.str_ptr) as i64));
        self.print(1713i32);
        self.print_int(((self.pool_ptr) as i64));
        // §1490
        self.sort_avail();
        self.var_used = 0i32;
        {
            { let __v2012 = self.lo_mem_max; self.fmt_file.buf.set_int(__v2012); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2013 = self.rover; self.fmt_file.buf.set_int(__v2013); }
            crate::system::put_word(&mut self.fmt_file);
        }
        if (self.eTeX_mode == 1i32) {
            {
                let __for_end_3 = tok_val;
                k = int_val;
                while k <= __for_end_3 {
                    {
                        { let __v2014 = self.sa_root[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2014); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
        }
        p = mem_bot;
        q = self.rover;
        x = 0i32;
        loop {
            {
                let __for_end_3 = (q).wrapping_add(1i32);
                k = p;
                while k <= __for_end_3 {
                    {
                        self.fmt_file.buf = __av_mem[crate::ix::U((k) as usize)];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
            x = (((x).wrapping_add(q)).wrapping_add(2i32)).wrapping_sub(p);
            self.var_used = ((self.var_used).wrapping_add(q)).wrapping_sub(p);
            p = (q).wrapping_add(__av_mem[crate::ix::U((q) as usize)].hh().lh());
            q = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
            if (q == self.rover) { break; }
        }
        self.var_used = ((self.var_used).wrapping_add(self.lo_mem_max)).wrapping_sub(p);
        self.dyn_used = ((self.mem_end).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        {
            let __for_end_2 = self.lo_mem_max;
            k = p;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = __av_mem[crate::ix::U((k) as usize)];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.lo_mem_max)).wrapping_add(1i32)).wrapping_sub(p);
        {
            { let __v2015 = self.hi_mem_min; self.fmt_file.buf.set_int(__v2015); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2016 = self.avail; self.fmt_file.buf.set_int(__v2016); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.mem_end;
            k = self.hi_mem_min;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = __av_mem[crate::ix::U((k) as usize)];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.mem_end)).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        p = self.avail;
        while (p != null) {
            {
                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        {
            { let __v2017 = self.var_used; self.fmt_file.buf.set_int(__v2017); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2018 = self.dyn_used; self.fmt_file.buf.set_int(__v2018); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((x) as i64));
        self.print(1714i32);
        self.print_int(((self.var_used) as i64));
        self.print_char(38i32);
        self.print_int(((self.dyn_used) as i64));
        // §1494
        k = active_base;
        loop {
            'l_done1_f: {
                'l_found1_f: {
                    j = k;
                    while (j < 29276i32) {
                        {
                            if (((__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().rh() == __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().rh()) && (__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().b0() == __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().b0())) && (__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().b1() == __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().b1())) {
                                break 'l_found1_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = int_base;
                    break 'l_done1_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < 29276i32) {
                    {
                        if (((__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().rh() != __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().rh()) || (__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().b0() != __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().b0())) || (__av_eqtb[crate::ix::U(((j) - 1) as usize)].hh().b1() != __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].hh().b1())) {
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
                        self.fmt_file.buf = __av_eqtb[crate::ix::U(((k) - 1) as usize)];
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
            if (k == int_base) { break; }
        }
        // §1495
        loop {
            'l_done2_f: {
                'l_found2_f: {
                    j = k;
                    while (j < eqtb_size) {
                        {
                            if (__av_eqtb[crate::ix::U(((j) - 1) as usize)].int() == __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].int()) {
                                break 'l_found2_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 30193i32;
                    break 'l_done2_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < eqtb_size) {
                    {
                        if (__av_eqtb[crate::ix::U(((j) - 1) as usize)].int() != __av_eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)].int()) {
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
                        self.fmt_file.buf = __av_eqtb[crate::ix::U(((k) - 1) as usize)];
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
            if (k > eqtb_size) { break; }
        }
        if (self.hash_high > 0i32) {
            {
                let __for_end_3 = (eqtb_size).wrapping_add(self.hash_high);
                k = 30193i32;
                while k <= __for_end_3 {
                    {
                        self.fmt_file.buf = __av_eqtb[crate::ix::U(((k) - 1) as usize)];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
        }
        // §1492
        {
            { let __v2019 = self.par_loc; self.fmt_file.buf.set_int(__v2019); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2020 = self.write_loc; self.fmt_file.buf.set_int(__v2020); }
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1497
        {
            let __for_end_2 = prim_size;
            p = 0i32;
            while p <= __for_end_2 {
                {
                    { let __v2021 = self.prim[crate::ix::U((p) as usize)]; self.fmt_file.buf.set_hh(__v2021); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        {
            { let __v2022 = self.hash_used; self.fmt_file.buf.set_int(__v2022); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.cs_count = ((15513i32).wrapping_sub(self.hash_used)).wrapping_add(self.hash_high);
        {
            let __for_end_2 = self.hash_used;
            p = hash_base;
            while p <= __for_end_2 {
                if (self.hash[crate::ix::U(((p) - 514) as usize)].rh() != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(p);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2023 = self.hash[crate::ix::U(((p) - 514) as usize)]; self.fmt_file.buf.set_hh(__v2023); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        self.cs_count = (self.cs_count).wrapping_add(1i32);
                    }
                }
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 26626i32;
            p = (self.hash_used).wrapping_add(1i32);
            while p <= __for_end_2 {
                {
                    { let __v2024 = self.hash[crate::ix::U(((p) - 514) as usize)]; self.fmt_file.buf.set_hh(__v2024); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        if (self.hash_high > 0i32) {
            {
                let __for_end_3 = (eqtb_size).wrapping_add(self.hash_high);
                p = 30193i32;
                while p <= __for_end_3 {
                    {
                        { let __v2025 = self.hash[crate::ix::U(((p) - 514) as usize)]; self.fmt_file.buf.set_hh(__v2025); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    p = p.wrapping_add(1);
                }
            }
        }
        {
            { let __v2026 = self.cs_count; self.fmt_file.buf.set_int(__v2026); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((self.cs_count) as i64));
        self.print(1715i32);
        // §1499
        {
            { let __v2027 = self.fmem_ptr; self.fmt_file.buf.set_int(__v2027); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = (self.fmem_ptr).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.font_info[crate::ix::U((k) as usize)];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            { let __v2028 = self.font_ptr; self.fmt_file.buf.set_int(__v2028); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.font_ptr;
            k = null_font;
            while k <= __for_end_2 {
                // §1501
                {
                    {
                        { let __v2029 = self.font_check[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_qqqq(__v2029); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2030 = self.font_size[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2030); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2031 = self.font_dsize[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2031); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2032 = self.font_params[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2032); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2033 = self.hyphen_char[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2033); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2034 = self.skew_char[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2034); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2035 = self.font_name[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2035); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2036 = self.font_area[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2036); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2037 = self.font_bc[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2037); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2038 = self.font_ec[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2038); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2039 = self.char_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2039); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2040 = self.width_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2040); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2041 = self.height_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2041); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2042 = self.depth_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2042); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2043 = self.italic_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2043); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2044 = self.lig_kern_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2044); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2045 = self.kern_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2045); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2046 = self.exten_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2046); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2047 = self.param_base[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2047); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2048 = self.font_glue[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2048); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2049 = self.bchar_label[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2049); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2050 = self.font_bchar[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2050); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2051 = self.font_false_bchar[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2051); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    self.print_nl(1719i32);
                    self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(k)) - 514) as usize)].rh());
                    self.print_char(61i32);
                    self.print_file_name(self.font_name[crate::ix::U((k) as usize)], self.font_area[crate::ix::U((k) as usize)], 345i32);
                    if (self.font_size[crate::ix::U((k) as usize)] != self.font_dsize[crate::ix::U((k) as usize)]) {
                        {
                            self.print(906i32);
                            self.print_scaled(self.font_size[crate::ix::U((k) as usize)]);
                            self.print(312i32);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        // §1499
        self.print_ln();
        self.print_int((((self.fmem_ptr).wrapping_sub(7i32)) as i64));
        self.print(1716i32);
        self.print_int((((self.font_ptr).wrapping_sub(0i32)) as i64));
        if (self.font_ptr != 1i32) {
            self.print(1717i32);
        } else {
            self.print(1718i32);
        }
        // §1503
        {
            { let __v2052 = self.hyph_count; self.fmt_file.buf.set_int(__v2052); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = hyph_size;
            k = 0i32;
            while k <= __for_end_2 {
                if (self.hyph_word[crate::ix::U((k) as usize)] != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2053 = self.hyph_word[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2053); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2054 = self.hyph_list[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2054); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.print_int(((self.hyph_count) as i64));
        if (self.hyph_count != 1i32) {
            self.print(1720i32);
        } else {
            self.print(1721i32);
        }
        if self.trie_not_ready {
            self.init_trie();
        }
        {
            { let __v2055 = self.trie_max; self.fmt_file.buf.set_int(__v2055); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2056 = self.hyph_start; self.fmt_file.buf.set_int(__v2056); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_max;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v2057 = self.trie[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_hh(__v2057); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            { let __v2058 = self.trie_op_ptr; self.fmt_file.buf.set_int(__v2058); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    {
                        { let __v2059 = self.hyf_distance[crate::ix::U(((k) - 1) as usize)]; self.fmt_file.buf.set_int(__v2059); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2060 = self.hyf_num[crate::ix::U(((k) - 1) as usize)]; self.fmt_file.buf.set_int(__v2060); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v2061 = self.hyf_next[crate::ix::U(((k) - 1) as usize)]; self.fmt_file.buf.set_int(__v2061); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_nl(1722i32);
        self.print_int(((self.trie_max) as i64));
        self.print(1723i32);
        self.print_int(((self.trie_op_ptr) as i64));
        if (self.trie_op_ptr != 1i32) {
            self.print(1724i32);
        } else {
            self.print(1725i32);
        }
        self.print(1726i32);
        self.print_int(((trie_op_size) as i64));
        {
            let __for_end_2 = 0i32;
            k = 255i32;
            while k >= __for_end_2 {
                if (self.trie_used[crate::ix::U((k) as usize)] > min_quarterword) {
                    {
                        self.print_nl(967i32);
                        self.print_int(((self.trie_used[crate::ix::U((k) as usize)]) as i64));
                        self.print(1727i32);
                        self.print_int(((k) as i64));
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2062 = self.trie_used[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2062); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_sub(1);
            }
        }
        // §1505
        {
            self.dumpimagemeta();
            {
                { let __v2063 = self.pdf_mem_size; self.fmt_file.buf.set_int(__v2063); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2064 = self.pdf_mem_ptr; self.fmt_file.buf.set_int(__v2064); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                let __for_end_3 = (self.pdf_mem_ptr).wrapping_sub(1i32);
                k = 1i32;
                while k <= __for_end_3 {
                    {
                        {
                            { let __v2065 = self.pdf_mem[crate::ix::U((k) as usize)]; self.fmt_file.buf.set_int(__v2065); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_ln();
            self.print_int((((self.pdf_mem_ptr).wrapping_sub(1i32)) as i64));
            self.print(1728i32);
            {
                { let __v2066 = self.obj_tab_size; self.fmt_file.buf.set_int(__v2066); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2067 = self.obj_ptr; self.fmt_file.buf.set_int(__v2067); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2068 = self.sys_obj_ptr; self.fmt_file.buf.set_int(__v2068); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                let __for_end_3 = self.sys_obj_ptr;
                k = 1i32;
                while k <= __for_end_3 {
                    {
                        {
                            { let __v2069 = self.obj_tab[crate::ix::U((k) as usize)].int0; self.fmt_file.buf.set_int(__v2069); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2070 = self.obj_tab[crate::ix::U((k) as usize)].int1; self.fmt_file.buf.set_int(__v2070); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2071 = self.obj_tab[crate::ix::U((k) as usize)].int3; self.fmt_file.buf.set_int(__v2071); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v2072 = self.obj_tab[crate::ix::U((k) as usize)].int4; self.fmt_file.buf.set_int(__v2072); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_ln();
            self.print_int(((self.sys_obj_ptr) as i64));
            self.print(1729i32);
            {
                { let __v2073 = self.pdf_obj_count; self.fmt_file.buf.set_int(__v2073); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2074 = self.pdf_xform_count; self.fmt_file.buf.set_int(__v2074); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2075 = self.pdf_ximage_count; self.fmt_file.buf.set_int(__v2075); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2076 = self.head_tab[crate::ix::U(((obj_type_obj) - 1) as usize)]; self.fmt_file.buf.set_int(__v2076); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2077 = self.head_tab[crate::ix::U(((obj_type_xform) - 1) as usize)]; self.fmt_file.buf.set_int(__v2077); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2078 = self.head_tab[crate::ix::U(((obj_type_ximage) - 1) as usize)]; self.fmt_file.buf.set_int(__v2078); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2079 = self.pdf_last_obj; self.fmt_file.buf.set_int(__v2079); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2080 = self.pdf_last_xform; self.fmt_file.buf.set_int(__v2080); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v2081 = self.pdf_last_ximage; self.fmt_file.buf.set_int(__v2081); }
                crate::system::put_word(&mut self.fmt_file);
            }
            self.dumptounicode();
        }
        // §1507
        {
            { let __v2082 = self.interaction; self.fmt_file.buf.set_int(__v2082); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v2083 = self.format_ident; self.fmt_file.buf.set_int(__v2083); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(69069i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        __av_eqtb[crate::ix::U(((29308i32) - 1) as usize)].set_int(0i32);
        // §1510
        { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
    }

    /// Here is a subroutine that creates a whatsit node having a given `subtype`
    /// and a given number of words. It initializes only the first word of the whatsit,
    /// and appends it to the current list.
    /// @<Declare procedures needed in `do_extension`
    // §1530
    pub fn new_whatsit(&mut self, mut s: small_number, mut w: small_number) {
        let mut p: halfword = 0; // §1530
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(w);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        { let __ix2084 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2084) as usize)].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// The next subroutine uses `cur_chr` to decide what sort of whatsit is
    /// involved, and also inserts a `write_stream` number.
    /// @<Declare procedures needed in `do_ext...
    // §1531
    pub fn new_write_whatsit(&mut self, mut w: small_number) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.new_whatsit(self.cur_chr, w);
        if (w != write_node_size) {
            self.scan_four_bit_int();
        } else {
            {
                self.scan_int();
                if (self.cur_val < 0i32) {
                    self.cur_val = 17i32;
                } else {
                    if ((self.cur_val > 15i32) && (self.cur_val != 18i32)) {
                        self.cur_val = 16i32;
                    }
                }
            }
        }
        { let __ix2085 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2086 = self.cur_val; __av_mem[crate::ix::U((__ix2085) as usize)].set_hh_b0(__v2086); }
        if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) < 0i32) {
            { let __ix2087 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2087) as usize)].set_hh_b1(0i32); }
        } else {
            if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) >= (2i32).wrapping_mul(mubyte_zero)) {
                { let __ix2088 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2088) as usize)].set_hh_b1(((2i32).wrapping_mul(mubyte_zero)).wrapping_sub(1i32)); }
            } else {
                { let __ix2089 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2090 = (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32); __av_mem[crate::ix::U((__ix2089) as usize)].set_hh_b1(__v2090); }
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1538
    pub fn check_pdfoutput(&mut self, mut s: str_number, mut is_error: bool) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() <= 0i32) {
            {
                if is_error {
                    self.pdf_error(s, 1793i32);
                } else {
                    self.pdf_warning(s, 1794i32, true, true);
                }
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1538
    pub fn scan_pdf_ext_toks(&mut self) {
        {
            if (self.scan_toks(false, true) != 0i32) {
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1538
    pub fn scan_pdf_ext_late_toks(&mut self) {
        {
            if (self.scan_toks(false, false) != 0i32) {
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1538
    pub fn compare_strings(&mut self) {
        let mut s1: str_number = 0; // §1538
        let mut s2: str_number = 0; // §1538
        let mut i1: pool_pointer = 0; // §1538
        let mut i2: pool_pointer = 0; // §1538
        let mut j1: pool_pointer = 0; // §1538
        let mut j2: pool_pointer = 0; // §1538
        let mut save_cur_cs: halfword = 0; // §1538
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
            i1 = self.str_start[crate::ix::U((s1) as usize)];
            j1 = self.str_start[crate::ix::U(((s1).wrapping_add(1i32)) as usize)];
            i2 = self.str_start[crate::ix::U((s2) as usize)];
            j2 = self.str_start[crate::ix::U(((s2).wrapping_add(1i32)) as usize)];
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

    /// @<Declare procedures needed in `do_ext...
    // §1553
    pub fn scale_image(&mut self, mut n: i32) {
        let mut x: i32 = 0; // §1553
        let mut y: i32 = 0; // §1553
        let mut xr: i32 = 0; // §1553
        let mut yr: i32 = 0; // §1553
        let mut w: scaled = 0; // §1553
        let mut h: scaled = 0; // §1553
        let mut default_res: i32 = 0; // §1553
        let mut image: i32 = 0; // §1553
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        image = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(4i32)) as usize)];
        if ((self.image_rotate(image) == 90i32) || (self.image_rotate(image) == 270i32)) {
            {
                y = self.image_width(image);
                x = self.image_height(image);
                yr = self.image_x_res(image);
                xr = self.image_y_res(image);
            }
        } else {
            {
                x = self.image_width(image);
                y = self.image_height(image);
                xr = self.image_x_res(image);
                yr = self.image_y_res(image);
            }
        }
        if ((xr > 65535i32) || (yr > 65535i32)) {
            {
                xr = 0i32;
                yr = 0i32;
                self.pdf_warning(1792i32, 1823i32, true, true);
            }
        }
        if ((((x <= 0i32) || (y <= 0i32)) || (xr < 0i32)) || (yr < 0i32)) {
            self.pdf_error(1792i32, 1824i32);
        }
        if ((xr == 0i32) && (yr == 0i32)) {
            {
            }
        } else {
            if (((((x) as f64) / ((self.one_inch) as f64)) >= ((xr) as f64)) || ((((y) as f64) / ((self.one_inch) as f64)) >= ((yr) as f64))) {
                {
                    xr = 0i32;
                    yr = 0i32;
                    self.pdf_warning(1792i32, 1825i32, true, true);
                }
            }
        }
        if self.is_pdf_image(image) {
            {
                w = x;
                h = y;
            }
        } else {
            {
                default_res = self.fix_int(__av_eqtb[crate::ix::U(((29346i32) - 1) as usize)].int(), 0i32, 65535i32);
                if ((default_res > 0i32) && ((xr == 0i32) || (yr == 0i32))) {
                    {
                        xr = default_res;
                        yr = default_res;
                    }
                }
                if ((self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] == (1073741824i32).wrapping_neg())) {
                    {
                        if ((xr > 0i32) && (yr > 0i32)) {
                            {
                                w = self.ext_xn_over_d(self.one_hundred_inch, x, (100i32).wrapping_mul(xr));
                                h = self.ext_xn_over_d(self.one_hundred_inch, y, (100i32).wrapping_mul(yr));
                            }
                        } else {
                            {
                                w = self.ext_xn_over_d(self.one_hundred_inch, x, 7200i32);
                                h = self.ext_xn_over_d(self.one_hundred_inch, y, 7200i32);
                            }
                        }
                    }
                }
            }
        }
        if (((self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] == (1073741824i32).wrapping_neg())) && (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] == (1073741824i32).wrapping_neg())) {
            {
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] = w;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] = h;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
            }
        } else {
            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] == (1073741824i32).wrapping_neg()) {
                {
                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] == (1073741824i32).wrapping_neg()) {
                        {
                            { let __ix2091 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32); let __v2092 = self.ext_xn_over_d(h, x, y); self.pdf_mem[crate::ix::U((__ix2091) as usize)] = __v2092; }
                            { let __ix2093 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32); let __v2094 = (h).wrapping_sub(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]); self.pdf_mem[crate::ix::U((__ix2093) as usize)] = __v2094; }
                        }
                    } else {
                        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] == (1073741824i32).wrapping_neg()) {
                            {
                                { let __ix2095 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32); let __v2096 = self.ext_xn_over_d(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)], x, y); self.pdf_mem[crate::ix::U((__ix2095) as usize)] = __v2096; }
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
                            }
                        } else {
                            {
                                { let __ix2097 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32); let __v2098 = self.ext_xn_over_d((self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)]).wrapping_add(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]), x, y); self.pdf_mem[crate::ix::U((__ix2097) as usize)] = __v2098; }
                            }
                        }
                    }
                }
            } else {
                {
                    if ((self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] == (1073741824i32).wrapping_neg())) {
                        {
                            { let __ix2099 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32); let __v2100 = self.ext_xn_over_d(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)], y, x); self.pdf_mem[crate::ix::U((__ix2099) as usize)] = __v2100; }
                            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
                        }
                    } else {
                        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] == (1073741824i32).wrapping_neg()) {
                            {
                                { let __ix2101 = (self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32); let __v2102 = (self.ext_xn_over_d(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)], y, x)).wrapping_sub(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]); self.pdf_mem[crate::ix::U((__ix2101) as usize)] = __v2102; }
                            }
                        } else {
                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] == (1073741824i32).wrapping_neg()) {
                                {
                                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
                                }
                            } else {
                            }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1553
    pub fn scan_pdf_box_spec(&mut self) -> i32 {
        let mut scan_pdf_box_spec: i32 = 0;
        scan_pdf_box_spec = 0i32;
        if self.scan_keyword(1826i32) {
            scan_pdf_box_spec = self.pdf_box_spec_media;
        } else {
            if self.scan_keyword(1827i32) {
                scan_pdf_box_spec = self.pdf_box_spec_crop;
            } else {
                if self.scan_keyword(1828i32) {
                    scan_pdf_box_spec = self.pdf_box_spec_bleed;
                } else {
                    if self.scan_keyword(1829i32) {
                        scan_pdf_box_spec = self.pdf_box_spec_trim;
                    } else {
                        if self.scan_keyword(1830i32) {
                            scan_pdf_box_spec = self.pdf_box_spec_art;
                        }
                    }
                }
            }
        }
        scan_pdf_box_spec
    }

    /// @<Declare procedures needed in `do_ext...
    // §1553
    pub fn scan_alt_rule(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.alt_rule == null) {
            self.alt_rule = self.new_rule();
        }
        { let __ix2103 = (self.alt_rule).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2103) as usize)].set_int((1073741824i32).wrapping_neg()); }
        { let __ix2104 = (self.alt_rule).wrapping_add(3i32); __av_mem[crate::ix::U((__ix2104) as usize)].set_int((1073741824i32).wrapping_neg()); }
        { let __ix2105 = (self.alt_rule).wrapping_add(2i32); __av_mem[crate::ix::U((__ix2105) as usize)].set_int((1073741824i32).wrapping_neg()); }
        'l_reswitch_b: loop {
            if self.scan_keyword(847i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix2106 = (self.alt_rule).wrapping_add(1i32); let __v2107 = self.cur_val; __av_mem[crate::ix::U((__ix2106) as usize)].set_int(__v2107); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(848i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix2108 = (self.alt_rule).wrapping_add(3i32); let __v2109 = self.cur_val; __av_mem[crate::ix::U((__ix2108) as usize)].set_int(__v2109); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(849i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix2110 = (self.alt_rule).wrapping_add(2i32); let __v2111 = self.cur_val; __av_mem[crate::ix::U((__ix2110) as usize)].set_int(__v2111); }
                    continue 'l_reswitch_b;
                }
            }
            break 'l_reswitch_b;
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1553
    pub fn scan_image(&mut self) {
        let mut k: i32 = 0; // §1553
        let mut named: str_number = 0; // §1553
        let mut s: str_number = 0; // §1553
        let mut page: i32 = 0; // §1553
        let mut pagebox: i32 = 0; // §1553
        let mut colorspace: i32 = 0; // §1553
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.pdf_ximage_count = (self.pdf_ximage_count).wrapping_add(1i32);
        self.pdf_create_obj(obj_type_ximage, self.pdf_ximage_count);
        k = self.obj_ptr;
        self.obj_tab[crate::ix::U((k) as usize)].int4 = self.pdf_get_mem(pdfmem_ximage_size);
        self.scan_alt_rule();
        { let __ix2112 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(0i32); let __v2113 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(1i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2112) as usize)] = __v2113; }
        { let __ix2114 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32); let __v2115 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(3i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2114) as usize)] = __v2115; }
        { let __ix2116 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32); let __v2117 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(2i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2116) as usize)] = __v2117; }
        if self.scan_keyword(1816i32) {
            {
                self.scan_pdf_ext_toks();
                { let __ix2118 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32); let __v2119 = self.def_ref; self.pdf_mem[crate::ix::U((__ix2118) as usize)] = __v2119; }
            }
        } else {
            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] = null;
        }
        named = 0i32;
        if self.scan_keyword(1831i32) {
            {
                self.scan_pdf_ext_toks();
                named = self.tokens_to_string(self.def_ref);
                self.delete_token_ref(self.def_ref);
            }
        } else {
            if self.scan_keyword(900i32) {
                {
                    self.scan_int();
                    page = self.cur_val;
                }
            } else {
                page = 1i32;
            }
        }
        if self.scan_keyword(1832i32) {
            {
                self.scan_int();
                colorspace = self.cur_val;
            }
        } else {
            colorspace = 0i32;
        }
        pagebox = self.scan_pdf_box_spec();
        if (pagebox == 0i32) {
            pagebox = __av_eqtb[crate::ix::U(((29354i32) - 1) as usize)].int();
        }
        self.scan_pdf_ext_toks();
        s = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        if (__av_eqtb[crate::ix::U(((29349i32) - 1) as usize)].int() != 0i32) {
            {
                self.pdf_warning(1833i32, 1834i32, true, true);
                { let __v2120 = __av_eqtb[crate::ix::U(((29349i32) - 1) as usize)].int(); __av_eqtb[crate::ix::U(((29353i32) - 1) as usize)].set_int(__v2120); }
                __av_eqtb[crate::ix::U(((29349i32) - 1) as usize)].set_int(0i32);
                self.warn_pdfpagebox = false;
            }
        }
        if (__av_eqtb[crate::ix::U(((29350i32) - 1) as usize)].int() != 0i32) {
            {
                self.pdf_warning(1833i32, 1835i32, true, true);
                { let __v2121 = __av_eqtb[crate::ix::U(((29350i32) - 1) as usize)].int(); __av_eqtb[crate::ix::U(((29355i32) - 1) as usize)].set_int(__v2121); }
                __av_eqtb[crate::ix::U(((29350i32) - 1) as usize)].set_int(0i32);
            }
        }
        if (__av_eqtb[crate::ix::U(((29353i32) - 1) as usize)].int() > 0i32) {
            {
                if self.warn_pdfpagebox {
                    {
                        self.pdf_warning(1833i32, 1836i32, true, true);
                        self.warn_pdfpagebox = false;
                    }
                }
                pagebox = __av_eqtb[crate::ix::U(((29353i32) - 1) as usize)].int();
            }
        }
        if (pagebox == 0i32) {
            pagebox = self.pdf_box_spec_crop;
        }
        { let __ix2122 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32); let __v2123 = self.read_image(s, page, named, colorspace, pagebox, __av_eqtb[crate::ix::U(((29351i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29352i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29355i32) - 1) as usize)].int()); self.pdf_mem[crate::ix::U((__ix2122) as usize)] = __v2123; }
        if (named != 0i32) {
            self.flush_str(named);
        }
        self.flush_str(s);
        self.scale_image(k);
        self.pdf_last_ximage = k;
        self.pdf_last_ximage_pages = self.image_pages(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)]);
        self.pdf_last_ximage_colordepth = self.image_colordepth(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)]);
    }

    /// @<Declare procedures needed in `do_ext...
    // §1557
    pub fn scan_action(&mut self) -> halfword {
        let mut scan_action: halfword = 0;
        let mut p: i32 = 0; // §1557
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(pdf_action_size);
        scan_action = p;
        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(null);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh(null);
        if self.scan_keyword(1839i32) {
            __av_mem[crate::ix::U((p) as usize)].set_hh_b0(pdf_action_user);
        } else {
            if self.scan_keyword(1840i32) {
                __av_mem[crate::ix::U((p) as usize)].set_hh_b0(pdf_action_goto);
            } else {
                if self.scan_keyword(1841i32) {
                    __av_mem[crate::ix::U((p) as usize)].set_hh_b0(pdf_action_thread);
                } else {
                    self.pdf_error(1792i32, 1842i32);
                }
            }
        }
        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_user) {
            {
                self.scan_pdf_ext_toks();
                { let __v2124 = self.def_ref; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh(__v2124); }
                return scan_action;
            }
        }
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        if self.scan_keyword(889i32) {
            {
                self.scan_pdf_ext_toks();
                { let __v2125 = self.def_ref; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v2125); }
            }
        }
        if self.scan_keyword(1843i32) {
            {
                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() != pdf_action_goto) {
                    self.pdf_error(1792i32, 1844i32);
                }
                if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __v2126 = (__av_mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(2i32); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v2126); }
                        { let __v2127 = self.def_ref; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh_rh(__v2127); }
                    }
                } else {
                    if self.scan_keyword(1845i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __v2128 = (__av_mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(2i32); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v2128); }
                            { let __v2129 = self.def_ref; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh_rh(__v2129); }
                        }
                    } else {
                        if self.scan_keyword(1208i32) {
                            {
                                self.scan_int();
                                if (self.cur_val <= 0i32) {
                                    self.pdf_error(1792i32, 1846i32);
                                }
                                { let __v2130 = self.cur_val; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh_rh(__v2130); }
                            }
                        } else {
                            self.pdf_error(1792i32, 1847i32);
                        }
                    }
                }
            }
        } else {
            __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh_rh(null);
        }
        if self.scan_keyword(900i32) {
            {
                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() != pdf_action_goto) {
                    self.pdf_error(1792i32, 1848i32);
                }
                __av_mem[crate::ix::U((p) as usize)].set_hh_b0(pdf_action_page);
                self.scan_int();
                if (self.cur_val <= 0i32) {
                    self.pdf_error(1792i32, 1849i32);
                }
                { let __v2131 = self.cur_val; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v2131); }
                self.scan_pdf_ext_toks();
                { let __v2132 = self.def_ref; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh(__v2132); }
            }
        } else {
            if self.scan_keyword(1845i32) {
                {
                    self.scan_pdf_ext_toks();
                    { let __v2133 = (__av_mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v2133); }
                    { let __v2134 = self.def_ref; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v2134); }
                }
            } else {
                if self.scan_keyword(1208i32) {
                    {
                        if ((__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_goto) && (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null)) {
                            self.pdf_error(1792i32, 1850i32);
                        }
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.pdf_error(1792i32, 1846i32);
                        }
                        { let __v2135 = self.cur_val; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v2135); }
                    }
                } else {
                    self.pdf_error(1792i32, 1847i32);
                }
            }
        }
        if self.scan_keyword(1851i32) {
            {
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(1i32);
                // §470
                {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) {
                        self.back_input();
                    }
                }
            }
        } else {
            // §1557
            if self.scan_keyword(1852i32) {
                {
                    __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
                    // §470
                    {
                        self.get_x_token();
                        if (self.cur_cmd != spacer) {
                            self.back_input();
                        }
                    }
                }
            } else {
                // §1557
                __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(0i32);
            }
        }
        if ((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() > 0i32) && (((__av_mem[crate::ix::U((p) as usize)].hh().b0() != pdf_action_goto) && (__av_mem[crate::ix::U((p) as usize)].hh().b0() != pdf_action_page)) || (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null))) {
            self.pdf_error(1792i32, 1853i32);
        }
        scan_action
    }

    /// @<Declare procedures needed in `do_ext...
    // §1557
    pub fn new_annot_whatsit(&mut self, mut w: small_number, mut s: small_number) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.new_whatsit(w, s);
        self.scan_alt_rule();
        { let __ix2136 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2137 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2136) as usize)].set_int(__v2137); }
        { let __ix2138 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2139 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2138) as usize)].set_int(__v2139); }
        { let __ix2140 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2141 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2140) as usize)].set_int(__v2141); }
        if (w == pdf_start_link_node) {
            {
                if self.scan_keyword(1816i32) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __ix2142 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2143 = self.def_ref; __av_mem[crate::ix::U((__ix2142) as usize)].set_hh_lh(__v2143); }
                    }
                } else {
                    { let __ix2144 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2144) as usize)].set_hh_lh(null); }
                }
            }
        }
        if ((w == pdf_thread_node) || (w == pdf_start_thread_node)) {
            {
                if self.scan_keyword(1816i32) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __ix2145 = (self.cur_list.tail_field).wrapping_add(6i32); let __v2146 = self.def_ref; __av_mem[crate::ix::U((__ix2145) as usize)].set_hh_lh(__v2146); }
                    }
                } else {
                    { let __ix2147 = (self.cur_list.tail_field).wrapping_add(6i32); __av_mem[crate::ix::U((__ix2147) as usize)].set_hh_lh(null); }
                }
            }
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1563
    pub fn outline_list_count(&mut self, mut p: halfword) -> i32 {
        let mut outline_list_count: i32 = 0;
        let mut k: i32 = 0; // §1563
        k = 1i32;
        while (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((p) as usize)].int4).wrapping_add(2i32)) as usize)] != 0i32) {
            {
                k = (k).wrapping_add(1i32);
                p = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((p) as usize)].int4).wrapping_add(2i32)) as usize)];
            }
        }
        outline_list_count = k;
        outline_list_count
    }

    /// @<Declare procedures needed in `do_ext...
    // §1567
    pub fn scan_thread_id(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.scan_keyword(1208i32) {
            {
                self.scan_int();
                if (self.cur_val <= 0i32) {
                    self.pdf_error(1792i32, 1846i32);
                }
                if (self.cur_val > max_halfword) {
                    self.pdf_error(1792i32, 1045i32);
                }
                { let __ix2148 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2149 = self.cur_val; __av_mem[crate::ix::U((__ix2148) as usize)].set_hh_rh(__v2149); }
                { let __ix2150 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2150) as usize)].set_hh_b1(0i32); }
            }
        } else {
            if self.scan_keyword(1845i32) {
                {
                    self.scan_pdf_ext_toks();
                    { let __ix2151 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2152 = self.def_ref; __av_mem[crate::ix::U((__ix2151) as usize)].set_hh_rh(__v2152); }
                    { let __ix2153 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2153) as usize)].set_hh_b1(1i32); }
                }
            } else {
                self.pdf_error(1792i32, 1847i32);
            }
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1574
    pub fn new_snap_node(&mut self, mut s: small_number) -> halfword {
        let mut new_snap_node: halfword = 0;
        let mut p: halfword = 0; // §1574
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.scan_glue(glue_val);
        if (__av_mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int() < 0i32) {
            self.pdf_error(1792i32, 1881i32);
        }
        p = self.get_node(snap_node_size);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
        { let __v2154 = self.cur_val; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v2154); }
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        new_snap_node = p;
        new_snap_node
    }

    /// To implement primitives as \.{\\pdfinfo}, \.{\\pdfcatalog} or
    /// \.{\\pdfnames} we need to concatenate tokens lists.
    /// @<Declare procedures needed in `do_ext...
    // §1578
    pub fn concat_tokens(&mut self, mut q: halfword, mut r: halfword) -> halfword {
        let mut concat_tokens: halfword = 0;
        let mut p: halfword = 0; // §1578
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (q == null) {
            {
                concat_tokens = r;
                return concat_tokens;
            }
        }
        p = q;
        while (__av_mem[crate::ix::U((p) as usize)].hh().rh() != null) {
            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
        }
        { let __v2155 = __av_mem[crate::ix::U((r) as usize)].hh().rh(); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v2155); }
        {
            { let __v2156 = self.avail; __av_mem[crate::ix::U((r) as usize)].set_hh_rh(__v2156); }
            self.avail = r;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        concat_tokens = q;
        concat_tokens
    }

    /// The following subroutines are about PDF-specific font issues.
    /// @<Declare procedures needed in `do_ext...
    // §1588
    pub fn pdf_include_chars(&mut self) {
        let mut s: str_number = 0; // §1588
        let mut k: pool_pointer = 0; // §1588
        let mut f: internal_font_number = 0; // §1588
        self.scan_font_ident();
        f = self.cur_val;
        if (f == null_font) {
            self.pdf_error(604i32, 884i32);
        }
        self.pdf_check_vf_cur_val();
        if (!self.font_used[crate::ix::U((f) as usize)]) {
            self.pdf_init_font(f);
        }
        self.scan_pdf_ext_toks();
        s = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        k = self.str_start[crate::ix::U((s) as usize)];
        while (k < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
            {
                self.pdf_mark_char(f, self.str_pool[crate::ix::U((k) as usize)]);
                k = (k).wrapping_add(1i32);
            }
        }
        self.flush_str(s);
    }

    /// The following subroutines are about PDF-specific font issues.
    /// @<Declare procedures needed in `do_ext...
    // §1588
    pub fn glyph_to_unicode(&mut self) {
        let mut s1: str_number = 0; // §1588
        let mut s2: str_number = 0; // §1588
        self.scan_pdf_ext_toks();
        s1 = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        self.scan_pdf_ext_toks();
        s2 = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        self.def_tounicode(s1, s2);
        self.flush_str(s2);
        self.flush_str(s1);
    }

    /// The following function are needed for outputting article thread.
    /// @<Declare procedures needed in `do_ext...
    // §1601
    pub fn thread_title(&mut self, mut thread: i32) {
        self.pdf_print(1901i32);
        if (self.obj_tab[crate::ix::U((thread) as usize)].int0 < 0i32) {
            self.pdf_print((self.obj_tab[crate::ix::U((thread) as usize)].int0).wrapping_neg());
        } else {
            self.pdf_print_int(((self.obj_tab[crate::ix::U((thread) as usize)].int0) as i64));
        }
        {
            self.pdf_print(41i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1018i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// The following function are needed for outputting article thread.
    /// @<Declare procedures needed in `do_ext...
    // §1601
    pub fn pdf_fix_thread(&mut self, mut thread: i32) {
        let mut a: halfword = 0; // §1601
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.pdf_warning(1841i32, 1902i32, true, false);
        if (self.obj_tab[crate::ix::U((thread) as usize)].int0 < 0i32) {
            {
                self.print(1207i32);
                self.print((self.obj_tab[crate::ix::U((thread) as usize)].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1208i32);
                self.print_int(((self.obj_tab[crate::ix::U((thread) as usize)].int0) as i64));
            }
        }
        self.print(1209i32);
        self.print_ln();
        self.print_ln();
        self.pdf_new_dict(obj_type_others, 0i32, 0i32);
        a = self.obj_ptr;
        self.pdf_indirect_ln(84i32, thread);
        self.pdf_indirect_ln(86i32, a);
        self.pdf_indirect_ln(78i32, a);
        self.pdf_indirect_ln(80i32, self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)]);
        self.pdf_print(1903i32);
        self.pdf_print_bp(__av_eqtb[crate::ix::U(((29926i32) - 1) as usize)].int());
        {
            {
                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                    self.pdf_os_get_os_buf(1i32);
                } else {
                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                        self.overflow(1018i32, pdf_op_buf_size);
                    } else {
                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                            self.pdf_flush();
                        }
                    }
                }
            }
            {
                self.pdf_buf_set(self.pdf_ptr, 32i32);
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
        self.pdf_print_bp(__av_eqtb[crate::ix::U(((29927i32) - 1) as usize)].int());
        {
            self.pdf_print(93i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1018i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_end_dict();
        self.pdf_begin_dict(thread, 1i32);
        {
            self.pdf_print(1904i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1018i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.thread_title(thread);
        {
            self.pdf_print(1035i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1018i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_indirect_ln(70i32, a);
        self.pdf_end_dict();
    }

    /// The following function are needed for outputting article thread.
    /// @<Declare procedures needed in `do_ext...
    // §1601
    pub fn out_thread(&mut self, mut thread: i32) {
        let mut a: halfword = 0; // §1601
        let mut b: halfword = 0; // §1601
        let mut last_attr: i32 = 0; // §1601
        if (self.obj_tab[crate::ix::U((thread) as usize)].int4 == 0i32) {
            {
                self.pdf_fix_thread(thread);
                return;
            }
        }
        self.pdf_begin_dict(thread, 1i32);
        a = self.obj_tab[crate::ix::U((thread) as usize)].int4;
        b = a;
        last_attr = 0i32;
        loop {
            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(4i32)) as usize)] != 0i32) {
                last_attr = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(4i32)) as usize)];
            }
            a = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(2i32)) as usize)];
            if (a == b) { break; }
        }
        if (last_attr != 0i32) {
            {
                self.pdf_print(last_attr);
                {
                    {
                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                            self.pdf_os_get_os_buf(1i32);
                        } else {
                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                self.overflow(1018i32, pdf_op_buf_size);
                            } else {
                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                    self.pdf_flush();
                                }
                            }
                        }
                    }
                    {
                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        } else {
            {
                {
                    self.pdf_print(1904i32);
                    {
                        {
                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_os_get_os_buf(1i32);
                            } else {
                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                    self.overflow(1018i32, pdf_op_buf_size);
                                } else {
                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                        self.pdf_flush();
                                    }
                                }
                            }
                        }
                        {
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
                self.thread_title(thread);
                {
                    self.pdf_print(1035i32);
                    {
                        {
                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_os_get_os_buf(1i32);
                            } else {
                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                    self.overflow(1018i32, pdf_op_buf_size);
                                } else {
                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                        self.pdf_flush();
                                    }
                                }
                            }
                        }
                        {
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.pdf_indirect_ln(70i32, a);
        self.pdf_end_dict();
        loop {
            self.pdf_begin_dict(a, 1i32);
            if (a == b) {
                self.pdf_indirect_ln(84i32, thread);
            }
            self.pdf_indirect_ln(86i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(3i32)) as usize)]);
            self.pdf_indirect_ln(78i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(2i32)) as usize)]);
            self.pdf_indirect_ln(80i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(1i32)) as usize)]);
            self.pdf_indirect_ln(82i32, self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((a) as usize)].int4) as usize)]);
            self.pdf_end_dict();
            a = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(2i32)) as usize)];
            if (a == b) { break; }
        }
    }

    /// @<Declare act...
    // §1529
    pub fn do_extension(&mut self) {
        let mut i: i32 = 0; // §1529
        let mut j: i32 = 0; // §1529
        let mut k: i32 = 0; // §1529
        let mut p: halfword = 0; // §1529
        let mut q: halfword = 0; // §1529
        let mut r: halfword = 0; // §1529
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        match self.cur_chr {
            open_node => {
                // §1532
                {
                    self.new_write_whatsit(open_node_size);
                    self.scan_optional_equals();
                    self.scan_file_name();
                    { let __ix2157 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2158 = self.cur_name; __av_mem[crate::ix::U((__ix2157) as usize)].set_hh_rh(__v2158); }
                    { let __ix2159 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2160 = self.cur_area; __av_mem[crate::ix::U((__ix2159) as usize)].set_hh_lh(__v2160); }
                    { let __ix2161 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2162 = self.cur_ext; __av_mem[crate::ix::U((__ix2161) as usize)].set_hh_rh(__v2162); }
                }
            }
            write_node => {
                // §1533
                {
                    k = self.cur_cs;
                    self.new_write_whatsit(write_node_size);
                    self.cur_cs = k;
                    p = self.scan_toks(false, false);
                    { let __ix2163 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2164 = self.def_ref; __av_mem[crate::ix::U((__ix2163) as usize)].set_hh_rh(__v2164); }
                }
            }
            close_node => {
                // §1534
                {
                    self.new_write_whatsit(write_node_size);
                    { let __ix2165 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2165) as usize)].set_hh_rh(null); }
                }
            }
            special_node => {
                // §1535
                {
                    if self.scan_keyword(1487i32) {
                        {
                            self.new_whatsit(latespecial_node, write_node_size);
                            if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32) < 0i32) {
                                { let __ix2166 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2166) as usize)].set_hh_b0(0i32); }
                            } else {
                                if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32) >= (2i32).wrapping_mul(mubyte_zero)) {
                                    { let __ix2167 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2167) as usize)].set_hh_b0(((2i32).wrapping_mul(mubyte_zero)).wrapping_sub(1i32)); }
                                } else {
                                    { let __ix2168 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2169 = (__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32); __av_mem[crate::ix::U((__ix2168) as usize)].set_hh_b0(__v2169); }
                                }
                            }
                            if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) < 0i32) {
                                { let __ix2170 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2170) as usize)].set_hh_b1(0i32); }
                            } else {
                                if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) >= (2i32).wrapping_mul(mubyte_zero)) {
                                    { let __ix2171 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2171) as usize)].set_hh_b1(((2i32).wrapping_mul(mubyte_zero)).wrapping_sub(1i32)); }
                                } else {
                                    { let __ix2172 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2173 = (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32); __av_mem[crate::ix::U((__ix2172) as usize)].set_hh_b1(__v2173); }
                                }
                            }
                            if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int() == 2i32) || (__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int() == 3i32)) {
                                if (((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() > 2i32) || (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() == (1i32).wrapping_neg())) || (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() == (2i32).wrapping_neg())) {
                                    self.write_noexpanding = true;
                                }
                            }
                            p = self.scan_toks(false, false);
                            { let __ix2174 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2175 = self.def_ref; __av_mem[crate::ix::U((__ix2174) as usize)].set_hh_rh(__v2175); }
                            self.write_noexpanding = false;
                        }
                    } else {
                        {
                            self.new_whatsit(special_node, write_node_size);
                            if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32) < 0i32) {
                                { let __ix2176 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2176) as usize)].set_hh_b0(0i32); }
                            } else {
                                if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32) >= (2i32).wrapping_mul(mubyte_zero)) {
                                    { let __ix2177 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2177) as usize)].set_hh_b0(((2i32).wrapping_mul(mubyte_zero)).wrapping_sub(1i32)); }
                                } else {
                                    { let __ix2178 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2179 = (__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int()).wrapping_add(64i32); __av_mem[crate::ix::U((__ix2178) as usize)].set_hh_b0(__v2179); }
                                }
                            }
                            if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) < 0i32) {
                                { let __ix2180 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2180) as usize)].set_hh_b1(0i32); }
                            } else {
                                if ((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32) >= (2i32).wrapping_mul(mubyte_zero)) {
                                    { let __ix2181 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2181) as usize)].set_hh_b1(((2i32).wrapping_mul(mubyte_zero)).wrapping_sub(1i32)); }
                                } else {
                                    { let __ix2182 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2183 = (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int()).wrapping_add(64i32); __av_mem[crate::ix::U((__ix2182) as usize)].set_hh_b1(__v2183); }
                                }
                            }
                            if ((__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int() == 2i32) || (__av_eqtb[crate::ix::U(((29341i32) - 1) as usize)].int() == 3i32)) {
                                if (((__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() > 2i32) || (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() == (1i32).wrapping_neg())) || (__av_eqtb[crate::ix::U(((29339i32) - 1) as usize)].int() == (2i32).wrapping_neg())) {
                                    self.write_noexpanding = true;
                                }
                            }
                            p = self.scan_toks(false, true);
                            { let __ix2184 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2185 = self.def_ref; __av_mem[crate::ix::U((__ix2184) as usize)].set_hh_rh(__v2185); }
                            self.write_noexpanding = false;
                        }
                    }
                }
            }
            immediate_code => {
                // §1624
                {
                    self.get_x_token();
                    if (self.cur_cmd == extension) {
                        {
                            if (self.cur_chr <= close_node) {
                                {
                                    p = self.cur_list.tail_field;
                                    self.do_extension();
                                    self.out_what(self.cur_list.tail_field);
                                    self.flush_node_list(self.cur_list.tail_field);
                                    self.cur_list.tail_field = p;
                                    __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                                }
                            } else {
                                match self.cur_chr {
                                    pdf_obj_code => {
                                        {
                                            self.do_extension();
                                            if (self.obj_tab[crate::ix::U((self.pdf_last_obj) as usize)].int4 == 0i32) {
                                                self.pdf_error(1792i32, 1947i32);
                                            }
                                            self.pdf_write_obj(self.pdf_last_obj);
                                        }
                                    }
                                    pdf_xform_code => {
                                        {
                                            self.do_extension();
                                            self.pdf_cur_form = self.pdf_last_xform;
                                            self.pdf_ship_out(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_last_xform) as usize)].int4).wrapping_add(3i32)) as usize)], false);
                                        }
                                    }
                                    pdf_ximage_code => {
                                        {
                                            self.do_extension();
                                            self.pdf_write_image(self.pdf_last_ximage);
                                        }
                                    }
                                    _ => {
                                        self.back_input();
                                    }
                                }
                            }
                        }
                    } else {
                        self.back_input();
                    }
                }
            }
            set_language_code => {
                // §1626
                if ((self.cur_list.mode_field).wrapping_abs() != hmode) {
                    self.report_illegal_case();
                } else {
                    {
                        self.new_whatsit(language_node, small_node_size);
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.cur_list.aux_field.set_hh_rh(0i32);
                        } else {
                            if (self.cur_val > 255i32) {
                                self.cur_list.aux_field.set_hh_rh(0i32);
                            } else {
                                { let __v2186 = self.cur_val; self.cur_list.aux_field.set_hh_rh(__v2186); }
                            }
                        }
                        { let __ix2187 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2188 = self.cur_list.aux_field.hh().rh(); __av_mem[crate::ix::U((__ix2187) as usize)].set_hh_rh(__v2188); }
                        { let __ix2189 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2190 = self.norm_min(__av_eqtb[crate::ix::U(((29328i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix2189) as usize)].set_hh_b0(__v2190); }
                        { let __ix2191 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2192 = self.norm_min(__av_eqtb[crate::ix::U(((29329i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix2191) as usize)].set_hh_b1(__v2192); }
                    }
                }
            }
            pdf_annot_node => {
                // §1559
                {
                    self.check_pdfoutput(1854i32, true);
                    if self.scan_keyword(1813i32) {
                        {
                            self.pdf_last_annot = self.pdf_new_objnum();
                            // §470
                            {
                                self.get_x_token();
                                if (self.cur_cmd != spacer) {
                                    self.back_input();
                                }
                            }
                        }
                    } else {
                        // §1559
                        {
                            if self.scan_keyword(1814i32) {
                                {
                                    self.scan_int();
                                    k = self.cur_val;
                                    if (((k <= 0i32) || (k > self.obj_ptr)) || (self.obj_tab[crate::ix::U((k) as usize)].int4 != 0i32)) {
                                        self.pdf_error(1792i32, 1855i32);
                                    }
                                }
                            } else {
                                k = self.pdf_new_objnum();
                            }
                            self.new_annot_whatsit(pdf_annot_node, pdf_annot_node_size);
                            { let __ix2193 = (self.cur_list.tail_field).wrapping_add(6i32); __av_mem[crate::ix::U((__ix2193) as usize)].set_int(k); }
                            self.scan_pdf_ext_toks();
                            { let __ix2194 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2195 = self.def_ref; __av_mem[crate::ix::U((__ix2194) as usize)].set_hh_lh(__v2195); }
                            self.pdf_last_annot = k;
                        }
                    }
                }
            }
            pdf_catalog_code => {
                // §1580
                {
                    self.check_pdfoutput(1885i32, false);
                    self.scan_pdf_ext_toks();
                    if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
                        self.pdf_catalog_toks = self.concat_tokens(self.pdf_catalog_toks, self.def_ref);
                    }
                    if self.scan_keyword(1886i32) {
                        {
                            if (self.pdf_catalog_openaction != 0i32) {
                                self.pdf_error(1792i32, 1887i32);
                            } else {
                                {
                                    p = self.scan_action();
                                    self.pdf_new_obj(obj_type_others, 0i32, 1i32);
                                    if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
                                        self.pdf_catalog_openaction = self.obj_ptr;
                                    }
                                    self.write_action(p);
                                    self.pdf_end_obj();
                                    {
                                        if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh() == null) {
                                            {
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_user) {
                                                    self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                                } else {
                                                    {
                                                        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                                        }
                                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_page) {
                                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                                        } else {
                                                            if ((((((__av_mem[crate::ix::U((p) as usize)].hh().b1()) != 0) && ((1i32) != 0))) as i32) == 1i32) {
                                                                self.delete_token_ref(__av_mem[crate::ix::U((p) as usize)].hh().rh());
                                                            }
                                                        }
                                                        if ((((((__av_mem[crate::ix::U((p) as usize)].hh().b1()) != 0) && ((2i32) != 0))) as i32) == 2i32) {
                                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh());
                                                        }
                                                    }
                                                }
                                                self.free_node(p, pdf_action_size);
                                            }
                                        } else {
                                            { let __v2196 = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh()).wrapping_sub(1i32); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh(__v2196); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            pdf_dest_node => {
                // §1566
                {
                    self.check_pdfoutput(1863i32, true);
                    q = self.cur_list.tail_field;
                    self.new_whatsit(pdf_dest_node, pdf_dest_node_size);
                    if self.scan_keyword(1843i32) {
                        {
                            self.scan_int();
                            if (self.cur_val <= 0i32) {
                                self.pdf_error(1792i32, 1864i32);
                            }
                            { let __ix2197 = (self.cur_list.tail_field).wrapping_add(6i32); let __v2198 = self.cur_val; __av_mem[crate::ix::U((__ix2197) as usize)].set_hh_rh(__v2198); }
                            j = obj_type_struct_dest;
                        }
                    } else {
                        {
                            { let __ix2199 = (self.cur_list.tail_field).wrapping_add(6i32); __av_mem[crate::ix::U((__ix2199) as usize)].set_hh_rh(null); }
                            j = obj_type_dest;
                        }
                    }
                    if self.scan_keyword(1208i32) {
                        {
                            self.scan_int();
                            if (self.cur_val <= 0i32) {
                                self.pdf_error(1792i32, 1846i32);
                            }
                            if (self.cur_val > max_halfword) {
                                self.pdf_error(1792i32, 1045i32);
                            }
                            { let __ix2200 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2201 = self.cur_val; __av_mem[crate::ix::U((__ix2200) as usize)].set_hh_rh(__v2201); }
                            { let __ix2202 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2202) as usize)].set_hh_b1(0i32); }
                        }
                    } else {
                        if self.scan_keyword(1845i32) {
                            {
                                self.scan_pdf_ext_toks();
                                { let __ix2203 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2204 = self.def_ref; __av_mem[crate::ix::U((__ix2203) as usize)].set_hh_rh(__v2204); }
                                { let __ix2205 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2205) as usize)].set_hh_b1(1i32); }
                            }
                        } else {
                            self.pdf_error(1792i32, 1847i32);
                        }
                    }
                    if self.scan_keyword(1865i32) {
                        {
                            { let __ix2206 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2206) as usize)].set_hh_b0(pdf_dest_xyz); }
                            if self.scan_keyword(1866i32) {
                                {
                                    self.scan_int();
                                    if (self.cur_val > max_halfword) {
                                        self.pdf_error(1792i32, 1045i32);
                                    }
                                    { let __ix2207 = (self.cur_list.tail_field).wrapping_add(6i32); let __v2208 = self.cur_val; __av_mem[crate::ix::U((__ix2207) as usize)].set_hh_lh(__v2208); }
                                }
                            } else {
                                { let __ix2209 = (self.cur_list.tail_field).wrapping_add(6i32); __av_mem[crate::ix::U((__ix2209) as usize)].set_hh_lh(null); }
                            }
                        }
                    } else {
                        if self.scan_keyword(1867i32) {
                            { let __ix2210 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2210) as usize)].set_hh_b0(pdf_dest_fitbh); }
                        } else {
                            if self.scan_keyword(1868i32) {
                                { let __ix2211 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2211) as usize)].set_hh_b0(pdf_dest_fitbv); }
                            } else {
                                if self.scan_keyword(1869i32) {
                                    { let __ix2212 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2212) as usize)].set_hh_b0(pdf_dest_fitb); }
                                } else {
                                    if self.scan_keyword(1870i32) {
                                        { let __ix2213 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2213) as usize)].set_hh_b0(pdf_dest_fith); }
                                    } else {
                                        if self.scan_keyword(1871i32) {
                                            { let __ix2214 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2214) as usize)].set_hh_b0(pdf_dest_fitv); }
                                        } else {
                                            if self.scan_keyword(1872i32) {
                                                { let __ix2215 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2215) as usize)].set_hh_b0(pdf_dest_fitr); }
                                            } else {
                                                if self.scan_keyword(1873i32) {
                                                    { let __ix2216 = (self.cur_list.tail_field).wrapping_add(5i32); __av_mem[crate::ix::U((__ix2216) as usize)].set_hh_b0(pdf_dest_fit); }
                                                } else {
                                                    self.pdf_error(1792i32, 1874i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §470
                    {
                        self.get_x_token();
                        if (self.cur_cmd != spacer) {
                            self.back_input();
                        }
                    }
                    // §1566
                    if (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().b0() == pdf_dest_fitr) {
                        {
                            self.scan_alt_rule();
                            { let __ix2217 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2218 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2217) as usize)].set_int(__v2218); }
                            { let __ix2219 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2220 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2219) as usize)].set_int(__v2220); }
                            { let __ix2221 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2222 = __av_mem[crate::ix::U(((self.alt_rule).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2221) as usize)].set_int(__v2222); }
                        }
                    }
                    if (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().b1() != 0i32) {
                        {
                            i = self.tokens_to_string(__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().rh());
                            k = self.find_obj(j, i, true);
                            self.flush_str(i);
                        }
                    } else {
                        k = self.find_obj(j, __av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().rh(), false);
                    }
                    if ((k != 0i32) && (self.obj_tab[crate::ix::U((k) as usize)].int4 != null)) {
                        {
                            self.warn_dest_dup(__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().rh(), __av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(5i32)) as usize)].hh().b1(), 1875i32, 1876i32);
                            self.flush_node_list(self.cur_list.tail_field);
                            self.cur_list.tail_field = q;
                            __av_mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                        }
                    }
                }
            }
            pdf_end_link_node => {
                // §1562
                {
                    self.check_pdfoutput(1858i32, true);
                    if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                        self.pdf_error(1792i32, 1859i32);
                    }
                    self.new_whatsit(pdf_end_link_node, small_node_size);
                }
            }
            pdf_end_thread_node => {
                // §1570
                {
                    self.check_pdfoutput(1879i32, true);
                    self.new_whatsit(pdf_end_thread_node, small_node_size);
                }
            }
            pdf_font_attr_code => {
                // §1590
                {
                    self.check_pdfoutput(1215i32, true);
                    self.scan_font_ident();
                    k = self.cur_val;
                    if (k == null_font) {
                        self.pdf_error(604i32, 884i32);
                    }
                    self.scan_pdf_ext_toks();
                    { let __v2223 = self.tokens_to_string(self.def_ref); self.pdf_font_attr[crate::ix::U((k) as usize)] = __v2223; }
                }
            }
            pdf_font_expand_code => {
                // §1529
                {
                    if self.bm_on {
                        self.flashtex_bm_font_changed();
                    }
                    // §1536
                    self.read_expand_font();
                }
            }
            pdf_include_chars_code => {
                // §1589
                {
                    self.check_pdfoutput(1891i32, true);
                    self.pdf_include_chars();
                }
            }
            pdf_info_code => {
                // §1579
                {
                    self.check_pdfoutput(1884i32, false);
                    self.scan_pdf_ext_toks();
                    if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
                        self.pdf_info_toks = self.concat_tokens(self.pdf_info_toks, self.def_ref);
                    }
                }
            }
            pdf_literal_node => {
                // §1539
                {
                    self.check_pdfoutput(1795i32, true);
                    if self.scan_keyword(1487i32) {
                        k = pdf_lateliteral_node;
                    } else {
                        k = pdf_literal_node;
                    }
                    self.new_whatsit(k, write_node_size);
                    if self.scan_keyword(901i32) {
                        { let __ix2224 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2224) as usize)].set_hh_lh(direct_always); }
                    } else {
                        if self.scan_keyword(900i32) {
                            { let __ix2225 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2225) as usize)].set_hh_lh(direct_page); }
                        } else {
                            { let __ix2226 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2226) as usize)].set_hh_lh(set_origin); }
                        }
                    }
                    if (k == pdf_literal_node) {
                        self.scan_pdf_ext_toks();
                    } else {
                        self.scan_pdf_ext_late_toks();
                    }
                    { let __ix2227 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2228 = self.def_ref; __av_mem[crate::ix::U((__ix2227) as usize)].set_hh_rh(__v2228); }
                }
            }
            pdf_colorstack_node => {
                // §1540
                {
                    self.check_pdfoutput(1796i32, true);
                    self.scan_int();
                    if (self.cur_val >= self.colorstackused()) {
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
                                self.print(1797i32);
                            }
                            self.print_int(((self.cur_val) as i64));
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1798i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1799i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1800i32;
                            }
                            self.error();
                            self.cur_val = 0i32;
                        }
                    }
                    if (self.cur_val < 0i32) {
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
                                self.print(1801i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1799i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1800i32;
                            }
                            self.error();
                            self.cur_val = 0i32;
                        }
                    }
                    if self.scan_keyword(1802i32) {
                        {
                            i = colorstack_set;
                            j = pdf_colorstack_setter_node_size;
                        }
                    } else {
                        if self.scan_keyword(1803i32) {
                            {
                                i = colorstack_push;
                                j = pdf_colorstack_setter_node_size;
                            }
                        } else {
                            if self.scan_keyword(1804i32) {
                                {
                                    i = colorstack_pop;
                                    j = pdf_colorstack_getter_node_size;
                                }
                            } else {
                                if self.scan_keyword(1805i32) {
                                    {
                                        i = colorstack_current;
                                        j = pdf_colorstack_getter_node_size;
                                    }
                                } else {
                                    {
                                        i = (1i32).wrapping_neg();
                                    }
                                }
                            }
                        }
                    }
                    if (i >= 0i32) {
                        {
                            self.new_whatsit(pdf_colorstack_node, j);
                            { let __ix2229 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2230 = self.cur_val; __av_mem[crate::ix::U((__ix2229) as usize)].set_hh_rh(__v2230); }
                            { let __ix2231 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2231) as usize)].set_hh_lh(i); }
                            if (i <= colorstack_data) {
                                {
                                    self.scan_pdf_ext_toks();
                                    { let __ix2232 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2233 = self.def_ref; __av_mem[crate::ix::U((__ix2232) as usize)].set_hh_rh(__v2233); }
                                }
                            }
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
                                self.print(1806i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 1807i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 1808i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 1809i32;
                            }
                            self.error();
                        }
                    }
                }
            }
            pdf_setmatrix_node => {
                // §1541
                {
                    self.check_pdfoutput(1146i32, true);
                    self.new_whatsit(pdf_setmatrix_node, pdf_setmatrix_node_size);
                    self.scan_pdf_ext_toks();
                    { let __ix2234 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2235 = self.def_ref; __av_mem[crate::ix::U((__ix2234) as usize)].set_hh_rh(__v2235); }
                }
            }
            pdf_save_node => {
                // §1542
                {
                    self.check_pdfoutput(1810i32, true);
                    self.new_whatsit(pdf_save_node, pdf_save_node_size);
                }
            }
            pdf_restore_node => {
                // §1543
                {
                    self.check_pdfoutput(1811i32, true);
                    self.new_whatsit(pdf_restore_node, pdf_restore_node_size);
                }
            }
            pdf_map_file_code => {
                // §1591
                {
                    self.check_pdfoutput(1892i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdfmapfile(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            pdf_map_line_code => {
                // §1592
                {
                    self.check_pdfoutput(1893i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdfmapline(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            pdf_names_code => {
                // §1581
                {
                    self.check_pdfoutput(1888i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdf_names_toks = self.concat_tokens(self.pdf_names_toks, self.def_ref);
                }
            }
            pdf_obj_code => {
                // §1545
                {
                    self.check_pdfoutput(1812i32, true);
                    if self.scan_keyword(1813i32) {
                        {
                            // §470
                            {
                                self.get_x_token();
                                if (self.cur_cmd != spacer) {
                                    self.back_input();
                                }
                            }
                            // §1545
                            self.pdf_obj_count = (self.pdf_obj_count).wrapping_add(1i32);
                            self.pdf_create_obj(obj_type_obj, self.pdf_obj_count);
                            self.pdf_last_obj = self.obj_ptr;
                        }
                    } else {
                        {
                            k = (1i32).wrapping_neg();
                            if self.scan_keyword(1814i32) {
                                {
                                    self.scan_int();
                                    k = self.cur_val;
                                    if (((k <= 0i32) || (k > self.obj_ptr)) || (self.obj_tab[crate::ix::U((k) as usize)].int4 != 0i32)) {
                                        {
                                            self.pdf_warning(1812i32, 1815i32, true, true);
                                            self.pdf_retval = (1i32).wrapping_neg();
                                            k = (1i32).wrapping_neg();
                                        }
                                    }
                                }
                            }
                            if (k < 0i32) {
                                {
                                    self.pdf_obj_count = (self.pdf_obj_count).wrapping_add(1i32);
                                    self.pdf_create_obj(obj_type_obj, self.pdf_obj_count);
                                    k = self.obj_ptr;
                                }
                            }
                            self.obj_tab[crate::ix::U((k) as usize)].int4 = self.pdf_get_mem(pdfmem_obj_size);
                            if self.scan_keyword(1036i32) {
                                {
                                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32)) as usize)] = 1i32;
                                    if self.scan_keyword(1816i32) {
                                        {
                                            self.scan_pdf_ext_toks();
                                            { let __ix2236 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32); let __v2237 = self.def_ref; self.pdf_mem[crate::ix::U((__ix2236) as usize)] = __v2237; }
                                        }
                                    } else {
                                        self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32)) as usize)] = null;
                                    }
                                }
                            } else {
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32)) as usize)] = 0i32;
                            }
                            if self.scan_keyword(889i32) {
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] = 1i32;
                            } else {
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] = 0i32;
                            }
                            self.scan_pdf_ext_toks();
                            { let __ix2238 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(0i32); let __v2239 = self.def_ref; self.pdf_mem[crate::ix::U((__ix2238) as usize)] = __v2239; }
                            self.pdf_last_obj = k;
                        }
                    }
                }
            }
            pdf_outline_code => {
                // §1564
                {
                    self.check_pdfoutput(1860i32, true);
                    if self.scan_keyword(1816i32) {
                        {
                            self.scan_pdf_ext_toks();
                            r = self.def_ref;
                        }
                    } else {
                        r = 0i32;
                    }
                    p = self.scan_action();
                    if self.scan_keyword(546i32) {
                        {
                            self.scan_int();
                            i = self.cur_val;
                        }
                    } else {
                        i = 0i32;
                    }
                    self.scan_pdf_ext_toks();
                    q = self.def_ref;
                    self.pdf_new_obj(obj_type_others, 0i32, 1i32);
                    j = self.obj_ptr;
                    self.write_action(p);
                    self.pdf_end_obj();
                    {
                        if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh() == null) {
                            {
                                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_user) {
                                    self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                } else {
                                    {
                                        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        }
                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_page) {
                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                        } else {
                                            if ((((((__av_mem[crate::ix::U((p) as usize)].hh().b1()) != 0) && ((1i32) != 0))) as i32) == 1i32) {
                                                self.delete_token_ref(__av_mem[crate::ix::U((p) as usize)].hh().rh());
                                            }
                                        }
                                        if ((((((__av_mem[crate::ix::U((p) as usize)].hh().b1()) != 0) && ((2i32) != 0))) as i32) == 2i32) {
                                            self.delete_token_ref(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh());
                                        }
                                    }
                                }
                                self.free_node(p, pdf_action_size);
                            }
                        } else {
                            { let __v2240 = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh()).wrapping_sub(1i32); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh(__v2240); }
                        }
                    }
                    self.pdf_create_obj(obj_type_outline, 0i32);
                    k = self.obj_ptr;
                    self.obj_tab[crate::ix::U((k) as usize)].int4 = self.pdf_get_mem(pdfmem_outline_size);
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(6i32)) as usize)] = j;
                    self.obj_tab[crate::ix::U((k) as usize)].int0 = i;
                    self.pdf_new_obj(obj_type_others, 0i32, 1i32);
                    { let __a2241_0 = self.tokens_to_string(q); self.pdf_print_str_ln(__a2241_0) };
                    self.flush_str(self.last_tokens_string);
                    self.delete_token_ref(q);
                    self.pdf_end_obj();
                    { let __ix2242 = self.obj_tab[crate::ix::U((k) as usize)].int4; let __v2243 = self.obj_ptr; self.pdf_mem[crate::ix::U((__ix2242) as usize)] = __v2243; }
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] = 0i32;
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)] = 0i32;
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(5i32)) as usize)] = 0i32;
                    { let __ix2244 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32); let __v2245 = self.pdf_parent_outline; self.pdf_mem[crate::ix::U((__ix2244) as usize)] = __v2245; }
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(7i32)) as usize)] = r;
                    if (self.pdf_first_outline == 0i32) {
                        self.pdf_first_outline = k;
                    }
                    if (self.pdf_last_outline == 0i32) {
                        {
                            if (self.pdf_parent_outline != 0i32) {
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int4).wrapping_add(4i32)) as usize)] = k;
                            }
                        }
                    } else {
                        {
                            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_last_outline) as usize)].int4).wrapping_add(3i32)) as usize)] = k;
                            { let __ix2246 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32); let __v2247 = self.pdf_last_outline; self.pdf_mem[crate::ix::U((__ix2246) as usize)] = __v2247; }
                        }
                    }
                    self.pdf_last_outline = k;
                    if (self.obj_tab[crate::ix::U((k) as usize)].int0 != 0i32) {
                        {
                            self.pdf_parent_outline = k;
                            self.pdf_last_outline = 0i32;
                        }
                    } else {
                        if ((self.pdf_parent_outline != 0i32) && (self.outline_list_count(k) == (self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int0).wrapping_abs())) {
                            {
                                j = self.pdf_last_outline;
                                loop {
                                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int4).wrapping_add(5i32)) as usize)] = j;
                                    j = self.pdf_parent_outline;
                                    self.pdf_parent_outline = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int4).wrapping_add(1i32)) as usize)];
                                    if ((self.pdf_parent_outline == 0i32) || (self.outline_list_count(j) < (self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int0).wrapping_abs())) { break; }
                                }
                                if (self.pdf_parent_outline == 0i32) {
                                    self.pdf_last_outline = self.pdf_first_outline;
                                } else {
                                    self.pdf_last_outline = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_parent_outline) as usize)].int4).wrapping_add(4i32)) as usize)];
                                }
                                while (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_last_outline) as usize)].int4).wrapping_add(3i32)) as usize)] != 0i32) {
                                    self.pdf_last_outline = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_last_outline) as usize)].int4).wrapping_add(3i32)) as usize)];
                                }
                            }
                        }
                    }
                }
            }
            pdf_refobj_node => {
                // §1547
                {
                    self.check_pdfoutput(1818i32, true);
                    self.scan_int();
                    self.pdf_check_obj(obj_type_obj, self.cur_val);
                    self.new_whatsit(pdf_refobj_node, pdf_refobj_node_size);
                    { let __ix2248 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2249 = self.cur_val; __av_mem[crate::ix::U((__ix2248) as usize)].set_hh_lh(__v2249); }
                }
            }
            pdf_refxform_node => {
                // §1550
                {
                    self.check_pdfoutput(1822i32, true);
                    self.scan_int();
                    self.pdf_check_obj(obj_type_xform, self.cur_val);
                    self.new_whatsit(pdf_refxform_node, pdf_refxform_node_size);
                    { let __ix2250 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2251 = self.cur_val; __av_mem[crate::ix::U((__ix2250) as usize)].set_hh_lh(__v2251); }
                    { let __ix2252 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2253 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(0i32)) as usize)]; __av_mem[crate::ix::U((__ix2252) as usize)].set_int(__v2253); }
                    { let __ix2254 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2255 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((__ix2254) as usize)].set_int(__v2255); }
                    { let __ix2256 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2257 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(2i32)) as usize)]; __av_mem[crate::ix::U((__ix2256) as usize)].set_int(__v2257); }
                    if self.bm_rec_on {
                        self.flashtex_bm_obj_ref(obj_type_xform, self.cur_val);
                    }
                }
            }
            pdf_refximage_node => {
                // §1555
                {
                    self.check_pdfoutput(1838i32, true);
                    self.scan_int();
                    self.pdf_check_obj(obj_type_ximage, self.cur_val);
                    self.new_whatsit(pdf_refximage_node, pdf_refximage_node_size);
                    { let __ix2258 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2259 = self.cur_val; __av_mem[crate::ix::U((__ix2258) as usize)].set_hh_lh(__v2259); }
                    { let __ix2260 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2261 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(0i32)) as usize)]; __av_mem[crate::ix::U((__ix2260) as usize)].set_int(__v2261); }
                    { let __ix2262 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2263 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((__ix2262) as usize)].set_int(__v2263); }
                    { let __ix2264 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2265 = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(2i32)) as usize)]; __av_mem[crate::ix::U((__ix2264) as usize)].set_int(__v2265); }
                    if self.bm_rec_on {
                        self.flashtex_bm_obj_ref(obj_type_ximage, self.cur_val);
                    }
                }
            }
            pdf_save_pos_node => {
                // §1577
                {
                    self.new_whatsit(pdf_save_pos_node, small_node_size);
                }
            }
            pdf_snap_ref_point_node => {
                // §1573
                {
                    self.check_pdfoutput(1880i32, true);
                    self.new_whatsit(pdf_snap_ref_point_node, small_node_size);
                }
            }
            pdf_snapy_comp_node => {
                // §1576
                {
                    self.check_pdfoutput(1883i32, true);
                    self.new_whatsit(pdf_snapy_comp_node, small_node_size);
                    self.scan_int();
                    { let __ix2266 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2267 = self.fix_int(self.cur_val, 0i32, 1000i32); __av_mem[crate::ix::U((__ix2266) as usize)].set_int(__v2267); }
                }
            }
            pdf_snapy_node => {
                // §1575
                {
                    self.check_pdfoutput(1882i32, true);
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix2268 = self.cur_list.tail_field; let __v2269 = self.new_snap_node(pdf_snapy_node); __av_mem[crate::ix::U((__ix2268) as usize)].set_hh_rh(__v2269); }
                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                }
            }
            pdf_start_link_node => {
                // §1561
                {
                    self.check_pdfoutput(1856i32, true);
                    if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                        self.pdf_error(1792i32, 1857i32);
                    }
                    k = self.pdf_new_objnum();
                    self.new_annot_whatsit(pdf_start_link_node, pdf_annot_node_size);
                    { let __ix2270 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2271 = self.scan_action(); __av_mem[crate::ix::U((__ix2270) as usize)].set_hh_rh(__v2271); }
                    { let __ix2272 = (self.cur_list.tail_field).wrapping_add(6i32); __av_mem[crate::ix::U((__ix2272) as usize)].set_int(k); }
                    self.pdf_last_link = k;
                    if self.bm_rec_on {
                        self.flashtex_bm_new_obj(k);
                    }
                }
            }
            pdf_start_thread_node => {
                // §1569
                {
                    self.check_pdfoutput(1878i32, true);
                    self.new_annot_whatsit(pdf_start_thread_node, pdf_thread_node_size);
                    self.scan_thread_id();
                }
            }
            pdf_thread_node => {
                // §1568
                {
                    self.check_pdfoutput(1877i32, true);
                    self.new_annot_whatsit(pdf_thread_node, pdf_thread_node_size);
                    self.scan_thread_id();
                }
            }
            pdf_trailer_code => {
                // §1582
                {
                    self.check_pdfoutput(1889i32, false);
                    self.scan_pdf_ext_toks();
                    if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
                        self.pdf_trailer_toks = self.concat_tokens(self.pdf_trailer_toks, self.def_ref);
                    }
                }
            }
            pdf_trailer_id_code => {
                // §1583
                {
                    self.check_pdfoutput(1890i32, false);
                    self.scan_pdf_ext_toks();
                    if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
                        self.pdf_trailer_id_toks = self.concat_tokens(self.pdf_trailer_id_toks, self.def_ref);
                    }
                }
            }
            pdf_xform_code => {
                // §1549
                {
                    self.check_pdfoutput(1819i32, true);
                    self.pdf_xform_count = (self.pdf_xform_count).wrapping_add(1i32);
                    self.pdf_create_obj(obj_type_xform, self.pdf_xform_count);
                    k = self.obj_ptr;
                    self.obj_tab[crate::ix::U((k) as usize)].int4 = self.pdf_get_mem(pdfmem_xform_size);
                    if self.scan_keyword(1816i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __ix2273 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32); let __v2274 = self.def_ref; self.pdf_mem[crate::ix::U((__ix2273) as usize)] = __v2274; }
                        }
                    } else {
                        self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)] = null;
                    }
                    if self.scan_keyword(1820i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __ix2275 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(5i32); let __v2276 = self.def_ref; self.pdf_mem[crate::ix::U((__ix2275) as usize)] = __v2276; }
                        }
                    } else {
                        self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(5i32)) as usize)] = null;
                    }
                    self.scan_register_num();
                    {
                        if self.bm_rec_on {
                            self.flashtex_bm_box(self.cur_val);
                        }
                        if (self.cur_val < 256i32) {
                            p = __av_eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr == null) {
                                    p = null;
                                } else {
                                    p = __av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    if (p == null) {
                        self.pdf_error(1792i32, 1821i32);
                    }
                    { let __ix2277 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(0i32); let __v2278 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2277) as usize)] = __v2278; }
                    { let __ix2279 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32); let __v2280 = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2279) as usize)] = __v2280; }
                    { let __ix2281 = (self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32); let __v2282 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.pdf_mem[crate::ix::U((__ix2281) as usize)] = __v2282; }
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] = p;
                    if (self.cur_val < 256i32) {
                        { let __ix2283 = (box_base).wrapping_add(self.cur_val); __av_eqtb[crate::ix::U(((__ix2283) - 1) as usize)].set_hh_rh(null); }
                    } else {
                        {
                            self.find_sa_element(box_val, self.cur_val, false);
                            if (self.cur_ptr != null) {
                                {
                                    { let __ix2284 = (self.cur_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2284) as usize)].set_hh_rh(null); }
                                    { let __ix2285 = (self.cur_ptr).wrapping_add(1i32); let __v2286 = (__av_mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2285) as usize)].set_hh_lh(__v2286); }
                                    self.delete_sa_ref(self.cur_ptr);
                                }
                            }
                        }
                    }
                    self.pdf_last_xform = k;
                }
            }
            pdf_ximage_code => {
                // §1554
                {
                    self.check_pdfoutput(1837i32, true);
                    self.check_pdfversion();
                    self.scan_image();
                }
            }
            reset_timer_code => {
                // §1587
                {
                    { let mut __f0 = ::core::mem::take(&mut self.epochseconds); let mut __f1 = ::core::mem::take(&mut self.microseconds); let __r = self.seconds_and_micros(&mut __f0, &mut __f1); self.epochseconds = __f0; self.microseconds = __f1; __r };
                }
            }
            set_random_seed_code => {
                // §1586
                {
                    self.scan_int();
                    if (self.cur_val < 0i32) {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                    self.random_seed = self.cur_val;
                    self.init_randoms(self.random_seed);
                }
            }
            pdf_glyph_to_unicode_code => {
                // §1593
                {
                    self.glyph_to_unicode();
                }
            }
            pdf_nobuiltin_tounicode_code => {
                // §1594
                {
                    self.check_pdfoutput(1894i32, true);
                    self.scan_font_ident();
                    k = self.cur_val;
                    if (k == null_font) {
                        self.pdf_error(604i32, 884i32);
                    }
                    { let __v2287 = true; self.pdf_font_nobuiltin_tounicode[crate::ix::U((k) as usize)] = __v2287; }
                }
            }
            pdf_interword_space_on_node => {
                // §1595
                {
                    self.check_pdfoutput(1895i32, true);
                    self.new_whatsit(pdf_interword_space_on_node, small_node_size);
                }
            }
            pdf_interword_space_off_node => {
                // §1596
                {
                    self.check_pdfoutput(1896i32, true);
                    self.new_whatsit(pdf_interword_space_off_node, small_node_size);
                }
            }
            pdf_fake_space_node => {
                // §1597
                {
                    self.check_pdfoutput(1897i32, true);
                    self.new_whatsit(pdf_fake_space_node, small_node_size);
                }
            }
            pdf_running_link_off_node => {
                // §1598
                {
                    self.check_pdfoutput(1898i32, true);
                    self.new_whatsit(pdf_running_link_off_node, small_node_size);
                }
            }
            pdf_running_link_on_node => {
                // §1599
                {
                    self.check_pdfoutput(1899i32, true);
                    self.new_whatsit(pdf_running_link_on_node, small_node_size);
                }
            }
            pdf_space_font_code => {
                // §1600
                {
                    self.check_pdfoutput(1900i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdf_space_font_name = self.tokens_to_string(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            _ => {
                // §1529
                self.confusion(1792i32);
            }
        }
    }

    /// The \.{\\language} extension is somewhat different.
    /// We need a subroutine that comes into play when a character of
    /// a non-`clang` language is being appended to the current paragraph.
    /// @<Declare action...
    // §1625
    pub fn fix_language(&mut self) {
        let mut l: ASCII_code = 0; // §1625
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (__av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() <= 0i32) {
            l = 0i32;
        } else {
            if (__av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() > 255i32) {
                l = 0i32;
            } else {
                l = __av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int();
            }
        }
        if (l != self.cur_list.aux_field.hh().rh()) {
            {
                self.new_whatsit(language_node, small_node_size);
                { let __ix2288 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2288) as usize)].set_hh_rh(l); }
                self.cur_list.aux_field.set_hh_rh(l);
                { let __ix2289 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2290 = self.norm_min(__av_eqtb[crate::ix::U(((29328i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix2289) as usize)].set_hh_b0(__v2290); }
                { let __ix2291 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2292 = self.norm_min(__av_eqtb[crate::ix::U(((29329i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix2291) as usize)].set_hh_b1(__v2292); }
            }
        }
    }

    /// @<Declare the procedure called `handle_right_brace`
    // §1247
    pub fn handle_right_brace(&mut self) {
        let mut p: halfword = 0; // §1247
        let mut q: halfword = 0; // §1247
        let mut d: scaled = 0; // §1247
        let mut f: i32 = 0; // §1247
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        match self.cur_group {
            simple_group => {
                self.unsave();
            }
            bottom_level => {
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
                        self.print(1469i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1470i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1471i32;
                    }
                    self.error();
                }
            }
            semi_simple_group | math_shift_group | math_left_group => {
                self.extra_right_brace();
            }
            hbox_group => {
                // §1264
                self.package(0i32);
            }
            adjusted_hbox_group => {
                {
                    self.adjust_tail = adjust_head;
                    self.pre_adjust_tail = pre_adjust_head;
                    self.package(0i32);
                }
            }
            vbox_group => {
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 0i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        self.package(0i32);
                    }
                }
            }
            vtop_group => {
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 0i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        self.package(vtop_code);
                    }
                }
            }
            insert_group => {
                // §1279
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 1i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        q = __av_eqtb[crate::ix::U(((26638i32) - 1) as usize)].hh().rh();
                        { let __v2293 = (__av_mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v2293); }
                        d = __av_eqtb[crate::ix::U(((29909i32) - 1) as usize)].int();
                        f = __av_eqtb[crate::ix::U(((29319i32) - 1) as usize)].int();
                        self.unsave();
                        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                        p = self.vpackage(__av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), 0i32, additional, max_dimen);
                        self.pop_nest();
                        if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int() < 255i32) {
                            {
                                {
                                    self.prev_tail = self.cur_list.tail_field;
                                    { let __ix2294 = self.cur_list.tail_field; let __v2295 = self.get_node(ins_node_size); __av_mem[crate::ix::U((__ix2294) as usize)].set_hh_rh(__v2295); }
                                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                }
                                { let __ix2296 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2296) as usize)].set_hh_b0(ins_node); }
                                { let __ix2297 = self.cur_list.tail_field; let __v2298 = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2297) as usize)].set_hh_b1(__v2298); }
                                { let __ix2299 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2300 = (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()); __av_mem[crate::ix::U((__ix2299) as usize)].set_int(__v2300); }
                                { let __ix2301 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2302 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix2301) as usize)].set_hh_lh(__v2302); }
                                { let __ix2303 = (self.cur_list.tail_field).wrapping_add(4i32); __av_mem[crate::ix::U((__ix2303) as usize)].set_hh_rh(q); }
                                { let __ix2304 = (self.cur_list.tail_field).wrapping_add(2i32); __av_mem[crate::ix::U((__ix2304) as usize)].set_int(d); }
                                { let __ix2305 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2305) as usize)].set_int(f); }
                            }
                        } else {
                            {
                                {
                                    self.prev_tail = self.cur_list.tail_field;
                                    { let __ix2306 = self.cur_list.tail_field; let __v2307 = self.get_node(small_node_size); __av_mem[crate::ix::U((__ix2306) as usize)].set_hh_rh(__v2307); }
                                    self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                }
                                { let __ix2308 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2308) as usize)].set_hh_b0(adjust_node); }
                                { let __ix2309 = self.cur_list.tail_field; let __v2310 = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2309) as usize)].set_hh_b1(__v2310); }
                                { let __ix2311 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2312 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix2311) as usize)].set_int(__v2312); }
                                self.delete_glue_ref(q);
                            }
                        }
                        self.free_node(p, box_node_size);
                        if (self.nest_ptr == 0i32) {
                            self.build_page();
                        }
                    }
                }
            }
            output_group => {
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 1i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    // §1204
                    {
                        while (((self.cur_input.state_field == token_list) && (self.cur_input.loc_field == null)) && (self.cur_input.index_field == backed_up)) {
                            self.end_token_list();
                        }
                        if (((self.cur_input.state_field != token_list) || (self.cur_input.loc_field != null)) || (self.cur_input.index_field != output_text)) {
                            // §1205
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
                                    self.print(688i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1438i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1439i32;
                                }
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_input.loc_field == null) { break; }
                                }
                            }
                        }
                        // §1204
                        self.output_can_end = true;
                        self.end_token_list();
                        self.output_can_end = false;
                        self.end_graf();
                        self.unsave();
                        self.output_active = false;
                        self.insert_penalties = 0i32;
                        // §1206
                        if (__av_eqtb[crate::ix::U(((27688i32) - 1) as usize)].hh().rh() != null) {
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
                                    self.print(1440i32);
                                }
                                self.print_esc(430i32);
                                self.print_int(((255i32) as i64));
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 1441i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 1442i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 1443i32;
                                }
                                self.box_error(255i32);
                            }
                        }
                        // §1204
                        if (self.cur_list.tail_field != self.cur_list.head_field) {
                            {
                                { let __ix2313 = self.page_tail; let __v2314 = __av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix2313) as usize)].set_hh_rh(__v2314); }
                                self.page_tail = self.cur_list.tail_field;
                            }
                        }
                        if (__av_mem[crate::ix::U((page_head) as usize)].hh().rh() != null) {
                            {
                                if (__av_mem[crate::ix::U((contrib_head) as usize)].hh().rh() == null) {
                                    self.nest[crate::ix::U((0i32) as usize)].tail_field = self.page_tail;
                                }
                                { let __ix2315 = self.page_tail; let __v2316 = __av_mem[crate::ix::U((contrib_head) as usize)].hh().rh(); __av_mem[crate::ix::U((__ix2315) as usize)].set_hh_rh(__v2316); }
                                { let __v2317 = __av_mem[crate::ix::U((page_head) as usize)].hh().rh(); __av_mem[crate::ix::U((contrib_head) as usize)].set_hh_rh(__v2317); }
                                __av_mem[crate::ix::U((page_head) as usize)].set_hh_rh(null);
                                self.page_tail = page_head;
                            }
                        }
                        self.flush_node_list(self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)]);
                        self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = null;
                        self.pop_nest();
                        self.build_page();
                    }
                }
            }
            disc_group => {
                // §1297
                self.build_discretionary();
            }
            align_group => {
                // §1311
                {
                    self.back_input();
                    self.cur_tok = 19610i32;
                    {
                        self.dg_mark();
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(723i32);
                    }
                    self.print_esc(1322i32);
                    self.print(724i32);
                    {
                        self.help_ptr = 1i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1553i32;
                    }
                    self.ins_error();
                }
            }
            no_align_group => {
                // §1312
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 1i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        self.unsave();
                        self.align_peek();
                    }
                }
            }
            vcenter_group => {
                // §1347
                if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 0i32) && (self.cur_list.mode_field == hmode)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        self.unsave();
                        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                        p = self.vpackage(__av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)].int(), self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(), max_dimen);
                        self.pop_nest();
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2318 = self.cur_list.tail_field; let __v2319 = self.new_noad(); __av_mem[crate::ix::U((__ix2318) as usize)].set_hh_rh(__v2319); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                        { let __ix2320 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2320) as usize)].set_hh_b0(vcenter_noad); }
                        { let __ix2321 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2321) as usize)].set_hh_rh(sub_box); }
                        { let __ix2322 = (self.cur_list.tail_field).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2322) as usize)].set_hh_lh(p); }
                    }
                }
            }
            math_choice_group => {
                // §1352
                self.build_choices();
            }
            math_group => {
                // §1365
                {
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    { let __ix2323 = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2323) as usize)].set_hh_rh(sub_mlist); }
                    p = self.fin_mlist(null);
                    { let __ix2324 = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2324) as usize)].set_hh_lh(p); }
                    if (p != null) {
                        if (__av_mem[crate::ix::U((p) as usize)].hh().rh() == null) {
                            if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == ord_noad) {
                                {
                                    if (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                                        if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh() == empty) {
                                            {
                                                { let __ix2325 = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int(); let __v2326 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh(); __av_mem[crate::ix::U((__ix2325) as usize)].set_hh(__v2326); }
                                                self.free_node(p, noad_size);
                                            }
                                        }
                                    }
                                }
                            } else {
                                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == accent_noad) {
                                    if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int() == (self.cur_list.tail_field).wrapping_add(1i32)) {
                                        if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == ord_noad) {
                                            // §1366
                                            {
                                                q = self.cur_list.head_field;
                                                while (__av_mem[crate::ix::U((q) as usize)].hh().rh() != self.cur_list.tail_field) {
                                                    q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                }
                                                __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                self.free_node(self.cur_list.tail_field, noad_size);
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
                // §1247
                self.confusion(1472i32);
            }
        }
    }

    /// We shall concentrate first on the inner loop of `main_control`, deferring
    /// consideration of the other cases until later.
    // §1208
    pub fn main_control(&mut self) {
        let mut t: i32 = 0; // §1208
        let mut tmp_k1: halfword = 0; // §1208
        let mut tmp_k2: halfword = 0; // §1208
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // goto labels: L60, reswitch, L70, L80, L90, L91, L92, L100, L101, L110, L111, L112, L95, L120, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if self.ckpt_resuming {
                    self.ckpt_resuming = false;
                } else {
                    if (__av_eqtb[crate::ix::U(((every_job_loc) - 1) as usize)].hh().rh() != null) {
                        self.begin_token_list(__av_eqtb[crate::ix::U(((every_job_loc) - 1) as usize)].hh().rh(), every_job_text);
                    }
                }
            }
            if __goto_1 <= 1 { // L60
                if (self.ckpt_request != 0i32) {
                    self.flashtex_checkpoint_hook();
                }
                if self.intr_on {
                    {
                        if self.intr_rec_on {
                            self.flashtex_intr_switch();
                        }
                        self.intr_at_switch = true;
                    }
                }
                if self.bm_on {
                    {
                        if self.bm_rec_on {
                            self.flashtex_bm_switch();
                        }
                        self.bm_at_switch = true;
                    }
                }
                self.get_x_token();
                self.intr_at_switch = false;
                self.bm_at_switch = false;
            }
            if __goto_1 <= 2 { // reswitch
                if (self.interrupt != 0i32) {
                    // §1209
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
                if (__av_eqtb[crate::ix::U(((29313i32) - 1) as usize)].int() > 0i32) {
                    self.show_cur_cmd_chr();
                }
                // §1208
                if self.intr_rec_on {
                    self.flashtex_intr_command();
                }
                if self.bm_rec_on {
                    self.flashtex_bm_command();
                }
                match ((self.cur_list.mode_field).wrapping_abs()).wrapping_add(self.cur_cmd) {
                    116 | 117 | 173 => {
                        { __goto_1 = 3; continue 'l_dispatch_1; }
                    }
                    121 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            { __goto_1 = 3; continue 'l_dispatch_1; }
                        }
                    }
                    170 => {
                        {
                            self.get_x_token();
                            if ((((self.cur_cmd == letter) || (self.cur_cmd == other_char)) || (self.cur_cmd == char_given)) || (self.cur_cmd == char_num)) {
                                self.cancel_boundary = true;
                            }
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    115 => {
                        if ((self.cur_list.aux_field.hh().lh() == 1000i32) || (__av_eqtb[crate::ix::U(((29364i32) - 1) as usize)].int() > 0i32)) {
                            { __goto_1 = 14; continue 'l_dispatch_1; }
                        } else {
                            self.app_space();
                        }
                    }
                    169 | 273 => {
                        { __goto_1 = 14; continue 'l_dispatch_1; }
                    }
                    1 | 105 | 209 | 11 | 219 | 274 => {
                        // §1224
                    }
                    40 | 144 | 248 => {
                        {
                            if (self.cur_chr == 0i32) {
                                {
                                    // §433
                                    loop {
                                        self.get_x_token();
                                        if (self.cur_cmd != spacer) { break; }
                                    }
                                    // §1224
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            } else {
                                {
                                    t = self.scanner_status;
                                    self.scanner_status = normal;
                                    self.get_next();
                                    self.scanner_status = t;
                                    if (self.cur_cs < hash_base) {
                                        self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                    } else {
                                        self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                                    }
                                    if (self.cur_cs != undefined_primitive) {
                                        {
                                            self.cur_cmd = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                            self.cur_chr = __av_eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                            self.cur_tok = (19621i32).wrapping_add(self.cur_cs);
                                            { __goto_1 = 2; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    15 => {
                        if self.its_all_over() {
                            { __goto_1 = 15; continue 'l_dispatch_1; }
                        }
                    }
                    23 | 126 | 230 | 71 | 175 | 279 | 39 | 45 | 49 | 153 | 7 | 111 | 215 => {
                        self.report_illegal_case();
                    }
                    8 | 112 | 9 | 113 | 18 | 122 | 70 | 174 | 51 | 155 | 16 | 120 | 50 | 154 | 53 | 157 | 67 | 171 | 54 | 158 | 55 | 159 | 57 | 161 | 56 | 160 | 31 | 135 | 52 | 156 | 29 | 133 | 47 | 151 | 218 | 222 | 223 | 236 | 233 | 242 | 245 => {
                        self.insert_dollar_sign();
                    }
                    37 | 140 | 244 => {
                        // §1235
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2327 = self.cur_list.tail_field; let __v2328 = self.scan_rule_spec(); __av_mem[crate::ix::U((__ix2327) as usize)].set_hh_rh(__v2328); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                            if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                                { let __v2329 = __av_eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v2329); }
                            } else {
                                if ((self.cur_list.mode_field).wrapping_abs() == hmode) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                }
                            }
                        }
                    }
                    28 | 131 | 235 | 237 => {
                        // §1236
                        self.append_glue();
                    }
                    30 | 134 | 238 | 239 => {
                        self.append_kern();
                    }
                    2 | 106 => {
                        // §1242
                        self.new_save_level(simple_group);
                    }
                    62 | 166 | 270 => {
                        self.new_save_level(semi_simple_group);
                    }
                    63 | 167 | 271 => {
                        if (self.cur_group == semi_simple_group) {
                            self.unsave();
                        } else {
                            self.off_save();
                        }
                    }
                    3 | 107 | 211 => {
                        // §1246
                        self.handle_right_brace();
                    }
                    22 | 127 | 231 => {
                        // §1252
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
                    32 | 136 | 240 => {
                        self.scan_box((1073807261i32).wrapping_add(self.cur_chr));
                    }
                    21 | 125 | 229 => {
                        self.begin_box(0i32);
                    }
                    44 => {
                        // §1269
                        self.new_graf((self.cur_chr > 0i32));
                    }
                    12 | 13 | 17 | 69 | 4 | 24 | 36 | 46 | 48 | 27 | 34 | 65 | 66 => {
                        {
                            self.back_input();
                            self.new_graf(true);
                        }
                    }
                    148 | 252 => {
                        // §1271
                        if (self.cur_chr != 2i32) {
                            self.indent_in_hmode();
                        }
                    }
                    14 => {
                        // §1273
                        {
                            self.normal_paragraph();
                            if (self.cur_list.mode_field > 0i32) {
                                self.build_page();
                            }
                        }
                    }
                    118 => {
                        {
                            if (self.align_state < 0i32) {
                                self.off_save();
                            }
                            self.end_graf();
                            if (self.cur_list.mode_field == vmode) {
                                self.build_page();
                            }
                        }
                    }
                    119 | 132 | 141 | 129 | 137 => {
                        self.head_for_vmode();
                    }
                    38 | 142 | 246 | 143 | 247 => {
                        // §1276
                        self.begin_insert_or_adjust();
                    }
                    19 | 123 | 227 => {
                        self.make_mark();
                    }
                    43 | 147 | 251 => {
                        // §1281
                        self.append_penalty();
                    }
                    26 | 130 | 234 => {
                        // §1283
                        self.delete_last();
                    }
                    25 | 128 | 232 => {
                        // §1288
                        self.unpackage();
                    }
                    149 => {
                        // §1291
                        self.append_italic_correction();
                    }
                    253 => {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2330 = self.cur_list.tail_field; let __v2331 = self.new_kern(0i32); __av_mem[crate::ix::U((__ix2330) as usize)].set_hh_rh(__v2331); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                    }
                    152 | 256 => {
                        // §1295
                        self.append_discretionary();
                    }
                    150 => {
                        // §1301
                        self.make_accent();
                    }
                    6 | 110 | 214 | 5 | 109 | 213 => {
                        // §1305
                        self.align_error();
                    }
                    35 | 139 | 243 => {
                        self.no_align_error();
                    }
                    64 | 168 | 272 => {
                        self.omit_error();
                    }
                    33 => {
                        // §1309
                        self.init_align();
                    }
                    138 => {
                        // §1704
                        if (self.cur_chr > 0i32) {
                            {
                                if self.eTeX_enabled((__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32), self.cur_cmd, self.cur_chr) {
                                    {
                                        self.prev_tail = self.cur_list.tail_field;
                                        { let __ix2332 = self.cur_list.tail_field; let __v2333 = self.new_math(0i32, self.cur_chr); __av_mem[crate::ix::U((__ix2332) as usize)].set_hh_rh(__v2333); }
                                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                    }
                                }
                            }
                        } else {
                            // §1309
                            self.init_align();
                        }
                    }
                    241 => {
                        if self.privileged() {
                            if (self.cur_group == math_shift_group) {
                                self.init_align();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    10 | 114 => {
                        if ((__av_eqtb[crate::ix::U(((29336i32) - 1) as usize)].int() > 1i32) && (self.cur_list.mode_field == hmode)) {
                            {
                                self.back_input();
                                self.cur_tok = self.par_token;
                                self.back_input();
                                self.cur_input.index_field = inserted;
                            }
                        } else {
                            self.do_endv();
                        }
                    }
                    68 | 172 | 276 => {
                        // §1313
                        self.cs_error();
                    }
                    108 => {
                        // §1316
                        self.init_math();
                    }
                    257 => {
                        // §1319
                        if self.privileged() {
                            if (self.cur_group == math_shift_group) {
                                self.start_eq_no();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    210 => {
                        // §1329
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2334 = self.cur_list.tail_field; let __v2335 = self.new_noad(); __av_mem[crate::ix::U((__ix2334) as usize)].set_hh_rh(__v2335); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                            self.back_input();
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    220 | 221 | 277 => {
                        // §1333
                        self.set_math_char(__av_eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh());
                    }
                    225 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.set_math_char(__av_eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh());
                        }
                    }
                    226 => {
                        {
                            self.scan_fifteen_bit_int();
                            self.set_math_char(self.cur_val);
                        }
                    }
                    278 => {
                        self.set_math_char(self.cur_chr);
                    }
                    224 => {
                        {
                            self.scan_twenty_seven_bit_int();
                            self.set_math_char((self.cur_val / 4096i32));
                        }
                    }
                    259 => {
                        // §1337
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2336 = self.cur_list.tail_field; let __v2337 = self.new_noad(); __av_mem[crate::ix::U((__ix2336) as usize)].set_hh_rh(__v2337); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                            { let __ix2338 = self.cur_list.tail_field; let __v2339 = self.cur_chr; __av_mem[crate::ix::U((__ix2338) as usize)].set_hh_b0(__v2339); }
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    260 => {
                        self.math_limit_switch();
                    }
                    275 => {
                        // §1341
                        self.math_radical();
                    }
                    254 | 255 => {
                        // §1343
                        self.math_ac();
                    }
                    265 => {
                        // §1346
                        {
                            self.scan_spec(vcenter_group, false);
                            self.normal_paragraph();
                            self.push_nest();
                            self.cur_list.mode_field = (1i32).wrapping_neg();
                            { let __v2340 = __av_eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v2340); }
                            if (__av_eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh() != null) {
                                self.begin_token_list(__av_eqtb[crate::ix::U(((every_vbox_loc) - 1) as usize)].hh().rh(), every_vbox_text);
                            }
                        }
                    }
                    262 => {
                        // §1350
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2341 = self.cur_list.tail_field; let __v2342 = self.new_style(self.cur_chr); __av_mem[crate::ix::U((__ix2341) as usize)].set_hh_rh(__v2342); }
                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                        }
                    }
                    264 => {
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2343 = self.cur_list.tail_field; let __v2344 = self.new_glue(zero_glue); __av_mem[crate::ix::U((__ix2343) as usize)].set_hh_rh(__v2344); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                            { let __ix2345 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2345) as usize)].set_hh_b1(cond_math_glue); }
                        }
                    }
                    263 => {
                        self.append_choices();
                    }
                    217 | 216 => {
                        // §1354
                        self.sub_sup();
                    }
                    261 => {
                        // §1359
                        self.math_fraction();
                    }
                    258 => {
                        // §1369
                        self.math_left_right();
                    }
                    212 => {
                        // §1372
                        if (self.cur_group == math_shift_group) {
                            self.after_math();
                        } else {
                            self.off_save();
                        }
                    }
                    72 | 176 | 280 | 73 | 177 | 281 | 74 | 178 | 282 | 75 | 179 | 283 | 76 | 180 | 284 | 77 | 181 | 285 | 78 | 182 | 286 | 79 | 183 | 287 | 80 | 184 | 288 | 81 | 185 | 289 | 82 | 186 | 290 | 83 | 187 | 291 | 84 | 188 | 292 | 85 | 189 | 293 | 86 | 190 | 294 | 87 | 191 | 295 | 88 | 192 | 296 | 89 | 193 | 297 | 102 | 206 | 310 | 103 | 207 | 311 | 90 | 194 | 298 | 91 | 195 | 299 | 92 | 196 | 300 | 93 | 197 | 301 | 94 | 198 | 302 | 95 | 199 | 303 | 96 | 200 | 304 | 97 | 201 | 305 | 98 | 202 | 306 | 99 | 203 | 307 | 100 | 204 | 308 | 101 | 205 | 309 => {
                        // §1389
                        self.prefixed_command();
                    }
                    41 | 145 | 249 => {
                        // §1447
                        {
                            self.get_token();
                            self.after_token = self.cur_tok;
                        }
                    }
                    42 | 146 | 250 => {
                        // §1450
                        {
                            self.get_token();
                            self.save_for_after(self.cur_tok);
                        }
                    }
                    104 | 208 | 312 => {
                        {
                            self.get_token();
                            if (self.cur_cs > 0i32) {
                                {
                                    self.par_loc = self.cur_cs;
                                    self.par_token = self.cur_tok;
                                }
                            }
                        }
                    }
                    61 | 165 | 269 => {
                        // §1453
                        self.open_or_close_in();
                    }
                    59 | 163 | 267 => {
                        // §1455
                        self.issue_message();
                    }
                    58 | 162 | 266 => {
                        // §1464
                        self.shift_case();
                    }
                    20 | 124 | 228 => {
                        // §1469
                        self.show_whatever();
                    }
                    60 | 164 | 268 => {
                        // §1528
                        self.do_extension();
                    }
                    _ => {}
                }
                // §1208
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 3 { // L70
                self.main_s = __av_eqtb[crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                // §1212
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v2346 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2346); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v2347 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2347); }
                        }
                    }
                }
                self.save_tail = null;
                self.main_f = __av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
                self.bchar = self.font_bchar[crate::ix::U((self.main_f) as usize)];
                self.false_bchar = self.font_false_bchar[crate::ix::U((self.main_f) as usize)];
                if (self.cur_list.mode_field > 0i32) {
                    if (__av_eqtb[crate::ix::U(((29327i32) - 1) as usize)].int() != self.cur_list.aux_field.hh().rh()) {
                        self.fix_language();
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == null) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail = __av_mem[crate::ix::U((self.lig_stack) as usize)].hh().rh();
                            { let __ix2348 = self.lig_stack; __av_mem[crate::ix::U((__ix2348) as usize)].set_hh_rh(null); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                            self.dl_new_node(self.lig_stack);
                        }
                    }
                }
                { let __ix2349 = self.lig_stack; let __v2350 = self.main_f; __av_mem[crate::ix::U((__ix2349) as usize)].set_hh_b0(__v2350); }
                self.cur_l = self.cur_chr;
                { let __ix2351 = self.lig_stack; let __v2352 = self.cur_l; __av_mem[crate::ix::U((__ix2351) as usize)].set_hh_b1(__v2352); }
                self.cur_q = self.cur_list.tail_field;
                tmp_k1 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                // §1218
                if (tmp_k1 != null) {
                    {
                        if (self.cur_l < non_char) {
                            {
                                if (__av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh() > null) {
                                    if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == self.hyphen_char[crate::ix::U((self.main_f) as usize)]) {
                                        self.ins_disc = true;
                                    }
                                }
                                if self.ligature_present {
                                    {
                                        self.main_p = self.new_ligature(self.main_f, self.cur_l, __av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                        if self.lft_hit {
                                            {
                                                { let __ix2353 = self.main_p; __av_mem[crate::ix::U((__ix2353) as usize)].set_hh_b1(2i32); }
                                                self.lft_hit = false;
                                            }
                                        }
                                        if self.rt_hit {
                                            if (self.lig_stack == null) {
                                                {
                                                    { let __ix2354 = self.main_p; let __v2355 = (__av_mem[crate::ix::U((self.main_p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2354) as usize)].set_hh_b1(__v2355); }
                                                    self.rt_hit = false;
                                                }
                                            }
                                        }
                                        if (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) {
                                            tmp_k2 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                                        } else {
                                            tmp_k2 = null;
                                        }
                                        if (tmp_k2 == null) {
                                            {
                                                { let __ix2356 = self.cur_q; let __v2357 = self.main_p; __av_mem[crate::ix::U((__ix2356) as usize)].set_hh_rh(__v2357); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        } else {
                                            {
                                                { let __ix2358 = self.cur_q; __av_mem[crate::ix::U((__ix2358) as usize)].set_hh_rh(tmp_k2); }
                                                { let __v2359 = self.main_p; __av_mem[crate::ix::U((tmp_k2) as usize)].set_hh_rh(__v2359); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        }
                                    }
                                }
                                if self.ins_disc {
                                    {
                                        self.ins_disc = false;
                                        if (self.cur_list.mode_field > 0i32) {
                                            {
                                                self.prev_tail = self.cur_list.tail_field;
                                                { let __ix2360 = self.cur_list.tail_field; let __v2361 = self.new_disc(); __av_mem[crate::ix::U((__ix2360) as usize)].set_hh_rh(__v2361); }
                                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.save_tail = self.cur_list.tail_field;
                        if ((((((!(self.cur_list.tail_field >= self.hi_mem_min)) && (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == disc_node)) && (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == 0i32)) && (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (__av_mem[crate::ix::U((self.prev_tail) as usize)].hh().rh() == self.cur_list.tail_field)) {
                            {
                                {
                                    { let __ix2362 = self.prev_tail; __av_mem[crate::ix::U((__ix2362) as usize)].set_hh_rh(tmp_k1); }
                                    { let __v2363 = self.cur_list.tail_field; __av_mem[crate::ix::U((tmp_k1) as usize)].set_hh_rh(__v2363); }
                                    self.prev_tail = tmp_k1;
                                }
                            }
                        } else {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2364 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2364) as usize)].set_hh_rh(tmp_k1); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                        }
                        { __goto_1 = 5; continue 'l_dispatch_1; }
                    }
                }
                // §1212
                if self.cancel_boundary {
                    {
                        self.cancel_boundary = false;
                        self.main_k = non_address;
                    }
                } else {
                    self.main_k = self.bchar_label[crate::ix::U((self.main_f) as usize)];
                }
                if (self.main_k == non_address) {
                    { __goto_1 = 7; continue 'l_dispatch_1; }
                }
                self.cur_r = self.cur_l;
                self.cur_l = non_char;
                { __goto_1 = 11; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 4 { // L80
                if (self.cur_l < non_char) {
                    // §1213
                    {
                        if (__av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh() > null) {
                            if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == self.hyphen_char[crate::ix::U((self.main_f) as usize)]) {
                                self.ins_disc = true;
                            }
                        }
                        if self.ligature_present {
                            {
                                self.main_p = self.new_ligature(self.main_f, self.cur_l, __av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                if self.lft_hit {
                                    {
                                        { let __ix2365 = self.main_p; __av_mem[crate::ix::U((__ix2365) as usize)].set_hh_b1(2i32); }
                                        self.lft_hit = false;
                                    }
                                }
                                if self.rt_hit {
                                    if (self.lig_stack == null) {
                                        {
                                            { let __ix2366 = self.main_p; let __v2367 = (__av_mem[crate::ix::U((self.main_p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2366) as usize)].set_hh_b1(__v2367); }
                                            self.rt_hit = false;
                                        }
                                    }
                                }
                                if (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) {
                                    tmp_k2 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                                } else {
                                    tmp_k2 = null;
                                }
                                if (tmp_k2 == null) {
                                    {
                                        { let __ix2368 = self.cur_q; let __v2369 = self.main_p; __av_mem[crate::ix::U((__ix2368) as usize)].set_hh_rh(__v2369); }
                                        self.cur_list.tail_field = self.main_p;
                                        self.ligature_present = false;
                                    }
                                } else {
                                    {
                                        { let __ix2370 = self.cur_q; __av_mem[crate::ix::U((__ix2370) as usize)].set_hh_rh(tmp_k2); }
                                        { let __v2371 = self.main_p; __av_mem[crate::ix::U((tmp_k2) as usize)].set_hh_rh(__v2371); }
                                        self.cur_list.tail_field = self.main_p;
                                        self.ligature_present = false;
                                    }
                                }
                            }
                        }
                        if self.ins_disc {
                            {
                                self.ins_disc = false;
                                if (self.cur_list.mode_field > 0i32) {
                                    {
                                        self.prev_tail = self.cur_list.tail_field;
                                        { let __ix2372 = self.cur_list.tail_field; let __v2373 = self.new_disc(); __av_mem[crate::ix::U((__ix2372) as usize)].set_hh_rh(__v2373); }
                                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 5 { // L90
                // §1212
                if (self.lig_stack == null) {
                    // §1214
                    { __goto_1 = 2; continue 'l_dispatch_1; }
                }
                self.cur_q = self.cur_list.tail_field;
                self.cur_l = __av_mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
            }
            if __goto_1 <= 6 { // L91
                if (!(self.lig_stack >= self.hi_mem_min)) {
                    { __goto_1 = 13; continue 'l_dispatch_1; }
                }
            }
            if __goto_1 <= 7 { // L92
                if ((self.effective_char(false, self.main_f, self.cur_chr) > self.font_ec[crate::ix::U((self.main_f) as usize)]) || (self.effective_char(false, self.main_f, self.cur_chr) < self.font_bc[crate::ix::U((self.main_f) as usize)])) {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            { let __ix2374 = self.lig_stack; let __v2375 = self.avail; __av_mem[crate::ix::U((__ix2374) as usize)].set_hh_rh(__v2375); }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                self.main_i = self.effective_char_info(self.main_f, self.cur_l);
                if (!(self.main_i.b0() > min_quarterword)) {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            { let __ix2376 = self.lig_stack; let __v2377 = self.avail; __av_mem[crate::ix::U((__ix2376) as usize)].set_hh_rh(__v2377); }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                { let __ix2378 = self.cur_list.tail_field; let __v2379 = self.lig_stack; __av_mem[crate::ix::U((__ix2378) as usize)].set_hh_rh(__v2379); }
                self.cur_list.tail_field = self.lig_stack;
            }
            if __goto_1 <= 8 { // L100
                // §1212
                self.get_next();
                // §1216
                if (self.cur_cmd == letter) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == other_char) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == char_given) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                self.x_token();
                if (self.cur_cmd == letter) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == other_char) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == char_given) {
                    { __goto_1 = 9; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd == char_num) {
                    {
                        self.scan_char_num();
                        self.cur_chr = self.cur_val;
                        { __goto_1 = 9; continue 'l_dispatch_1; }
                    }
                }
                if (self.cur_cmd == no_boundary) {
                    self.bchar = non_char;
                }
                self.cur_r = self.bchar;
                self.lig_stack = null;
                { __goto_1 = 10; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 9 { // L101
                self.main_s = __av_eqtb[crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v2380 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2380); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v2381 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2381); }
                        }
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == null) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail = __av_mem[crate::ix::U((self.lig_stack) as usize)].hh().rh();
                            { let __ix2382 = self.lig_stack; __av_mem[crate::ix::U((__ix2382) as usize)].set_hh_rh(null); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                            self.dl_new_node(self.lig_stack);
                        }
                    }
                }
                { let __ix2383 = self.lig_stack; let __v2384 = self.main_f; __av_mem[crate::ix::U((__ix2383) as usize)].set_hh_b0(__v2384); }
                self.cur_r = self.cur_chr;
                { let __ix2385 = self.lig_stack; let __v2386 = self.cur_r; __av_mem[crate::ix::U((__ix2385) as usize)].set_hh_b1(__v2386); }
                if (self.cur_r == self.false_bchar) {
                    self.cur_r = non_char;
                }
            }
            if __goto_1 <= 10 { // L110
                // §1212
                tmp_k1 = self.get_auto_kern(self.main_f, self.cur_l, self.cur_r);
                // §1218
                if (tmp_k1 != null) {
                    {
                        if (self.cur_l < non_char) {
                            {
                                if (__av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh() > null) {
                                    if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == self.hyphen_char[crate::ix::U((self.main_f) as usize)]) {
                                        self.ins_disc = true;
                                    }
                                }
                                if self.ligature_present {
                                    {
                                        self.main_p = self.new_ligature(self.main_f, self.cur_l, __av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                        if self.lft_hit {
                                            {
                                                { let __ix2387 = self.main_p; __av_mem[crate::ix::U((__ix2387) as usize)].set_hh_b1(2i32); }
                                                self.lft_hit = false;
                                            }
                                        }
                                        if self.rt_hit {
                                            if (self.lig_stack == null) {
                                                {
                                                    { let __ix2388 = self.main_p; let __v2389 = (__av_mem[crate::ix::U((self.main_p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2388) as usize)].set_hh_b1(__v2389); }
                                                    self.rt_hit = false;
                                                }
                                            }
                                        }
                                        if (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) {
                                            tmp_k2 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                                        } else {
                                            tmp_k2 = null;
                                        }
                                        if (tmp_k2 == null) {
                                            {
                                                { let __ix2390 = self.cur_q; let __v2391 = self.main_p; __av_mem[crate::ix::U((__ix2390) as usize)].set_hh_rh(__v2391); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        } else {
                                            {
                                                { let __ix2392 = self.cur_q; __av_mem[crate::ix::U((__ix2392) as usize)].set_hh_rh(tmp_k2); }
                                                { let __v2393 = self.main_p; __av_mem[crate::ix::U((tmp_k2) as usize)].set_hh_rh(__v2393); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        }
                                    }
                                }
                                if self.ins_disc {
                                    {
                                        self.ins_disc = false;
                                        if (self.cur_list.mode_field > 0i32) {
                                            {
                                                self.prev_tail = self.cur_list.tail_field;
                                                { let __ix2394 = self.cur_list.tail_field; let __v2395 = self.new_disc(); __av_mem[crate::ix::U((__ix2394) as usize)].set_hh_rh(__v2395); }
                                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.save_tail = self.cur_list.tail_field;
                        if ((((((!(self.cur_list.tail_field >= self.hi_mem_min)) && (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b0() == disc_node)) && (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == 0i32)) && (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (__av_mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (__av_mem[crate::ix::U((self.prev_tail) as usize)].hh().rh() == self.cur_list.tail_field)) {
                            {
                                {
                                    { let __ix2396 = self.prev_tail; __av_mem[crate::ix::U((__ix2396) as usize)].set_hh_rh(tmp_k1); }
                                    { let __v2397 = self.cur_list.tail_field; __av_mem[crate::ix::U((tmp_k1) as usize)].set_hh_rh(__v2397); }
                                    self.prev_tail = tmp_k1;
                                }
                            }
                        } else {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2398 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix2398) as usize)].set_hh_rh(tmp_k1); }
                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            }
                        }
                        { __goto_1 = 5; continue 'l_dispatch_1; }
                    }
                }
                // §1217
                if ((self.main_i.b2() % 4i32) != lig_tag) {
                    { __goto_1 = 4; continue 'l_dispatch_1; }
                }
                if (self.cur_r == non_char) {
                    { __goto_1 = 4; continue 'l_dispatch_1; }
                }
                self.main_k = (self.lig_kern_base[crate::ix::U((self.main_f) as usize)]).wrapping_add(self.main_i.b3());
                self.main_j = self.font_info[crate::ix::U((self.main_k) as usize)].qqqq();
                if (self.main_j.b0() <= stop_flag) {
                    { __goto_1 = 12; continue 'l_dispatch_1; }
                }
                self.main_k = ((((self.lig_kern_base[crate::ix::U((self.main_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.main_j.b2()))).wrapping_add(self.main_j.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
            }
            if __goto_1 <= 11 { // L111
                self.main_j = self.font_info[crate::ix::U((self.main_k) as usize)].qqqq();
            }
            if __goto_1 <= 12 { // L112
                if (self.main_j.b1() == self.cur_r) {
                    if (self.main_j.b0() <= stop_flag) {
                        // §1219
                        {
                            if (self.main_j.b2() >= kern_flag) {
                                {
                                    if (self.cur_l < non_char) {
                                        {
                                            if (__av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh() > null) {
                                                if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == self.hyphen_char[crate::ix::U((self.main_f) as usize)]) {
                                                    self.ins_disc = true;
                                                }
                                            }
                                            if self.ligature_present {
                                                {
                                                    self.main_p = self.new_ligature(self.main_f, self.cur_l, __av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                                    if self.lft_hit {
                                                        {
                                                            { let __ix2399 = self.main_p; __av_mem[crate::ix::U((__ix2399) as usize)].set_hh_b1(2i32); }
                                                            self.lft_hit = false;
                                                        }
                                                    }
                                                    if self.rt_hit {
                                                        if (self.lig_stack == null) {
                                                            {
                                                                { let __ix2400 = self.main_p; let __v2401 = (__av_mem[crate::ix::U((self.main_p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2400) as usize)].set_hh_b1(__v2401); }
                                                                self.rt_hit = false;
                                                            }
                                                        }
                                                    }
                                                    if (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) {
                                                        tmp_k2 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                                                    } else {
                                                        tmp_k2 = null;
                                                    }
                                                    if (tmp_k2 == null) {
                                                        {
                                                            { let __ix2402 = self.cur_q; let __v2403 = self.main_p; __av_mem[crate::ix::U((__ix2402) as usize)].set_hh_rh(__v2403); }
                                                            self.cur_list.tail_field = self.main_p;
                                                            self.ligature_present = false;
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix2404 = self.cur_q; __av_mem[crate::ix::U((__ix2404) as usize)].set_hh_rh(tmp_k2); }
                                                            { let __v2405 = self.main_p; __av_mem[crate::ix::U((tmp_k2) as usize)].set_hh_rh(__v2405); }
                                                            self.cur_list.tail_field = self.main_p;
                                                            self.ligature_present = false;
                                                        }
                                                    }
                                                }
                                            }
                                            if self.ins_disc {
                                                {
                                                    self.ins_disc = false;
                                                    if (self.cur_list.mode_field > 0i32) {
                                                        {
                                                            self.prev_tail = self.cur_list.tail_field;
                                                            { let __ix2406 = self.cur_list.tail_field; let __v2407 = self.new_disc(); __av_mem[crate::ix::U((__ix2406) as usize)].set_hh_rh(__v2407); }
                                                            self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        self.prev_tail = self.cur_list.tail_field;
                                        { let __ix2408 = self.cur_list.tail_field; let __v2409 = self.new_kern(self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((self.main_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.main_j.b2()))).wrapping_add(self.main_j.b3())) as usize)].int()); __av_mem[crate::ix::U((__ix2408) as usize)].set_hh_rh(__v2409); }
                                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                    }
                                    { __goto_1 = 5; continue 'l_dispatch_1; }
                                }
                            }
                            if (self.cur_l == non_char) {
                                self.lft_hit = true;
                            } else {
                                if (self.lig_stack == null) {
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
                                        self.main_i = { let __s2410 = ((self.char_base[crate::ix::U((self.main_f) as usize)]).wrapping_add(self.effective_char(true, self.main_f, self.cur_l))) as usize; self.font_info[crate::ix::U(__s2410)] }.qqqq();
                                        self.ligature_present = true;
                                    }
                                }
                                2 | 6 => {
                                    {
                                        self.cur_r = self.main_j.b3();
                                        if (self.lig_stack == null) {
                                            {
                                                self.lig_stack = self.new_lig_item(self.cur_r);
                                                self.bchar = non_char;
                                            }
                                        } else {
                                            if (self.lig_stack >= self.hi_mem_min) {
                                                {
                                                    self.main_p = self.lig_stack;
                                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                                    { let __ix2411 = (self.lig_stack).wrapping_add(1i32); let __v2412 = self.main_p; __av_mem[crate::ix::U((__ix2411) as usize)].set_hh_rh(__v2412); }
                                                }
                                            } else {
                                                { let __ix2413 = self.lig_stack; let __v2414 = self.cur_r; __av_mem[crate::ix::U((__ix2413) as usize)].set_hh_b1(__v2414); }
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    {
                                        self.cur_r = self.main_j.b3();
                                        self.main_p = self.lig_stack;
                                        self.lig_stack = self.new_lig_item(self.cur_r);
                                        { let __ix2415 = self.lig_stack; let __v2416 = self.main_p; __av_mem[crate::ix::U((__ix2415) as usize)].set_hh_rh(__v2416); }
                                    }
                                }
                                7 | 11 => {
                                    {
                                        if (self.cur_l < non_char) {
                                            {
                                                if (__av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh() > null) {
                                                    if (__av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().b1() == self.hyphen_char[crate::ix::U((self.main_f) as usize)]) {
                                                        self.ins_disc = true;
                                                    }
                                                }
                                                if self.ligature_present {
                                                    {
                                                        self.main_p = self.new_ligature(self.main_f, self.cur_l, __av_mem[crate::ix::U((self.cur_q) as usize)].hh().rh());
                                                        if self.lft_hit {
                                                            {
                                                                { let __ix2417 = self.main_p; __av_mem[crate::ix::U((__ix2417) as usize)].set_hh_b1(2i32); }
                                                                self.lft_hit = false;
                                                            }
                                                        }
                                                        if false {
                                                            if (self.lig_stack == null) {
                                                                {
                                                                    { let __ix2418 = self.main_p; let __v2419 = (__av_mem[crate::ix::U((self.main_p) as usize)].hh().b1()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2418) as usize)].set_hh_b1(__v2419); }
                                                                    self.rt_hit = false;
                                                                }
                                                            }
                                                        }
                                                        if (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) {
                                                            tmp_k2 = self.get_auto_kern(self.main_f, non_char, self.cur_l);
                                                        } else {
                                                            tmp_k2 = null;
                                                        }
                                                        if (tmp_k2 == null) {
                                                            {
                                                                { let __ix2420 = self.cur_q; let __v2421 = self.main_p; __av_mem[crate::ix::U((__ix2420) as usize)].set_hh_rh(__v2421); }
                                                                self.cur_list.tail_field = self.main_p;
                                                                self.ligature_present = false;
                                                            }
                                                        } else {
                                                            {
                                                                { let __ix2422 = self.cur_q; __av_mem[crate::ix::U((__ix2422) as usize)].set_hh_rh(tmp_k2); }
                                                                { let __v2423 = self.main_p; __av_mem[crate::ix::U((tmp_k2) as usize)].set_hh_rh(__v2423); }
                                                                self.cur_list.tail_field = self.main_p;
                                                                self.ligature_present = false;
                                                            }
                                                        }
                                                    }
                                                }
                                                if self.ins_disc {
                                                    {
                                                        self.ins_disc = false;
                                                        if (self.cur_list.mode_field > 0i32) {
                                                            {
                                                                self.prev_tail = self.cur_list.tail_field;
                                                                { let __ix2424 = self.cur_list.tail_field; let __v2425 = self.new_disc(); __av_mem[crate::ix::U((__ix2424) as usize)].set_hh_rh(__v2425); }
                                                                self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.cur_q = self.cur_list.tail_field;
                                        self.cur_l = self.main_j.b3();
                                        self.main_i = { let __s2426 = ((self.char_base[crate::ix::U((self.main_f) as usize)]).wrapping_add(self.effective_char(true, self.main_f, self.cur_l))) as usize; self.font_info[crate::ix::U(__s2426)] }.qqqq();
                                        self.ligature_present = true;
                                    }
                                }
                                _ => {
                                    {
                                        self.cur_l = self.main_j.b3();
                                        self.ligature_present = true;
                                        if (self.lig_stack == null) {
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
                            if (self.cur_l < non_char) {
                                { __goto_1 = 10; continue 'l_dispatch_1; }
                            }
                            self.main_k = self.bchar_label[crate::ix::U((self.main_f) as usize)];
                            { __goto_1 = 11; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §1217
                if (self.main_j.b0() == 0i32) {
                    self.main_k = (self.main_k).wrapping_add(1i32);
                } else {
                    {
                        if (self.main_j.b0() >= stop_flag) {
                            { __goto_1 = 4; continue 'l_dispatch_1; }
                        }
                        self.main_k = ((self.main_k).wrapping_add(self.main_j.b0())).wrapping_add(1i32);
                    }
                }
                { __goto_1 = 11; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 13 { // L95
                // §1212
                self.main_p = __av_mem[crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)].hh().rh();
                // §1215
                if (self.main_p > null) {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix2427 = self.cur_list.tail_field; let __v2428 = self.main_p; __av_mem[crate::ix::U((__ix2427) as usize)].set_hh_rh(__v2428); }
                        self.cur_list.tail_field = __av_mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                    }
                }
                self.temp_ptr = self.lig_stack;
                self.lig_stack = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                self.free_node(self.temp_ptr, small_node_size);
                self.main_i = { let __s2429 = ((self.char_base[crate::ix::U((self.main_f) as usize)]).wrapping_add(self.effective_char(true, self.main_f, self.cur_l))) as usize; self.font_info[crate::ix::U(__s2429)] }.qqqq();
                self.ligature_present = true;
                if (self.lig_stack == null) {
                    if (self.main_p > null) {
                        { __goto_1 = 8; continue 'l_dispatch_1; }
                    } else {
                        self.cur_r = self.bchar;
                    }
                } else {
                    self.cur_r = __av_mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                }
                { __goto_1 = 10; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 14 { // L120
                // §1208
                if (__av_eqtb[crate::ix::U(((26640i32) - 1) as usize)].hh().rh() == zero_glue) {
                    // §1220
                    {
                        // §1221
                        {
                            self.main_p = self.font_glue[crate::ix::U((__av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)];
                            if (self.main_p == null) {
                                {
                                    self.main_p = self.new_spec(zero_glue);
                                    self.main_k = (self.param_base[crate::ix::U((__av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)]).wrapping_add(2i32);
                                    { let __ix2430 = (self.main_p).wrapping_add(1i32); let __v2431 = self.font_info[crate::ix::U((self.main_k) as usize)].int(); __av_mem[crate::ix::U((__ix2430) as usize)].set_int(__v2431); }
                                    { let __ix2432 = (self.main_p).wrapping_add(2i32); let __v2433 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2432) as usize)].set_int(__v2433); }
                                    { let __ix2434 = (self.main_p).wrapping_add(3i32); let __v2435 = self.font_info[crate::ix::U(((self.main_k).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U((__ix2434) as usize)].set_int(__v2435); }
                                    { let __ix2436 = __av_eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(); let __v2437 = self.main_p; self.font_glue[crate::ix::U((__ix2436) as usize)] = __v2437; }
                                }
                            }
                        }
                        // §1220
                        self.temp_ptr = self.new_glue(self.main_p);
                    }
                } else {
                    self.temp_ptr = self.new_param_glue(space_skip_code);
                }
                if (__av_eqtb[crate::ix::U(((29364i32) - 1) as usize)].int() > 0i32) {
                    self.adjust_interword_glue(self.cur_list.tail_field, self.temp_ptr);
                }
                { let __ix2438 = self.cur_list.tail_field; let __v2439 = self.temp_ptr; __av_mem[crate::ix::U((__ix2438) as usize)].set_hh_rh(__v2439); }
                self.cur_list.tail_field = self.temp_ptr;
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 15 { // exit
                // §1208
            }
            break 'l_dispatch_1;
        }
    }

    /// The `error` routine calls on `give_err_help` if help is requested from
    /// the `err_help` parameter.
    // §1463
    pub fn give_err_help(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.token_show(__av_eqtb[crate::ix::U(((err_help_loc) - 1) as usize)].hh().rh());
    }

    /// Here is the only place we use `pack_buffered_name`. This part of the program
    /// becomes active when a ``virgin'' \TeX\ is trying to get going, just after
    /// the preliminary initialization, or when the user is substituting another
    /// format file by typing `\.\&' after the initial `\.{**}' prompt.  The buffer
    /// contains the first line of input in `buffer[loc..(last-1)]`, where
    /// `loc<last` and `buffer[loc]<>" "`.
    /// @<Declare the function called `open_fmt_file`
    // §551
    pub fn open_fmt_file(&mut self) -> bool {
        let mut open_fmt_file: bool = false;
        let mut j: i32 = 0; // §551
        'l_exit_f: {
            'l_found_f: {
                j = self.cur_input.loc_field;
                if (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 38i32) {
                    {
                        self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                        j = self.cur_input.loc_field;
                        self.buffer[crate::ix::U((self.last) as usize)] = 32i32;
                        while (self.buffer[crate::ix::U((j) as usize)] != 32i32) {
                            j = (j).wrapping_add(1i32);
                        }
                        self.pack_buffered_name(0i32, self.cur_input.loc_field, (j).wrapping_sub(1i32));
                        if { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_in(&mut __f0); self.fmt_file = __f0; __r } {
                            break 'l_found_f;
                        }
                        {
                            crate::system::wr_str(&mut self.term_out, "Sorry, I can't find the format `");
                        }
                        self.wterm_name_of_file();
                        {
                            crate::system::wr_str(&mut self.term_out, "'; will try `");
                        }
                        self.wterm_format_default();
                        {
                            crate::system::wr_str(&mut self.term_out, "'.");
                            crate::system::wr_ln(&mut self.term_out);
                        }
                        crate::system::break_out(&mut self.term_out);
                    }
                }
                self.pack_default_format_name();
                if (!{ let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_in(&mut __f0); self.fmt_file = __f0; __r }) {
                    {
                        {
                            crate::system::wr_str(&mut self.term_out, "I can't find the format file `");
                        }
                        self.wterm_format_default();
                        {
                            crate::system::wr_str(&mut self.term_out, "'!");
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
    // §1482
    pub fn load_fmt_file(&mut self) -> bool {
        let mut load_fmt_file: bool = false;
        let mut j: i32 = 0; // §1482
        let mut k: i32 = 0; // §1482
        let mut p: halfword = 0; // §1482
        let mut q: halfword = 0; // §1482
        let mut x: i32 = 0; // §1482
        let mut w: four_quarters = four_quarters::default(); // §1482
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            'l_L6666_f: {
                // §1487
                x = self.fmt_file.buf.int();
                if (x != 312141437i32) {
                    break 'l_L6666_f;
                }
                // §1929
                if self.translate_filename_p {
                    {
                        {
                            let __for_end_6 = 767i32;
                            k = 0i32;
                            while k <= __for_end_6 {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                } else {
                    {
                        {
                            let __for_end_6 = 255i32;
                            k = 0i32;
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_word(&mut self.fmt_file);
                                        x = self.fmt_file.buf.int();
                                    }
                                    self.xord[crate::ix::U((k) as usize)] = x;
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        {
                            let __for_end_6 = 255i32;
                            k = 0i32;
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_word(&mut self.fmt_file);
                                        x = self.fmt_file.buf.int();
                                    }
                                    self.xchr[crate::ix::U((k) as usize)] = ((x) as u8);
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        {
                            let __for_end_6 = 255i32;
                            k = 0i32;
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_word(&mut self.fmt_file);
                                        x = self.fmt_file.buf.int();
                                    }
                                    self.xprn[crate::ix::U((k) as usize)] = x;
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        if self.eight_bit_p {
                            {
                                let __for_end_7 = 255i32;
                                k = 0i32;
                                while k <= __for_end_7 {
                                    self.xprn[crate::ix::U((k) as usize)] = 1i32;
                                    k = k.wrapping_add(1);
                                }
                            }
                        }
                    }
                }
                // §1656
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > 1i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.eTeX_mode = x;
                    }
                }
                if (self.eTeX_mode == 1i32) {
                    {
                        // §1814
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 2092i32;
                    }
                } else {
                    // §1656
                    {
                        // §1813
                        self.max_reg_num = 255i32;
                        self.max_reg_help_line = 800i32;
                    }
                }
                // §1487
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != mem_bot) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != mem_top) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != eqtb_size) {
                    break 'l_L6666_f;
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > hash_extra)) {
                        break 'l_L6666_f;
                    } else {
                        self.hash_high = x;
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != hash_prime) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != hyph_size) {
                    break 'l_L6666_f;
                }
                // §1909
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 1296847960i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x == 1i32) {
                    self.mltex_enabled_p = true;
                } else {
                    if (x != 0i32) {
                        break 'l_L6666_f;
                    }
                }
                // §1876
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 1162040408i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x == 0i32) {
                    self.enctex_enabled_p = false;
                } else {
                    if (x != 1i32) {
                        break 'l_L6666_f;
                    } else {
                        {
                            self.enctex_enabled_p = true;
                            {
                                let __for_end_7 = 255i32;
                                k = 0i32;
                                while k <= __for_end_7 {
                                    {
                                        {
                                            crate::system::get_word(&mut self.fmt_file);
                                            x = self.fmt_file.buf.int();
                                        }
                                        self.mubyte_read[crate::ix::U((k) as usize)] = x;
                                    }
                                    k = k.wrapping_add(1);
                                }
                            }
                            {
                                let __for_end_7 = 255i32;
                                k = 0i32;
                                while k <= __for_end_7 {
                                    {
                                        {
                                            crate::system::get_word(&mut self.fmt_file);
                                            x = self.fmt_file.buf.int();
                                        }
                                        self.mubyte_write[crate::ix::U((k) as usize)] = x;
                                    }
                                    k = k.wrapping_add(1);
                                }
                            }
                            {
                                let __for_end_7 = 127i32;
                                k = 0i32;
                                while k <= __for_end_7 {
                                    {
                                        {
                                            crate::system::get_word(&mut self.fmt_file);
                                            x = self.fmt_file.buf.int();
                                        }
                                        self.mubyte_cswrite[crate::ix::U((k) as usize)] = x;
                                    }
                                    k = k.wrapping_add(1);
                                }
                            }
                        }
                    }
                }
                // §1489
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
                                self.str_start[crate::ix::U((k) as usize)] = x;
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
                        self.str_pool[crate::ix::U((k) as usize)] = w.b0();
                        self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)] = w.b1();
                        self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)] = w.b2();
                        self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)] = w.b3();
                        k = (k).wrapping_add(4i32);
                    }
                }
                k = (self.pool_ptr).wrapping_sub(4i32);
                {
                    crate::system::get_word(&mut self.fmt_file);
                    w = self.fmt_file.buf.qqqq();
                }
                self.str_pool[crate::ix::U((k) as usize)] = w.b0();
                self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)] = w.b1();
                self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)] = w.b2();
                self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)] = w.b3();
                self.init_str_ptr = self.str_ptr;
                self.init_pool_ptr = self.pool_ptr;
                // §1491
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1019i32) || (x > 4999984i32)) {
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
                if (self.eTeX_mode == 1i32) {
                    {
                        let __for_end_5 = tok_val;
                        k = int_val;
                        while k <= __for_end_5 {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < null) || (x > self.lo_mem_max)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.sa_root[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                }
                p = mem_bot;
                q = self.rover;
                loop {
                    {
                        let __for_end_5 = (q).wrapping_add(1i32);
                        k = p;
                        while k <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2440 = self.fmt_file.buf; __av_mem[crate::ix::U((k) as usize)] = __v2440; }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    p = (q).wrapping_add(__av_mem[crate::ix::U((q) as usize)].hh().lh());
                    if ((p > self.lo_mem_max) || ((q >= __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh()) && (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() != self.rover))) {
                        break 'l_L6666_f;
                    }
                    q = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
                    if (q == self.rover) { break; }
                }
                {
                    let __for_end_4 = self.lo_mem_max;
                    k = p;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2441 = self.fmt_file.buf; __av_mem[crate::ix::U((k) as usize)] = __v2441; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                if (mem_min < (2i32).wrapping_neg()) {
                    {
                        p = __av_mem[crate::ix::U(((self.rover).wrapping_add(1i32)) as usize)].hh().lh();
                        q = (mem_min).wrapping_add(1i32);
                        { let __ix2442 = mem_min; __av_mem[crate::ix::U((__ix2442) as usize)].set_hh_rh(null); }
                        { let __ix2443 = mem_min; __av_mem[crate::ix::U((__ix2443) as usize)].set_hh_lh(null); }
                        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(q);
                        { let __ix2444 = (self.rover).wrapping_add(1i32); __av_mem[crate::ix::U((__ix2444) as usize)].set_hh_lh(q); }
                        { let __v2445 = self.rover; __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v2445); }
                        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(empty_flag);
                        __av_mem[crate::ix::U((q) as usize)].set_hh_lh(((0i32).wrapping_neg()).wrapping_sub(q));
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < (self.lo_mem_max).wrapping_add(1i32)) || (x > hi_mem_stat_min)) {
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
                    if ((x < null) || (x > mem_top)) {
                        break 'l_L6666_f;
                    } else {
                        self.avail = x;
                    }
                }
                self.mem_end = mem_top;
                {
                    let __for_end_4 = self.mem_end;
                    k = self.hi_mem_min;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2446 = self.fmt_file.buf; __av_mem[crate::ix::U((k) as usize)] = __v2446; }
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
                // §1496
                k = active_base;
                loop {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1i32) || ((k).wrapping_add(x) > 30193i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2447 = self.fmt_file.buf; __av_eqtb[crate::ix::U(((j) - 1) as usize)] = __v2447; }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || ((k).wrapping_add(x) > 30193i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            { let __v2448 = __av_eqtb[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]; __av_eqtb[crate::ix::U(((j) - 1) as usize)] = __v2448; }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    if (k > eqtb_size) { break; }
                }
                {
                    let __for_end_4 = eqtb_top;
                    j = 30193i32;
                    while j <= __for_end_4 {
                        { let __v2449 = __av_eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)]; __av_eqtb[crate::ix::U(((j) - 1) as usize)] = __v2449; }
                        j = j.wrapping_add(1);
                    }
                }
                if (self.hash_high > 0i32) {
                    {
                        let __for_end_5 = (eqtb_size).wrapping_add(self.hash_high);
                        j = 30193i32;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2450 = self.fmt_file.buf; __av_eqtb[crate::ix::U(((j) - 1) as usize)] = __v2450; }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                }
                // §1493
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.par_loc = self.fmt_file.buf.int();
                }
                self.par_token = (cs_token_flag).wrapping_add(self.par_loc);
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < hash_base) || (x > hash_top)) {
                        break 'l_L6666_f;
                    } else {
                        self.write_loc = x;
                    }
                }
                // §1498
                {
                    let __for_end_4 = prim_size;
                    p = 0i32;
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2451 = self.fmt_file.buf.hh(); self.prim[crate::ix::U((p) as usize)] = __v2451; }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < hash_base) || (x > frozen_control_sequence)) {
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
                        { let __v2452 = self.fmt_file.buf.hh(); self.hash[crate::ix::U(((p) - 514) as usize)] = __v2452; }
                    }
                    if (p == self.hash_used) { break; }
                }
                {
                    let __for_end_4 = 26626i32;
                    p = (self.hash_used).wrapping_add(1i32);
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2453 = self.fmt_file.buf.hh(); self.hash[crate::ix::U(((p) - 514) as usize)] = __v2453; }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                if (self.hash_high > 0i32) {
                    {
                        let __for_end_5 = (eqtb_size).wrapping_add(self.hash_high);
                        p = 30193i32;
                        while p <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2454 = self.fmt_file.buf.hh(); self.hash[crate::ix::U(((p) - 514) as usize)] = __v2454; }
                            }
                            p = p.wrapping_add(1);
                        }
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.cs_count = self.fmt_file.buf.int();
                }
                // §1500
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
                            { let __v2455 = self.fmt_file.buf; self.font_info[crate::ix::U((k) as usize)] = __v2455; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if (x < font_base) {
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
                self.make_pdftex_banner();
                {
                    let __for_end_4 = self.font_ptr;
                    k = null_font;
                    while k <= __for_end_4 {
                        // §1502
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2456 = self.fmt_file.buf.qqqq(); self.font_check[crate::ix::U((k) as usize)] = __v2456; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2457 = self.fmt_file.buf.int(); self.font_size[crate::ix::U((k) as usize)] = __v2457; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2458 = self.fmt_file.buf.int(); self.font_dsize[crate::ix::U((k) as usize)] = __v2458; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_halfword) || (x > max_halfword)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_params[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2459 = self.fmt_file.buf.int(); self.hyphen_char[crate::ix::U((k) as usize)] = __v2459; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2460 = self.fmt_file.buf.int(); self.skew_char[crate::ix::U((k) as usize)] = __v2460; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.str_ptr)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_name[crate::ix::U((k) as usize)] = x;
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
                                    self.font_area[crate::ix::U((k) as usize)] = x;
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
                                    self.font_bc[crate::ix::U((k) as usize)] = x;
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
                                    self.font_ec[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2461 = self.fmt_file.buf.int(); self.char_base[crate::ix::U((k) as usize)] = __v2461; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2462 = self.fmt_file.buf.int(); self.width_base[crate::ix::U((k) as usize)] = __v2462; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2463 = self.fmt_file.buf.int(); self.height_base[crate::ix::U((k) as usize)] = __v2463; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2464 = self.fmt_file.buf.int(); self.depth_base[crate::ix::U((k) as usize)] = __v2464; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2465 = self.fmt_file.buf.int(); self.italic_base[crate::ix::U((k) as usize)] = __v2465; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2466 = self.fmt_file.buf.int(); self.lig_kern_base[crate::ix::U((k) as usize)] = __v2466; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2467 = self.fmt_file.buf.int(); self.kern_base[crate::ix::U((k) as usize)] = __v2467; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2468 = self.fmt_file.buf.int(); self.exten_base[crate::ix::U((k) as usize)] = __v2468; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2469 = self.fmt_file.buf.int(); self.param_base[crate::ix::U((k) as usize)] = __v2469; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_halfword) || (x > self.lo_mem_max)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_glue[crate::ix::U((k) as usize)] = x;
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
                                    self.bchar_label[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_quarterword) || (x > non_char)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_bchar[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_quarterword) || (x > non_char)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_false_bchar[crate::ix::U((k) as usize)] = x;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §1504
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > hyph_size)) {
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
                                if ((x < 0i32) || (x > hyph_size)) {
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
                                    self.hyph_word[crate::ix::U((j) as usize)] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_halfword) || (x > max_halfword)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyph_list[crate::ix::U((j) as usize)] = x;
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
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > j)) {
                        break 'l_L6666_f;
                    } else {
                        self.hyph_start = x;
                    }
                }
                {
                    let __for_end_4 = j;
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2470 = self.fmt_file.buf.hh(); self.trie[crate::ix::U((k) as usize)] = __v2470; }
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
                                    self.hyf_distance[crate::ix::U(((k) - 1) as usize)] = x;
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
                                    self.hyf_num[crate::ix::U(((k) - 1) as usize)] = x;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < min_quarterword) || (x > max_trie_op)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.hyf_next[crate::ix::U(((k) - 1) as usize)] = x;
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
                        self.trie_used[crate::ix::U((k) as usize)] = min_quarterword;
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
                        self.trie_used[crate::ix::U((k) as usize)] = x;
                        j = (j).wrapping_sub(x);
                        self.op_start[crate::ix::U((k) as usize)] = j;
                    }
                }
                self.trie_not_ready = false;
                // §1506
                {
                    self.undumpimagemeta(__av_eqtb[crate::ix::U(((29351i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29352i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29355i32) - 1) as usize)].int());
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_mem_size = self.fmt_file.buf.int();
                    }
                    self.pdf_mem.resize_len(((self.pdf_mem_size) as usize) + 1);
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_mem_ptr = self.fmt_file.buf.int();
                    }
                    {
                        let __for_end_5 = (self.pdf_mem_ptr).wrapping_sub(1i32);
                        k = 1i32;
                        while k <= __for_end_5 {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    { let __v2471 = self.fmt_file.buf.int(); self.pdf_mem[crate::ix::U((k) as usize)] = __v2471; }
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.obj_tab_size = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.obj_ptr = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.sys_obj_ptr = self.fmt_file.buf.int();
                    }
                    {
                        let __for_end_5 = self.sys_obj_ptr;
                        k = 1i32;
                        while k <= __for_end_5 {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[crate::ix::U((k) as usize)].int0 = self.fmt_file.buf.int();
                                }
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[crate::ix::U((k) as usize)].int1 = self.fmt_file.buf.int();
                                }
                                self.obj_tab[crate::ix::U((k) as usize)].int2 = (((1i32).wrapping_neg()) as i64);
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[crate::ix::U((k) as usize)].int3 = self.fmt_file.buf.int();
                                }
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[crate::ix::U((k) as usize)].int4 = self.fmt_file.buf.int();
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_obj_count = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_xform_count = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_ximage_count = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v2472 = self.fmt_file.buf.int(); self.head_tab[crate::ix::U(((obj_type_obj) - 1) as usize)] = __v2472; }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v2473 = self.fmt_file.buf.int(); self.head_tab[crate::ix::U(((obj_type_xform) - 1) as usize)] = __v2473; }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v2474 = self.fmt_file.buf.int(); self.head_tab[crate::ix::U(((obj_type_ximage) - 1) as usize)] = __v2474; }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_last_obj = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_last_xform = self.fmt_file.buf.int();
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_last_ximage = self.fmt_file.buf.int();
                    }
                    self.undumptounicode();
                }
                // §1508
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < batch_mode) || (x > error_stop_mode)) {
                        break 'l_L6666_f;
                    } else {
                        self.interaction = x;
                    }
                }
                if (self.interaction_option != unspecified_mode) {
                    self.interaction = self.interaction_option;
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
                // §1482
                { let __v2475 = __av_eqtb[crate::ix::U(((29935i32) - 1) as usize)].int(); self.cur_list.aux_field.set_int(__v2475); }
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
    // §1514
    pub fn close_files_and_terminate(&mut self) {
        let mut a: i32 = 0; // §1514
        let mut b: i32 = 0; // §1514
        let mut c: i32 = 0; // §1514
        let mut i: i32 = 0; // §1514
        let mut j: i32 = 0; // §1514
        let mut k: i32 = 0; // §1514
        let mut l: i32 = 0; // §1514
        let mut is_root: bool = false; // §1514
        let mut is_names: bool = false; // §1514
        let mut root: i32 = 0; // §1514
        let mut outlines: i32 = 0; // §1514
        let mut threads: i32 = 0; // §1514
        let mut names_tree: i32 = 0; // §1514
        let mut dests: i32 = 0; // §1514
        let mut xref_offset_width: i32 = 0; // §1514
        let mut names_head: i32 = 0; // §1514
        let mut names_tail: i32 = 0; // §1514
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        // §1627
        {
            let __for_end_2 = 15i32;
            k = 0i32;
            while k <= __for_end_2 {
                if self.write_open[crate::ix::U((k) as usize)] {
                    { let mut __f0 = ::core::mem::take(&mut self.write_file[crate::ix::U((k) as usize)]); let __r = self.a_close(&mut __f0); self.write_file[crate::ix::U((k) as usize)] = __f0; __r };
                }
                k = k.wrapping_add(1);
            }
        }
        // §1514
        __av_eqtb[crate::ix::U(((29326i32) - 1) as usize)].set_int((1i32).wrapping_neg());
        if (__av_eqtb[crate::ix::U(((29308i32) - 1) as usize)].int() > 0i32) {
            // §1515
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
                        let __w3 = hash_size;
                        let __w4 = b'+';
                        let __w5 = hash_extra;
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " multiletter control sequences out of ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_char(&mut self.log_file, __w4);
                        crate::system::wr_int(&mut self.log_file, __w5, 1i32);
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
                        let __w1 = hyph_size;
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
        // §1514
        if (!self.fixed_pdfoutput_set) {
            self.fix_pdfoutput();
        }
        if (self.fixed_pdfoutput > 0i32) {
            {
                if (self.history == fatal_error_stop) {
                    {
                        self.remove_pdffile();
                        {
                            self.dg_mark();
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1733i32);
                        }
                    }
                } else {
                    {
                        // §795
                        if (self.total_pages == 0i32) {
                            {
                                self.print_nl(1011i32);
                                if (self.pdf_gone > ((0i32) as i64)) {
                                    self.garbage_warning();
                                }
                            }
                        } else {
                            {
                                if (self.fixed_pdf_draftmode == 0i32) {
                                    {
                                        'l_done1_f: {
                                            'l_done_f: {
                                                self.pdf_flush();
                                                if ((self.total_pages % pages_tree_kids_max) != 0i32) {
                                                    self.obj_tab[crate::ix::U((self.pdf_last_pages) as usize)].int0 = (self.total_pages % pages_tree_kids_max);
                                                }
                                                self.flush_jbig2_page0_objects();
                                                // §800
                                                k = self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)];
                                                while (self.obj_tab[crate::ix::U((k) as usize)].int4 == 0i32) {
                                                    {
                                                        self.pdf_warning(1206i32, 1213i32, true, false);
                                                        self.print_int(((self.obj_tab[crate::ix::U((k) as usize)].int0) as i64));
                                                        self.print(1214i32);
                                                        self.print_ln();
                                                        self.print_ln();
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                                self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)] = k;
                                                // §801
                                                k = self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)];
                                                l = 0i32;
                                                loop {
                                                    i = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    self.obj_tab[crate::ix::U((k) as usize)].int1 = l;
                                                    l = k;
                                                    k = i;
                                                    if (k == 0i32) { break; }
                                                }
                                                self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)] = l;
                                                k = self.head_tab[crate::ix::U(((obj_type_pages) - 1) as usize)];
                                                self.pages_tail = k;
                                                l = 0i32;
                                                loop {
                                                    i = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    self.obj_tab[crate::ix::U((k) as usize)].int1 = l;
                                                    l = k;
                                                    k = i;
                                                    if (k == 0i32) { break; }
                                                }
                                                self.head_tab[crate::ix::U(((obj_type_pages) - 1) as usize)] = l;
                                                // §797
                                                k = self.head_tab[crate::ix::U(((obj_type_dest) - 1) as usize)];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_fix_dest(k);
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                                // §799
                                                k = self.head_tab[crate::ix::U(((obj_type_struct_dest) - 1) as usize)];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_fix_struct_dest(k);
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                                // §802
                                                {
                                                    let __for_end_12 = self.font_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if ((self.font_used[crate::ix::U((k) as usize)] && self.hasfmentry(k)) && (self.pdf_font_num[crate::ix::U((k) as usize)] < 0i32)) {
                                                            {
                                                                i = (self.pdf_font_num[crate::ix::U((k) as usize)]).wrapping_neg();
                                                                self.pdfassert((self.pdf_font_num[crate::ix::U((i) as usize)] > 0i32));
                                                                {
                                                                    let __for_end_16 = 255i32;
                                                                    j = 0i32;
                                                                    while j <= __for_end_16 {
                                                                        if self.pdf_char_marked(k, j) {
                                                                            self.pdf_mark_char(i, j);
                                                                        }
                                                                        j = j.wrapping_add(1);
                                                                    }
                                                                }
                                                                if (((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((i) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((i) as usize)]) as usize)]) == 0i32) && ((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((k) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((k) as usize)]) as usize)]) != 0i32)) {
                                                                    { let __v2476 = self.pdf_font_attr[crate::ix::U((k) as usize)]; self.pdf_font_attr[crate::ix::U((i) as usize)] = __v2476; }
                                                                } else {
                                                                    if (((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((k) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((k) as usize)]) as usize)]) == 0i32) && ((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((i) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((i) as usize)]) as usize)]) != 0i32)) {
                                                                        { let __v2477 = self.pdf_font_attr[crate::ix::U((i) as usize)]; self.pdf_font_attr[crate::ix::U((k) as usize)] = __v2477; }
                                                                    } else {
                                                                        if ((((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((i) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((i) as usize)]) as usize)]) != 0i32) && ((self.str_start[crate::ix::U(((self.pdf_font_attr[crate::ix::U((k) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.pdf_font_attr[crate::ix::U((k) as usize)]) as usize)]) != 0i32)) && (!self.str_eq_str(self.pdf_font_attr[crate::ix::U((i) as usize)], self.pdf_font_attr[crate::ix::U((k) as usize)]))) {
                                                                            {
                                                                                self.pdf_warning(1215i32, 1216i32, true, false);
                                                                                self.print_font_identifier(i);
                                                                                self.print(1217i32);
                                                                                self.print_font_identifier(k);
                                                                                self.print(1218i32);
                                                                                self.print_font_identifier(i);
                                                                                self.print_ln();
                                                                                self.print_ln();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.fixed_gen_tounicode = __av_eqtb[crate::ix::U(((29367i32) - 1) as usize)].int();
                                                k = self.head_tab[crate::ix::U(((obj_type_font) - 1) as usize)];
                                                while (k != 0i32) {
                                                    {
                                                        self.f = self.obj_tab[crate::ix::U((k) as usize)].int0;
                                                        self.pdfassert((self.pdf_font_num[crate::ix::U((self.f) as usize)] > 0i32));
                                                        self.do_pdf_font(k, self.f);
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                                self.write_fontstuff();
                                                // §803
                                                a = (self.sys_obj_ptr).wrapping_add(1i32);
                                                l = self.head_tab[crate::ix::U(((obj_type_pages) - 1) as usize)];
                                                k = self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)];
                                                b = 0i32;
                                                loop {
                                                    i = 0i32;
                                                    c = 0i32;
                                                    if (self.obj_tab[crate::ix::U((l) as usize)].int1 == 0i32) {
                                                        is_root = true;
                                                    } else {
                                                        is_root = false;
                                                    }
                                                    loop {
                                                        if (!is_root) {
                                                            {
                                                                if ((i % pages_tree_kids_max) == 0i32) {
                                                                    {
                                                                        self.pdf_last_pages = self.pdf_new_objnum();
                                                                        if (c == 0i32) {
                                                                            c = self.pdf_last_pages;
                                                                        }
                                                                        self.obj_tab[crate::ix::U((self.pages_tail) as usize)].int1 = self.pdf_last_pages;
                                                                        self.pages_tail = self.pdf_last_pages;
                                                                        self.obj_tab[crate::ix::U((self.pdf_last_pages) as usize)].int1 = 0i32;
                                                                        self.obj_tab[crate::ix::U((self.pdf_last_pages) as usize)].int0 = self.obj_tab[crate::ix::U((l) as usize)].int0;
                                                                    }
                                                                } else {
                                                                    self.obj_tab[crate::ix::U((self.pdf_last_pages) as usize)].int0 = (self.obj_tab[crate::ix::U((self.pdf_last_pages) as usize)].int0).wrapping_add(self.obj_tab[crate::ix::U((l) as usize)].int0);
                                                                }
                                                            }
                                                        }
                                                        // §804
                                                        self.pdf_begin_dict(l, 1i32);
                                                        {
                                                            self.pdf_print(1219i32);
                                                            {
                                                                {
                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_os_get_os_buf(1i32);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_flush();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                {
                                                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                        self.pdf_int_entry_ln(1201i32, self.obj_tab[crate::ix::U((l) as usize)].int0);
                                                        if (!is_root) {
                                                            self.pdf_indirect_ln(1175i32, self.pdf_last_pages);
                                                        }
                                                        self.pdf_print(1220i32);
                                                        j = 0i32;
                                                        loop {
                                                            self.pdf_print_int(((k) as i64));
                                                            self.pdf_print(1160i32);
                                                            k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                            j = (j).wrapping_add(1i32);
                                                            if (((((l < a) && (j == self.obj_tab[crate::ix::U((l) as usize)].int0)) || (k == 0i32)) || ((k == b) && (b != 0i32))) || (j == pages_tree_kids_max)) { break; }
                                                        }
                                                        self.remove_last_space();
                                                        {
                                                            self.pdf_print(93i32);
                                                            {
                                                                {
                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_os_get_os_buf(1i32);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_flush();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                {
                                                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                        if (k == 0i32) {
                                                            {
                                                                k = self.head_tab[crate::ix::U(((obj_type_pages) - 1) as usize)];
                                                                self.head_tab[crate::ix::U(((obj_type_pages) - 1) as usize)] = 0i32;
                                                            }
                                                        }
                                                        if (is_root && (__av_eqtb[crate::ix::U(((pdf_pages_attr_loc) - 1) as usize)].hh().rh() != null)) {
                                                            self.pdf_print_toks_ln(__av_eqtb[crate::ix::U(((pdf_pages_attr_loc) - 1) as usize)].hh().rh());
                                                        }
                                                        self.pdf_end_dict();
                                                        // §803
                                                        i = (i).wrapping_add(1i32);
                                                        l = self.obj_tab[crate::ix::U((l) as usize)].int1;
                                                        if (l == c) { break; }
                                                    }
                                                    b = c;
                                                    if (l == 0i32) {
                                                        break 'l_done_f;
                                                    }
                                                    if false { break; }
                                                }
                                            }
                                            // §789
                                            if (self.pdf_first_outline != 0i32) {
                                                {
                                                    self.pdf_new_dict(obj_type_others, 0i32, 1i32);
                                                    outlines = self.obj_ptr;
                                                    l = self.pdf_first_outline;
                                                    k = 0i32;
                                                    loop {
                                                        k = (k).wrapping_add(1i32);
                                                        a = self.open_subentries(l);
                                                        if (self.obj_tab[crate::ix::U((l) as usize)].int0 > 0i32) {
                                                            k = (k).wrapping_add(a);
                                                        }
                                                        { let __ix2478 = (self.obj_tab[crate::ix::U((l) as usize)].int4).wrapping_add(1i32); let __v2479 = self.obj_ptr; self.pdf_mem[crate::ix::U((__ix2478) as usize)] = __v2479; }
                                                        l = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((l) as usize)].int4).wrapping_add(3i32)) as usize)];
                                                        if (l == 0i32) { break; }
                                                    }
                                                    {
                                                        self.pdf_print(1198i32);
                                                        {
                                                            {
                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_os_get_os_buf(1i32);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                        self.overflow(1018i32, pdf_op_buf_size);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_flush();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            {
                                                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                    self.pdf_indirect_ln(1199i32, self.pdf_first_outline);
                                                    self.pdf_indirect_ln(1200i32, self.pdf_last_outline);
                                                    self.pdf_int_entry_ln(1201i32, k);
                                                    self.pdf_end_dict();
                                                    // §790
                                                    k = self.head_tab[crate::ix::U(((obj_type_outline) - 1) as usize)];
                                                    while (k != 0i32) {
                                                        {
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32)) as usize)] == self.pdf_parent_outline) {
                                                                {
                                                                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32)) as usize)] == 0i32) {
                                                                        self.pdf_first_outline = k;
                                                                    }
                                                                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] == 0i32) {
                                                                        self.pdf_last_outline = k;
                                                                    }
                                                                }
                                                            }
                                                            self.pdf_begin_dict(k, 1i32);
                                                            self.pdf_indirect_ln(1202i32, self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((k) as usize)].int4) as usize)]);
                                                            self.pdf_indirect_ln(65i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(6i32)) as usize)]);
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32)) as usize)] != 0i32) {
                                                                self.pdf_indirect_ln(1175i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(1i32)) as usize)]);
                                                            }
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32)) as usize)] != 0i32) {
                                                                self.pdf_indirect_ln(1203i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(2i32)) as usize)]);
                                                            }
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)] != 0i32) {
                                                                self.pdf_indirect_ln(1204i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(3i32)) as usize)]);
                                                            }
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)] != 0i32) {
                                                                self.pdf_indirect_ln(1199i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(4i32)) as usize)]);
                                                            }
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(5i32)) as usize)] != 0i32) {
                                                                self.pdf_indirect_ln(1200i32, self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(5i32)) as usize)]);
                                                            }
                                                            if (self.obj_tab[crate::ix::U((k) as usize)].int0 != 0i32) {
                                                                self.pdf_int_entry_ln(1201i32, self.obj_tab[crate::ix::U((k) as usize)].int0);
                                                            }
                                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(7i32)) as usize)] != 0i32) {
                                                                {
                                                                    self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(7i32)) as usize)]);
                                                                    {
                                                                        self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(7i32)) as usize)]);
                                                                        self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((k) as usize)].int4).wrapping_add(7i32)) as usize)] = null;
                                                                    }
                                                                }
                                                            }
                                                            self.pdf_end_dict();
                                                            k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                        }
                                                    }
                                                }
                                            } else {
                                                // §789
                                                outlines = 0i32;
                                            }
                                            // §805
                                            if (self.pdf_dest_names_ptr == 0i32) {
                                                {
                                                    dests = 0i32;
                                                    break 'l_done1_f;
                                                }
                                            }
                                            self.sort_dest_names(0i32, (self.pdf_dest_names_ptr).wrapping_sub(1i32));
                                            names_head = 0i32;
                                            names_tail = 0i32;
                                            k = 0i32;
                                            is_names = true;
                                            b = 0i32;
                                            loop {
                                                loop {
                                                    self.pdf_create_obj(obj_type_others, 0i32);
                                                    l = self.obj_ptr;
                                                    if (b == 0i32) {
                                                        b = l;
                                                    }
                                                    if (names_head == 0i32) {
                                                        {
                                                            names_head = l;
                                                            names_tail = l;
                                                        }
                                                    } else {
                                                        {
                                                            self.obj_tab[crate::ix::U((names_tail) as usize)].int1 = l;
                                                            names_tail = l;
                                                        }
                                                    }
                                                    self.obj_tab[crate::ix::U((names_tail) as usize)].int1 = 0i32;
                                                    // §806
                                                    self.pdf_begin_dict(l, 1i32);
                                                    j = 0i32;
                                                    if is_names {
                                                        {
                                                            self.obj_tab[crate::ix::U((l) as usize)].int0 = self.dest_names[crate::ix::U((k) as usize)].objname;
                                                            self.pdf_print(1222i32);
                                                            loop {
                                                                self.pdf_print_str(self.dest_names[crate::ix::U((k) as usize)].objname);
                                                                {
                                                                    {
                                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_os_get_os_buf(1i32);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_flush();
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        self.pdf_buf_set(self.pdf_ptr, 32i32);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                                self.pdf_print_int(((self.dest_names[crate::ix::U((k) as usize)].objnum) as i64));
                                                                self.pdf_print(1160i32);
                                                                j = (j).wrapping_add(1i32);
                                                                k = (k).wrapping_add(1i32);
                                                                if ((j == name_tree_kids_max) || (k == self.pdf_dest_names_ptr)) { break; }
                                                            }
                                                            self.remove_last_space();
                                                            {
                                                                self.pdf_print(93i32);
                                                                {
                                                                    {
                                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_os_get_os_buf(1i32);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_flush();
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                            self.obj_tab[crate::ix::U((l) as usize)].int4 = self.dest_names[crate::ix::U(((k).wrapping_sub(1i32)) as usize)].objname;
                                                            if (k == self.pdf_dest_names_ptr) {
                                                                {
                                                                    is_names = false;
                                                                    k = names_head;
                                                                    b = 0i32;
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            self.obj_tab[crate::ix::U((l) as usize)].int0 = self.obj_tab[crate::ix::U((k) as usize)].int0;
                                                            self.pdf_print(1220i32);
                                                            loop {
                                                                self.pdf_print_int(((k) as i64));
                                                                self.pdf_print(1160i32);
                                                                j = (j).wrapping_add(1i32);
                                                                self.obj_tab[crate::ix::U((l) as usize)].int4 = self.obj_tab[crate::ix::U((k) as usize)].int4;
                                                                k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                                if (((j == name_tree_kids_max) || (k == b)) || (self.obj_tab[crate::ix::U((k) as usize)].int1 == 0i32)) { break; }
                                                            }
                                                            self.remove_last_space();
                                                            {
                                                                self.pdf_print(93i32);
                                                                {
                                                                    {
                                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_os_get_os_buf(1i32);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_flush();
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                            if (k == b) {
                                                                b = 0i32;
                                                            }
                                                        }
                                                    }
                                                    self.pdf_print(1223i32);
                                                    self.pdf_print_str(self.obj_tab[crate::ix::U((l) as usize)].int0);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 32i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                    self.pdf_print_str(self.obj_tab[crate::ix::U((l) as usize)].int4);
                                                    {
                                                        self.pdf_print(93i32);
                                                        {
                                                            {
                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_os_get_os_buf(1i32);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                        self.overflow(1018i32, pdf_op_buf_size);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_flush();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            {
                                                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                    self.pdf_end_dict();
                                                    if (b == 0i32) { break; }
                                                }
                                                // §805
                                                if (k == l) {
                                                    {
                                                        dests = l;
                                                        break 'l_done1_f;
                                                    }
                                                }
                                                if false { break; }
                                            }
                                        }
                                        if ((dests != 0i32) || (self.pdf_names_toks != null)) {
                                            {
                                                self.pdf_new_dict(obj_type_others, 0i32, 1i32);
                                                if (dests != 0i32) {
                                                    self.pdf_indirect_ln(1221i32, dests);
                                                }
                                                if (self.pdf_names_toks != null) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_names_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_names_toks);
                                                            self.pdf_names_toks = null;
                                                        }
                                                    }
                                                }
                                                self.pdf_end_dict();
                                                names_tree = self.obj_ptr;
                                            }
                                        } else {
                                            names_tree = 0i32;
                                        }
                                        // §791
                                        if (self.head_tab[crate::ix::U(((obj_type_thread) - 1) as usize)] != 0i32) {
                                            {
                                                self.pdf_new_obj(obj_type_others, 0i32, 1i32);
                                                threads = self.obj_ptr;
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, 91i32);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                k = self.head_tab[crate::ix::U(((obj_type_thread) - 1) as usize)];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_print_int(((k) as i64));
                                                        self.pdf_print(1160i32);
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                                self.remove_last_space();
                                                {
                                                    self.pdf_print(93i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_end_obj();
                                                k = self.head_tab[crate::ix::U(((obj_type_thread) - 1) as usize)];
                                                while (k != 0i32) {
                                                    {
                                                        self.out_thread(k);
                                                        k = self.obj_tab[crate::ix::U((k) as usize)].int1;
                                                    }
                                                }
                                            }
                                        } else {
                                            threads = 0i32;
                                        }
                                        // §807
                                        self.pdf_new_dict(obj_type_others, 0i32, 1i32);
                                        root = self.obj_ptr;
                                        {
                                            self.pdf_print(1224i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.pdf_indirect_ln(1225i32, self.pdf_last_pages);
                                        if (threads != 0i32) {
                                            self.pdf_indirect_ln(1226i32, threads);
                                        }
                                        if (outlines != 0i32) {
                                            self.pdf_indirect_ln(1227i32, outlines);
                                        }
                                        if (names_tree != 0i32) {
                                            self.pdf_indirect_ln(1228i32, names_tree);
                                        }
                                        if (self.pdf_catalog_toks != null) {
                                            {
                                                self.pdf_print_toks_ln(self.pdf_catalog_toks);
                                                {
                                                    self.delete_token_ref(self.pdf_catalog_toks);
                                                    self.pdf_catalog_toks = null;
                                                }
                                            }
                                        }
                                        if (self.pdf_catalog_openaction != 0i32) {
                                            self.pdf_indirect_ln(1229i32, self.pdf_catalog_openaction);
                                        }
                                        self.pdf_end_dict();
                                        // §795
                                        if (__av_eqtb[crate::ix::U(((29376i32) - 1) as usize)].int() == 0i32) {
                                            self.pdf_print_info();
                                        }
                                        if self.pdf_os_enable {
                                            {
                                                self.pdf_os_switch(true);
                                                self.pdf_os_write_objstream();
                                                self.pdf_flush();
                                                self.pdf_os_switch(false);
                                                // §815
                                                self.pdf_new_dict(obj_type_others, 0i32, 0i32);
                                                if (((self.obj_tab[crate::ix::U((self.sys_obj_ptr) as usize)].int2) as f64 / (((256i32) as i64)) as f64) > ((16777215i32) as f64)) {
                                                    xref_offset_width = 5i32;
                                                } else {
                                                    if (self.obj_tab[crate::ix::U((self.sys_obj_ptr) as usize)].int2 > ((16777215i32) as i64)) {
                                                        xref_offset_width = 4i32;
                                                    } else {
                                                        if (self.obj_tab[crate::ix::U((self.sys_obj_ptr) as usize)].int2 > ((65535i32) as i64)) {
                                                            xref_offset_width = 3i32;
                                                        } else {
                                                            xref_offset_width = 2i32;
                                                        }
                                                    }
                                                }
                                                // §813
                                                l = 0i32;
                                                self.obj_tab[crate::ix::U((l) as usize)].int2 = (((2i32).wrapping_neg()) as i64);
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if (!(self.obj_tab[crate::ix::U((k) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                            {
                                                                self.obj_tab[crate::ix::U((l) as usize)].int1 = k;
                                                                l = k;
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.obj_tab[crate::ix::U((l) as usize)].int1 = 0i32;
                                                // §815
                                                {
                                                    self.pdf_print(1246i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1247i32);
                                                self.pdf_print_int((((self.obj_ptr).wrapping_add(1i32)) as i64));
                                                {
                                                    self.pdf_print(93i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_int_entry_ln(1248i32, (self.obj_ptr).wrapping_add(1i32));
                                                self.pdf_print(1249i32);
                                                self.pdf_print_int(((xref_offset_width) as i64));
                                                {
                                                    self.pdf_print(1250i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_indirect_ln(1251i32, root);
                                                if (__av_eqtb[crate::ix::U(((29376i32) - 1) as usize)].int() == 0i32) {
                                                    self.pdf_indirect_ln(1252i32, (self.obj_ptr).wrapping_sub(1i32));
                                                }
                                                if (self.pdf_trailer_toks != null) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_trailer_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_trailer_toks);
                                                            self.pdf_trailer_toks = null;
                                                        }
                                                    }
                                                }
                                                if (self.pdf_trailer_id_toks != null) {
                                                    self.print_ID_alt(self.pdf_trailer_id_toks);
                                                } else {
                                                    self.print_ID(self.output_file_name);
                                                }
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                self.pdf_begin_stream();
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 0i32;
                                                    while k <= __for_end_12 {
                                                        {
                                                            if (!(self.obj_tab[crate::ix::U((k) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                                {
                                                                    {
                                                                        {
                                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_os_get_os_buf(1i32);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_flush();
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        {
                                                                            self.pdf_buf_set(self.pdf_ptr, 0i32);
                                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                    self.pdf_out_bytes(((self.obj_tab[crate::ix::U((k) as usize)].int1) as i64), xref_offset_width);
                                                                    {
                                                                        {
                                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_os_get_os_buf(1i32);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_flush();
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        {
                                                                            self.pdf_buf_set(self.pdf_ptr, 255i32);
                                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.obj_tab[crate::ix::U((k) as usize)].int3 == (1i32).wrapping_neg()) {
                                                                        {
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                                        } else {
                                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                                self.pdf_flush();
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                {
                                                                                    self.pdf_buf_set(self.pdf_ptr, 1i32);
                                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                            self.pdf_out_bytes(self.obj_tab[crate::ix::U((k) as usize)].int2, xref_offset_width);
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                                        } else {
                                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                                self.pdf_flush();
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                {
                                                                                    self.pdf_buf_set(self.pdf_ptr, 0i32);
                                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        {
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                                        } else {
                                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                                self.pdf_flush();
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                {
                                                                                    self.pdf_buf_set(self.pdf_ptr, 2i32);
                                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                            self.pdf_out_bytes(self.obj_tab[crate::ix::U((k) as usize)].int2, xref_offset_width);
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                                                        } else {
                                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                                self.pdf_flush();
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                {
                                                                                    self.pdf_buf_set(self.pdf_ptr, self.obj_tab[crate::ix::U((k) as usize)].int3);
                                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.pdf_end_stream();
                                                // §795
                                                self.pdf_flush();
                                            }
                                        } else {
                                            {
                                                // §813
                                                l = 0i32;
                                                self.obj_tab[crate::ix::U((l) as usize)].int2 = (((2i32).wrapping_neg()) as i64);
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if (!(self.obj_tab[crate::ix::U((k) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                            {
                                                                self.obj_tab[crate::ix::U((l) as usize)].int1 = k;
                                                                l = k;
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.obj_tab[crate::ix::U((l) as usize)].int1 = 0i32;
                                                // §814
                                                self.pdf_save_offset = (self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64));
                                                {
                                                    self.pdf_print(1241i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1242i32);
                                                {
                                                    self.pdf_print_int((((self.obj_ptr).wrapping_add(1i32)) as i64));
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print_fw_int(((self.obj_tab[crate::ix::U((0i32) as usize)].int1) as i64), 10i32);
                                                {
                                                    self.pdf_print(1243i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                {
                                                    let __for_end_12 = self.obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        {
                                                            if (!(self.obj_tab[crate::ix::U((k) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                                {
                                                                    self.pdf_print_fw_int(((self.obj_tab[crate::ix::U((k) as usize)].int1) as i64), 10i32);
                                                                    {
                                                                        self.pdf_print(1244i32);
                                                                        {
                                                                            {
                                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_os_get_os_buf(1i32);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                        self.overflow(1018i32, pdf_op_buf_size);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                            self.pdf_flush();
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            {
                                                                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    self.pdf_print_fw_int(self.obj_tab[crate::ix::U((k) as usize)].int2, 10i32);
                                                                    {
                                                                        self.pdf_print(1245i32);
                                                                        {
                                                                            {
                                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_os_get_os_buf(1i32);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                        self.overflow(1018i32, pdf_op_buf_size);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                            self.pdf_flush();
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            {
                                                                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                            }
                                        }
                                        // §816
                                        if (!self.pdf_os_enable) {
                                            {
                                                {
                                                    self.pdf_print(1253i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1254i32);
                                                self.pdf_int_entry_ln(1248i32, (self.sys_obj_ptr).wrapping_add(1i32));
                                                self.pdf_indirect_ln(1251i32, root);
                                                if (__av_eqtb[crate::ix::U(((29376i32) - 1) as usize)].int() == 0i32) {
                                                    self.pdf_indirect_ln(1252i32, self.sys_obj_ptr);
                                                }
                                                if (self.pdf_trailer_toks != null) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_trailer_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_trailer_toks);
                                                            self.pdf_trailer_toks = null;
                                                        }
                                                    }
                                                }
                                                if (self.pdf_trailer_id_toks != null) {
                                                    self.print_ID_alt(self.pdf_trailer_id_toks);
                                                } else {
                                                    self.print_ID(self.output_file_name);
                                                }
                                                {
                                                    self.pdf_print(1255i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1018i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        {
                                            self.pdf_print(1256i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        if self.pdf_os_enable {
                                            {
                                                self.pdf_print_int(self.obj_tab[crate::ix::U((self.sys_obj_ptr) as usize)].int2);
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        } else {
                                            {
                                                self.pdf_print_int(self.pdf_save_offset);
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1018i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                        {
                                            self.pdf_print(1257i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1018i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        // §795
                                        self.pdf_flush();
                                        self.print_nl(1012i32);
                                        self.print_file_name(0i32, self.output_file_name, 0i32);
                                        self.print(286i32);
                                        self.print_int(((self.total_pages) as i64));
                                        self.print(1014i32);
                                        if (self.total_pages != 1i32) {
                                            self.print_char(115i32);
                                        }
                                        self.print(1015i32);
                                        self.print_int((self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64)));
                                        self.print(1016i32);
                                    }
                                }
                                self.libpdffinish();
                                if (self.fixed_pdf_draftmode == 0i32) {
                                    { let mut __f0 = ::core::mem::take(&mut self.pdf_file); let __r = self.b_close(&mut __f0); self.pdf_file = __f0; __r };
                                } else {
                                    self.pdf_warning(0i32, 1205i32, true, true);
                                }
                            }
                        }
                        // §1514
                        if self.log_opened {
                            {
                                {
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    crate::system::wr_str(&mut self.log_file, "PDF statistics:");
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    let __w0 = b' ';
                                    let __w1 = self.obj_ptr;
                                    let __w3 = self.obj_tab_size;
                                    let __w5 = sup_obj_tab_size;
                                    let __w6 = b')';
                                    crate::system::wr_char(&mut self.log_file, __w0);
                                    crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " PDF objects out of ");
                                    crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " (max. ");
                                    crate::system::wr_int(&mut self.log_file, __w5, 1i32);
                                    crate::system::wr_char(&mut self.log_file, __w6);
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                if (self.pdf_os_cntr > 0i32) {
                                    {
                                        {
                                            let __w0 = b' ';
                                            let __w1 = ((((self.pdf_os_cntr).wrapping_sub(1i32)).wrapping_mul(pdf_os_max_objs)).wrapping_add(self.pdf_os_objidx)).wrapping_add(1i32);
                                            let __w3 = self.pdf_os_cntr;
                                            crate::system::wr_char(&mut self.log_file, __w0);
                                            crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                                            crate::system::wr_str(&mut self.log_file, " compressed objects within ");
                                            crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                                            crate::system::wr_str(&mut self.log_file, " object stream");
                                        }
                                        if (self.pdf_os_cntr > 1i32) {
                                            {
                                                let __w0 = b's';
                                                crate::system::wr_char(&mut self.log_file, __w0);
                                            }
                                        }
                                        {
                                            crate::system::wr_ln(&mut self.log_file);
                                        }
                                    }
                                }
                                {
                                    let __w0 = b' ';
                                    let __w1 = self.pdf_dest_names_ptr;
                                    let __w3 = self.dest_names_size;
                                    let __w5 = sup_dest_names_size;
                                    let __w6 = b')';
                                    crate::system::wr_char(&mut self.log_file, __w0);
                                    crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " named destinations out of ");
                                    crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " (max. ");
                                    crate::system::wr_int(&mut self.log_file, __w5, 1i32);
                                    crate::system::wr_char(&mut self.log_file, __w6);
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    let __w0 = b' ';
                                    let __w1 = self.pdf_mem_ptr;
                                    let __w3 = self.pdf_mem_size;
                                    let __w5 = sup_pdf_mem_size;
                                    let __w6 = b')';
                                    crate::system::wr_char(&mut self.log_file, __w0);
                                    crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " words of extra memory for PDF output out of ");
                                    crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                                    crate::system::wr_str(&mut self.log_file, " (max. ");
                                    crate::system::wr_int(&mut self.log_file, __w5, 1i32);
                                    crate::system::wr_char(&mut self.log_file, __w6);
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            {
                // §671
                while (self.cur_s > (1i32).wrapping_neg()) {
                    {
                        if (self.cur_s > 0i32) {
                            {
                                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = pop;
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                        } else {
                            {
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = eop;
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
                    self.print_nl(1011i32);
                } else {
                    if (self.cur_s != (2i32).wrapping_neg()) {
                        {
                            {
                                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = post;
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
                            self.dvi_four(__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int());
                            self.dvi_four(self.max_v);
                            self.dvi_four(self.max_h);
                            {
                                { let __ix2480 = self.dvi_ptr; let __v2481 = (self.max_push / 256i32); self.dvi_buf[crate::ix::U((__ix2480) as usize)] = __v2481; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            {
                                { let __ix2482 = self.dvi_ptr; let __v2483 = (self.max_push % 256i32); self.dvi_buf[crate::ix::U((__ix2482) as usize)] = __v2483; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            {
                                { let __ix2484 = self.dvi_ptr; let __v2485 = ((self.total_pages / 256i32) % 256i32); self.dvi_buf[crate::ix::U((__ix2484) as usize)] = __v2485; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            {
                                { let __ix2486 = self.dvi_ptr; let __v2487 = (self.total_pages % 256i32); self.dvi_buf[crate::ix::U((__ix2486) as usize)] = __v2487; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            // §672
                            while (self.font_ptr > font_base) {
                                {
                                    if self.font_used[crate::ix::U((self.font_ptr) as usize)] {
                                        self.dvi_font_def(self.font_ptr);
                                    }
                                    self.font_ptr = (self.font_ptr).wrapping_sub(1i32);
                                }
                            }
                            // §671
                            {
                                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = post_post;
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            self.dvi_four(self.last_bop);
                            {
                                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = id_byte;
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            k = (4i32).wrapping_add(((dvi_buf_size).wrapping_sub(self.dvi_ptr) % 4i32));
                            while (k > 0i32) {
                                {
                                    {
                                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = 223i32;
                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                        if (self.dvi_ptr == self.dvi_limit) {
                                            self.dvi_swap();
                                        }
                                    }
                                    k = (k).wrapping_sub(1i32);
                                }
                            }
                            // §627
                            if (self.dvi_limit == self.half_buf) {
                                self.write_dvi(self.half_buf, (dvi_buf_size).wrapping_sub(1i32));
                            }
                            if (self.dvi_ptr > (2147483647i32).wrapping_sub(self.dvi_offset)) {
                                {
                                    self.cur_s = (2i32).wrapping_neg();
                                    self.fatal_error(1000i32);
                                }
                            }
                            if (self.dvi_ptr > 0i32) {
                                self.write_dvi(0i32, (self.dvi_ptr).wrapping_sub(1i32));
                            }
                            // §671
                            self.print_nl(1012i32);
                            self.print_file_name(0i32, self.output_file_name, 0i32);
                            self.print(286i32);
                            self.print_int(((self.total_pages) as i64));
                            if (self.total_pages != 1i32) {
                                self.print(1013i32);
                            } else {
                                self.print(1014i32);
                            }
                            self.print(1015i32);
                            self.print_int((((self.dvi_offset).wrapping_add(self.dvi_ptr)) as i64));
                            self.print(1016i32);
                            { let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.b_close(&mut __f0); self.dvi_file = __f0; __r };
                        }
                    }
                }
            }
        }
        // §1514
        if self.log_opened {
            {
                {
                    crate::system::wr_ln(&mut self.log_file);
                }
                { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_close(&mut __f0); self.log_file = __f0; __r };
                self.selector = (self.selector).wrapping_sub(2i32);
                if (self.selector == term_only) {
                    {
                        self.print_nl(1734i32);
                        self.print_file_name(0i32, self.log_name, 0i32);
                        self.print_char(46i32);
                    }
                }
            }
        }
        self.print_ln();
        if ((self.edit_name_start != 0i32) && (self.interaction > batch_mode)) {
            self.call_edit(self.edit_name_start, self.edit_name_length, self.edit_line);
        }
    }

    /// We get to the `final_cleanup` routine when \.{\\end} or \.{\\dump} has
    /// been scanned and `its_all_over`\kern-2pt.
    /// @<Last-minute...
    // §1516
    pub fn final_cleanup(&mut self) {
        let mut c: small_number = 0; // §1516
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            c = self.cur_chr;
            if (c != 1i32) {
                __av_eqtb[crate::ix::U(((29326i32) - 1) as usize)].set_int((1i32).wrapping_neg());
            }
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            while (self.input_ptr > 0i32) {
                if (self.cur_input.state_field == token_list) {
                    self.end_token_list();
                } else {
                    self.end_file_reading();
                }
            }
            while (self.open_parens > 0i32) {
                {
                    self.print(1735i32);
                    self.open_parens = (self.open_parens).wrapping_sub(1i32);
                }
            }
            if (self.cur_level > level_one) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1736i32);
                    self.print(1737i32);
                    self.print_int((((self.cur_level).wrapping_sub(1i32)) as i64));
                    self.print_char(41i32);
                    if (self.eTeX_mode == 1i32) {
                        self.show_save_groups();
                    }
                }
            }
            while (self.cond_ptr != null) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1736i32);
                    self.print(1738i32);
                    self.print_cmd_chr(if_test, self.cur_if);
                    if (self.if_line != 0i32) {
                        {
                            self.ls_print_unknown(self.if_line);
                            self.print(1739i32);
                            self.print_int(((self.if_line) as i64));
                        }
                    }
                    self.print(1740i32);
                    self.if_line = __av_mem[crate::ix::U(((self.cond_ptr).wrapping_add(1i32)) as usize)].int();
                    self.cur_if = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().b1();
                    self.temp_ptr = self.cond_ptr;
                    self.cond_ptr = __av_mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
                    self.free_node(self.temp_ptr, if_node_size);
                    self.ls_cond_depth = (self.ls_cond_depth).wrapping_sub(1i32);
                }
            }
            if (self.history != spotless) {
                if ((self.history == warning_issued) || (self.interaction < error_stop_mode)) {
                    if (self.selector == term_and_log) {
                        {
                            self.selector = term_only;
                            self.print_nl(1741i32);
                            self.selector = term_and_log;
                        }
                    }
                }
            }
            if (c == 1i32) {
                {
                    {
                        let __for_end_5 = split_bot_mark_code;
                        c = top_mark_code;
                        while c <= __for_end_5 {
                            if (self.cur_mark[crate::ix::U((c) as usize)] != null) {
                                self.delete_token_ref(self.cur_mark[crate::ix::U((c) as usize)]);
                            }
                            c = c.wrapping_add(1);
                        }
                    }
                    if (self.sa_root[crate::ix::U((mark_val) as usize)] != null) {
                        if self.do_marks(destroy_marks, 0i32, self.sa_root[crate::ix::U((mark_val) as usize)]) {
                            self.sa_root[crate::ix::U((mark_val) as usize)] = null;
                        }
                    }
                    {
                        let __for_end_5 = vsplit_code;
                        c = last_box_code;
                        while c <= __for_end_5 {
                            self.flush_node_list(self.disc_ptr[crate::ix::U(((c) - 1) as usize)]);
                            c = c.wrapping_add(1);
                        }
                    }
                    if (self.last_glue != max_halfword) {
                        self.delete_glue_ref(self.last_glue);
                    }
                    self.store_fmt_file();
                    break 'l_exit_f;
                    self.print_nl(1742i32);
                    break 'l_exit_f;
                }
            }
        }
    }

    /// @<Last-minute...
    // §1517
    pub fn init_prim(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.no_new_control_sequence = false;
        self.first = 0i32;
        // §244
        self.primitive(394i32, assign_glue, 26628i32);
        self.primitive(395i32, assign_glue, 26629i32);
        self.primitive(396i32, assign_glue, 26630i32);
        self.primitive(397i32, assign_glue, 26631i32);
        self.primitive(398i32, assign_glue, 26632i32);
        self.primitive(399i32, assign_glue, 26633i32);
        self.primitive(400i32, assign_glue, 26634i32);
        self.primitive(401i32, assign_glue, 26635i32);
        self.primitive(402i32, assign_glue, 26636i32);
        self.primitive(403i32, assign_glue, 26637i32);
        self.primitive(404i32, assign_glue, 26638i32);
        self.primitive(405i32, assign_glue, 26639i32);
        self.primitive(406i32, assign_glue, 26640i32);
        self.primitive(407i32, assign_glue, 26641i32);
        self.primitive(408i32, assign_glue, 26642i32);
        self.primitive(409i32, assign_mu_glue, 26643i32);
        self.primitive(410i32, assign_mu_glue, 26644i32);
        self.primitive(411i32, assign_mu_glue, 26645i32);
        // §248
        self.primitive(415i32, assign_toks, output_routine_loc);
        self.primitive(416i32, assign_toks, every_par_loc);
        self.primitive(417i32, assign_toks, every_math_loc);
        self.primitive(418i32, assign_toks, every_display_loc);
        self.primitive(419i32, assign_toks, every_hbox_loc);
        self.primitive(420i32, assign_toks, every_vbox_loc);
        self.primitive(421i32, assign_toks, every_job_loc);
        self.primitive(422i32, assign_toks, every_cr_loc);
        self.primitive(423i32, assign_toks, err_help_loc);
        self.primitive(424i32, assign_toks, pdf_pages_attr_loc);
        self.primitive(425i32, assign_toks, pdf_page_attr_loc);
        self.primitive(426i32, assign_toks, pdf_page_resources_loc);
        self.primitive(427i32, assign_toks, pdf_pk_mode_loc);
        // §256
        self.primitive(441i32, assign_int, 29277i32);
        self.primitive(442i32, assign_int, 29278i32);
        self.primitive(443i32, assign_int, 29279i32);
        self.primitive(444i32, assign_int, 29280i32);
        self.primitive(445i32, assign_int, 29281i32);
        self.primitive(446i32, assign_int, 29282i32);
        self.primitive(447i32, assign_int, 29283i32);
        self.primitive(448i32, assign_int, 29284i32);
        self.primitive(449i32, assign_int, 29285i32);
        self.primitive(450i32, assign_int, 29286i32);
        self.primitive(451i32, assign_int, 29287i32);
        self.primitive(452i32, assign_int, 29288i32);
        self.primitive(453i32, assign_int, 29289i32);
        self.primitive(454i32, assign_int, 29290i32);
        self.primitive(455i32, assign_int, 29291i32);
        self.primitive(456i32, assign_int, 29292i32);
        self.primitive(457i32, assign_int, 29293i32);
        self.primitive(458i32, assign_int, 29294i32);
        self.primitive(459i32, assign_int, 29295i32);
        self.primitive(460i32, assign_int, 29296i32);
        self.primitive(461i32, assign_int, 29297i32);
        self.primitive(462i32, assign_int, 29298i32);
        self.primitive(463i32, assign_int, 29299i32);
        self.primitive(464i32, assign_int, 29300i32);
        self.primitive(465i32, assign_int, 29301i32);
        self.primitive(466i32, assign_int, 29302i32);
        self.primitive(467i32, assign_int, 29303i32);
        self.primitive(468i32, assign_int, 29304i32);
        self.primitive(469i32, assign_int, 29305i32);
        self.primitive(470i32, assign_int, 29306i32);
        self.primitive(471i32, assign_int, 29307i32);
        self.primitive(472i32, assign_int, 29308i32);
        self.primitive(473i32, assign_int, 29309i32);
        self.primitive(474i32, assign_int, 29310i32);
        self.primitive(475i32, assign_int, 29311i32);
        self.primitive(476i32, assign_int, 29312i32);
        self.primitive(477i32, assign_int, 29313i32);
        self.primitive(478i32, assign_int, 29314i32);
        self.primitive(479i32, assign_int, 29315i32);
        self.primitive(480i32, assign_int, 29316i32);
        self.primitive(481i32, assign_int, 29317i32);
        self.primitive(482i32, assign_int, 29318i32);
        self.primitive(483i32, assign_int, 29319i32);
        self.primitive(484i32, assign_int, 29320i32);
        self.primitive(485i32, assign_int, 29321i32);
        self.primitive(486i32, assign_int, 29322i32);
        self.primitive(487i32, assign_int, 29323i32);
        self.primitive(488i32, assign_int, 29324i32);
        self.primitive(489i32, assign_int, 29325i32);
        self.primitive(490i32, assign_int, 29326i32);
        self.primitive(491i32, assign_int, 29327i32);
        self.primitive(492i32, assign_int, 29328i32);
        self.primitive(493i32, assign_int, 29329i32);
        self.primitive(494i32, assign_int, 29330i32);
        self.primitive(495i32, assign_int, 29331i32);
        if self.mltex_p {
            {
                self.mltex_enabled_p = true;
                if false {
                    self.primitive(496i32, assign_int, 29332i32);
                }
                self.primitive(497i32, assign_int, 29333i32);
                self.primitive(498i32, assign_int, 29334i32);
            }
        }
        if self.enctex_p {
            {
                self.enctex_enabled_p = true;
                self.primitive(499i32, assign_int, 29338i32);
                self.primitive(500i32, assign_int, 29339i32);
                self.primitive(501i32, assign_int, 29340i32);
                self.primitive(502i32, assign_int, 29341i32);
            }
        }
        self.primitive(503i32, assign_int, 29335i32);
        self.primitive(544i32, partoken_name, 0i32);
        self.primitive(504i32, assign_int, 29336i32);
        self.primitive(505i32, assign_int, 29337i32);
        self.primitive(506i32, assign_int, 29342i32);
        self.primitive(507i32, assign_int, 29343i32);
        self.primitive(508i32, assign_int, 29363i32);
        self.primitive(509i32, assign_int, 29344i32);
        self.primitive(510i32, assign_int, 29345i32);
        self.primitive(511i32, assign_int, 29346i32);
        self.primitive(512i32, assign_int, 29347i32);
        self.primitive(513i32, assign_int, 29348i32);
        self.primitive(545i32, assign_int, 29352i32);
        self.primitive(514i32, assign_int, 29349i32);
        self.primitive(515i32, assign_int, 29350i32);
        self.primitive(516i32, assign_int, 29351i32);
        self.primitive(517i32, assign_int, 29352i32);
        self.primitive(518i32, assign_int, 29353i32);
        self.primitive(519i32, assign_int, 29354i32);
        self.primitive(520i32, assign_int, 29355i32);
        self.primitive(521i32, assign_int, 29356i32);
        self.primitive(522i32, assign_int, 29357i32);
        self.primitive(523i32, assign_int, 29358i32);
        self.primitive(524i32, assign_int, 29359i32);
        self.primitive(525i32, assign_int, 29360i32);
        self.primitive(526i32, assign_int, 29361i32);
        self.primitive(527i32, assign_int, 29362i32);
        self.primitive(528i32, assign_int, 29364i32);
        self.primitive(529i32, assign_int, 29365i32);
        self.primitive(530i32, assign_int, 29366i32);
        self.primitive(531i32, assign_int, 29367i32);
        self.primitive(532i32, assign_int, 29368i32);
        self.primitive(533i32, assign_int, 29369i32);
        self.primitive(534i32, assign_int, 29370i32);
        self.primitive(535i32, assign_int, 29371i32);
        self.primitive(536i32, assign_int, 29372i32);
        self.primitive(537i32, assign_int, 29373i32);
        self.primitive(538i32, assign_int, 29374i32);
        self.primitive(539i32, assign_int, 29375i32);
        self.primitive(540i32, assign_int, 29376i32);
        self.primitive(541i32, assign_int, 29377i32);
        self.primitive(542i32, assign_int, 29378i32);
        // §266
        self.primitive(548i32, assign_dimen, 29903i32);
        self.primitive(549i32, assign_dimen, 29904i32);
        self.primitive(550i32, assign_dimen, 29905i32);
        self.primitive(551i32, assign_dimen, 29906i32);
        self.primitive(552i32, assign_dimen, 29907i32);
        self.primitive(553i32, assign_dimen, 29908i32);
        self.primitive(554i32, assign_dimen, 29909i32);
        self.primitive(555i32, assign_dimen, 29910i32);
        self.primitive(556i32, assign_dimen, 29911i32);
        self.primitive(557i32, assign_dimen, 29912i32);
        self.primitive(558i32, assign_dimen, 29913i32);
        self.primitive(559i32, assign_dimen, 29914i32);
        self.primitive(560i32, assign_dimen, 29915i32);
        self.primitive(561i32, assign_dimen, 29916i32);
        self.primitive(562i32, assign_dimen, 29917i32);
        self.primitive(563i32, assign_dimen, 29918i32);
        self.primitive(564i32, assign_dimen, 29919i32);
        self.primitive(565i32, assign_dimen, 29920i32);
        self.primitive(566i32, assign_dimen, 29921i32);
        self.primitive(567i32, assign_dimen, 29922i32);
        self.primitive(568i32, assign_dimen, 29923i32);
        self.primitive(569i32, assign_dimen, 29924i32);
        self.primitive(570i32, assign_dimen, 29925i32);
        self.primitive(571i32, assign_dimen, 29926i32);
        self.primitive(572i32, assign_dimen, 29927i32);
        self.primitive(573i32, assign_dimen, 29928i32);
        self.primitive(574i32, assign_dimen, 29929i32);
        self.primitive(575i32, assign_dimen, 29930i32);
        self.primitive(576i32, assign_dimen, 29931i32);
        self.primitive(577i32, assign_dimen, 29932i32);
        self.primitive(578i32, assign_dimen, 29933i32);
        self.primitive(579i32, assign_dimen, 29934i32);
        self.primitive(580i32, assign_dimen, 29935i32);
        self.primitive(581i32, assign_dimen, 29936i32);
        // §287
        self.primitive(32i32, ex_space, 0i32);
        self.primitive(47i32, ital_corr, 0i32);
        self.primitive(593i32, accent, 0i32);
        self.primitive(594i32, advance, 0i32);
        self.primitive(595i32, after_assignment, 0i32);
        self.primitive(596i32, after_group, 0i32);
        self.primitive(597i32, begin_group, 0i32);
        self.primitive(598i32, char_num, 0i32);
        self.primitive(589i32, cs_name, 0i32);
        self.primitive(599i32, delim_num, 0i32);
        self.primitive(600i32, divide, 0i32);
        self.primitive(590i32, end_cs_name, 0i32);
        if self.enctex_p {
            {
                self.primitive(601i32, end_cs_name, 10i32);
            }
        }
        self.primitive(602i32, end_group, 0i32);
        self.hash[crate::ix::U(((frozen_end_group) - 514) as usize)].set_rh(602i32);
        { let __v2488 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_end_group) - 1) as usize)] = __v2488; }
        self.primitive(603i32, expand_after, 0i32);
        self.primitive(604i32, def_font, 0i32);
        self.primitive(605i32, letterspace_font, 0i32);
        self.primitive(606i32, pdf_copy_font, 0i32);
        self.primitive(607i32, assign_font_dimen, 0i32);
        self.primitive(608i32, halign, 0i32);
        self.primitive(609i32, hrule, 0i32);
        self.primitive(610i32, ignore_spaces, 0i32);
        self.primitive(337i32, insert, 0i32);
        self.primitive(360i32, mark, 0i32);
        self.primitive(611i32, math_accent, 0i32);
        self.primitive(612i32, math_char_num, 0i32);
        self.primitive(613i32, math_choice, 0i32);
        self.primitive(614i32, multiply, 0i32);
        self.primitive(615i32, no_align, 0i32);
        self.primitive(616i32, no_boundary, 0i32);
        self.primitive(617i32, no_expand, 0i32);
        self.primitive(586i32, no_expand, 1i32);
        self.primitive(342i32, non_script, 0i32);
        self.primitive(618i32, omit, 0i32);
        self.primitive(619i32, set_shape, par_shape_loc);
        self.primitive(620i32, break_penalty, 0i32);
        self.primitive(621i32, set_prev_graf, 0i32);
        self.primitive(622i32, radical, 0i32);
        self.primitive(623i32, read_to_cs, 0i32);
        self.primitive(624i32, relax, 256i32);
        self.hash[crate::ix::U(((frozen_relax) - 514) as usize)].set_rh(624i32);
        { let __v2489 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_relax) - 1) as usize)] = __v2489; }
        self.primitive(625i32, set_box, 0i32);
        self.primitive(626i32, the, 0i32);
        self.primitive(428i32, toks_register, mem_bot);
        self.primitive(361i32, vadjust, 0i32);
        self.primitive(627i32, valign, 0i32);
        self.primitive(628i32, vcenter, 0i32);
        self.primitive(629i32, vrule, 0i32);
        // §356
        self.primitive(691i32, par_end, 256i32);
        self.par_loc = self.cur_val;
        self.par_token = (cs_token_flag).wrapping_add(self.par_loc);
        // §403
        self.primitive(727i32, input, 0i32);
        self.primitive(728i32, input, 1i32);
        // §411
        self.primitive(729i32, top_bot_mark, top_mark_code);
        self.primitive(730i32, top_bot_mark, first_mark_code);
        self.primitive(731i32, top_bot_mark, bot_mark_code);
        self.primitive(732i32, top_bot_mark, split_first_mark_code);
        self.primitive(733i32, top_bot_mark, split_bot_mark_code);
        // §438
        self.primitive(546i32, register, 0i32);
        self.primitive(583i32, register, 1i32);
        self.primitive(413i32, register, 2i32);
        self.primitive(414i32, register, 3i32);
        // §443
        self.primitive(766i32, set_aux, hmode);
        self.primitive(767i32, set_aux, vmode);
        self.primitive(768i32, set_page_int, 0i32);
        self.primitive(769i32, set_page_int, 1i32);
        self.primitive(770i32, set_box_dimen, width_offset);
        self.primitive(771i32, set_box_dimen, height_offset);
        self.primitive(772i32, set_box_dimen, depth_offset);
        self.primitive(773i32, last_item, int_val);
        self.primitive(774i32, last_item, dimen_val);
        self.primitive(775i32, last_item, glue_val);
        self.primitive(776i32, last_item, input_line_no_code);
        self.primitive(777i32, last_item, badness_code);
        self.primitive(778i32, last_item, pdftex_version_code);
        self.primitive(779i32, last_item, pdf_last_obj_code);
        self.primitive(780i32, last_item, pdf_last_xform_code);
        self.primitive(781i32, last_item, pdf_last_ximage_code);
        self.primitive(782i32, last_item, pdf_last_ximage_pages_code);
        self.primitive(783i32, last_item, pdf_last_annot_code);
        self.primitive(784i32, last_item, pdf_last_x_pos_code);
        self.primitive(785i32, last_item, pdf_last_y_pos_code);
        self.primitive(786i32, last_item, pdf_retval_code);
        self.primitive(787i32, last_item, pdf_last_ximage_colordepth_code);
        self.primitive(788i32, last_item, elapsed_time_code);
        self.primitive(789i32, last_item, pdf_shell_escape_code);
        self.primitive(790i32, last_item, random_seed_code);
        self.primitive(791i32, last_item, pdf_last_link_code);
        // §495
        self.primitive(850i32, convert, number_code);
        self.primitive(851i32, convert, roman_numeral_code);
        self.primitive(852i32, convert, string_code);
        self.primitive(853i32, convert, meaning_code);
        self.primitive(854i32, convert, font_name_code);
        self.primitive(855i32, convert, expanded_code);
        self.primitive(856i32, convert, pdftex_revision_code);
        self.primitive(857i32, convert, pdftex_banner_code);
        self.primitive(858i32, convert, pdf_font_name_code);
        self.primitive(859i32, convert, pdf_font_objnum_code);
        self.primitive(860i32, convert, pdf_font_size_code);
        self.primitive(861i32, convert, pdf_page_ref_code);
        self.primitive(862i32, convert, left_margin_kern_code);
        self.primitive(863i32, convert, right_margin_kern_code);
        self.primitive(864i32, convert, pdf_xform_name_code);
        self.primitive(865i32, convert, pdf_escape_string_code);
        self.primitive(866i32, convert, pdf_escape_name_code);
        self.primitive(867i32, convert, pdf_escape_hex_code);
        self.primitive(868i32, convert, pdf_unescape_hex_code);
        self.primitive(869i32, convert, pdf_creation_date_code);
        self.primitive(870i32, convert, pdf_file_mod_date_code);
        self.primitive(871i32, convert, pdf_file_size_code);
        self.primitive(872i32, convert, pdf_mdfive_sum_code);
        self.primitive(873i32, convert, pdf_file_dump_code);
        self.primitive(874i32, convert, pdf_match_code);
        self.primitive(875i32, convert, pdf_last_match_code);
        self.primitive(876i32, convert, pdf_strcmp_code);
        self.primitive(877i32, convert, pdf_colorstack_init_code);
        self.primitive(878i32, convert, uniform_deviate_code);
        self.primitive(879i32, convert, normal_deviate_code);
        self.primitive(880i32, convert, job_name_code);
        self.primitive(881i32, convert, pdf_insert_ht_code);
        self.primitive(882i32, convert, pdf_ximage_bbox_code);
        // §514
        self.primitive(922i32, if_test, if_char_code);
        self.primitive(923i32, if_test, if_cat_code);
        self.primitive(924i32, if_test, if_int_code);
        self.primitive(925i32, if_test, if_dim_code);
        self.primitive(926i32, if_test, if_odd_code);
        self.primitive(927i32, if_test, if_vmode_code);
        self.primitive(928i32, if_test, if_hmode_code);
        self.primitive(929i32, if_test, if_mmode_code);
        self.primitive(930i32, if_test, if_inner_code);
        self.primitive(931i32, if_test, if_void_code);
        self.primitive(932i32, if_test, if_hbox_code);
        self.primitive(933i32, if_test, if_vbox_code);
        self.primitive(934i32, if_test, ifx_code);
        self.primitive(935i32, if_test, if_eof_code);
        self.primitive(936i32, if_test, if_true_code);
        self.primitive(937i32, if_test, if_false_code);
        self.primitive(938i32, if_test, if_case_code);
        self.primitive(939i32, if_test, if_pdfprimitive_code);
        // §518
        self.primitive(941i32, fi_or_else, fi_code);
        self.hash[crate::ix::U(((frozen_fi) - 514) as usize)].set_rh(941i32);
        { let __v2490 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_fi) - 1) as usize)] = __v2490; }
        self.primitive(942i32, fi_or_else, or_code);
        self.primitive(943i32, fi_or_else, else_code);
        // §580
        self.primitive(969i32, set_font, null_font);
        self.hash[crate::ix::U(((frozen_null_font) - 514) as usize)].set_rh(969i32);
        { let __v2491 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_null_font) - 1) as usize)] = __v2491; }
        // §957
        self.primitive(1321i32, tab_mark, span_code);
        self.primitive(1322i32, car_ret, cr_code);
        self.hash[crate::ix::U(((frozen_cr) - 514) as usize)].set_rh(1322i32);
        { let __v2492 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_cr) - 1) as usize)] = __v2492; }
        self.primitive(1323i32, car_ret, cr_cr_code);
        self.hash[crate::ix::U(((frozen_end_template) - 514) as usize)].set_rh(1324i32);
        self.hash[crate::ix::U(((frozen_endv) - 514) as usize)].set_rh(1324i32);
        __av_eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_b0(endv);
        __av_eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_rh(null_list);
        __av_eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_b1(level_one);
        { let __v2493 = __av_eqtb[crate::ix::U(((frozen_endv) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_end_template) - 1) as usize)] = __v2493; }
        __av_eqtb[crate::ix::U(((frozen_end_template) - 1) as usize)].set_hh_b0(end_template);
        // §1161
        self.primitive(1399i32, set_page_dimen, 0i32);
        self.primitive(1400i32, set_page_dimen, 1i32);
        self.primitive(1401i32, set_page_dimen, 2i32);
        self.primitive(1402i32, set_page_dimen, 3i32);
        self.primitive(1403i32, set_page_dimen, 4i32);
        self.primitive(1404i32, set_page_dimen, 5i32);
        self.primitive(1405i32, set_page_dimen, 6i32);
        self.primitive(1406i32, set_page_dimen, 7i32);
        // §1231
        self.primitive(350i32, stop, 0i32);
        self.primitive(1451i32, stop, 1i32);
        // §1237
        self.primitive(1452i32, hskip, skip_code);
        self.primitive(1453i32, hskip, fil_code);
        self.primitive(1454i32, hskip, fill_code);
        self.primitive(1455i32, hskip, ss_code);
        self.primitive(1456i32, hskip, fil_neg_code);
        self.primitive(1457i32, vskip, skip_code);
        self.primitive(1458i32, vskip, fil_code);
        self.primitive(1459i32, vskip, fill_code);
        self.primitive(1460i32, vskip, ss_code);
        self.primitive(1461i32, vskip, fil_neg_code);
        self.primitive(343i32, mskip, mskip_code);
        self.primitive(322i32, kern, explicit);
        self.primitive(349i32, mkern, mu_glue);
        // §1250
        self.primitive(1479i32, hmove, 1i32);
        self.primitive(1480i32, hmove, 0i32);
        self.primitive(1481i32, vmove, 1i32);
        self.primitive(1482i32, vmove, 0i32);
        self.primitive(430i32, make_box, box_code);
        self.primitive(1483i32, make_box, copy_code);
        self.primitive(1484i32, make_box, last_box_code);
        self.primitive(1394i32, make_box, vsplit_code);
        self.primitive(1485i32, make_box, vtop_code);
        self.primitive(1396i32, make_box, 5i32);
        self.primitive(1486i32, make_box, 109i32);
        self.primitive(1487i32, leader_ship, 99i32);
        self.primitive(1488i32, leader_ship, a_leaders);
        self.primitive(1489i32, leader_ship, c_leaders);
        self.primitive(1490i32, leader_ship, x_leaders);
        // §1267
        self.primitive(1506i32, start_par, 1i32);
        self.primitive(1507i32, start_par, 0i32);
        self.primitive(1508i32, start_par, 2i32);
        // §1286
        self.primitive(1518i32, remove_item, penalty_node);
        self.primitive(1519i32, remove_item, kern_node);
        self.primitive(1520i32, remove_item, glue_node);
        self.primitive(1521i32, un_hbox, box_code);
        self.primitive(1522i32, un_hbox, copy_code);
        self.primitive(1523i32, un_vbox, box_code);
        self.primitive(1524i32, un_vbox, copy_code);
        // §1293
        self.primitive(45i32, discretionary, 1i32);
        self.primitive(358i32, discretionary, 0i32);
        // §1320
        self.primitive(1556i32, eq_no, 0i32);
        self.primitive(1557i32, eq_no, 1i32);
        // §1335
        self.primitive(1288i32, math_comp, ord_noad);
        self.primitive(1289i32, math_comp, op_noad);
        self.primitive(1290i32, math_comp, bin_noad);
        self.primitive(1291i32, math_comp, rel_noad);
        self.primitive(1292i32, math_comp, open_noad);
        self.primitive(1293i32, math_comp, close_noad);
        self.primitive(1294i32, math_comp, punct_noad);
        self.primitive(1295i32, math_comp, inner_noad);
        self.primitive(1297i32, math_comp, under_noad);
        self.primitive(1296i32, math_comp, over_noad);
        self.primitive(1558i32, limit_switch, normal);
        self.primitive(1301i32, limit_switch, limits);
        self.primitive(1302i32, limit_switch, no_limits);
        // §1348
        self.primitive(1283i32, math_style, display_style);
        self.primitive(1284i32, math_style, text_style);
        self.primitive(1285i32, math_style, script_style);
        self.primitive(1286i32, math_style, script_script_style);
        // §1357
        self.primitive(1576i32, above, above_code);
        self.primitive(1577i32, above, over_code);
        self.primitive(1578i32, above, atop_code);
        self.primitive(1579i32, above, 3i32);
        self.primitive(1580i32, above, 4i32);
        self.primitive(1581i32, above, 5i32);
        // §1367
        self.primitive(1298i32, left_right, left_noad);
        self.primitive(1299i32, left_right, right_noad);
        self.hash[crate::ix::U(((frozen_right) - 514) as usize)].set_rh(1299i32);
        { let __v2494 = __av_eqtb[crate::ix::U(((self.cur_val) - 1) as usize)]; __av_eqtb[crate::ix::U(((frozen_right) - 1) as usize)] = __v2494; }
        // §1387
        self.primitive(1601i32, prefix, 1i32);
        self.primitive(1602i32, prefix, 2i32);
        self.primitive(1603i32, prefix, 4i32);
        self.primitive(1604i32, def, 0i32);
        self.primitive(1605i32, def, 1i32);
        self.primitive(1606i32, def, 2i32);
        self.primitive(1607i32, def, 3i32);
        // §1398
        self.primitive(1624i32, let_, normal);
        self.primitive(1625i32, let_, 1i32);
        if self.enctex_p {
            {
                self.primitive(1626i32, let_, 10i32);
                self.primitive(1627i32, let_, 11i32);
            }
        }
        // §1401
        self.primitive(1633i32, shorthand_def, char_def_code);
        self.primitive(1634i32, shorthand_def, math_char_def_code);
        self.primitive(1635i32, shorthand_def, count_def_code);
        self.primitive(1636i32, shorthand_def, dimen_def_code);
        self.primitive(1637i32, shorthand_def, skip_def_code);
        self.primitive(1638i32, shorthand_def, mu_skip_def_code);
        self.primitive(1639i32, shorthand_def, toks_def_code);
        if self.mltex_p {
            {
                self.primitive(1640i32, shorthand_def, char_sub_def_code);
            }
        }
        // §1409
        self.primitive(436i32, def_code, cat_code_base);
        if self.enctex_p {
            {
                self.primitive(1645i32, def_code, xord_code_base);
                self.primitive(1646i32, def_code, xchr_code_base);
                self.primitive(1647i32, def_code, xprn_code_base);
            }
        }
        self.primitive(440i32, def_code, math_code_base);
        self.primitive(437i32, def_code, lc_code_base);
        self.primitive(438i32, def_code, uc_code_base);
        self.primitive(439i32, def_code, sf_code_base);
        self.primitive(547i32, def_code, del_code_base);
        self.primitive(433i32, def_family, math_font_base);
        self.primitive(434i32, def_family, 27709i32);
        self.primitive(435i32, def_family, 27725i32);
        // §1429
        self.primitive(1368i32, hyph_data, 0i32);
        self.primitive(1380i32, hyph_data, 1i32);
        // §1433
        self.primitive(1663i32, assign_font_int, 0i32);
        self.primitive(1664i32, assign_font_int, 1i32);
        self.primitive(1665i32, assign_font_int, lp_code_base);
        self.primitive(1666i32, assign_font_int, rp_code_base);
        self.primitive(1667i32, assign_font_int, ef_code_base);
        self.primitive(1668i32, assign_font_int, tag_code);
        self.primitive(1669i32, assign_font_int, kn_bs_code_base);
        self.primitive(1670i32, assign_font_int, st_bs_code_base);
        self.primitive(1671i32, assign_font_int, sh_bs_code_base);
        self.primitive(1672i32, assign_font_int, kn_bc_code_base);
        self.primitive(1673i32, assign_font_int, kn_ac_code_base);
        self.primitive(1674i32, assign_font_int, no_lig_code);
        // §1441
        self.primitive(274i32, set_interaction, batch_mode);
        self.primitive(275i32, set_interaction, nonstop_mode);
        self.primitive(276i32, set_interaction, scroll_mode);
        self.primitive(1682i32, set_interaction, error_stop_mode);
        // §1451
        self.primitive(1683i32, in_stream, 1i32);
        self.primitive(1684i32, in_stream, 0i32);
        // §1456
        self.primitive(1685i32, message, 0i32);
        self.primitive(1686i32, message, 1i32);
        // §1465
        self.primitive(1692i32, case_shift, lc_code_base);
        self.primitive(1693i32, case_shift, uc_code_base);
        // §1470
        self.primitive(1694i32, xray, show_code);
        self.primitive(1695i32, xray, show_box_code);
        self.primitive(1696i32, xray, show_the_code);
        self.primitive(1697i32, xray, show_lists_code);
        // §1525
        self.primitive(1744i32, extension, open_node);
        self.primitive(687i32, extension, write_node);
        self.write_loc = self.cur_val;
        self.primitive(1745i32, extension, close_node);
        self.primitive(1746i32, extension, special_node);
        self.primitive(1747i32, extension, immediate_code);
        self.primitive(1748i32, extension, set_language_code);
        self.primitive(1749i32, extension, pdf_literal_node);
        self.primitive(1145i32, extension, pdf_colorstack_node);
        self.primitive(1750i32, extension, pdf_setmatrix_node);
        self.primitive(1751i32, extension, pdf_save_node);
        self.primitive(1752i32, extension, pdf_restore_node);
        self.primitive(1753i32, extension, pdf_obj_code);
        self.primitive(1754i32, extension, pdf_refobj_node);
        self.primitive(1755i32, extension, pdf_xform_code);
        self.primitive(1756i32, extension, pdf_refxform_node);
        self.primitive(1757i32, extension, pdf_ximage_code);
        self.primitive(1758i32, extension, pdf_refximage_node);
        self.primitive(1759i32, extension, pdf_annot_node);
        self.primitive(1760i32, extension, pdf_start_link_node);
        self.primitive(1761i32, extension, pdf_end_link_node);
        self.primitive(1762i32, extension, pdf_outline_code);
        self.primitive(1763i32, extension, pdf_dest_node);
        self.primitive(1764i32, extension, pdf_thread_node);
        self.primitive(1765i32, extension, pdf_start_thread_node);
        self.primitive(1766i32, extension, pdf_end_thread_node);
        self.primitive(1767i32, extension, pdf_save_pos_node);
        self.primitive(1768i32, extension, pdf_snap_ref_point_node);
        self.primitive(1769i32, extension, pdf_snapy_node);
        self.primitive(1770i32, extension, pdf_snapy_comp_node);
        self.primitive(1771i32, extension, pdf_info_code);
        self.primitive(1772i32, extension, pdf_catalog_code);
        self.primitive(1773i32, extension, pdf_names_code);
        self.primitive(1774i32, extension, pdf_include_chars_code);
        self.primitive(1775i32, extension, pdf_font_attr_code);
        self.primitive(1776i32, extension, pdf_map_file_code);
        self.primitive(1777i32, extension, pdf_map_line_code);
        self.primitive(1778i32, extension, pdf_trailer_code);
        self.primitive(1779i32, extension, pdf_trailer_id_code);
        self.primitive(1780i32, extension, reset_timer_code);
        self.primitive(1781i32, extension, set_random_seed_code);
        self.primitive(1782i32, extension, pdf_font_expand_code);
        self.primitive(1783i32, extension, pdf_glyph_to_unicode_code);
        self.primitive(1784i32, extension, pdf_nobuiltin_tounicode_code);
        self.primitive(1785i32, extension, pdf_interword_space_on_node);
        self.primitive(1786i32, extension, pdf_interword_space_off_node);
        self.primitive(1787i32, extension, pdf_fake_space_node);
        self.primitive(1788i32, extension, pdf_running_link_off_node);
        self.primitive(1789i32, extension, pdf_running_link_on_node);
        self.primitive(1790i32, extension, pdf_space_font_code);
        // §1911
        self.primitive(2104i32, assign_int, 29390i32);
        // §1517
        self.no_new_control_sequence = true;
    }

}
