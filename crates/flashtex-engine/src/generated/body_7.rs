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
    /// @<Declare act...
    // §1305
    pub fn align_error(&mut self) {
        if ((self.align_state).wrapping_abs() > 2i32) {
            // §1306
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1529i32);
                }
                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                if (self.cur_tok == 1062i32) {
                    {
                        {
                            self.help_ptr = 6i32;
                            self.help_line[(5i32) as usize] = 1530i32;
                            self.help_line[(4i32) as usize] = 1531i32;
                            self.help_line[(3i32) as usize] = 1532i32;
                            self.help_line[(2i32) as usize] = 1533i32;
                            self.help_line[(1i32) as usize] = 1534i32;
                            self.help_line[(0i32) as usize] = 1535i32;
                        }
                    }
                } else {
                    {
                        {
                            self.help_ptr = 5i32;
                            self.help_line[(4i32) as usize] = 1530i32;
                            self.help_line[(3i32) as usize] = 1536i32;
                            self.help_line[(2i32) as usize] = 1533i32;
                            self.help_line[(1i32) as usize] = 1534i32;
                            self.help_line[(0i32) as usize] = 1535i32;
                        }
                    }
                }
                self.error();
            }
        } else {
            // §1305
            {
                self.back_input();
                if (self.align_state < 0i32) {
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
                        self.align_state = (self.align_state).wrapping_add(1i32);
                        self.cur_tok = 379i32;
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
                            self.print(1525i32);
                        }
                        self.align_state = (self.align_state).wrapping_sub(1i32);
                        self.cur_tok = 637i32;
                    }
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1526i32;
                    self.help_line[(1i32) as usize] = 1527i32;
                    self.help_line[(0i32) as usize] = 1528i32;
                }
                self.ins_error();
            }
        }
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1307
    pub fn no_align_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1529i32);
        }
        self.print_esc(605i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 1537i32;
            self.help_line[(0i32) as usize] = 1538i32;
        }
        self.error();
    }

    /// The help messages here contain a little white lie, since \.{\\noalign}
    /// and \.{\\omit} are allowed also after `\.{\\noalign\{...\}}'.
    /// @<Declare act...
    // §1307
    pub fn omit_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1529i32);
        }
        self.print_esc(608i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 1539i32;
            self.help_line[(0i32) as usize] = 1538i32;
        }
        self.error();
    }

    /// An `align_group` code is supposed to remain on the `save_stack`
    /// during an entire alignment, until `fin_align` removes it.
    /// A devious user might force an `endv` command to occur just about anywhere;
    /// we must defeat such hacks.
    /// @<Declare act...
    // §1309
    pub fn do_endv(&mut self) {
        self.base_ptr = self.input_ptr;
        { let __ix1608 = self.base_ptr; let __v1609 = self.cur_input; self.input_stack[(__ix1608) as usize] = __v1609; }
        while (((self.input_stack[(self.base_ptr) as usize].index_field != 2i32) && (self.input_stack[(self.base_ptr) as usize].loc_field == 0i32)) && (self.input_stack[(self.base_ptr) as usize].state_field == 0i32)) {
            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
        }
        if (((self.input_stack[(self.base_ptr) as usize].index_field != 2i32) || (self.input_stack[(self.base_ptr) as usize].loc_field != 0i32)) || (self.input_stack[(self.base_ptr) as usize].state_field != 0i32)) {
            self.fatal_error(680i32);
        }
        if (self.cur_group == 6i32) {
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
    // §1313
    pub fn cs_error(&mut self) {
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
        self.print_esc(581i32);
        {
            self.help_ptr = 1i32;
            self.help_line[(0i32) as usize] = 1541i32;
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
    // §1314
    pub fn push_math(&mut self, mut c: group_code) {
        self.push_nest();
        self.cur_list.mode_field = (209i32).wrapping_neg();
        self.cur_list.aux_field.set_int(0i32);
        self.new_save_level(c);
    }

    /// When calculating the natural width, `w`, of the final line preceding
    /// the display, we may have to copy all or part of its hlist.  We copy,
    /// however, only those parts of the original list that are relevant for the
    /// computation of `pre_display_size`.
    /// @<Declare subprocedures for `init_math`
    // §1733
    pub fn just_copy(&mut self, mut p: halfword, mut h: halfword, mut t: halfword) {
        let mut r: halfword = 0; // §1733
        let mut words: i32 = 0; // §1733
        while (p != 0i32) {
            {
                'l_not_found_f: {
                    'l_found_f: {
                        words = 1i32;
                        if (p >= self.hi_mem_min) {
                            r = self.get_avail();
                        } else {
                            match self.mem[(p) as usize].hh().b0() {
                                0 | 1 => {
                                    {
                                        r = self.get_node(7i32);
                                        { let __v1610 = self.mem[((p).wrapping_add(6i32)) as usize]; self.mem[((r).wrapping_add(6i32)) as usize] = __v1610; }
                                        { let __v1611 = self.mem[((p).wrapping_add(5i32)) as usize]; self.mem[((r).wrapping_add(5i32)) as usize] = __v1611; }
                                        words = 5i32;
                                        self.mem[((r).wrapping_add(5i32)) as usize].set_hh_rh(0i32);
                                    }
                                }
                                2 => {
                                    {
                                        r = self.get_node(4i32);
                                        words = 4i32;
                                    }
                                }
                                6 => {
                                    {
                                        r = self.get_avail();
                                        { let __v1612 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[(r) as usize] = __v1612; }
                                        break 'l_found_f;
                                    }
                                }
                                11 | 9 => {
                                    {
                                        r = self.get_node(2i32);
                                        words = 2i32;
                                    }
                                }
                                10 => {
                                    {
                                        r = self.get_node(2i32);
                                        { let __ix1613 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); let __v1614 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix1613) as usize].set_hh_rh(__v1614); }
                                        { let __v1615 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); self.mem[((r).wrapping_add(1i32)) as usize].set_hh_lh(__v1615); }
                                        self.mem[((r).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
                                    }
                                }
                                8 => {
                                    // §1604
                                    match self.mem[(p) as usize].hh().b1() {
                                        0 => {
                                            {
                                                r = self.get_node(3i32);
                                                words = 3i32;
                                            }
                                        }
                                        1 | 3 | 4 => {
                                            {
                                                r = self.get_node(2i32);
                                                { let __ix1616 = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(); let __v1617 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1616) as usize].set_hh_lh(__v1617); }
                                                words = 2i32;
                                            }
                                        }
                                        2 | 5 => {
                                            {
                                                r = self.get_node(2i32);
                                                words = 2i32;
                                            }
                                        }
                                        7 | 8 => {
                                            {
                                                r = self.get_node(2i32);
                                                { let __ix1618 = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(); let __v1619 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1618) as usize].set_hh_lh(__v1619); }
                                                words = 2i32;
                                            }
                                        }
                                        40 => {
                                            {
                                                if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() <= 1i32) {
                                                    {
                                                        r = self.get_node(3i32);
                                                        { let __ix1620 = self.mem[((p).wrapping_add(2i32)) as usize].hh().rh(); let __v1621 = (self.mem[(self.mem[((p).wrapping_add(2i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1620) as usize].set_hh_lh(__v1621); }
                                                        words = 3i32;
                                                    }
                                                } else {
                                                    {
                                                        r = self.get_node(2i32);
                                                        words = 2i32;
                                                    }
                                                }
                                            }
                                        }
                                        41 => {
                                            {
                                                r = self.get_node(2i32);
                                                { let __ix1622 = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(); let __v1623 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1622) as usize].set_hh_lh(__v1623); }
                                                words = 2i32;
                                            }
                                        }
                                        42 => {
                                            {
                                                r = self.get_node(2i32);
                                                words = 2i32;
                                            }
                                        }
                                        43 => {
                                            {
                                                r = self.get_node(2i32);
                                                words = 2i32;
                                            }
                                        }
                                        10 => {
                                            {
                                                r = self.get_node(2i32);
                                                words = 2i32;
                                            }
                                        }
                                        12 => {
                                            {
                                                r = self.get_node(5i32);
                                                words = 5i32;
                                            }
                                        }
                                        14 => {
                                            {
                                                r = self.get_node(5i32);
                                                words = 5i32;
                                            }
                                        }
                                        15 => {
                                            {
                                                r = self.get_node(7i32);
                                                { let __ix1624 = self.mem[((p).wrapping_add(5i32)) as usize].hh().lh(); let __v1625 = (self.mem[(self.mem[((p).wrapping_add(5i32)) as usize].hh().lh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1624) as usize].set_hh_lh(__v1625); }
                                                words = 7i32;
                                            }
                                        }
                                        16 => {
                                            {
                                                r = self.get_node(7i32);
                                                { let __v1626 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v1626); }
                                                { let __v1627 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v1627); }
                                                { let __v1628 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v1628); }
                                                { let __v1629 = self.mem[((p).wrapping_add(5i32)) as usize].hh().lh(); self.mem[((r).wrapping_add(5i32)) as usize].set_hh_lh(__v1629); }
                                                if (self.mem[((r).wrapping_add(5i32)) as usize].hh().lh() != 0i32) {
                                                    { let __ix1630 = self.mem[((r).wrapping_add(5i32)) as usize].hh().lh(); let __v1631 = (self.mem[(self.mem[((r).wrapping_add(5i32)) as usize].hh().lh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1630) as usize].set_hh_lh(__v1631); }
                                                }
                                                { let __v1632 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[((r).wrapping_add(5i32)) as usize].set_hh_rh(__v1632); }
                                                { let __ix1633 = (self.mem[((r).wrapping_add(5i32)) as usize].hh().rh()).wrapping_add(2i32); let __v1634 = (self.mem[((self.mem[((r).wrapping_add(5i32)) as usize].hh().rh()).wrapping_add(2i32)) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix1633) as usize].set_hh_rh(__v1634); }
                                                { let __v1635 = self.mem[((p).wrapping_add(6i32)) as usize].int(); self.mem[((r).wrapping_add(6i32)) as usize].set_int(__v1635); }
                                            }
                                        }
                                        17 => {
                                            r = self.get_node(2i32);
                                        }
                                        19 => {
                                            {
                                                r = self.get_node(7i32);
                                                if (self.mem[((p).wrapping_add(5i32)) as usize].hh().b1() > 0i32) {
                                                    { let __ix1636 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); let __v1637 = (self.mem[(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1636) as usize].set_hh_lh(__v1637); }
                                                }
                                                words = 7i32;
                                            }
                                        }
                                        20 | 21 => {
                                            {
                                                r = self.get_node(7i32);
                                                if (self.mem[((p).wrapping_add(5i32)) as usize].hh().b1() > 0i32) {
                                                    { let __ix1638 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); let __v1639 = (self.mem[(self.mem[((p).wrapping_add(5i32)) as usize].hh().rh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1638) as usize].set_hh_lh(__v1639); }
                                                }
                                                if (self.mem[((p).wrapping_add(6i32)) as usize].hh().lh() != 0i32) {
                                                    { let __ix1640 = self.mem[((p).wrapping_add(6i32)) as usize].hh().lh(); let __v1641 = (self.mem[(self.mem[((p).wrapping_add(6i32)) as usize].hh().lh()) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1640) as usize].set_hh_lh(__v1641); }
                                                }
                                                words = 7i32;
                                            }
                                        }
                                        22 => {
                                            r = self.get_node(2i32);
                                        }
                                        23 => {
                                            r = self.get_node(2i32);
                                        }
                                        36 => {
                                            r = self.get_node(2i32);
                                        }
                                        37 => {
                                            {
                                                { let __ix1642 = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(); let __v1643 = (self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix1642) as usize].set_hh_rh(__v1643); }
                                                r = self.get_node(3i32);
                                                words = 3i32;
                                            }
                                        }
                                        38 => {
                                            r = self.get_node(2i32);
                                        }
                                        45 => {
                                            r = self.get_node(2i32);
                                        }
                                        46 => {
                                            r = self.get_node(2i32);
                                        }
                                        47 => {
                                            r = self.get_node(2i32);
                                        }
                                        48 => {
                                            r = self.get_node(2i32);
                                        }
                                        49 => {
                                            r = self.get_node(2i32);
                                        }
                                        _ => {
                                            self.confusion(1901i32);
                                        }
                                    }
                                }
                                _ => {
                                    // §1733
                                    break 'l_not_found_f;
                                }
                            }
                        }
                        while (words > 0i32) {
                            {
                                words = (words).wrapping_sub(1i32);
                                { let __v1644 = self.mem[((p).wrapping_add(words)) as usize]; self.mem[((r).wrapping_add(words)) as usize] = __v1644; }
                            }
                        }
                    }
                    self.mem[(h) as usize].set_hh_rh(r);
                    h = r;
                }
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        self.mem[(h) as usize].set_hh_rh(t);
    }

    /// @<Declare subprocedures for `init_math`
    // §1738
    pub fn just_reverse(&mut self, mut p: halfword) {
        let mut l: halfword = 0; // §1738
        let mut t: halfword = 0; // §1738
        let mut q: halfword = 0; // §1738
        let mut m: halfword = 0; // §1738
        let mut n: halfword = 0; // §1738
        'l_done_f: {
            'l_found_f: {
                m = 0i32;
                n = 0i32;
                if (self.mem[(4999996i32) as usize].hh().rh() == 0i32) {
                    {
                        self.just_copy(self.mem[(p) as usize].hh().rh(), 4999996i32, 0i32);
                        q = self.mem[(4999996i32) as usize].hh().rh();
                    }
                } else {
                    {
                        q = self.mem[(p) as usize].hh().rh();
                        self.mem[(p) as usize].set_hh_rh(0i32);
                        self.flush_node_list(self.mem[(4999996i32) as usize].hh().rh());
                    }
                }
                t = self.new_edge(self.cur_dir, 0i32);
                l = t;
                self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                while (q != 0i32) {
                    if (q >= self.hi_mem_min) {
                        loop {
                            p = q;
                            q = self.mem[(p) as usize].hh().rh();
                            self.mem[(p) as usize].set_hh_rh(l);
                            l = p;
                            if (!(q >= self.hi_mem_min)) { break; }
                        }
                    } else {
                        {
                            p = q;
                            q = self.mem[(p) as usize].hh().rh();
                            if (self.mem[(p) as usize].hh().b0() == 9i32) {
                                // §1739
                                if (((self.mem[(p) as usize].hh().b1()) % 2) != 0) {
                                    if (self.mem[(self.LR_ptr) as usize].hh().lh() != ((4i32).wrapping_mul((self.mem[(p) as usize].hh().b1() / 4i32))).wrapping_add(3i32)) {
                                        {
                                            self.mem[(p) as usize].set_hh_b0(11i32);
                                            self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            {
                                                self.temp_ptr = self.LR_ptr;
                                                self.LR_ptr = self.mem[(self.temp_ptr) as usize].hh().rh();
                                                {
                                                    { let __ix1645 = self.temp_ptr; let __v1646 = self.avail; self.mem[(__ix1645) as usize].set_hh_rh(__v1646); }
                                                    self.avail = self.temp_ptr;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                            if (n > 0i32) {
                                                {
                                                    n = (n).wrapping_sub(1i32);
                                                    { let __v1647 = (self.mem[(p) as usize].hh().b1()).wrapping_sub(1i32); self.mem[(p) as usize].set_hh_b1(__v1647); }
                                                }
                                            } else {
                                                {
                                                    if (m > 0i32) {
                                                        m = (m).wrapping_sub(1i32);
                                                    } else {
                                                        break 'l_found_f;
                                                    }
                                                    self.mem[(p) as usize].set_hh_b0(11i32);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        {
                                            self.temp_ptr = self.get_avail();
                                            { let __ix1648 = self.temp_ptr; let __v1649 = ((4i32).wrapping_mul((self.mem[(p) as usize].hh().b1() / 4i32))).wrapping_add(3i32); self.mem[(__ix1648) as usize].set_hh_lh(__v1649); }
                                            { let __ix1650 = self.temp_ptr; let __v1651 = self.LR_ptr; self.mem[(__ix1650) as usize].set_hh_rh(__v1651); }
                                            self.LR_ptr = self.temp_ptr;
                                        }
                                        if ((n > 0i32) || ((self.mem[(p) as usize].hh().b1() / 8i32) != self.cur_dir)) {
                                            {
                                                n = (n).wrapping_add(1i32);
                                                { let __v1652 = (self.mem[(p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(p) as usize].set_hh_b1(__v1652); }
                                            }
                                        } else {
                                            {
                                                self.mem[(p) as usize].set_hh_b0(11i32);
                                                m = (m).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                            }
                            // §1738
                            self.mem[(p) as usize].set_hh_rh(l);
                            l = p;
                        }
                    }
                }
                break 'l_done_f;
            }
            { let __v1653 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.mem[((t).wrapping_add(1i32)) as usize].set_int(__v1653); }
            self.mem[(t) as usize].set_hh_rh(q);
            self.free_node(p, 2i32);
        }
        self.mem[(4999996i32) as usize].set_hh_rh(l);
    }

    /// @<Declare act...
    // §1316
    pub fn init_math(&mut self) {
        let mut w: scaled = 0; // §1316
        let mut j: halfword = 0; // §1316
        let mut x: i32 = 0; // §1316
        let mut l: scaled = 0; // §1316
        let mut s: scaled = 0; // §1316
        let mut p: halfword = 0; // §1316
        let mut q: halfword = 0; // §1316
        let mut f: internal_font_number = 0; // §1316
        let mut n: i32 = 0; // §1316
        let mut v: scaled = 0; // §1316
        let mut d: scaled = 0; // §1316
        self.get_token();
        if ((self.cur_cmd == 3i32) && (self.cur_list.mode_field > 0i32)) {
            // §1323
            {
                j = 0i32;
                w = (1073741823i32).wrapping_neg();
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    // §1732
                    {
                        self.pop_nest();
                        // §1731
                        if (self.cur_list.eTeX_aux_field == 0i32) {
                            x = 0i32;
                        } else {
                            if (self.mem[(self.cur_list.eTeX_aux_field) as usize].hh().lh() >= 8i32) {
                                x = (1i32).wrapping_neg();
                            } else {
                                x = 1i32;
                            }
                        }
                    }
                } else {
                    // §1323
                    {
                        'l_done_f: {
                            self.line_break(true);
                            // §1734
                            if (self.eTeX_mode == 1i32) {
                                // §1740
                                {
                                    if (self.eqtb[((626636i32) - 1) as usize].hh().rh() == 0i32) {
                                        j = self.new_kern(0i32);
                                    } else {
                                        j = self.new_param_glue(8i32);
                                    }
                                    if (self.eqtb[((626635i32) - 1) as usize].hh().rh() == 0i32) {
                                        p = self.new_kern(0i32);
                                    } else {
                                        p = self.new_param_glue(7i32);
                                    }
                                    self.mem[(p) as usize].set_hh_rh(j);
                                    j = self.new_null_box();
                                    { let __v1654 = self.mem[((self.just_box).wrapping_add(1i32)) as usize].int(); self.mem[((j).wrapping_add(1i32)) as usize].set_int(__v1654); }
                                    { let __v1655 = self.mem[((self.just_box).wrapping_add(4i32)) as usize].int(); self.mem[((j).wrapping_add(4i32)) as usize].set_int(__v1655); }
                                    self.mem[((j).wrapping_add(5i32)) as usize].set_hh_rh(p);
                                    { let __v1656 = self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b1(); self.mem[((j).wrapping_add(5i32)) as usize].set_hh_b1(__v1656); }
                                    { let __v1657 = self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b0(); self.mem[((j).wrapping_add(5i32)) as usize].set_hh_b0(__v1657); }
                                    { let __v1658 = self.mem[((self.just_box).wrapping_add(6i32)) as usize].gr(); self.mem[((j).wrapping_add(6i32)) as usize].set_gr(__v1658); }
                                }
                            }
                            // §1734
                            v = self.mem[((self.just_box).wrapping_add(4i32)) as usize].int();
                            // §1731
                            if (self.cur_list.eTeX_aux_field == 0i32) {
                                x = 0i32;
                            } else {
                                if (self.mem[(self.cur_list.eTeX_aux_field) as usize].hh().lh() >= 8i32) {
                                    x = (1i32).wrapping_neg();
                                } else {
                                    x = 1i32;
                                }
                            }
                            // §1734
                            if (x >= 0i32) {
                                {
                                    p = self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().rh();
                                    self.mem[(4999996i32) as usize].set_hh_rh(0i32);
                                }
                            } else {
                                {
                                    v = ((v).wrapping_neg()).wrapping_sub(self.mem[((self.just_box).wrapping_add(1i32)) as usize].int());
                                    p = self.new_math(0i32, 6i32);
                                    self.mem[(4999996i32) as usize].set_hh_rh(p);
                                    { let __a1659_0 = self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().rh(); let __a1659_1 = p; let __a1659_2 = self.new_math(0i32, 7i32); self.just_copy(__a1659_0, __a1659_1, __a1659_2) };
                                    self.cur_dir = 1i32;
                                }
                            }
                            v = (v).wrapping_add((2i32).wrapping_mul(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[((627689i32) - 1) as usize].hh().rh()) as usize])) as usize].int()));
                            if (self.eqtb[((629126i32) - 1) as usize].int() > 0i32) {
                                // §1710
                                {
                                    self.temp_ptr = self.get_avail();
                                    { let __ix1660 = self.temp_ptr; self.mem[(__ix1660) as usize].set_hh_lh(0i32); }
                                    { let __ix1661 = self.temp_ptr; let __v1662 = self.LR_ptr; self.mem[(__ix1661) as usize].set_hh_rh(__v1662); }
                                    self.LR_ptr = self.temp_ptr;
                                }
                            }
                            // §1324
                            while (p != 0i32) {
                                {
                                    // goto labels: reswitch, found, not_found
                                    let mut __goto_1: i32 = 0;
                                    'l_dispatch_1: loop {
                                        if __goto_1 <= 0 {
                                            // §1325
                                            if (p >= self.hi_mem_min) {
                                                {
                                                    f = self.mem[(p) as usize].hh().b0();
                                                    d = self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.mem[(p) as usize].hh().b1())) as usize].qqqq().b0())) as usize].int();
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                            match self.mem[(p) as usize].hh().b0() {
                                                0 | 1 | 2 => {
                                                    {
                                                        d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                6 => {
                                                    // §826
                                                    {
                                                        { let __v1663 = self.mem[((p).wrapping_add(1i32)) as usize]; self.mem[(4999987i32) as usize] = __v1663; }
                                                        { let __v1664 = self.mem[(p) as usize].hh().rh(); self.mem[(4999987i32) as usize].set_hh_rh(__v1664); }
                                                        p = 4999987i32;
                                                        { __goto_1 = 0; continue 'l_dispatch_1; }
                                                    }
                                                }
                                                40 => {
                                                    // §1325
                                                    d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                }
                                                11 => {
                                                    d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                }
                                                9 => {
                                                    // §1736
                                                    {
                                                        d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                        if (self.eqtb[((629126i32) - 1) as usize].int() > 0i32) {
                                                            // §1737
                                                            if (((self.mem[(p) as usize].hh().b1()) % 2) != 0) {
                                                                {
                                                                    if (self.mem[(self.LR_ptr) as usize].hh().lh() == ((4i32).wrapping_mul((self.mem[(p) as usize].hh().b1() / 4i32))).wrapping_add(3i32)) {
                                                                        {
                                                                            self.temp_ptr = self.LR_ptr;
                                                                            self.LR_ptr = self.mem[(self.temp_ptr) as usize].hh().rh();
                                                                            {
                                                                                { let __ix1665 = self.temp_ptr; let __v1666 = self.avail; self.mem[(__ix1665) as usize].set_hh_rh(__v1666); }
                                                                                self.avail = self.temp_ptr;
                                                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                            }
                                                                        }
                                                                    } else {
                                                                        if (self.mem[(p) as usize].hh().b1() > 4i32) {
                                                                            {
                                                                                w = 1073741823i32;
                                                                                break 'l_done_f;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    {
                                                                        self.temp_ptr = self.get_avail();
                                                                        { let __ix1667 = self.temp_ptr; let __v1668 = ((4i32).wrapping_mul((self.mem[(p) as usize].hh().b1() / 4i32))).wrapping_add(3i32); self.mem[(__ix1667) as usize].set_hh_lh(__v1668); }
                                                                        { let __ix1669 = self.temp_ptr; let __v1670 = self.LR_ptr; self.mem[(__ix1669) as usize].set_hh_rh(__v1670); }
                                                                        self.LR_ptr = self.temp_ptr;
                                                                    }
                                                                    if ((self.mem[(p) as usize].hh().b1() / 8i32) != self.cur_dir) {
                                                                        {
                                                                            self.just_reverse(p);
                                                                            p = 4999996i32;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            // §1736
                                                            if (self.mem[(p) as usize].hh().b1() >= 4i32) {
                                                                {
                                                                    w = 1073741823i32;
                                                                    break 'l_done_f;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                14 => {
                                                    {
                                                        d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                        self.cur_dir = self.mem[(p) as usize].hh().b1();
                                                    }
                                                }
                                                10 => {
                                                    // §1326
                                                    {
                                                        q = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                                        d = self.mem[((q).wrapping_add(1i32)) as usize].int();
                                                        if (self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b0() == 1i32) {
                                                            {
                                                                if ((self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b1() == self.mem[(q) as usize].hh().b0()) && (self.mem[((q).wrapping_add(2i32)) as usize].int() != 0i32)) {
                                                                    v = 1073741823i32;
                                                                }
                                                            }
                                                        } else {
                                                            if (self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b0() == 2i32) {
                                                                {
                                                                    if ((self.mem[((self.just_box).wrapping_add(5i32)) as usize].hh().b1() == self.mem[(q) as usize].hh().b1()) && (self.mem[((q).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                                                        v = 1073741823i32;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if (self.mem[(p) as usize].hh().b1() >= 100i32) {
                                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                                        }
                                                    }
                                                }
                                                8 => {
                                                    // §1608
                                                    if ((self.mem[(p) as usize].hh().b1() == 12i32) || (self.mem[(p) as usize].hh().b1() == 14i32)) {
                                                        d = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                    } else {
                                                        d = 0i32;
                                                    }
                                                }
                                                _ => {
                                                    // §1325
                                                    d = 0i32;
                                                }
                                            }
                                            // §1324
                                            if (v < 1073741823i32) {
                                                v = (v).wrapping_add(d);
                                            }
                                            { __goto_1 = 2; continue 'l_dispatch_1; }
                                        }
                                        if __goto_1 <= 1 { // found
                                            if (v < 1073741823i32) {
                                                {
                                                    v = (v).wrapping_add(d);
                                                    w = v;
                                                }
                                            } else {
                                                {
                                                    w = 1073741823i32;
                                                    break 'l_done_f;
                                                }
                                            }
                                        }
                                        if __goto_1 <= 2 { // not_found
                                            p = self.mem[(p) as usize].hh().rh();
                                        }
                                        break 'l_dispatch_1;
                                    }
                                }
                            }
                        }
                        if (self.eqtb[((629126i32) - 1) as usize].int() > 0i32) {
                            // §1735
                            {
                                while (self.LR_ptr != 0i32) {
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = self.mem[(self.temp_ptr) as usize].hh().rh();
                                        {
                                            { let __ix1671 = self.temp_ptr; let __v1672 = self.avail; self.mem[(__ix1671) as usize].set_hh_rh(__v1672); }
                                            self.avail = self.temp_ptr;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                }
                                if (self.LR_problems != 0i32) {
                                    {
                                        w = 1073741823i32;
                                        self.LR_problems = 0i32;
                                    }
                                }
                            }
                        }
                        self.cur_dir = 0i32;
                        self.flush_node_list(self.mem[(4999996i32) as usize].hh().rh());
                    }
                }
                // §1327
                if (self.eqtb[((627158i32) - 1) as usize].hh().rh() == 0i32) {
                    if ((self.eqtb[((629657i32) - 1) as usize].int() != 0i32) && (((self.eqtb[((629059i32) - 1) as usize].int() >= 0i32) && ((self.cur_list.pg_field).wrapping_add(2i32) > self.eqtb[((629059i32) - 1) as usize].int())) || ((self.cur_list.pg_field).wrapping_add(1i32) < (self.eqtb[((629059i32) - 1) as usize].int()).wrapping_neg()))) {
                        {
                            l = (self.eqtb[((629643i32) - 1) as usize].int()).wrapping_sub((self.eqtb[((629657i32) - 1) as usize].int()).wrapping_abs());
                            if (self.eqtb[((629657i32) - 1) as usize].int() > 0i32) {
                                s = self.eqtb[((629657i32) - 1) as usize].int();
                            } else {
                                s = 0i32;
                            }
                        }
                    } else {
                        {
                            l = self.eqtb[((629643i32) - 1) as usize].int();
                            s = 0i32;
                        }
                    }
                } else {
                    {
                        n = self.mem[(self.eqtb[((627158i32) - 1) as usize].hh().rh()) as usize].hh().lh();
                        if ((self.cur_list.pg_field).wrapping_add(2i32) >= n) {
                            p = (self.eqtb[((627158i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul(n));
                        } else {
                            p = (self.eqtb[((627158i32) - 1) as usize].hh().rh()).wrapping_add((2i32).wrapping_mul((self.cur_list.pg_field).wrapping_add(2i32)));
                        }
                        s = self.mem[((p).wrapping_sub(1i32)) as usize].int();
                        l = self.mem[(p) as usize].int();
                    }
                }
                // §1323
                self.push_math(15i32);
                self.cur_list.mode_field = 209i32;
                self.eq_word_define(629062i32, (1i32).wrapping_neg());
                self.eq_word_define(629653i32, w);
                self.cur_list.eTeX_aux_field = j;
                if (self.eTeX_mode == 1i32) {
                    self.eq_word_define(629121i32, x);
                }
                self.eq_word_define(629654i32, l);
                self.eq_word_define(629655i32, s);
                if (self.eqtb[((627162i32) - 1) as usize].hh().rh() != 0i32) {
                    self.begin_token_list(self.eqtb[((627162i32) - 1) as usize].hh().rh(), 9i32);
                }
                if (self.nest_ptr == 1i32) {
                    self.build_page();
                }
            }
        } else {
            // §1316
            {
                self.back_input();
                // §1317
                {
                    self.push_math(15i32);
                    self.eq_word_define(629062i32, (1i32).wrapping_neg());
                    if (self.eqtb[((627161i32) - 1) as usize].hh().rh() != 0i32) {
                        self.begin_token_list(self.eqtb[((627161i32) - 1) as usize].hh().rh(), 8i32);
                    }
                }
            }
        }
    }

    /// When \TeX\ is in display math mode, `cur_group=math_shift_group`,
    /// so it is not necessary for the `start_eq_no` procedure to test for
    /// this condition.
    /// @<Declare act...
    // §1320
    pub fn start_eq_no(&mut self) {
        { let __ix1673 = (self.save_ptr).wrapping_add(0i32); let __v1674 = self.cur_chr; self.save_stack[(__ix1673) as usize].set_int(__v1674); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        // §1317
        {
            self.push_math(15i32);
            self.eq_word_define(629062i32, (1i32).wrapping_neg());
            if (self.eqtb[((627161i32) - 1) as usize].hh().rh() != 0i32) {
                self.begin_token_list(self.eqtb[((627161i32) - 1) as usize].hh().rh(), 8i32);
            }
        }
    }

    /// Recall that the `nucleus`, `subscr`, and `supscr` fields in a noad are
    /// broken down into subfields called `math_type` and either `info` or
    /// `(fam,character)`. The job of `scan_math` is to figure out what to place
    /// in one of these principal fields; it looks at the subformula that
    /// comes next in the input, and places an encoding of that subformula
    /// into a given word of `mem`.
    // §1329
    pub fn scan_math(&mut self, mut p: halfword) {
        let mut c: i32 = 0; // §1329
        // goto labels: restart, reswitch, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                loop {
                    // §430
                    self.get_x_token();
                    if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                }
            }
            if __goto_1 <= 1 { // reswitch
                // §1329
                match self.cur_cmd {
                    11 | 12 | 68 => {
                        {
                            c = (self.eqtb[(((628762i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32);
                            if (c == 32768i32) {
                                {
                                    // §1330
                                    {
                                        self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                        self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                        self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                        self.x_token();
                                        self.back_input();
                                    }
                                    // §1329
                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                }
                            }
                        }
                    }
                    16 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.cur_cmd = 68i32;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    17 => {
                        {
                            self.scan_fifteen_bit_int();
                            c = self.cur_val;
                        }
                    }
                    69 => {
                        c = self.cur_chr;
                    }
                    15 => {
                        {
                            self.scan_twenty_seven_bit_int();
                            c = (self.cur_val / 4096i32);
                        }
                    }
                    _ => {
                        // §1331
                        {
                            self.back_input();
                            self.scan_left_brace();
                            { let __ix1675 = (self.save_ptr).wrapping_add(0i32); self.save_stack[(__ix1675) as usize].set_int(p); }
                            self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                            self.push_math(9i32);
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §1329
                self.mem[(p) as usize].set_hh_rh(1i32);
                self.mem[(p) as usize].set_hh_b1(((c % 256i32)).wrapping_add(0i32));
                if ((c >= 28672i32) && ((self.eqtb[((629062i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629062i32) - 1) as usize].int() < 16i32))) {
                    { let __v1676 = self.eqtb[((629062i32) - 1) as usize].int(); self.mem[(p) as usize].set_hh_b0(__v1676); }
                } else {
                    self.mem[(p) as usize].set_hh_b0(((c / 256i32) % 16i32));
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
    // §1333
    pub fn set_math_char(&mut self, mut c: i32) {
        let mut p: halfword = 0; // §1333
        if (c >= 32768i32) {
            // §1330
            {
                self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                self.x_token();
                self.back_input();
            }
        } else {
            // §1333
            {
                p = self.new_noad();
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(1i32);
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b1(((c % 256i32)).wrapping_add(0i32));
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b0(((c / 256i32) % 16i32));
                if (c >= 28672i32) {
                    {
                        if ((self.eqtb[((629062i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629062i32) - 1) as usize].int() < 16i32)) {
                            { let __v1677 = self.eqtb[((629062i32) - 1) as usize].int(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b0(__v1677); }
                        }
                        self.mem[(p) as usize].set_hh_b0(16i32);
                    }
                } else {
                    self.mem[(p) as usize].set_hh_b0((16i32).wrapping_add((c / 4096i32)));
                }
                { let __ix1678 = self.cur_list.tail_field; self.mem[(__ix1678) as usize].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
    }

    /// @<Declare act...
    // §1337
    pub fn math_limit_switch(&mut self) {
        'l_exit_f: {
            if (self.cur_list.head_field != self.cur_list.tail_field) {
                if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 17i32) {
                    {
                        { let __ix1679 = self.cur_list.tail_field; let __v1680 = self.cur_chr; self.mem[(__ix1679) as usize].set_hh_b1(__v1680); }
                        break 'l_exit_f;
                    }
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
                self.print(1545i32);
            }
            {
                self.help_ptr = 1i32;
                self.help_line[(0i32) as usize] = 1546i32;
            }
            self.error();
        }
    }

    /// Delimiter fields of noads are filled in by the `scan_delimiter` routine.
    /// The first parameter of this procedure is the `mem` address where the
    /// delimiter is to be placed; the second tells if this delimiter follows
    /// \.{\\radical} or not.
    /// @<Declare act...
    // §1338
    pub fn scan_delimiter(&mut self, mut p: halfword, mut r: bool) {
        if r {
            self.scan_twenty_seven_bit_int();
        } else {
            {
                // §430
                loop {
                    self.get_x_token();
                    if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                }
                // §1338
                match self.cur_cmd {
                    11 | 12 => {
                        self.cur_val = self.eqtb[(((629384i32).wrapping_add(self.cur_chr)) - 1) as usize].int();
                    }
                    15 => {
                        self.scan_twenty_seven_bit_int();
                    }
                    _ => {
                        self.cur_val = (1i32).wrapping_neg();
                    }
                }
            }
        }
        if (self.cur_val < 0i32) {
            // §1339
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1547i32);
                }
                {
                    self.help_ptr = 6i32;
                    self.help_line[(5i32) as usize] = 1548i32;
                    self.help_line[(4i32) as usize] = 1549i32;
                    self.help_line[(3i32) as usize] = 1550i32;
                    self.help_line[(2i32) as usize] = 1551i32;
                    self.help_line[(1i32) as usize] = 1552i32;
                    self.help_line[(0i32) as usize] = 1553i32;
                }
                self.back_error();
                self.cur_val = 0i32;
            }
        }
        // §1338
        { let __v1681 = ((self.cur_val / 1048576i32) % 16i32); self.mem[(p) as usize].set_qqqq_b0(__v1681); }
        { let __v1682 = (((self.cur_val / 4096i32) % 256i32)).wrapping_add(0i32); self.mem[(p) as usize].set_qqqq_b1(__v1682); }
        { let __v1683 = ((self.cur_val / 256i32) % 16i32); self.mem[(p) as usize].set_qqqq_b2(__v1683); }
        { let __v1684 = ((self.cur_val % 256i32)).wrapping_add(0i32); self.mem[(p) as usize].set_qqqq_b3(__v1684); }
    }

    /// @<Declare act...
    // §1341
    pub fn math_radical(&mut self) {
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1685 = self.cur_list.tail_field; let __v1686 = self.get_node(5i32); self.mem[(__ix1685) as usize].set_hh_rh(__v1686); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix1687 = self.cur_list.tail_field; self.mem[(__ix1687) as usize].set_hh_b0(24i32); }
        { let __ix1688 = self.cur_list.tail_field; self.mem[(__ix1688) as usize].set_hh_b1(0i32); }
        { let __ix1689 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1690 = self.empty_field; self.mem[(__ix1689) as usize].set_hh(__v1690); }
        { let __ix1691 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1692 = self.empty_field; self.mem[(__ix1691) as usize].set_hh(__v1692); }
        { let __ix1693 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1694 = self.empty_field; self.mem[(__ix1693) as usize].set_hh(__v1694); }
        self.scan_delimiter((self.cur_list.tail_field).wrapping_add(4i32), true);
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// @<Declare act...
    // §1343
    pub fn math_ac(&mut self) {
        if (self.cur_cmd == 45i32) {
            // §1344
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1554i32);
                }
                self.print_esc(601i32);
                self.print(1555i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 1556i32;
                    self.help_line[(0i32) as usize] = 1557i32;
                }
                self.error();
            }
        }
        // §1343
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1695 = self.cur_list.tail_field; let __v1696 = self.get_node(5i32); self.mem[(__ix1695) as usize].set_hh_rh(__v1696); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        { let __ix1697 = self.cur_list.tail_field; self.mem[(__ix1697) as usize].set_hh_b0(28i32); }
        { let __ix1698 = self.cur_list.tail_field; self.mem[(__ix1698) as usize].set_hh_b1(0i32); }
        { let __ix1699 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1700 = self.empty_field; self.mem[(__ix1699) as usize].set_hh(__v1700); }
        { let __ix1701 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1702 = self.empty_field; self.mem[(__ix1701) as usize].set_hh(__v1702); }
        { let __ix1703 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1704 = self.empty_field; self.mem[(__ix1703) as usize].set_hh(__v1704); }
        { let __ix1705 = (self.cur_list.tail_field).wrapping_add(4i32); self.mem[(__ix1705) as usize].set_hh_rh(1i32); }
        self.scan_fifteen_bit_int();
        { let __ix1706 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1707 = ((self.cur_val % 256i32)).wrapping_add(0i32); self.mem[(__ix1706) as usize].set_hh_b1(__v1707); }
        if ((self.cur_val >= 28672i32) && ((self.eqtb[((629062i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629062i32) - 1) as usize].int() < 16i32))) {
            { let __ix1708 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1709 = self.eqtb[((629062i32) - 1) as usize].int(); self.mem[(__ix1708) as usize].set_hh_b0(__v1709); }
        } else {
            { let __ix1710 = (self.cur_list.tail_field).wrapping_add(4i32); let __v1711 = ((self.cur_val / 256i32) % 16i32); self.mem[(__ix1710) as usize].set_hh_b0(__v1711); }
        }
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// The routine that scans the four mlists of a \.{\\mathchoice} is very
    /// much like the routine that builds discretionary nodes.
    /// @<Declare act...
    // §1350
    pub fn append_choices(&mut self) {
        {
            self.prev_tail = self.cur_list.tail_field;
            { let __ix1712 = self.cur_list.tail_field; let __v1713 = self.new_choice(); self.mem[(__ix1712) as usize].set_hh_rh(__v1713); }
            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        { let __ix1714 = (self.save_ptr).wrapping_sub(1i32); self.save_stack[(__ix1714) as usize].set_int(0i32); }
        self.push_math(13i32);
        self.scan_left_brace();
    }

    /// At the end of a math formula or subformula, the `fin_mlist` routine is
    /// called upon to return a pointer to the newly completed mlist, and to
    /// pop the nest back to the enclosing semantic level. The parameter to
    /// `fin_mlist`, if not null, points to a `right_noad` that ends the
    /// current mlist; this `right_noad` has not yet been appended.
    /// @<Declare the function called `fin_mlist`
    // §1362
    pub fn fin_mlist(&mut self, mut p: halfword) -> halfword {
        let mut fin_mlist: halfword = 0;
        let mut q: halfword = 0; // §1362
        if (self.cur_list.aux_field.int() != 0i32) {
            // §1363
            {
                { let __ix1715 = (self.cur_list.aux_field.int()).wrapping_add(3i32); self.mem[(__ix1715) as usize].set_hh_rh(3i32); }
                { let __ix1716 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v1717 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix1716) as usize].set_hh_lh(__v1717); }
                if (p == 0i32) {
                    q = self.cur_list.aux_field.int();
                } else {
                    {
                        q = self.mem[((self.cur_list.aux_field.int()).wrapping_add(2i32)) as usize].hh().lh();
                        if ((self.mem[(q) as usize].hh().b0() != 30i32) || (self.cur_list.eTeX_aux_field == 0i32)) {
                            self.confusion(1285i32);
                        }
                        { let __ix1718 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v1719 = self.mem[(self.cur_list.eTeX_aux_field) as usize].hh().rh(); self.mem[(__ix1718) as usize].set_hh_lh(__v1719); }
                        { let __ix1720 = self.cur_list.eTeX_aux_field; let __v1721 = self.cur_list.aux_field.int(); self.mem[(__ix1720) as usize].set_hh_rh(__v1721); }
                        { let __ix1722 = self.cur_list.aux_field.int(); self.mem[(__ix1722) as usize].set_hh_rh(p); }
                    }
                }
            }
        } else {
            // §1362
            {
                { let __ix1723 = self.cur_list.tail_field; self.mem[(__ix1723) as usize].set_hh_rh(p); }
                q = self.mem[(self.cur_list.head_field) as usize].hh().rh();
            }
        }
        self.pop_nest();
        fin_mlist = q;
        fin_mlist
    }

    /// @<Declare act...
    // §1352
    pub fn build_choices(&mut self) {
        let mut p: halfword = 0; // §1352
        'l_exit_f: {
            self.unsave();
            p = self.fin_mlist(0i32);
            match self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int() {
                0 => {
                    { let __ix1724 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1724) as usize].set_hh_lh(p); }
                }
                1 => {
                    { let __ix1725 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1725) as usize].set_hh_rh(p); }
                }
                2 => {
                    { let __ix1726 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix1726) as usize].set_hh_lh(p); }
                }
                3 => {
                    {
                        { let __ix1727 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix1727) as usize].set_hh_rh(p); }
                        self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                        break 'l_exit_f;
                    }
                }
                _ => {}
            }
            { let __ix1728 = (self.save_ptr).wrapping_sub(1i32); let __v1729 = (self.save_stack[((self.save_ptr).wrapping_sub(1i32)) as usize].int()).wrapping_add(1i32); self.save_stack[(__ix1728) as usize].set_int(__v1729); }
            self.push_math(13i32);
            self.scan_left_brace();
        }
    }

    /// @<Declare act...
    // §1354
    pub fn sub_sup(&mut self) {
        let mut t: small_number = 0; // §1354
        let mut p: halfword = 0; // §1354
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
            // §1355
            {
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1730 = self.cur_list.tail_field; let __v1731 = self.new_noad(); self.mem[(__ix1730) as usize].set_hh_rh(__v1731); }
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
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1558i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1559i32;
                                }
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
                                    self.print(1560i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1561i32;
                                }
                            }
                        }
                        self.error();
                    }
                }
            }
        }
        // §1354
        self.scan_math(p);
    }

    /// @<Declare act...
    // §1359
    pub fn math_fraction(&mut self) {
        let mut c: small_number = 0; // §1359
        c = self.cur_chr;
        if (self.cur_list.aux_field.int() != 0i32) {
            // §1361
            {
                if (c >= 3i32) {
                    {
                        self.scan_delimiter(4999987i32, false);
                        self.scan_delimiter(4999987i32, false);
                    }
                }
                if ((c % 3i32) == 0i32) {
                    self.scan_dimen(false, false, false);
                }
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1568i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1569i32;
                    self.help_line[(1i32) as usize] = 1570i32;
                    self.help_line[(0i32) as usize] = 1571i32;
                }
                self.error();
            }
        } else {
            // §1359
            {
                { let __v1732 = self.get_node(6i32); self.cur_list.aux_field.set_int(__v1732); }
                { let __ix1733 = self.cur_list.aux_field.int(); self.mem[(__ix1733) as usize].set_hh_b0(25i32); }
                { let __ix1734 = self.cur_list.aux_field.int(); self.mem[(__ix1734) as usize].set_hh_b1(0i32); }
                { let __ix1735 = (self.cur_list.aux_field.int()).wrapping_add(2i32); self.mem[(__ix1735) as usize].set_hh_rh(3i32); }
                { let __ix1736 = (self.cur_list.aux_field.int()).wrapping_add(2i32); let __v1737 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix1736) as usize].set_hh_lh(__v1737); }
                { let __ix1738 = (self.cur_list.aux_field.int()).wrapping_add(3i32); let __v1739 = self.empty_field; self.mem[(__ix1738) as usize].set_hh(__v1739); }
                { let __ix1740 = (self.cur_list.aux_field.int()).wrapping_add(4i32); let __v1741 = self.null_delimiter; self.mem[(__ix1740) as usize].set_qqqq(__v1741); }
                { let __ix1742 = (self.cur_list.aux_field.int()).wrapping_add(5i32); let __v1743 = self.null_delimiter; self.mem[(__ix1742) as usize].set_qqqq(__v1743); }
                { let __ix1744 = self.cur_list.head_field; self.mem[(__ix1744) as usize].set_hh_rh(0i32); }
                self.cur_list.tail_field = self.cur_list.head_field;
                // §1360
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
                            { let __ix1745 = (self.cur_list.aux_field.int()).wrapping_add(1i32); let __v1746 = self.cur_val; self.mem[(__ix1745) as usize].set_int(__v1746); }
                        }
                    }
                    1 => {
                        { let __ix1747 = (self.cur_list.aux_field.int()).wrapping_add(1i32); self.mem[(__ix1747) as usize].set_int(1073741824i32); }
                    }
                    2 => {
                        { let __ix1748 = (self.cur_list.aux_field.int()).wrapping_add(1i32); self.mem[(__ix1748) as usize].set_int(0i32); }
                    }
                    _ => {}
                }
            }
        }
    }

    /// @<Declare act...
    // §1369
    pub fn math_left_right(&mut self) {
        let mut t: small_number = 0; // §1369
        let mut p: halfword = 0; // §1369
        let mut q: halfword = 0; // §1369
        t = self.cur_chr;
        if ((t != 30i32) && (self.cur_group != 16i32)) {
            // §1370
            {
                if (self.cur_group == 15i32) {
                    {
                        self.scan_delimiter(4999987i32, false);
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
                        if (t == 1i32) {
                            {
                                self.print_esc(1286i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1572i32;
                                }
                            }
                        } else {
                            {
                                self.print_esc(1285i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1573i32;
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
            // §1369
            {
                p = self.new_noad();
                self.mem[(p) as usize].set_hh_b0(t);
                self.scan_delimiter((p).wrapping_add(1i32), false);
                if (t == 1i32) {
                    {
                        self.mem[(p) as usize].set_hh_b0(31i32);
                        self.mem[(p) as usize].set_hh_b1(1i32);
                    }
                }
                if (t == 30i32) {
                    q = p;
                } else {
                    {
                        q = self.fin_mlist(p);
                        self.unsave();
                    }
                }
                if (t != 31i32) {
                    {
                        self.push_math(16i32);
                        { let __ix1749 = self.cur_list.head_field; self.mem[(__ix1749) as usize].set_hh_rh(q); }
                        self.cur_list.tail_field = p;
                        self.cur_list.eTeX_aux_field = p;
                    }
                } else {
                    {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1750 = self.cur_list.tail_field; let __v1751 = self.new_noad(); self.mem[(__ix1750) as usize].set_hh_rh(__v1751); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                        { let __ix1752 = self.cur_list.tail_field; self.mem[(__ix1752) as usize].set_hh_b0(23i32); }
                        { let __ix1753 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1753) as usize].set_hh_rh(3i32); }
                        { let __ix1754 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix1754) as usize].set_hh_lh(q); }
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
    // §1744
    pub fn app_display(&mut self, mut j: halfword, mut b: halfword, mut d: scaled) {
        let mut z: scaled = 0; // §1744
        let mut s: scaled = 0; // §1744
        let mut e: scaled = 0; // §1744
        let mut x: i32 = 0; // §1744
        let mut p: halfword = 0; // §1744
        let mut q: halfword = 0; // §1744
        let mut r: halfword = 0; // §1744
        let mut t: halfword = 0; // §1744
        let mut u: halfword = 0; // §1744
        s = self.eqtb[((629655i32) - 1) as usize].int();
        x = self.eqtb[((629121i32) - 1) as usize].int();
        if (x == 0i32) {
            self.mem[((b).wrapping_add(4i32)) as usize].set_int((s).wrapping_add(d));
        } else {
            {
                z = self.eqtb[((629654i32) - 1) as usize].int();
                p = b;
                // §1745
                if (x > 0i32) {
                    e = ((z).wrapping_sub(d)).wrapping_sub(self.mem[((p).wrapping_add(1i32)) as usize].int());
                } else {
                    {
                        e = d;
                        d = ((z).wrapping_sub(e)).wrapping_sub(self.mem[((p).wrapping_add(1i32)) as usize].int());
                    }
                }
                if (j != 0i32) {
                    {
                        b = self.copy_node_list(j);
                        { let __v1755 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.mem[((b).wrapping_add(3i32)) as usize].set_int(__v1755); }
                        { let __v1756 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.mem[((b).wrapping_add(2i32)) as usize].set_int(__v1756); }
                        s = (s).wrapping_sub(self.mem[((b).wrapping_add(4i32)) as usize].int());
                        d = (d).wrapping_add(s);
                        e = (((e).wrapping_add(self.mem[((b).wrapping_add(1i32)) as usize].int())).wrapping_sub(z)).wrapping_sub(s);
                    }
                }
                if ((self.mem[(p) as usize].hh().b1()).wrapping_sub(0i32) == 2i32) {
                    q = p;
                } else {
                    {
                        r = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh();
                        self.free_node(p, 7i32);
                        if (r == 0i32) {
                            self.confusion(2027i32);
                        }
                        if (x > 0i32) {
                            {
                                p = r;
                                loop {
                                    q = r;
                                    r = self.mem[(r) as usize].hh().rh();
                                    if (r == 0i32) { break; }
                                }
                            }
                        } else {
                            {
                                p = 0i32;
                                q = r;
                                loop {
                                    t = self.mem[(r) as usize].hh().rh();
                                    self.mem[(r) as usize].set_hh_rh(p);
                                    p = r;
                                    r = t;
                                    if (r == 0i32) { break; }
                                }
                            }
                        }
                    }
                }
                // §1746
                if (j == 0i32) {
                    {
                        r = self.new_kern(0i32);
                        t = self.new_kern(0i32);
                    }
                } else {
                    {
                        r = self.mem[((b).wrapping_add(5i32)) as usize].hh().rh();
                        t = self.mem[(r) as usize].hh().rh();
                    }
                }
                u = self.new_math(0i32, 3i32);
                if (self.mem[(t) as usize].hh().b0() == 10i32) {
                    {
                        j = self.new_skip_param(8i32);
                        self.mem[(q) as usize].set_hh_rh(j);
                        self.mem[(j) as usize].set_hh_rh(u);
                        j = self.mem[((t).wrapping_add(1i32)) as usize].hh().lh();
                        { let __ix1757 = self.temp_ptr; let __v1758 = self.mem[(j) as usize].hh().b0(); self.mem[(__ix1757) as usize].set_hh_b0(__v1758); }
                        { let __ix1759 = self.temp_ptr; let __v1760 = self.mem[(j) as usize].hh().b1(); self.mem[(__ix1759) as usize].set_hh_b1(__v1760); }
                        { let __ix1761 = (self.temp_ptr).wrapping_add(1i32); let __v1762 = (e).wrapping_sub(self.mem[((j).wrapping_add(1i32)) as usize].int()); self.mem[(__ix1761) as usize].set_int(__v1762); }
                        { let __ix1763 = (self.temp_ptr).wrapping_add(2i32); let __v1764 = (self.mem[((j).wrapping_add(2i32)) as usize].int()).wrapping_neg(); self.mem[(__ix1763) as usize].set_int(__v1764); }
                        { let __ix1765 = (self.temp_ptr).wrapping_add(3i32); let __v1766 = (self.mem[((j).wrapping_add(3i32)) as usize].int()).wrapping_neg(); self.mem[(__ix1765) as usize].set_int(__v1766); }
                        self.mem[(u) as usize].set_hh_rh(t);
                    }
                } else {
                    {
                        self.mem[((t).wrapping_add(1i32)) as usize].set_int(e);
                        self.mem[(t) as usize].set_hh_rh(u);
                        self.mem[(q) as usize].set_hh_rh(t);
                    }
                }
                u = self.new_math(0i32, 2i32);
                if (self.mem[(r) as usize].hh().b0() == 10i32) {
                    {
                        j = self.new_skip_param(7i32);
                        self.mem[(u) as usize].set_hh_rh(j);
                        self.mem[(j) as usize].set_hh_rh(p);
                        j = self.mem[((r).wrapping_add(1i32)) as usize].hh().lh();
                        { let __ix1767 = self.temp_ptr; let __v1768 = self.mem[(j) as usize].hh().b0(); self.mem[(__ix1767) as usize].set_hh_b0(__v1768); }
                        { let __ix1769 = self.temp_ptr; let __v1770 = self.mem[(j) as usize].hh().b1(); self.mem[(__ix1769) as usize].set_hh_b1(__v1770); }
                        { let __ix1771 = (self.temp_ptr).wrapping_add(1i32); let __v1772 = (d).wrapping_sub(self.mem[((j).wrapping_add(1i32)) as usize].int()); self.mem[(__ix1771) as usize].set_int(__v1772); }
                        { let __ix1773 = (self.temp_ptr).wrapping_add(2i32); let __v1774 = (self.mem[((j).wrapping_add(2i32)) as usize].int()).wrapping_neg(); self.mem[(__ix1773) as usize].set_int(__v1774); }
                        { let __ix1775 = (self.temp_ptr).wrapping_add(3i32); let __v1776 = (self.mem[((j).wrapping_add(3i32)) as usize].int()).wrapping_neg(); self.mem[(__ix1775) as usize].set_int(__v1776); }
                        self.mem[(r) as usize].set_hh_rh(u);
                    }
                } else {
                    {
                        self.mem[((r).wrapping_add(1i32)) as usize].set_int(d);
                        self.mem[(r) as usize].set_hh_rh(p);
                        self.mem[(u) as usize].set_hh_rh(r);
                        if (j == 0i32) {
                            {
                                b = self.hpack(u, 0i32, 1i32);
                                self.mem[((b).wrapping_add(4i32)) as usize].set_int(s);
                            }
                        } else {
                            self.mem[((b).wrapping_add(5i32)) as usize].set_hh_rh(u);
                        }
                    }
                }
            }
        }
        // §1744
        self.append_to_vlist(b);
    }

    /// @<Declare act...
    // §1372
    pub fn after_math(&mut self) {
        let mut l: bool = false; // §1372
        let mut danger: bool = false; // §1372
        let mut m: i32 = 0; // §1372
        let mut p: halfword = 0; // §1372
        let mut a: halfword = 0; // §1372
        let mut b: halfword = 0; // §1376
        let mut w: scaled = 0; // §1376
        let mut z: scaled = 0; // §1376
        let mut e: scaled = 0; // §1376
        let mut q: scaled = 0; // §1376
        let mut d: scaled = 0; // §1376
        let mut s: scaled = 0; // §1376
        let mut g1: small_number = 0; // §1376
        let mut g2: small_number = 0; // §1376
        let mut r: halfword = 0; // §1376
        let mut t: halfword = 0; // §1376
        let mut pre_t: halfword = 0; // §1376
        let mut j: halfword = 0; // §1741
        danger = false;
        // §1742
        if (self.cur_list.mode_field == 209i32) {
            j = self.cur_list.eTeX_aux_field;
        }
        // §1373
        if (((self.font_params[(self.eqtb[((627692i32) - 1) as usize].hh().rh()) as usize] < 22i32) || (self.font_params[(self.eqtb[((627708i32) - 1) as usize].hh().rh()) as usize] < 22i32)) || (self.font_params[(self.eqtb[((627724i32) - 1) as usize].hh().rh()) as usize] < 22i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1574i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 1575i32;
                    self.help_line[(1i32) as usize] = 1576i32;
                    self.help_line[(0i32) as usize] = 1577i32;
                }
                self.error();
                self.flush_math();
                danger = true;
            }
        } else {
            if (((self.font_params[(self.eqtb[((627693i32) - 1) as usize].hh().rh()) as usize] < 13i32) || (self.font_params[(self.eqtb[((627709i32) - 1) as usize].hh().rh()) as usize] < 13i32)) || (self.font_params[(self.eqtb[((627725i32) - 1) as usize].hh().rh()) as usize] < 13i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(1578i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[(2i32) as usize] = 1579i32;
                        self.help_line[(1i32) as usize] = 1580i32;
                        self.help_line[(0i32) as usize] = 1581i32;
                    }
                    self.error();
                    self.flush_math();
                    danger = true;
                }
            }
        }
        // §1372
        m = self.cur_list.mode_field;
        l = false;
        p = self.fin_mlist(0i32);
        if (self.cur_list.mode_field == (m).wrapping_neg()) {
            {
                // §1375
                {
                    self.get_x_token();
                    if (self.cur_cmd != 3i32) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1582i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1583i32;
                                self.help_line[(0i32) as usize] = 1584i32;
                            }
                            self.back_error();
                        }
                    }
                }
                // §1372
                self.cur_mlist = p;
                self.cur_style = 2i32;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                a = self.hpack(self.mem[(4999996i32) as usize].hh().rh(), 0i32, 1i32);
                self.mem[(a) as usize].set_hh_b1(2i32);
                self.unsave();
                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() == 1i32) {
                    l = true;
                }
                danger = false;
                // §1742
                if (self.cur_list.mode_field == 209i32) {
                    j = self.cur_list.eTeX_aux_field;
                }
                // §1373
                if (((self.font_params[(self.eqtb[((627692i32) - 1) as usize].hh().rh()) as usize] < 22i32) || (self.font_params[(self.eqtb[((627708i32) - 1) as usize].hh().rh()) as usize] < 22i32)) || (self.font_params[(self.eqtb[((627724i32) - 1) as usize].hh().rh()) as usize] < 22i32)) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1574i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 1575i32;
                            self.help_line[(1i32) as usize] = 1576i32;
                            self.help_line[(0i32) as usize] = 1577i32;
                        }
                        self.error();
                        self.flush_math();
                        danger = true;
                    }
                } else {
                    if (((self.font_params[(self.eqtb[((627693i32) - 1) as usize].hh().rh()) as usize] < 13i32) || (self.font_params[(self.eqtb[((627709i32) - 1) as usize].hh().rh()) as usize] < 13i32)) || (self.font_params[(self.eqtb[((627725i32) - 1) as usize].hh().rh()) as usize] < 13i32)) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1578i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1579i32;
                                self.help_line[(1i32) as usize] = 1580i32;
                                self.help_line[(0i32) as usize] = 1581i32;
                            }
                            self.error();
                            self.flush_math();
                            danger = true;
                        }
                    }
                }
                // §1372
                m = self.cur_list.mode_field;
                p = self.fin_mlist(0i32);
            }
        } else {
            a = 0i32;
        }
        if (m < 0i32) {
            // §1374
            {
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1777 = self.cur_list.tail_field; let __v1778 = self.new_math(self.eqtb[((629641i32) - 1) as usize].int(), 0i32); self.mem[(__ix1777) as usize].set_hh_rh(__v1778); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                self.cur_mlist = p;
                self.cur_style = 2i32;
                self.mlist_penalties = (self.cur_list.mode_field > 0i32);
                self.mlist_to_hlist();
                { let __ix1779 = self.cur_list.tail_field; let __v1780 = self.mem[(4999996i32) as usize].hh().rh(); self.mem[(__ix1779) as usize].set_hh_rh(__v1780); }
                while (self.mem[(self.cur_list.tail_field) as usize].hh().rh() != 0i32) {
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1781 = self.cur_list.tail_field; let __v1782 = self.new_math(self.eqtb[((629641i32) - 1) as usize].int(), 1i32); self.mem[(__ix1781) as usize].set_hh_rh(__v1782); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                self.cur_list.aux_field.set_hh_lh(1000i32);
                self.unsave();
            }
        } else {
            // §1372
            {
                if (a == 0i32) {
                    // §1375
                    {
                        self.get_x_token();
                        if (self.cur_cmd != 3i32) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1582i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1583i32;
                                    self.help_line[(0i32) as usize] = 1584i32;
                                }
                                self.back_error();
                            }
                        }
                    }
                }
                // §1377
                self.cur_mlist = p;
                self.cur_style = 0i32;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                p = self.mem[(4999996i32) as usize].hh().rh();
                self.adjust_tail = 4999994i32;
                self.pre_adjust_tail = 4999985i32;
                b = self.hpack(p, 0i32, 1i32);
                p = self.mem[((b).wrapping_add(5i32)) as usize].hh().rh();
                t = self.adjust_tail;
                self.adjust_tail = 0i32;
                pre_t = self.pre_adjust_tail;
                self.pre_adjust_tail = 0i32;
                w = self.mem[((b).wrapping_add(1i32)) as usize].int();
                z = self.eqtb[((629654i32) - 1) as usize].int();
                s = self.eqtb[((629655i32) - 1) as usize].int();
                if (self.eqtb[((629121i32) - 1) as usize].int() < 0i32) {
                    s = ((s).wrapping_neg()).wrapping_sub(z);
                }
                if ((a == 0i32) || danger) {
                    {
                        e = 0i32;
                        q = 0i32;
                    }
                } else {
                    {
                        e = self.mem[((a).wrapping_add(1i32)) as usize].int();
                        q = (e).wrapping_add(self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[((627692i32) - 1) as usize].hh().rh()) as usize])) as usize].int());
                    }
                }
                if ((w).wrapping_add(q) > z) {
                    // §1379
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
                // §1380
                self.mem[(b) as usize].set_hh_b1(2i32);
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
                // §1381
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1783 = self.cur_list.tail_field; let __v1784 = self.new_penalty(self.eqtb[((629029i32) - 1) as usize].int()); self.mem[(__ix1783) as usize].set_hh_rh(__v1784); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                if (((d).wrapping_add(s) <= self.eqtb[((629653i32) - 1) as usize].int()) || l) {
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
                        self.app_display(j, a, 0i32);
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1785 = self.cur_list.tail_field; let __v1786 = self.new_penalty(10000i32); self.mem[(__ix1785) as usize].set_hh_rh(__v1786); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                } else {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1787 = self.cur_list.tail_field; let __v1788 = self.new_param_glue(g1); self.mem[(__ix1787) as usize].set_hh_rh(__v1788); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
                // §1382
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
                self.app_display(j, b, d);
                // §1383
                if (((a != 0i32) && (e == 0i32)) && (!l)) {
                    {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix1789 = self.cur_list.tail_field; let __v1790 = self.new_penalty(10000i32); self.mem[(__ix1789) as usize].set_hh_rh(__v1790); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                        self.app_display(j, a, (z).wrapping_sub(self.mem[((a).wrapping_add(1i32)) as usize].int()));
                        g2 = 0i32;
                    }
                }
                if (t != 4999994i32) {
                    {
                        { let __ix1791 = self.cur_list.tail_field; let __v1792 = self.mem[(4999994i32) as usize].hh().rh(); self.mem[(__ix1791) as usize].set_hh_rh(__v1792); }
                        self.cur_list.tail_field = t;
                    }
                }
                if (pre_t != 4999985i32) {
                    {
                        { let __ix1793 = self.cur_list.tail_field; let __v1794 = self.mem[(4999985i32) as usize].hh().rh(); self.mem[(__ix1793) as usize].set_hh_rh(__v1794); }
                        self.cur_list.tail_field = pre_t;
                    }
                }
                {
                    self.prev_tail = self.cur_list.tail_field;
                    { let __ix1795 = self.cur_list.tail_field; let __v1796 = self.new_penalty(self.eqtb[((629030i32) - 1) as usize].int()); self.mem[(__ix1795) as usize].set_hh_rh(__v1796); }
                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                }
                if (g2 > 0i32) {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix1797 = self.cur_list.tail_field; let __v1798 = self.new_param_glue(g2); self.mem[(__ix1797) as usize].set_hh_rh(__v1798); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
                // §1743
                self.flush_node_list(j);
                // §1377
                self.resume_after_display();
            }
        }
    }

    /// @<Declare act...
    // §1378
    pub fn resume_after_display(&mut self) {
        if (self.cur_group != 15i32) {
            self.confusion(1585i32);
        }
        self.unsave();
        self.cur_list.pg_field = (self.cur_list.pg_field).wrapping_add(3i32);
        self.push_nest();
        self.cur_list.mode_field = 105i32;
        self.cur_list.aux_field.set_hh_lh(1000i32);
        if (self.eqtb[((629068i32) - 1) as usize].int() <= 0i32) {
            self.cur_lang = 0i32;
        } else {
            if (self.eqtb[((629068i32) - 1) as usize].int() > 255i32) {
                self.cur_lang = 0i32;
            } else {
                self.cur_lang = self.eqtb[((629068i32) - 1) as usize].int();
            }
        }
        { let __v1799 = self.cur_lang; self.cur_list.aux_field.set_hh_rh(__v1799); }
        self.cur_list.pg_field = ((((self.norm_min(self.eqtb[((629069i32) - 1) as usize].int())).wrapping_mul(64i32)).wrapping_add(self.norm_min(self.eqtb[((629070i32) - 1) as usize].int()))).wrapping_mul(65536i32)).wrapping_add(self.cur_lang);
        // §469
        {
            self.get_x_token();
            if (self.cur_cmd != 10i32) {
                self.back_input();
            }
        }
        // §1378
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// When a control sequence is to be defined, by \.{\\def} or \.{\\let} or
    /// something similar, the `get_r_token` routine will substitute a special
    /// control sequence for a token that is not redefinable.
    /// @<Declare subprocedures for `prefixed_command`
    // §1393
    pub fn get_r_token(&mut self) {
        'l_restart_b: loop {
            loop {
                self.get_token();
                if (self.cur_tok != 2592i32) { break; }
            }
            if ((self.cur_cs == 0i32) || (self.cur_cs > 615514i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(1603i32);
                    }
                    {
                        self.help_ptr = 5i32;
                        self.help_line[(4i32) as usize] = 1604i32;
                        self.help_line[(3i32) as usize] = 1605i32;
                        self.help_line[(2i32) as usize] = 1606i32;
                        self.help_line[(1i32) as usize] = 1607i32;
                        self.help_line[(0i32) as usize] = 1608i32;
                    }
                    if (self.cur_cs == 0i32) {
                        self.back_input();
                    }
                    self.cur_tok = 619609i32;
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
    // §1407
    pub fn trap_zero_glue(&mut self) {
        if (((self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int() == 0i32) && (self.mem[((self.cur_val).wrapping_add(2i32)) as usize].int() == 0i32)) && (self.mem[((self.cur_val).wrapping_add(3i32)) as usize].int() == 0i32)) {
            {
                { let __v1800 = (self.mem[(0i32) as usize].hh().rh()).wrapping_add(1i32); self.mem[(0i32) as usize].set_hh_rh(__v1800); }
                self.delete_glue_ref(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// We use the fact that `register<advance<multiply<divide`.
    /// @<Declare subprocedures for `prefixed_command`
    // §1414
    pub fn do_register_command(&mut self, mut a: small_number) {
        let mut l: halfword = 0; // §1414
        let mut q: halfword = 0; // §1414
        let mut r: halfword = 0; // §1414
        let mut s: halfword = 0; // §1414
        let mut p: i32 = 0; // §1414
        let mut e: bool = false; // §1414
        let mut w: i32 = 0; // §1414
        'l_exit_f: {
            'l_found_f: {
                q = self.cur_cmd;
                e = false;
                // §1415
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
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(786i32);
                                    }
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print(787i32);
                                    self.print_cmd_chr(q, 0i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 1629i32;
                                    }
                                    self.error();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    if ((self.cur_chr < 0i32) || (self.cur_chr > 19i32)) {
                        {
                            l = self.cur_chr;
                            p = (self.mem[(l) as usize].hh().b0() / 16i32);
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
                                    0 => {
                                        l = (self.cur_val).wrapping_add(629128i32);
                                    }
                                    1 => {
                                        l = (self.cur_val).wrapping_add(629674i32);
                                    }
                                    2 => {
                                        l = (self.cur_val).wrapping_add(626646i32);
                                    }
                                    3 => {
                                        l = (self.cur_val).wrapping_add(626902i32);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            if (p < 2i32) {
                if e {
                    w = self.mem[((l).wrapping_add(2i32)) as usize].int();
                } else {
                    w = self.eqtb[((l) - 1) as usize].int();
                }
            } else {
                if e {
                    s = self.mem[((l).wrapping_add(1i32)) as usize].hh().rh();
                } else {
                    s = self.eqtb[((l) - 1) as usize].hh().rh();
                }
            }
            // §1414
            if (q == 89i32) {
                self.scan_optional_equals();
            } else {
                if self.scan_keyword(1625i32) {
                }
            }
            self.arith_error = false;
            if (q < 91i32) {
                // §1416
                if (p < 2i32) {
                    {
                        if (p == 0i32) {
                            self.scan_int();
                        } else {
                            self.scan_dimen(false, false, false);
                        }
                        if (q == 90i32) {
                            self.cur_val = (self.cur_val).wrapping_add(w);
                        }
                    }
                } else {
                    {
                        self.scan_glue(p);
                        if (q == 90i32) {
                            // §1417
                            {
                                q = self.new_spec(self.cur_val);
                                r = s;
                                self.delete_glue_ref(self.cur_val);
                                { let __v1801 = (self.mem[((q).wrapping_add(1i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(1i32)) as usize].int()); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v1801); }
                                if (self.mem[((q).wrapping_add(2i32)) as usize].int() == 0i32) {
                                    self.mem[(q) as usize].set_hh_b0(0i32);
                                }
                                if (self.mem[(q) as usize].hh().b0() == self.mem[(r) as usize].hh().b0()) {
                                    { let __v1802 = (self.mem[((q).wrapping_add(2i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(2i32)) as usize].int()); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v1802); }
                                } else {
                                    if ((self.mem[(q) as usize].hh().b0() < self.mem[(r) as usize].hh().b0()) && (self.mem[((r).wrapping_add(2i32)) as usize].int() != 0i32)) {
                                        {
                                            { let __v1803 = self.mem[((r).wrapping_add(2i32)) as usize].int(); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v1803); }
                                            { let __v1804 = self.mem[(r) as usize].hh().b0(); self.mem[(q) as usize].set_hh_b0(__v1804); }
                                        }
                                    }
                                }
                                if (self.mem[((q).wrapping_add(3i32)) as usize].int() == 0i32) {
                                    self.mem[(q) as usize].set_hh_b1(0i32);
                                }
                                if (self.mem[(q) as usize].hh().b1() == self.mem[(r) as usize].hh().b1()) {
                                    { let __v1805 = (self.mem[((q).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((r).wrapping_add(3i32)) as usize].int()); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v1805); }
                                } else {
                                    if ((self.mem[(q) as usize].hh().b1() < self.mem[(r) as usize].hh().b1()) && (self.mem[((r).wrapping_add(3i32)) as usize].int() != 0i32)) {
                                        {
                                            { let __v1806 = self.mem[((r).wrapping_add(3i32)) as usize].int(); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v1806); }
                                            { let __v1807 = self.mem[(r) as usize].hh().b1(); self.mem[(q) as usize].set_hh_b1(__v1807); }
                                        }
                                    }
                                }
                                self.cur_val = q;
                            }
                        }
                    }
                }
            } else {
                // §1418
                {
                    self.scan_int();
                    if (p < 2i32) {
                        if (q == 91i32) {
                            if (p == 0i32) {
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
                            if (q == 91i32) {
                                {
                                    { let __v1808 = self.mult_and_add(self.mem[((s).wrapping_add(1i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v1808); }
                                    { let __v1809 = self.mult_and_add(self.mem[((s).wrapping_add(2i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v1809); }
                                    { let __v1810 = self.mult_and_add(self.mem[((s).wrapping_add(3i32)) as usize].int(), self.cur_val, 0i32, 1073741823i32); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v1810); }
                                }
                            } else {
                                {
                                    { let __v1811 = self.x_over_n(self.mem[((s).wrapping_add(1i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(1i32)) as usize].set_int(__v1811); }
                                    { let __v1812 = self.x_over_n(self.mem[((s).wrapping_add(2i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(2i32)) as usize].set_int(__v1812); }
                                    { let __v1813 = self.x_over_n(self.mem[((s).wrapping_add(3i32)) as usize].int(), self.cur_val); self.mem[((r).wrapping_add(3i32)) as usize].set_int(__v1813); }
                                }
                            }
                            self.cur_val = r;
                        }
                    }
                }
            }
            // §1414
            if self.arith_error {
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
                        self.help_line[(1i32) as usize] = 1627i32;
                        self.help_line[(0i32) as usize] = 1628i32;
                    }
                    if (p >= 2i32) {
                        self.delete_glue_ref(self.cur_val);
                    }
                    self.error();
                    break 'l_exit_f;
                }
            }
            if (p < 2i32) {
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
                            self.geq_define(l, 120i32, self.cur_val);
                        } else {
                            self.eq_define(l, 120i32, self.cur_val);
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1421
    pub fn alter_aux(&mut self) {
        let mut c: halfword = 0; // §1421
        if (self.cur_chr != (self.cur_list.mode_field).wrapping_abs()) {
            self.report_illegal_case();
        } else {
            {
                c = self.cur_chr;
                self.scan_optional_equals();
                if (c == 1i32) {
                    {
                        self.scan_dimen(false, false, false);
                        { let __v1814 = self.cur_val; self.cur_list.aux_field.set_int(__v1814); }
                    }
                } else {
                    {
                        self.scan_int();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32767i32)) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1632i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1633i32;
                                }
                                self.int_error(self.cur_val);
                            }
                        } else {
                            { let __v1815 = self.cur_val; self.cur_list.aux_field.set_hh_lh(__v1815); }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1422
    pub fn alter_prev_graf(&mut self) {
        let mut p: i32 = 0; // §1422
        { let __ix1816 = self.nest_ptr; let __v1817 = self.cur_list; self.nest[(__ix1816) as usize] = __v1817; }
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
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1368i32);
                }
                self.print_esc(611i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 1634i32;
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
    // §1423
    pub fn alter_page_so_far(&mut self) {
        let mut c: i32 = 0; // §1423
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        { let __v1818 = self.cur_val; self.page_so_far[(c) as usize] = __v1818; }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1424
    pub fn alter_integer(&mut self) {
        let mut c: small_number = 0; // §1424
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_int();
        if (c == 0i32) {
            self.dead_cycles = self.cur_val;
        } else {
            // §1696
            if (c == 2i32) {
                {
                    if ((self.cur_val < 0i32) || (self.cur_val > 3i32)) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(2012i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 2013i32;
                                self.help_line[(0i32) as usize] = 2014i32;
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
                // §1424
                self.insert_penalties = self.cur_val;
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1425
    pub fn alter_box_dimen(&mut self) {
        let mut c: small_number = 0; // §1425
        let mut b: halfword = 0; // §1425
        c = self.cur_chr;
        self.scan_register_num();
        if (self.cur_val < 256i32) {
            b = self.eqtb[(((627433i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
        } else {
            {
                self.find_sa_element(4i32, self.cur_val, false);
                if (self.cur_ptr == 0i32) {
                    b = 0i32;
                } else {
                    b = self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().rh();
                }
            }
        }
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        if (b != 0i32) {
            { let __v1819 = self.cur_val; self.mem[((b).wrapping_add(c)) as usize].set_int(__v1819); }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1435
    pub fn new_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §1435
        let mut s: scaled = 0; // §1435
        let mut f: internal_font_number = 0; // §1435
        let mut t: str_number = 0; // §1435
        let mut old_setting: i32 = 0; // §1435
        let mut flushable_string: str_number = 0; // §1435
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
                        t = 1095i32;
                    } else {
                        t = (u).wrapping_sub(257i32);
                    }
                } else {
                    {
                        old_setting = self.selector;
                        self.selector = 21i32;
                        self.print(1095i32);
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
                self.geq_define(u, 87i32, 0i32);
            } else {
                self.eq_define(u, 87i32, 0i32);
            }
            self.scan_optional_equals();
            self.scan_file_name();
            // §1436
            self.name_in_progress = true;
            if self.scan_keyword(1648i32) {
                // §1437
                {
                    self.scan_dimen(false, false, false);
                    s = self.cur_val;
                    if ((s <= 0i32) || (s >= 134217728i32)) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1650i32);
                            }
                            self.print_scaled(s);
                            self.print(1651i32);
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1652i32;
                                self.help_line[(0i32) as usize] = 1653i32;
                            }
                            self.error();
                            s = (10i32).wrapping_mul(65536i32);
                        }
                    }
                }
            } else {
                // §1436
                if self.scan_keyword(1649i32) {
                    {
                        self.scan_int();
                        s = (self.cur_val).wrapping_neg();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32768i32)) {
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
                                    self.help_line[(0i32) as usize] = 636i32;
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
            // §1438
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
            // §1435
            f = self.read_font_info(u, self.cur_name, self.cur_area, s);
        }
        if (a >= 4i32) {
            self.geq_define(u, 87i32, f);
        } else {
            self.eq_define(u, 87i32, f);
        }
        { let __v1820 = self.eqtb[((u) - 1) as usize]; self.eqtb[(((617626i32).wrapping_add(f)) - 1) as usize] = __v1820; }
        self.hash[(((617626i32).wrapping_add(f)) - 514) as usize].set_rh(t);
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1443
    pub fn new_interaction(&mut self) {
        self.print_ln();
        self.interaction = self.cur_chr;
        // §75
        if (self.interaction == 0i32) {
            self.selector = 16i32;
        } else {
            self.selector = 17i32;
        }
        // §1443
        if self.log_opened {
            self.selector = (self.selector).wrapping_add(2i32);
        }
    }

    /// If the user says, e.g., `\.{\\global\\global}', the redundancy is
    /// silently accepted.
    /// @<Declare act...
    // §1389
    pub fn prefixed_command(&mut self) {
        let mut a: small_number = 0; // §1389
        let mut f: internal_font_number = 0; // §1389
        let mut j: halfword = 0; // §1389
        let mut k: font_index = 0; // §1389
        let mut p: halfword = 0; // §1389
        let mut q: halfword = 0; // §1389
        let mut n: i32 = 0; // §1389
        let mut e: bool = false; // §1389
        'l_exit_f: {
            'l_done_f: {
                a = 0i32;
                while (self.cur_cmd == 93i32) {
                    {
                        if (!((((a / self.cur_chr)) % 2) != 0)) {
                            a = (a).wrapping_add(self.cur_chr);
                        }
                        // §430
                        loop {
                            self.get_x_token();
                            if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                        }
                        // §1389
                        if (self.cur_cmd <= 70i32) {
                            // §1390
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1595i32);
                                }
                                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                self.print_char(39i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 1596i32;
                                }
                                if (self.eTeX_mode == 1i32) {
                                    self.help_line[(0i32) as usize] = 1597i32;
                                }
                                self.back_error();
                                break 'l_exit_f;
                            }
                        }
                        // §1389
                        if (self.eqtb[((629054i32) - 1) as usize].int() > 2i32) {
                            if (self.eTeX_mode == 1i32) {
                                self.show_cur_cmd_chr();
                            }
                        }
                    }
                }
                // §1391
                if (a >= 8i32) {
                    {
                        j = 3585i32;
                        a = (a).wrapping_sub(8i32);
                    }
                } else {
                    j = 0i32;
                }
                if ((self.cur_cmd != 97i32) && (((a % 4i32) != 0i32) || (j != 0i32))) {
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
                        self.print_esc(1587i32);
                        self.print(1598i32);
                        self.print_esc(1588i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1599i32;
                        }
                        if (self.eTeX_mode == 1i32) {
                            {
                                self.help_line[(0i32) as usize] = 1600i32;
                                self.print(1598i32);
                                self.print_esc(1601i32);
                            }
                        }
                        self.print(1602i32);
                        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                        self.print_char(39i32);
                        self.error();
                    }
                }
                // §1392
                if (self.eqtb[((629061i32) - 1) as usize].int() != 0i32) {
                    if (self.eqtb[((629061i32) - 1) as usize].int() < 0i32) {
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
                // §1389
                match self.cur_cmd {
                    87 => {
                        // §1395
                        if (a >= 4i32) {
                            self.geq_define(627689i32, 123i32, self.cur_chr);
                        } else {
                            self.eq_define(627689i32, 123i32, self.cur_chr);
                        }
                    }
                    97 => {
                        // §1396
                        {
                            if (((((self.cur_chr) % 2) != 0) && (!(a >= 4i32))) && (self.eqtb[((629061i32) - 1) as usize].int() >= 0i32)) {
                                a = (a).wrapping_add(4i32);
                            }
                            e = (self.cur_chr >= 2i32);
                            self.get_r_token();
                            p = self.cur_cs;
                            q = self.scan_toks(true, e);
                            if (j != 0i32) {
                                {
                                    q = self.get_avail();
                                    self.mem[(q) as usize].set_hh_lh(j);
                                    { let __v1821 = self.mem[(self.def_ref) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v1821); }
                                    { let __ix1822 = self.def_ref; self.mem[(__ix1822) as usize].set_hh_rh(q); }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(p, (114i32).wrapping_add((a % 4i32)), self.def_ref);
                            } else {
                                self.eq_define(p, (114i32).wrapping_add((a % 4i32)), self.def_ref);
                            }
                        }
                    }
                    94 => {
                        // §1399
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
                            if (self.cur_cmd >= 114i32) {
                                { let __ix1823 = self.cur_chr; let __v1824 = (self.mem[(self.cur_chr) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1823) as usize].set_hh_lh(__v1824); }
                            } else {
                                if ((self.cur_cmd == 89i32) || (self.cur_cmd == 71i32)) {
                                    if ((self.cur_chr < 0i32) || (self.cur_chr > 19i32)) {
                                        { let __ix1825 = (self.cur_chr).wrapping_add(1i32); let __v1826 = (self.mem[((self.cur_chr).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1825) as usize].set_hh_lh(__v1826); }
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
                    95 => {
                        // §1402
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
                                        self.scan_register_num();
                                        if (self.cur_val > 255i32) {
                                            {
                                                j = (n).wrapping_sub(2i32);
                                                if (j > 3i32) {
                                                    j = 5i32;
                                                }
                                                self.find_sa_element(j, self.cur_val, true);
                                                { let __ix1827 = (self.cur_ptr).wrapping_add(1i32); let __v1828 = (self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix1827) as usize].set_hh_lh(__v1828); }
                                                if (j == 5i32) {
                                                    j = 71i32;
                                                } else {
                                                    j = 89i32;
                                                }
                                                if (a >= 4i32) {
                                                    self.geq_define(p, j, self.cur_ptr);
                                                } else {
                                                    self.eq_define(p, j, self.cur_ptr);
                                                }
                                            }
                                        } else {
                                            match n {
                                                2 => {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 73i32, (629128i32).wrapping_add(self.cur_val));
                                                    } else {
                                                        self.eq_define(p, 73i32, (629128i32).wrapping_add(self.cur_val));
                                                    }
                                                }
                                                3 => {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 74i32, (629674i32).wrapping_add(self.cur_val));
                                                    } else {
                                                        self.eq_define(p, 74i32, (629674i32).wrapping_add(self.cur_val));
                                                    }
                                                }
                                                4 => {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 75i32, (626646i32).wrapping_add(self.cur_val));
                                                    } else {
                                                        self.eq_define(p, 75i32, (626646i32).wrapping_add(self.cur_val));
                                                    }
                                                }
                                                5 => {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 76i32, (626902i32).wrapping_add(self.cur_val));
                                                    } else {
                                                        self.eq_define(p, 76i32, (626902i32).wrapping_add(self.cur_val));
                                                    }
                                                }
                                                6 => {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 72i32, (627173i32).wrapping_add(self.cur_val));
                                                    } else {
                                                        self.eq_define(p, 72i32, (627173i32).wrapping_add(self.cur_val));
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
                    96 => {
                        // §1403
                        {
                            j = self.cur_chr;
                            self.scan_int();
                            n = self.cur_val;
                            if (!self.scan_keyword(1244i32)) {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(1486i32);
                                    }
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 1619i32;
                                        self.help_line[(0i32) as usize] = 1620i32;
                                    }
                                    self.error();
                                }
                            }
                            self.get_r_token();
                            p = self.cur_cs;
                            self.read_toks(n, p, j);
                            if (a >= 4i32) {
                                self.geq_define(p, 114i32, self.cur_val);
                            } else {
                                self.eq_define(p, 114i32, self.cur_val);
                            }
                        }
                    }
                    71 | 72 => {
                        // §1404
                        {
                            q = self.cur_cs;
                            e = false;
                            if (self.cur_cmd == 71i32) {
                                if (self.cur_chr == 0i32) {
                                    {
                                        self.scan_register_num();
                                        if (self.cur_val > 255i32) {
                                            {
                                                self.find_sa_element(5i32, self.cur_val, true);
                                                self.cur_chr = self.cur_ptr;
                                                e = true;
                                            }
                                        } else {
                                            self.cur_chr = (627173i32).wrapping_add(self.cur_val);
                                        }
                                    }
                                } else {
                                    e = true;
                                }
                            }
                            p = self.cur_chr;
                            self.scan_optional_equals();
                            // §430
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                            }
                            // §1404
                            if (self.cur_cmd != 1i32) {
                                // §1405
                                if ((self.cur_cmd == 71i32) || (self.cur_cmd == 72i32)) {
                                    {
                                        if (self.cur_cmd == 71i32) {
                                            if (self.cur_chr == 0i32) {
                                                {
                                                    self.scan_register_num();
                                                    if (self.cur_val < 256i32) {
                                                        q = self.eqtb[(((627173i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                                                    } else {
                                                        {
                                                            self.find_sa_element(5i32, self.cur_val, false);
                                                            if (self.cur_ptr == 0i32) {
                                                                q = 0i32;
                                                            } else {
                                                                q = self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                q = self.mem[((self.cur_chr).wrapping_add(1i32)) as usize].hh().rh();
                                            }
                                        } else {
                                            q = self.eqtb[((self.cur_chr) - 1) as usize].hh().rh();
                                        }
                                        if (q == 0i32) {
                                            if e {
                                                if (a >= 4i32) {
                                                    self.gsa_def(p, 0i32);
                                                } else {
                                                    self.sa_def(p, 0i32);
                                                }
                                            } else {
                                                if (a >= 4i32) {
                                                    self.geq_define(p, 104i32, 0i32);
                                                } else {
                                                    self.eq_define(p, 104i32, 0i32);
                                                }
                                            }
                                        } else {
                                            {
                                                { let __v1829 = (self.mem[(q) as usize].hh().lh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_lh(__v1829); }
                                                if e {
                                                    if (a >= 4i32) {
                                                        self.gsa_def(p, q);
                                                    } else {
                                                        self.sa_def(p, q);
                                                    }
                                                } else {
                                                    if (a >= 4i32) {
                                                        self.geq_define(p, 114i32, q);
                                                    } else {
                                                        self.eq_define(p, 114i32, q);
                                                    }
                                                }
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                            }
                            // §1404
                            self.back_input();
                            self.cur_cs = q;
                            q = self.scan_toks(false, false);
                            if (self.mem[(self.def_ref) as usize].hh().rh() == 0i32) {
                                {
                                    if e {
                                        if (a >= 4i32) {
                                            self.gsa_def(p, 0i32);
                                        } else {
                                            self.sa_def(p, 0i32);
                                        }
                                    } else {
                                        if (a >= 4i32) {
                                            self.geq_define(p, 104i32, 0i32);
                                        } else {
                                            self.eq_define(p, 104i32, 0i32);
                                        }
                                    }
                                    {
                                        { let __ix1830 = self.def_ref; let __v1831 = self.avail; self.mem[(__ix1830) as usize].set_hh_rh(__v1831); }
                                        self.avail = self.def_ref;
                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                    }
                                }
                            } else {
                                {
                                    if ((p == 627159i32) && (!e)) {
                                        {
                                            { let __v1832 = self.get_avail(); self.mem[(q) as usize].set_hh_rh(__v1832); }
                                            q = self.mem[(q) as usize].hh().rh();
                                            self.mem[(q) as usize].set_hh_lh(637i32);
                                            q = self.get_avail();
                                            self.mem[(q) as usize].set_hh_lh(379i32);
                                            { let __v1833 = self.mem[(self.def_ref) as usize].hh().rh(); self.mem[(q) as usize].set_hh_rh(__v1833); }
                                            { let __ix1834 = self.def_ref; self.mem[(__ix1834) as usize].set_hh_rh(q); }
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
                                            self.geq_define(p, 114i32, self.def_ref);
                                        } else {
                                            self.eq_define(p, 114i32, self.def_ref);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    73 => {
                        // §1406
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
                                self.geq_define(p, 120i32, self.cur_val);
                            } else {
                                self.eq_define(p, 120i32, self.cur_val);
                            }
                        }
                    }
                    85 => {
                        // §1410
                        {
                            // §1411
                            if (self.cur_chr == 627738i32) {
                                n = 15i32;
                            } else {
                                if (self.cur_chr == 628762i32) {
                                    n = 32768i32;
                                } else {
                                    if (self.cur_chr == 628506i32) {
                                        n = 32767i32;
                                    } else {
                                        if (self.cur_chr == 629384i32) {
                                            n = 16777215i32;
                                        } else {
                                            n = 255i32;
                                        }
                                    }
                                }
                            }
                            // §1410
                            p = self.cur_chr;
                            self.scan_char_num();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_int();
                            if (((self.cur_val < 0i32) && (p < 629384i32)) || (self.cur_val > n)) {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(1621i32);
                                    }
                                    self.print_int(((self.cur_val) as i64));
                                    if (p < 629384i32) {
                                        self.print(1622i32);
                                    } else {
                                        self.print(1623i32);
                                    }
                                    self.print_int(((n) as i64));
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 1624i32;
                                    }
                                    self.error();
                                    self.cur_val = 0i32;
                                }
                            }
                            if (p < 628762i32) {
                                if (a >= 4i32) {
                                    self.geq_define(p, 123i32, self.cur_val);
                                } else {
                                    self.eq_define(p, 123i32, self.cur_val);
                                }
                            } else {
                                if (p < 629384i32) {
                                    if (a >= 4i32) {
                                        self.geq_define(p, 123i32, (self.cur_val).wrapping_add(0i32));
                                    } else {
                                        self.eq_define(p, 123i32, (self.cur_val).wrapping_add(0i32));
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
                        // §1412
                        {
                            p = self.cur_chr;
                            self.scan_four_bit_int();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_font_ident();
                            if (a >= 4i32) {
                                self.geq_define(p, 123i32, self.cur_val);
                            } else {
                                self.eq_define(p, 123i32, self.cur_val);
                            }
                        }
                    }
                    89 | 90 | 91 | 92 => {
                        // §1413
                        self.do_register_command(a);
                    }
                    98 => {
                        // §1419
                        {
                            self.scan_register_num();
                            if (a >= 4i32) {
                                n = (1073774592i32).wrapping_add(self.cur_val);
                            } else {
                                n = (1073741824i32).wrapping_add(self.cur_val);
                            }
                            self.scan_optional_equals();
                            if self.set_box_allowed {
                                self.scan_box(n);
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
                                        self.print(781i32);
                                    }
                                    self.print_esc(615i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 1630i32;
                                        self.help_line[(0i32) as usize] = 1631i32;
                                    }
                                    self.error();
                                }
                            }
                        }
                    }
                    79 => {
                        // §1420
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
                        // §1426
                        {
                            q = self.cur_chr;
                            self.scan_optional_equals();
                            self.scan_int();
                            n = self.cur_val;
                            if (n <= 0i32) {
                                p = 0i32;
                            } else {
                                if (q > 627158i32) {
                                    {
                                        n = ((self.cur_val / 2i32)).wrapping_add(1i32);
                                        p = self.get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        self.mem[(p) as usize].set_hh_lh(n);
                                        n = self.cur_val;
                                        self.mem[((p).wrapping_add(1i32)) as usize].set_int(n);
                                        {
                                            let __for_end_10 = ((p).wrapping_add(n)).wrapping_add(1i32);
                                            j = (p).wrapping_add(2i32);
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_int();
                                                    { let __v1835 = self.cur_val; self.mem[(j) as usize].set_int(__v1835); }
                                                }
                                                j = j.wrapping_add(1);
                                            }
                                        }
                                        if (!(((n) % 2) != 0)) {
                                            self.mem[(((p).wrapping_add(n)).wrapping_add(2i32)) as usize].set_int(0i32);
                                        }
                                    }
                                } else {
                                    {
                                        p = self.get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        self.mem[(p) as usize].set_hh_lh(n);
                                        {
                                            let __for_end_10 = n;
                                            j = 1i32;
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_dimen(false, false, false);
                                                    { let __v1836 = self.cur_val; self.mem[(((p).wrapping_add((2i32).wrapping_mul(j))).wrapping_sub(1i32)) as usize].set_int(__v1836); }
                                                    self.scan_dimen(false, false, false);
                                                    { let __v1837 = self.cur_val; self.mem[((p).wrapping_add((2i32).wrapping_mul(j))) as usize].set_int(__v1837); }
                                                }
                                                j = j.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(q, 121i32, p);
                            } else {
                                self.eq_define(q, 121i32, p);
                            }
                        }
                    }
                    99 => {
                        // §1430
                        if (self.cur_chr == 1i32) {
                            {
                                self.new_patterns();
                                break 'l_done_f;
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1635i32);
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
                        // §1431
                        {
                            self.find_font_dimen(true);
                            k = self.cur_val;
                            self.scan_optional_equals();
                            self.scan_dimen(false, false, false);
                            { let __v1838 = self.cur_val; self.font_info[(k) as usize].set_int(__v1838); }
                        }
                    }
                    78 => {
                        {
                            n = self.cur_chr;
                            self.scan_font_ident();
                            f = self.cur_val;
                            if (n == 6i32) {
                                self.set_no_ligatures(f);
                            } else {
                                if (n < 2i32) {
                                    {
                                        self.scan_optional_equals();
                                        self.scan_int();
                                        if (n == 0i32) {
                                            { let __v1839 = self.cur_val; self.hyphen_char[(f) as usize] = __v1839; }
                                        } else {
                                            { let __v1840 = self.cur_val; self.skew_char[(f) as usize] = __v1840; }
                                        }
                                    }
                                } else {
                                    {
                                        self.scan_char_num();
                                        p = self.cur_val;
                                        self.scan_optional_equals();
                                        self.scan_int();
                                        match n {
                                            2 => {
                                                self.set_lp_code(f, p, self.cur_val);
                                            }
                                            3 => {
                                                self.set_rp_code(f, p, self.cur_val);
                                            }
                                            4 => {
                                                self.set_ef_code(f, p, self.cur_val);
                                            }
                                            5 => {
                                                self.set_tag_code(f, p, self.cur_val);
                                            }
                                            7 => {
                                                self.set_kn_bs_code(f, p, self.cur_val);
                                            }
                                            8 => {
                                                self.set_st_bs_code(f, p, self.cur_val);
                                            }
                                            9 => {
                                                self.set_sh_bs_code(f, p, self.cur_val);
                                            }
                                            10 => {
                                                self.set_kn_bc_code(f, p, self.cur_val);
                                            }
                                            11 => {
                                                self.set_kn_ac_code(f, p, self.cur_val);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    88 => {
                        // §1434
                        self.new_font(a);
                    }
                    101 => {
                        self.new_letterspaced_font(a);
                    }
                    102 => {
                        self.make_font_copy(a);
                    }
                    100 => {
                        // §1442
                        self.new_interaction();
                    }
                    _ => {
                        // §1389
                        self.confusion(1594i32);
                    }
                }
            }
            if (self.after_token != 0i32) {
                // §1447
                {
                    self.cur_tok = self.after_token;
                    self.back_input();
                    self.after_token = 0i32;
                }
            }
        }
        // §1389
    }

    /// Here is a procedure that might be called `Get the next non-blank non-relax
    /// non-call non-assignment token'.
    /// @<Declare act...
    // §1448
    pub fn do_assignments(&mut self) {
        'l_exit_f: {
            while true {
                {
                    // §430
                    loop {
                        self.get_x_token();
                        if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
                    }
                    // §1448
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
    // §1453
    pub fn open_or_close_in(&mut self) {
        let mut c: i32 = 0; // §1453
        let mut n: i32 = 0; // §1453
        c = self.cur_chr;
        self.scan_four_bit_int();
        n = self.cur_val;
        if (self.read_open[(n) as usize] != 2i32) {
            {
                { let mut __f0 = ::core::mem::take(&mut self.read_file[(n) as usize]); let __r = self.a_close(&mut __f0); self.read_file[(n) as usize] = __f0; __r };
                self.read_open[(n) as usize] = 2i32;
            }
        }
        if (c != 0i32) {
            {
                self.scan_optional_equals();
                self.scan_file_name();
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                if (self.kpse_in_name_ok() && { let mut __f0 = ::core::mem::take(&mut self.read_file[(n) as usize]); let __r = self.a_open_in(&mut __f0); self.read_file[(n) as usize] = __f0; __r }) {
                    self.read_open[(n) as usize] = 1i32;
                }
            }
        }
    }

    /// @<Declare act...
    // §1457
    pub fn issue_message(&mut self) {
        let mut old_setting: i32 = 0; // §1457
        let mut c: i32 = 0; // §1457
        let mut s: str_number = 0; // §1457
        c = self.cur_chr;
        { let __v1841 = self.scan_toks(false, true); self.mem[(4999987i32) as usize].set_hh_rh(__v1841); }
        old_setting = self.selector;
        self.selector = 21i32;
        self.token_show(self.def_ref);
        self.selector = old_setting;
        self.flush_list(self.def_ref);
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        if (c == 0i32) {
            // §1458
            {
                if ((self.term_offset).wrapping_add((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) > (self.max_print_line).wrapping_sub(2i32)) {
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
            // §1461
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(348i32);
                }
                self.slow_print(s);
                if (self.eqtb[((627167i32) - 1) as usize].hh().rh() != 0i32) {
                    self.use_err_help = true;
                } else {
                    if self.long_help_seen {
                        {
                            self.help_ptr = 1i32;
                            self.help_line[(0i32) as usize] = 1660i32;
                        }
                    } else {
                        {
                            if (self.interaction < 3i32) {
                                self.long_help_seen = true;
                            }
                            {
                                self.help_ptr = 4i32;
                                self.help_line[(3i32) as usize] = 1661i32;
                                self.help_line[(2i32) as usize] = 1662i32;
                                self.help_line[(1i32) as usize] = 1663i32;
                                self.help_line[(0i32) as usize] = 1664i32;
                            }
                        }
                    }
                }
                self.error();
                self.use_err_help = false;
            }
        }
        // §1457
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
        }
    }

    /// @<Declare act...
    // §1466
    pub fn shift_case(&mut self) {
        let mut b: halfword = 0; // §1466
        let mut p: halfword = 0; // §1466
        let mut t: halfword = 0; // §1466
        let mut c: eight_bits = 0; // §1466
        b = self.cur_chr;
        p = self.scan_toks(false, false);
        p = self.mem[(self.def_ref) as usize].hh().rh();
        while (p != 0i32) {
            {
                // §1467
                t = self.mem[(p) as usize].hh().lh();
                if (t < 4352i32) {
                    {
                        c = (t % 256i32);
                        if (self.eqtb[(((b).wrapping_add(c)) - 1) as usize].hh().rh() != 0i32) {
                            { let __v1842 = ((t).wrapping_sub(c)).wrapping_add(self.eqtb[(((b).wrapping_add(c)) - 1) as usize].hh().rh()); self.mem[(p) as usize].set_hh_lh(__v1842); }
                        }
                    }
                }
                // §1466
                p = self.mem[(p) as usize].hh().rh();
            }
        }
        self.begin_token_list(self.mem[(self.def_ref) as usize].hh().rh(), 3i32);
        {
            { let __ix1843 = self.def_ref; let __v1844 = self.avail; self.mem[(__ix1843) as usize].set_hh_rh(__v1844); }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
    }

    /// @<Declare act...
    // §1471
    pub fn show_whatever(&mut self) {
        let mut p: halfword = 0; // §1471
        let mut t: small_number = 0; // §1471
        let mut m: i32 = 0; // §1471
        let mut l: i32 = 0; // §1471
        let mut n: i32 = 0; // §1471
        'l_common_ending_f: {
            match self.cur_chr {
                3 => {
                    {
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1471
                        self.begin_diagnostic();
                        self.show_activities();
                    }
                }
                1 => {
                    // §1474
                    {
                        self.scan_register_num();
                        if (self.cur_val < 256i32) {
                            p = self.eqtb[(((627433i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                        } else {
                            {
                                self.find_sa_element(4i32, self.cur_val, false);
                                if (self.cur_ptr == 0i32) {
                                    p = 0i32;
                                } else {
                                    p = self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().rh();
                                }
                            }
                        }
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1474
                        self.begin_diagnostic();
                        self.print_nl(1680i32);
                        self.print_int(((self.cur_val) as i64));
                        self.print_char(61i32);
                        if (p == 0i32) {
                            self.print(426i32);
                        } else {
                            self.show_box(p);
                        }
                    }
                }
                0 => {
                    // §1472
                    {
                        self.get_token();
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1472
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(1676i32);
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
                4 => {
                    // §1677
                    {
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1677
                        self.begin_diagnostic();
                        self.show_save_groups();
                    }
                }
                6 => {
                    // §1691
                    {
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1691
                        self.begin_diagnostic();
                        self.print_nl(348i32);
                        self.print_ln();
                        if (self.cond_ptr == 0i32) {
                            {
                                self.print_nl(376i32);
                                self.print(2009i32);
                            }
                        } else {
                            {
                                p = self.cond_ptr;
                                n = 0i32;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    p = self.mem[(p) as usize].hh().rh();
                                    if (p == 0i32) { break; }
                                }
                                p = self.cond_ptr;
                                t = self.cur_if;
                                l = self.if_line;
                                m = self.if_limit;
                                loop {
                                    self.print_nl(2010i32);
                                    self.print_int(((n) as i64));
                                    self.print(650i32);
                                    self.print_cmd_chr(108i32, t);
                                    if (m == 2i32) {
                                        self.print_esc(932i32);
                                    }
                                    if (l != 0i32) {
                                        {
                                            self.print(2008i32);
                                            self.print_int(((l) as i64));
                                        }
                                    }
                                    n = (n).wrapping_sub(1i32);
                                    t = self.mem[(p) as usize].hh().b1();
                                    l = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                    m = self.mem[(p) as usize].hh().b0();
                                    p = self.mem[(p) as usize].hh().rh();
                                    if (p == 0i32) { break; }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // §1475
                    {
                        p = self.the_toks();
                        // §1881
                        if (((self.eqtb[((629078i32) - 1) as usize].int() >= 0i32) && (self.eqtb[((629078i32) - 1) as usize].int() < 16i32)) && self.write_open[(self.eqtb[((629078i32) - 1) as usize].int()) as usize]) {
                            self.selector = self.eqtb[((629078i32) - 1) as usize].int();
                        }
                        // §1475
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(1676i32);
                        self.token_show(4999996i32);
                        self.flush_list(self.mem[(4999996i32) as usize].hh().rh());
                        break 'l_common_ending_f;
                    }
                }
            }
            // §1476
            self.end_diagnostic(true);
            {
                if (self.interaction == 3i32) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(264i32);
                }
                self.print(1681i32);
            }
            if (self.selector == 19i32) {
                if (self.eqtb[((629047i32) - 1) as usize].int() <= 0i32) {
                    {
                        self.selector = 17i32;
                        self.print(1682i32);
                        self.selector = 19i32;
                    }
                }
            }
        }
        // §1471
        if (self.selector < 16i32) {
            {
                self.print_ln();
                // §75
                if (self.interaction == 0i32) {
                    self.selector = 16i32;
                } else {
                    self.selector = 17i32;
                }
                // §1471
                if self.log_opened {
                    self.selector = (self.selector).wrapping_add(2i32);
                }
            }
        } else {
            {
                if (self.interaction < 3i32) {
                    {
                        self.help_ptr = 0i32;
                        self.error_count = (self.error_count).wrapping_sub(1i32);
                    }
                } else {
                    if (self.eqtb[((629047i32) - 1) as usize].int() > 0i32) {
                        {
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1671i32;
                                self.help_line[(1i32) as usize] = 1672i32;
                                self.help_line[(0i32) as usize] = 1673i32;
                            }
                        }
                    } else {
                        {
                            {
                                self.help_ptr = 5i32;
                                self.help_line[(4i32) as usize] = 1671i32;
                                self.help_line[(3i32) as usize] = 1672i32;
                                self.help_line[(2i32) as usize] = 1673i32;
                                self.help_line[(1i32) as usize] = 1674i32;
                                self.help_line[(0i32) as usize] = 1675i32;
                            }
                        }
                    }
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1480
    pub fn store_fmt_file(&mut self) {
        let mut j: i32 = 0; // §1480
        let mut k: i32 = 0; // §1480
        let mut l: i32 = 0; // §1480
        let mut p: halfword = 0; // §1480
        let mut q: halfword = 0; // §1480
        let mut x: i32 = 0; // §1480
        let mut w: four_quarters = four_quarters::default(); // §1480
        // §1482
        if (self.save_ptr != 0i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(1684i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 1685i32;
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
        // §1508
        self.selector = 21i32;
        self.print(1700i32);
        self.print(self.job_name);
        self.print_char(32i32);
        self.print_int(((self.eqtb[((629041i32) - 1) as usize].int()) as i64));
        self.print_char(46i32);
        self.print_int(((self.eqtb[((629040i32) - 1) as usize].int()) as i64));
        self.print_char(46i32);
        self.print_int(((self.eqtb[((629039i32) - 1) as usize].int()) as i64));
        self.print_char(41i32);
        if (self.interaction == 0i32) {
            self.selector = 18i32;
        } else {
            self.selector = 19i32;
        }
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.format_ident = self.make_string();
        self.pack_job_name(942i32);
        while (!{ let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_open_out(&mut __f0); self.fmt_file = __f0; __r }) {
            self.prompt_file_name(1701i32, 942i32);
        }
        self.print_nl(1702i32);
        { let __a1845_0 = { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_make_name_string(&mut __f0); self.fmt_file = __f0; __r }; self.slow_print(__a1845_0) };
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
        }
        self.print_nl(348i32);
        self.slow_print(self.format_ident);
        // §1485
        {
            self.fmt_file.buf.set_int(399034618i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1890
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1846 = self.xord[(k) as usize]; self.fmt_file.buf.set_int(__v1846); }
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
                    { let __v1847 = ((self.xchr[(k) as usize]) as i32); self.fmt_file.buf.set_int(__v1847); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                if self.xprn[(k) as usize] {
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
                k = k.wrapping_add(1);
            }
        }
        // §1654
        {
            { let __v1848 = self.eTeX_mode; self.fmt_file.buf.set_int(__v1848); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = (0i32).wrapping_neg();
            j = 0i32;
            while j <= __for_end_2 {
                self.eqtb[(((629126i32).wrapping_add(j)) - 1) as usize].set_int(0i32);
                j = j.wrapping_add(1);
            }
        }
        // §1758
        while (self.pseudo_files != 0i32) {
            self.pseudo_close();
        }
        // §1485
        {
            self.fmt_file.buf.set_int(0i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(4999999i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(629929i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(522749i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(8191i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1487
        {
            { let __v1849 = self.pool_ptr; self.fmt_file.buf.set_int(__v1849); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1850 = self.str_ptr; self.fmt_file.buf.set_int(__v1850); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.str_ptr;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1851 = self.str_start[(k) as usize]; self.fmt_file.buf.set_int(__v1851); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        k = 0i32;
        while ((k).wrapping_add(4i32) < self.pool_ptr) {
            {
                { let __v1852 = (self.str_pool[(k) as usize]).wrapping_add(0i32); w.set_b0(__v1852); }
                { let __v1853 = (self.str_pool[((k).wrapping_add(1i32)) as usize]).wrapping_add(0i32); w.set_b1(__v1853); }
                { let __v1854 = (self.str_pool[((k).wrapping_add(2i32)) as usize]).wrapping_add(0i32); w.set_b2(__v1854); }
                { let __v1855 = (self.str_pool[((k).wrapping_add(3i32)) as usize]).wrapping_add(0i32); w.set_b3(__v1855); }
                {
                    self.fmt_file.buf.set_qqqq(w);
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = (k).wrapping_add(4i32);
            }
        }
        k = (self.pool_ptr).wrapping_sub(4i32);
        { let __v1856 = (self.str_pool[(k) as usize]).wrapping_add(0i32); w.set_b0(__v1856); }
        { let __v1857 = (self.str_pool[((k).wrapping_add(1i32)) as usize]).wrapping_add(0i32); w.set_b1(__v1857); }
        { let __v1858 = (self.str_pool[((k).wrapping_add(2i32)) as usize]).wrapping_add(0i32); w.set_b2(__v1858); }
        { let __v1859 = (self.str_pool[((k).wrapping_add(3i32)) as usize]).wrapping_add(0i32); w.set_b3(__v1859); }
        {
            self.fmt_file.buf.set_qqqq(w);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((self.str_ptr) as i64));
        self.print(1686i32);
        self.print_int(((self.pool_ptr) as i64));
        // §1489
        self.sort_avail();
        self.var_used = 0i32;
        {
            { let __v1860 = self.lo_mem_max; self.fmt_file.buf.set_int(__v1860); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1861 = self.rover; self.fmt_file.buf.set_int(__v1861); }
            crate::system::put_word(&mut self.fmt_file);
        }
        if (self.eTeX_mode == 1i32) {
            {
                let __for_end_3 = 5i32;
                k = 0i32;
                while k <= __for_end_3 {
                    {
                        { let __v1862 = self.sa_root[(k) as usize]; self.fmt_file.buf.set_int(__v1862); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
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
            { let __v1863 = self.hi_mem_min; self.fmt_file.buf.set_int(__v1863); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1864 = self.avail; self.fmt_file.buf.set_int(__v1864); }
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
            { let __v1865 = self.var_used; self.fmt_file.buf.set_int(__v1865); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1866 = self.dyn_used; self.fmt_file.buf.set_int(__v1866); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((x) as i64));
        self.print(1687i32);
        self.print_int(((self.var_used) as i64));
        self.print_char(38i32);
        self.print_int(((self.dyn_used) as i64));
        // §1493
        k = 1i32;
        loop {
            'l_done1_f: {
                'l_found1_f: {
                    j = k;
                    while (j < 629017i32) {
                        {
                            if (((self.eqtb[((j) - 1) as usize].hh().rh() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().rh()) && (self.eqtb[((j) - 1) as usize].hh().b0() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b0())) && (self.eqtb[((j) - 1) as usize].hh().b1() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].hh().b1())) {
                                break 'l_found1_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 629018i32;
                    break 'l_done1_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < 629017i32) {
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
            if (k == 629018i32) { break; }
        }
        // §1494
        loop {
            'l_done2_f: {
                'l_found2_f: {
                    j = k;
                    while (j < 629929i32) {
                        {
                            if (self.eqtb[((j) - 1) as usize].int() == self.eqtb[(((j).wrapping_add(1i32)) - 1) as usize].int()) {
                                break 'l_found2_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 629930i32;
                    break 'l_done2_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < 629929i32) {
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
            if (k > 629929i32) { break; }
        }
        // §1491
        {
            { let __v1867 = self.par_loc; self.fmt_file.buf.set_int(__v1867); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1868 = self.write_loc; self.fmt_file.buf.set_int(__v1868); }
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1496
        {
            let __for_end_2 = 2100i32;
            p = 0i32;
            while p <= __for_end_2 {
                {
                    { let __v1869 = self.prim[(p) as usize]; self.fmt_file.buf.set_hh(__v1869); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        {
            { let __v1870 = self.hash_used; self.fmt_file.buf.set_int(__v1870); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.cs_count = (615513i32).wrapping_sub(self.hash_used);
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
                            { let __v1871 = self.hash[((p) - 514) as usize]; self.fmt_file.buf.set_hh(__v1871); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        self.cs_count = (self.cs_count).wrapping_add(1i32);
                    }
                }
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 626626i32;
            p = (self.hash_used).wrapping_add(1i32);
            while p <= __for_end_2 {
                {
                    { let __v1872 = self.hash[((p) - 514) as usize]; self.fmt_file.buf.set_hh(__v1872); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        {
            { let __v1873 = self.cs_count; self.fmt_file.buf.set_int(__v1873); }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(((self.cs_count) as i64));
        self.print(1688i32);
        // §1498
        {
            { let __v1874 = self.fmem_ptr; self.fmt_file.buf.set_int(__v1874); }
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
            { let __v1875 = self.font_ptr; self.fmt_file.buf.set_int(__v1875); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.font_ptr;
            k = 0i32;
            while k <= __for_end_2 {
                // §1500
                {
                    {
                        { let __v1876 = self.font_check[(k) as usize]; self.fmt_file.buf.set_qqqq(__v1876); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1877 = self.font_size[(k) as usize]; self.fmt_file.buf.set_int(__v1877); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1878 = self.font_dsize[(k) as usize]; self.fmt_file.buf.set_int(__v1878); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1879 = self.font_params[(k) as usize]; self.fmt_file.buf.set_int(__v1879); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1880 = self.hyphen_char[(k) as usize]; self.fmt_file.buf.set_int(__v1880); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1881 = self.skew_char[(k) as usize]; self.fmt_file.buf.set_int(__v1881); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1882 = self.font_name[(k) as usize]; self.fmt_file.buf.set_int(__v1882); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1883 = self.font_area[(k) as usize]; self.fmt_file.buf.set_int(__v1883); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1884 = self.font_bc[(k) as usize]; self.fmt_file.buf.set_int(__v1884); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1885 = self.font_ec[(k) as usize]; self.fmt_file.buf.set_int(__v1885); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1886 = self.char_base[(k) as usize]; self.fmt_file.buf.set_int(__v1886); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1887 = self.width_base[(k) as usize]; self.fmt_file.buf.set_int(__v1887); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1888 = self.height_base[(k) as usize]; self.fmt_file.buf.set_int(__v1888); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1889 = self.depth_base[(k) as usize]; self.fmt_file.buf.set_int(__v1889); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1890 = self.italic_base[(k) as usize]; self.fmt_file.buf.set_int(__v1890); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1891 = self.lig_kern_base[(k) as usize]; self.fmt_file.buf.set_int(__v1891); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1892 = self.kern_base[(k) as usize]; self.fmt_file.buf.set_int(__v1892); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1893 = self.exten_base[(k) as usize]; self.fmt_file.buf.set_int(__v1893); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1894 = self.param_base[(k) as usize]; self.fmt_file.buf.set_int(__v1894); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1895 = self.font_glue[(k) as usize]; self.fmt_file.buf.set_int(__v1895); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1896 = self.bchar_label[(k) as usize]; self.fmt_file.buf.set_int(__v1896); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1897 = self.font_bchar[(k) as usize]; self.fmt_file.buf.set_int(__v1897); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1898 = self.font_false_bchar[(k) as usize]; self.fmt_file.buf.set_int(__v1898); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    self.print_nl(1691i32);
                    self.print_esc(self.hash[(((617626i32).wrapping_add(k)) - 514) as usize].rh());
                    self.print_char(61i32);
                    self.print_file_name(self.font_name[(k) as usize], self.font_area[(k) as usize], 348i32);
                    if (self.font_size[(k) as usize] != self.font_dsize[(k) as usize]) {
                        {
                            self.print(895i32);
                            self.print_scaled(self.font_size[(k) as usize]);
                            self.print(314i32);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        // §1498
        self.print_ln();
        self.print_int((((self.fmem_ptr).wrapping_sub(7i32)) as i64));
        self.print(1689i32);
        self.print_int((((self.font_ptr).wrapping_sub(0i32)) as i64));
        self.print(1690i32);
        if (self.font_ptr != 1i32) {
            self.print_char(115i32);
        }
        // §1502
        {
            { let __v1899 = self.hyph_count; self.fmt_file.buf.set_int(__v1899); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = 8191i32;
            k = 0i32;
            while k <= __for_end_2 {
                if (self.hyph_word[(k) as usize] != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1900 = self.hyph_word[(k) as usize]; self.fmt_file.buf.set_int(__v1900); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1901 = self.hyph_list[(k) as usize]; self.fmt_file.buf.set_int(__v1901); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.print_int(((self.hyph_count) as i64));
        self.print(1692i32);
        if (self.hyph_count != 1i32) {
            self.print_char(115i32);
        }
        if self.trie_not_ready {
            self.init_trie();
        }
        {
            { let __v1902 = self.trie_max; self.fmt_file.buf.set_int(__v1902); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1903 = self.hyph_start; self.fmt_file.buf.set_int(__v1903); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_max;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    { let __v1904 = self.trie[(k) as usize]; self.fmt_file.buf.set_hh(__v1904); }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            { let __v1905 = self.trie_op_ptr; self.fmt_file.buf.set_int(__v1905); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    {
                        { let __v1906 = self.hyf_distance[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1906); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1907 = self.hyf_num[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1907); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        { let __v1908 = self.hyf_next[((k) - 1) as usize]; self.fmt_file.buf.set_int(__v1908); }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_nl(1693i32);
        self.print_int(((self.trie_max) as i64));
        self.print(1694i32);
        self.print_int(((self.trie_op_ptr) as i64));
        self.print(1695i32);
        if (self.trie_op_ptr != 1i32) {
            self.print_char(115i32);
        }
        self.print(1696i32);
        self.print_int(((trie_op_size) as i64));
        {
            let __for_end_2 = 0i32;
            k = 255i32;
            while k >= __for_end_2 {
                if (self.trie_used[(k) as usize] > 0i32) {
                    {
                        self.print_nl(957i32);
                        self.print_int((((self.trie_used[(k) as usize]).wrapping_sub(0i32)) as i64));
                        self.print(1697i32);
                        self.print_int(((k) as i64));
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1909 = (self.trie_used[(k) as usize]).wrapping_sub(0i32); self.fmt_file.buf.set_int(__v1909); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_sub(1);
            }
        }
        // §1504
        {
            self.dumpimagemeta();
            {
                { let __v1910 = self.pdf_mem_size; self.fmt_file.buf.set_int(__v1910); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1911 = self.pdf_mem_ptr; self.fmt_file.buf.set_int(__v1911); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                let __for_end_3 = (self.pdf_mem_ptr).wrapping_sub(1i32);
                k = 1i32;
                while k <= __for_end_3 {
                    {
                        {
                            { let __v1912 = self.pdf_mem[(k) as usize]; self.fmt_file.buf.set_int(__v1912); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_ln();
            self.print_int((((self.pdf_mem_ptr).wrapping_sub(1i32)) as i64));
            self.print(1698i32);
            {
                { let __v1913 = self.obj_tab_size; self.fmt_file.buf.set_int(__v1913); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1914 = self.obj_ptr; self.fmt_file.buf.set_int(__v1914); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1915 = self.sys_obj_ptr; self.fmt_file.buf.set_int(__v1915); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                let __for_end_3 = self.sys_obj_ptr;
                k = 1i32;
                while k <= __for_end_3 {
                    {
                        {
                            { let __v1916 = self.obj_tab[(k) as usize].int0; self.fmt_file.buf.set_int(__v1916); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1917 = self.obj_tab[(k) as usize].int1; self.fmt_file.buf.set_int(__v1917); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1918 = self.obj_tab[(k) as usize].int3; self.fmt_file.buf.set_int(__v1918); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            { let __v1919 = self.obj_tab[(k) as usize].int4; self.fmt_file.buf.set_int(__v1919); }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_ln();
            self.print_int(((self.sys_obj_ptr) as i64));
            self.print(1699i32);
            {
                { let __v1920 = self.pdf_obj_count; self.fmt_file.buf.set_int(__v1920); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1921 = self.pdf_xform_count; self.fmt_file.buf.set_int(__v1921); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1922 = self.pdf_ximage_count; self.fmt_file.buf.set_int(__v1922); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1923 = self.head_tab[((7i32) - 1) as usize]; self.fmt_file.buf.set_int(__v1923); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1924 = self.head_tab[((8i32) - 1) as usize]; self.fmt_file.buf.set_int(__v1924); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1925 = self.head_tab[((9i32) - 1) as usize]; self.fmt_file.buf.set_int(__v1925); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1926 = self.pdf_last_obj; self.fmt_file.buf.set_int(__v1926); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1927 = self.pdf_last_xform; self.fmt_file.buf.set_int(__v1927); }
                crate::system::put_word(&mut self.fmt_file);
            }
            {
                { let __v1928 = self.pdf_last_ximage; self.fmt_file.buf.set_int(__v1928); }
                crate::system::put_word(&mut self.fmt_file);
            }
            self.dumptounicode();
        }
        // §1506
        {
            { let __v1929 = self.interaction; self.fmt_file.buf.set_int(__v1929); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            { let __v1930 = self.format_ident; self.fmt_file.buf.set_int(__v1930); }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(69069i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.eqtb[((629049i32) - 1) as usize].set_int(0i32);
        // §1509
        { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
    }

    /// Here is a subroutine that creates a whatsit node having a given `subtype`
    /// and a given number of words. It initializes only the first word of the whatsit,
    /// and appends it to the current list.
    /// @<Declare procedures needed in `do_extension`
    // §1529
    pub fn new_whatsit(&mut self, mut s: small_number, mut w: small_number) {
        let mut p: halfword = 0; // §1529
        p = self.get_node(w);
        self.mem[(p) as usize].set_hh_b0(8i32);
        self.mem[(p) as usize].set_hh_b1(s);
        { let __ix1931 = self.cur_list.tail_field; self.mem[(__ix1931) as usize].set_hh_rh(p); }
        self.cur_list.tail_field = p;
    }

    /// The next subroutine uses `cur_chr` to decide what sort of whatsit is
    /// involved, and also inserts a `write_stream` number.
    /// @<Declare procedures needed in `do_ext...
    // §1530
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
                    if ((self.cur_val > 15i32) && (self.cur_val != 18i32)) {
                        self.cur_val = 16i32;
                    }
                }
            }
        }
        { let __ix1932 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1933 = self.cur_val; self.mem[(__ix1932) as usize].set_hh_lh(__v1933); }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1537
    pub fn check_pdfoutput(&mut self, mut s: str_number, mut is_error: bool) {
        if (self.eqtb[((629079i32) - 1) as usize].int() <= 0i32) {
            {
                if is_error {
                    self.pdf_error(s, 1763i32);
                } else {
                    self.pdf_warning(s, 1764i32, true, true);
                }
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1537
    pub fn scan_pdf_ext_toks(&mut self) {
        {
            if (self.scan_toks(false, true) != 0i32) {
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1537
    pub fn scan_pdf_ext_late_toks(&mut self) {
        {
            if (self.scan_toks(false, false) != 0i32) {
            }
        }
    }

    /// We have to check whether \.{\\pdfoutput} is set for using \pdfTeX{}
    /// extensions.
    /// @<Declare procedures needed in `do_ext...
    // §1537
    pub fn compare_strings(&mut self) {
        let mut s1: str_number = 0; // §1537
        let mut s2: str_number = 0; // §1537
        let mut i1: pool_pointer = 0; // §1537
        let mut i2: pool_pointer = 0; // §1537
        let mut j1: pool_pointer = 0; // §1537
        let mut j2: pool_pointer = 0; // §1537
        let mut save_cur_cs: halfword = 0; // §1537
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
            i1 = self.str_start[(s1) as usize];
            j1 = self.str_start[((s1).wrapping_add(1i32)) as usize];
            i2 = self.str_start[(s2) as usize];
            j2 = self.str_start[((s2).wrapping_add(1i32)) as usize];
            while ((i1 < j1) && (i2 < j2)) {
                {
                    if (self.str_pool[(i1) as usize] < self.str_pool[(i2) as usize]) {
                        {
                            self.cur_val = (1i32).wrapping_neg();
                            break 'l_done_f;
                        }
                    }
                    if (self.str_pool[(i1) as usize] > self.str_pool[(i2) as usize]) {
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
        self.cur_val_level = 0i32;
    }

    /// @<Declare procedures needed in `do_ext...
    // §1552
    pub fn scale_image(&mut self, mut n: i32) {
        let mut x: i32 = 0; // §1552
        let mut y: i32 = 0; // §1552
        let mut xr: i32 = 0; // §1552
        let mut yr: i32 = 0; // §1552
        let mut w: scaled = 0; // §1552
        let mut h: scaled = 0; // §1552
        let mut default_res: i32 = 0; // §1552
        let mut image: i32 = 0; // §1552
        image = self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(4i32)) as usize];
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
                self.pdf_warning(1762i32, 1793i32, true, true);
            }
        }
        if ((((x <= 0i32) || (y <= 0i32)) || (xr < 0i32)) || (yr < 0i32)) {
            self.pdf_error(1762i32, 1794i32);
        }
        if ((xr == 0i32) && (yr == 0i32)) {
            {
            }
        } else {
            if (((((x) as f64) / ((self.one_inch) as f64)) >= ((xr) as f64)) || ((((y) as f64) / ((self.one_inch) as f64)) >= ((yr) as f64))) {
                {
                    xr = 0i32;
                    yr = 0i32;
                    self.pdf_warning(1762i32, 1795i32, true, true);
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
                default_res = self.fix_int(self.eqtb[((629083i32) - 1) as usize].int(), 0i32, 65535i32);
                if ((default_res > 0i32) && ((xr == 0i32) || (yr == 0i32))) {
                    {
                        xr = default_res;
                        yr = default_res;
                    }
                }
                if ((self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] == (1073741824i32).wrapping_neg())) {
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
        if (((self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] == (1073741824i32).wrapping_neg())) && (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] == (1073741824i32).wrapping_neg())) {
            {
                self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize] = w;
                self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] = h;
                self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
            }
        } else {
            if (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize] == (1073741824i32).wrapping_neg()) {
                {
                    if (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] == (1073741824i32).wrapping_neg()) {
                        {
                            { let __ix1934 = (self.obj_tab[(n) as usize].int4).wrapping_add(0i32); let __v1935 = self.ext_xn_over_d(h, x, y); self.pdf_mem[(__ix1934) as usize] = __v1935; }
                            { let __ix1936 = (self.obj_tab[(n) as usize].int4).wrapping_add(1i32); let __v1937 = (h).wrapping_sub(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize]); self.pdf_mem[(__ix1936) as usize] = __v1937; }
                        }
                    } else {
                        if (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] == (1073741824i32).wrapping_neg()) {
                            {
                                { let __ix1938 = (self.obj_tab[(n) as usize].int4).wrapping_add(0i32); let __v1939 = self.ext_xn_over_d(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize], x, y); self.pdf_mem[(__ix1938) as usize] = __v1939; }
                                self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
                            }
                        } else {
                            {
                                { let __ix1940 = (self.obj_tab[(n) as usize].int4).wrapping_add(0i32); let __v1941 = self.ext_xn_over_d((self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize]).wrapping_add(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize]), x, y); self.pdf_mem[(__ix1940) as usize] = __v1941; }
                            }
                        }
                    }
                }
            } else {
                {
                    if ((self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] == (1073741824i32).wrapping_neg()) && (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] == (1073741824i32).wrapping_neg())) {
                        {
                            { let __ix1942 = (self.obj_tab[(n) as usize].int4).wrapping_add(1i32); let __v1943 = self.ext_xn_over_d(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize], y, x); self.pdf_mem[(__ix1942) as usize] = __v1943; }
                            self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
                        }
                    } else {
                        if (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(1i32)) as usize] == (1073741824i32).wrapping_neg()) {
                            {
                                { let __ix1944 = (self.obj_tab[(n) as usize].int4).wrapping_add(1i32); let __v1945 = (self.ext_xn_over_d(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(0i32)) as usize], y, x)).wrapping_sub(self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize]); self.pdf_mem[(__ix1944) as usize] = __v1945; }
                            }
                        } else {
                            if (self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] == (1073741824i32).wrapping_neg()) {
                                {
                                    self.pdf_mem[((self.obj_tab[(n) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
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
    // §1552
    pub fn scan_pdf_box_spec(&mut self) -> i32 {
        let mut scan_pdf_box_spec: i32 = 0;
        scan_pdf_box_spec = 0i32;
        if self.scan_keyword(1796i32) {
            scan_pdf_box_spec = self.pdf_box_spec_media;
        } else {
            if self.scan_keyword(1797i32) {
                scan_pdf_box_spec = self.pdf_box_spec_crop;
            } else {
                if self.scan_keyword(1798i32) {
                    scan_pdf_box_spec = self.pdf_box_spec_bleed;
                } else {
                    if self.scan_keyword(1799i32) {
                        scan_pdf_box_spec = self.pdf_box_spec_trim;
                    } else {
                        if self.scan_keyword(1800i32) {
                            scan_pdf_box_spec = self.pdf_box_spec_art;
                        }
                    }
                }
            }
        }
        scan_pdf_box_spec
    }

    /// @<Declare procedures needed in `do_ext...
    // §1552
    pub fn scan_alt_rule(&mut self) {
        if (self.alt_rule == 0i32) {
            self.alt_rule = self.new_rule();
        }
        { let __ix1946 = (self.alt_rule).wrapping_add(1i32); self.mem[(__ix1946) as usize].set_int((1073741824i32).wrapping_neg()); }
        { let __ix1947 = (self.alt_rule).wrapping_add(3i32); self.mem[(__ix1947) as usize].set_int((1073741824i32).wrapping_neg()); }
        { let __ix1948 = (self.alt_rule).wrapping_add(2i32); self.mem[(__ix1948) as usize].set_int((1073741824i32).wrapping_neg()); }
        'l_reswitch_b: loop {
            if self.scan_keyword(836i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix1949 = (self.alt_rule).wrapping_add(1i32); let __v1950 = self.cur_val; self.mem[(__ix1949) as usize].set_int(__v1950); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(837i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix1951 = (self.alt_rule).wrapping_add(3i32); let __v1952 = self.cur_val; self.mem[(__ix1951) as usize].set_int(__v1952); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(838i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __ix1953 = (self.alt_rule).wrapping_add(2i32); let __v1954 = self.cur_val; self.mem[(__ix1953) as usize].set_int(__v1954); }
                    continue 'l_reswitch_b;
                }
            }
            break 'l_reswitch_b;
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1552
    pub fn scan_image(&mut self) {
        let mut k: i32 = 0; // §1552
        let mut named: str_number = 0; // §1552
        let mut s: str_number = 0; // §1552
        let mut page: i32 = 0; // §1552
        let mut pagebox: i32 = 0; // §1552
        let mut colorspace: i32 = 0; // §1552
        self.pdf_ximage_count = (self.pdf_ximage_count).wrapping_add(1i32);
        self.pdf_create_obj(9i32, self.pdf_ximage_count);
        k = self.obj_ptr;
        self.obj_tab[(k) as usize].int4 = self.pdf_get_mem(5i32);
        self.scan_alt_rule();
        { let __ix1955 = (self.obj_tab[(k) as usize].int4).wrapping_add(0i32); let __v1956 = self.mem[((self.alt_rule).wrapping_add(1i32)) as usize].int(); self.pdf_mem[(__ix1955) as usize] = __v1956; }
        { let __ix1957 = (self.obj_tab[(k) as usize].int4).wrapping_add(1i32); let __v1958 = self.mem[((self.alt_rule).wrapping_add(3i32)) as usize].int(); self.pdf_mem[(__ix1957) as usize] = __v1958; }
        { let __ix1959 = (self.obj_tab[(k) as usize].int4).wrapping_add(2i32); let __v1960 = self.mem[((self.alt_rule).wrapping_add(2i32)) as usize].int(); self.pdf_mem[(__ix1959) as usize] = __v1960; }
        if self.scan_keyword(1786i32) {
            {
                self.scan_pdf_ext_toks();
                { let __ix1961 = (self.obj_tab[(k) as usize].int4).wrapping_add(3i32); let __v1962 = self.def_ref; self.pdf_mem[(__ix1961) as usize] = __v1962; }
            }
        } else {
            self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] = 0i32;
        }
        named = 0i32;
        if self.scan_keyword(1801i32) {
            {
                self.scan_pdf_ext_toks();
                named = self.tokens_to_string(self.def_ref);
                self.delete_token_ref(self.def_ref);
            }
        } else {
            if self.scan_keyword(889i32) {
                {
                    self.scan_int();
                    page = self.cur_val;
                }
            } else {
                page = 1i32;
            }
        }
        if self.scan_keyword(1802i32) {
            {
                self.scan_int();
                colorspace = self.cur_val;
            }
        } else {
            colorspace = 0i32;
        }
        pagebox = self.scan_pdf_box_spec();
        if (pagebox == 0i32) {
            pagebox = self.eqtb[((629091i32) - 1) as usize].int();
        }
        self.scan_pdf_ext_toks();
        s = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        if (self.eqtb[((629086i32) - 1) as usize].int() != 0i32) {
            {
                self.pdf_warning(1803i32, 1804i32, true, true);
                { let __v1963 = self.eqtb[((629086i32) - 1) as usize].int(); self.eqtb[((629090i32) - 1) as usize].set_int(__v1963); }
                self.eqtb[((629086i32) - 1) as usize].set_int(0i32);
                self.warn_pdfpagebox = false;
            }
        }
        if (self.eqtb[((629087i32) - 1) as usize].int() != 0i32) {
            {
                self.pdf_warning(1803i32, 1805i32, true, true);
                { let __v1964 = self.eqtb[((629087i32) - 1) as usize].int(); self.eqtb[((629092i32) - 1) as usize].set_int(__v1964); }
                self.eqtb[((629087i32) - 1) as usize].set_int(0i32);
            }
        }
        if (self.eqtb[((629090i32) - 1) as usize].int() > 0i32) {
            {
                if self.warn_pdfpagebox {
                    {
                        self.pdf_warning(1803i32, 1806i32, true, true);
                        self.warn_pdfpagebox = false;
                    }
                }
                pagebox = self.eqtb[((629090i32) - 1) as usize].int();
            }
        }
        if (pagebox == 0i32) {
            pagebox = self.pdf_box_spec_crop;
        }
        { let __ix1965 = (self.obj_tab[(k) as usize].int4).wrapping_add(4i32); let __v1966 = self.read_image(s, page, named, colorspace, pagebox, self.eqtb[((629088i32) - 1) as usize].int(), self.eqtb[((629089i32) - 1) as usize].int(), self.eqtb[((629092i32) - 1) as usize].int()); self.pdf_mem[(__ix1965) as usize] = __v1966; }
        if (named != 0i32) {
            self.flush_str(named);
        }
        self.flush_str(s);
        self.scale_image(k);
        self.pdf_last_ximage = k;
        self.pdf_last_ximage_pages = self.image_pages(self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize]);
        self.pdf_last_ximage_colordepth = self.image_colordepth(self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize]);
    }

    /// @<Declare procedures needed in `do_ext...
    // §1556
    pub fn scan_action(&mut self) -> halfword {
        let mut scan_action: halfword = 0;
        let mut p: i32 = 0; // §1556
        p = self.get_node(4i32);
        scan_action = p;
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
        self.mem[((p).wrapping_add(2i32)) as usize].set_hh_rh(0i32);
        if self.scan_keyword(1809i32) {
            self.mem[(p) as usize].set_hh_b0(3i32);
        } else {
            if self.scan_keyword(1810i32) {
                self.mem[(p) as usize].set_hh_b0(1i32);
            } else {
                if self.scan_keyword(1811i32) {
                    self.mem[(p) as usize].set_hh_b0(2i32);
                } else {
                    self.pdf_error(1762i32, 1812i32);
                }
            }
        }
        if (self.mem[(p) as usize].hh().b0() == 3i32) {
            {
                self.scan_pdf_ext_toks();
                { let __v1967 = self.def_ref; self.mem[((p).wrapping_add(2i32)) as usize].set_hh_lh(__v1967); }
                return scan_action;
            }
        }
        self.mem[(p) as usize].set_hh_b1(0i32);
        if self.scan_keyword(878i32) {
            {
                self.scan_pdf_ext_toks();
                { let __v1968 = self.def_ref; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(__v1968); }
            }
        }
        if self.scan_keyword(1813i32) {
            {
                if (self.mem[(p) as usize].hh().b0() != 1i32) {
                    self.pdf_error(1762i32, 1814i32);
                }
                if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() != 0i32) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __v1969 = (self.mem[(p) as usize].hh().b1()).wrapping_add(2i32); self.mem[(p) as usize].set_hh_b1(__v1969); }
                        { let __v1970 = self.def_ref; self.mem[((p).wrapping_add(3i32)) as usize].set_hh_rh(__v1970); }
                    }
                } else {
                    if self.scan_keyword(1815i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __v1971 = (self.mem[(p) as usize].hh().b1()).wrapping_add(2i32); self.mem[(p) as usize].set_hh_b1(__v1971); }
                            { let __v1972 = self.def_ref; self.mem[((p).wrapping_add(3i32)) as usize].set_hh_rh(__v1972); }
                        }
                    } else {
                        if self.scan_keyword(1194i32) {
                            {
                                self.scan_int();
                                if (self.cur_val <= 0i32) {
                                    self.pdf_error(1762i32, 1816i32);
                                }
                                { let __v1973 = self.cur_val; self.mem[((p).wrapping_add(3i32)) as usize].set_hh_rh(__v1973); }
                            }
                        } else {
                            self.pdf_error(1762i32, 1817i32);
                        }
                    }
                }
            }
        } else {
            self.mem[((p).wrapping_add(3i32)) as usize].set_hh_rh(0i32);
        }
        if self.scan_keyword(889i32) {
            {
                if (self.mem[(p) as usize].hh().b0() != 1i32) {
                    self.pdf_error(1762i32, 1818i32);
                }
                self.mem[(p) as usize].set_hh_b0(0i32);
                self.scan_int();
                if (self.cur_val <= 0i32) {
                    self.pdf_error(1762i32, 1819i32);
                }
                { let __v1974 = self.cur_val; self.mem[(p) as usize].set_hh_rh(__v1974); }
                self.scan_pdf_ext_toks();
                { let __v1975 = self.def_ref; self.mem[((p).wrapping_add(2i32)) as usize].set_hh_lh(__v1975); }
            }
        } else {
            if self.scan_keyword(1815i32) {
                {
                    self.scan_pdf_ext_toks();
                    { let __v1976 = (self.mem[(p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(p) as usize].set_hh_b1(__v1976); }
                    { let __v1977 = self.def_ref; self.mem[(p) as usize].set_hh_rh(__v1977); }
                }
            } else {
                if self.scan_keyword(1194i32) {
                    {
                        if ((self.mem[(p) as usize].hh().b0() == 1i32) && (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() != 0i32)) {
                            self.pdf_error(1762i32, 1820i32);
                        }
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.pdf_error(1762i32, 1816i32);
                        }
                        { let __v1978 = self.cur_val; self.mem[(p) as usize].set_hh_rh(__v1978); }
                    }
                } else {
                    self.pdf_error(1762i32, 1817i32);
                }
            }
        }
        if self.scan_keyword(1821i32) {
            {
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(1i32);
                // §469
                {
                    self.get_x_token();
                    if (self.cur_cmd != 10i32) {
                        self.back_input();
                    }
                }
            }
        } else {
            // §1556
            if self.scan_keyword(1822i32) {
                {
                    self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(2i32);
                    // §469
                    {
                        self.get_x_token();
                        if (self.cur_cmd != 10i32) {
                            self.back_input();
                        }
                    }
                }
            } else {
                // §1556
                self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
            }
        }
        if ((self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() > 0i32) && (((self.mem[(p) as usize].hh().b0() != 1i32) && (self.mem[(p) as usize].hh().b0() != 0i32)) || (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() == 0i32))) {
            self.pdf_error(1762i32, 1823i32);
        }
        scan_action
    }

    /// @<Declare procedures needed in `do_ext...
    // §1556
    pub fn new_annot_whatsit(&mut self, mut w: small_number, mut s: small_number) {
        self.new_whatsit(w, s);
        self.scan_alt_rule();
        { let __ix1979 = (self.cur_list.tail_field).wrapping_add(1i32); let __v1980 = self.mem[((self.alt_rule).wrapping_add(1i32)) as usize].int(); self.mem[(__ix1979) as usize].set_int(__v1980); }
        { let __ix1981 = (self.cur_list.tail_field).wrapping_add(2i32); let __v1982 = self.mem[((self.alt_rule).wrapping_add(3i32)) as usize].int(); self.mem[(__ix1981) as usize].set_int(__v1982); }
        { let __ix1983 = (self.cur_list.tail_field).wrapping_add(3i32); let __v1984 = self.mem[((self.alt_rule).wrapping_add(2i32)) as usize].int(); self.mem[(__ix1983) as usize].set_int(__v1984); }
        if (w == 16i32) {
            {
                if self.scan_keyword(1786i32) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __ix1985 = (self.cur_list.tail_field).wrapping_add(5i32); let __v1986 = self.def_ref; self.mem[(__ix1985) as usize].set_hh_lh(__v1986); }
                    }
                } else {
                    { let __ix1987 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix1987) as usize].set_hh_lh(0i32); }
                }
            }
        }
        if ((w == 20i32) || (w == 21i32)) {
            {
                if self.scan_keyword(1786i32) {
                    {
                        self.scan_pdf_ext_toks();
                        { let __ix1988 = (self.cur_list.tail_field).wrapping_add(6i32); let __v1989 = self.def_ref; self.mem[(__ix1988) as usize].set_hh_lh(__v1989); }
                    }
                } else {
                    { let __ix1990 = (self.cur_list.tail_field).wrapping_add(6i32); self.mem[(__ix1990) as usize].set_hh_lh(0i32); }
                }
            }
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1562
    pub fn outline_list_count(&mut self, mut p: halfword) -> i32 {
        let mut outline_list_count: i32 = 0;
        let mut k: i32 = 0; // §1562
        k = 1i32;
        while (self.pdf_mem[((self.obj_tab[(p) as usize].int4).wrapping_add(2i32)) as usize] != 0i32) {
            {
                k = (k).wrapping_add(1i32);
                p = self.pdf_mem[((self.obj_tab[(p) as usize].int4).wrapping_add(2i32)) as usize];
            }
        }
        outline_list_count = k;
        outline_list_count
    }

    /// @<Declare procedures needed in `do_ext...
    // §1566
    pub fn scan_thread_id(&mut self) {
        if self.scan_keyword(1194i32) {
            {
                self.scan_int();
                if (self.cur_val <= 0i32) {
                    self.pdf_error(1762i32, 1816i32);
                }
                if (self.cur_val > 268435455i32) {
                    self.pdf_error(1762i32, 1031i32);
                }
                { let __ix1991 = (self.cur_list.tail_field).wrapping_add(5i32); let __v1992 = self.cur_val; self.mem[(__ix1991) as usize].set_hh_rh(__v1992); }
                { let __ix1993 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix1993) as usize].set_hh_b1(0i32); }
            }
        } else {
            if self.scan_keyword(1815i32) {
                {
                    self.scan_pdf_ext_toks();
                    { let __ix1994 = (self.cur_list.tail_field).wrapping_add(5i32); let __v1995 = self.def_ref; self.mem[(__ix1994) as usize].set_hh_rh(__v1995); }
                    { let __ix1996 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix1996) as usize].set_hh_b1(1i32); }
                }
            } else {
                self.pdf_error(1762i32, 1817i32);
            }
        }
    }

    /// @<Declare procedures needed in `do_ext...
    // §1573
    pub fn new_snap_node(&mut self, mut s: small_number) -> halfword {
        let mut new_snap_node: halfword = 0;
        let mut p: halfword = 0; // §1573
        self.scan_glue(2i32);
        if (self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int() < 0i32) {
            self.pdf_error(1762i32, 1851i32);
        }
        p = self.get_node(3i32);
        self.mem[(p) as usize].set_hh_b0(8i32);
        self.mem[(p) as usize].set_hh_b1(s);
        self.mem[(p) as usize].set_hh_rh(0i32);
        { let __v1997 = self.cur_val; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(__v1997); }
        self.mem[((p).wrapping_add(2i32)) as usize].set_int(0i32);
        new_snap_node = p;
        new_snap_node
    }

    /// To implement primitives as \.{\\pdfinfo}, \.{\\pdfcatalog} or
    /// \.{\\pdfnames} we need to concatenate tokens lists.
    /// @<Declare procedures needed in `do_ext...
    // §1577
    pub fn concat_tokens(&mut self, mut q: halfword, mut r: halfword) -> halfword {
        let mut concat_tokens: halfword = 0;
        let mut p: halfword = 0; // §1577
        if (q == 0i32) {
            {
                concat_tokens = r;
                return concat_tokens;
            }
        }
        p = q;
        while (self.mem[(p) as usize].hh().rh() != 0i32) {
            p = self.mem[(p) as usize].hh().rh();
        }
        { let __v1998 = self.mem[(r) as usize].hh().rh(); self.mem[(p) as usize].set_hh_rh(__v1998); }
        {
            { let __v1999 = self.avail; self.mem[(r) as usize].set_hh_rh(__v1999); }
            self.avail = r;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        concat_tokens = q;
        concat_tokens
    }

    /// The following subroutines are about PDF-specific font issues.
    /// @<Declare procedures needed in `do_ext...
    // §1587
    pub fn pdf_include_chars(&mut self) {
        let mut s: str_number = 0; // §1587
        let mut k: pool_pointer = 0; // §1587
        let mut f: internal_font_number = 0; // §1587
        self.scan_font_ident();
        f = self.cur_val;
        if (f == 0i32) {
            self.pdf_error(594i32, 873i32);
        }
        self.pdf_check_vf_cur_val();
        if (!self.font_used[(f) as usize]) {
            self.pdf_init_font(f);
        }
        self.scan_pdf_ext_toks();
        s = self.tokens_to_string(self.def_ref);
        self.delete_token_ref(self.def_ref);
        k = self.str_start[(s) as usize];
        while (k < self.str_start[((s).wrapping_add(1i32)) as usize]) {
            {
                self.pdf_mark_char(f, self.str_pool[(k) as usize]);
                k = (k).wrapping_add(1i32);
            }
        }
        self.flush_str(s);
    }

    /// The following subroutines are about PDF-specific font issues.
    /// @<Declare procedures needed in `do_ext...
    // §1587
    pub fn glyph_to_unicode(&mut self) {
        let mut s1: str_number = 0; // §1587
        let mut s2: str_number = 0; // §1587
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
    // §1600
    pub fn thread_title(&mut self, mut thread: i32) {
        self.pdf_print(1871i32);
        if (self.obj_tab[(thread) as usize].int0 < 0i32) {
            self.pdf_print((self.obj_tab[(thread) as usize].int0).wrapping_neg());
        } else {
            self.pdf_print_int(((self.obj_tab[(thread) as usize].int0) as i64));
        }
        {
            self.pdf_print(41i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1004i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// The following function are needed for outputting article thread.
    /// @<Declare procedures needed in `do_ext...
    // §1600
    pub fn pdf_fix_thread(&mut self, mut thread: i32) {
        let mut a: halfword = 0; // §1600
        self.pdf_warning(1811i32, 1872i32, true, false);
        if (self.obj_tab[(thread) as usize].int0 < 0i32) {
            {
                self.print(1193i32);
                self.print((self.obj_tab[(thread) as usize].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1194i32);
                self.print_int(((self.obj_tab[(thread) as usize].int0) as i64));
            }
        }
        self.print(1195i32);
        self.print_ln();
        self.print_ln();
        self.pdf_new_dict(0i32, 0i32, 0i32);
        a = self.obj_ptr;
        self.pdf_indirect_ln(84i32, thread);
        self.pdf_indirect_ln(86i32, a);
        self.pdf_indirect_ln(78i32, a);
        self.pdf_indirect_ln(80i32, self.head_tab[((1i32) - 1) as usize]);
        self.pdf_print(1873i32);
        self.pdf_print_bp(self.eqtb[((629663i32) - 1) as usize].int());
        {
            {
                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                    self.pdf_os_get_os_buf(1i32);
                } else {
                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                        self.overflow(1004i32, pdf_op_buf_size);
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
        self.pdf_print_bp(self.eqtb[((629664i32) - 1) as usize].int());
        {
            self.pdf_print(93i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1004i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_end_dict();
        self.pdf_begin_dict(thread, 1i32);
        {
            self.pdf_print(1874i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1004i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.thread_title(thread);
        {
            self.pdf_print(1021i32);
            {
                {
                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf(1i32);
                    } else {
                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                            self.overflow(1004i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                {
                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_indirect_ln(70i32, a);
        self.pdf_end_dict();
    }

    /// The following function are needed for outputting article thread.
    /// @<Declare procedures needed in `do_ext...
    // §1600
    pub fn out_thread(&mut self, mut thread: i32) {
        let mut a: halfword = 0; // §1600
        let mut b: halfword = 0; // §1600
        let mut last_attr: i32 = 0; // §1600
        if (self.obj_tab[(thread) as usize].int4 == 0i32) {
            {
                self.pdf_fix_thread(thread);
                return;
            }
        }
        self.pdf_begin_dict(thread, 1i32);
        a = self.obj_tab[(thread) as usize].int4;
        b = a;
        last_attr = 0i32;
        loop {
            if (self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(4i32)) as usize] != 0i32) {
                last_attr = self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(4i32)) as usize];
            }
            a = self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(2i32)) as usize];
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
                                self.overflow(1004i32, pdf_op_buf_size);
                            } else {
                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                    self.pdf_flush();
                                }
                            }
                        }
                    }
                    {
                        self.pdf_buf_set(self.pdf_ptr, 10i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        } else {
            {
                {
                    self.pdf_print(1874i32);
                    {
                        {
                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_os_get_os_buf(1i32);
                            } else {
                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                    self.overflow(1004i32, pdf_op_buf_size);
                                } else {
                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                        self.pdf_flush();
                                    }
                                }
                            }
                        }
                        {
                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
                self.thread_title(thread);
                {
                    self.pdf_print(1021i32);
                    {
                        {
                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_os_get_os_buf(1i32);
                            } else {
                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                    self.overflow(1004i32, pdf_op_buf_size);
                                } else {
                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                        self.pdf_flush();
                                    }
                                }
                            }
                        }
                        {
                            self.pdf_buf_set(self.pdf_ptr, 10i32);
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
            self.pdf_indirect_ln(86i32, self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(3i32)) as usize]);
            self.pdf_indirect_ln(78i32, self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(2i32)) as usize]);
            self.pdf_indirect_ln(80i32, self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(1i32)) as usize]);
            self.pdf_indirect_ln(82i32, self.pdf_mem[(self.obj_tab[(a) as usize].int4) as usize]);
            self.pdf_end_dict();
            a = self.pdf_mem[((self.obj_tab[(a) as usize].int4).wrapping_add(2i32)) as usize];
            if (a == b) { break; }
        }
    }

    /// @<Declare act...
    // §1528
    pub fn do_extension(&mut self) {
        let mut i: i32 = 0; // §1528
        let mut j: i32 = 0; // §1528
        let mut k: i32 = 0; // §1528
        let mut p: halfword = 0; // §1528
        let mut q: halfword = 0; // §1528
        let mut r: halfword = 0; // §1528
        match self.cur_chr {
            0 => {
                // §1531
                {
                    self.new_write_whatsit(3i32);
                    self.scan_optional_equals();
                    self.scan_file_name();
                    { let __ix2000 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2001 = self.cur_name; self.mem[(__ix2000) as usize].set_hh_rh(__v2001); }
                    { let __ix2002 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2003 = self.cur_area; self.mem[(__ix2002) as usize].set_hh_lh(__v2003); }
                    { let __ix2004 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2005 = self.cur_ext; self.mem[(__ix2004) as usize].set_hh_rh(__v2005); }
                }
            }
            1 => {
                // §1532
                {
                    k = self.cur_cs;
                    self.new_write_whatsit(2i32);
                    self.cur_cs = k;
                    p = self.scan_toks(false, false);
                    { let __ix2006 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2007 = self.def_ref; self.mem[(__ix2006) as usize].set_hh_rh(__v2007); }
                }
            }
            2 => {
                // §1533
                {
                    self.new_write_whatsit(2i32);
                    { let __ix2008 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2008) as usize].set_hh_rh(0i32); }
                }
            }
            3 => {
                // §1534
                {
                    if self.scan_keyword(1474i32) {
                        {
                            self.new_whatsit(4i32, 2i32);
                            { let __ix2009 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2009) as usize].set_hh_lh(0i32); }
                            p = self.scan_toks(false, false);
                            { let __ix2010 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2011 = self.def_ref; self.mem[(__ix2010) as usize].set_hh_rh(__v2011); }
                        }
                    } else {
                        {
                            self.new_whatsit(3i32, 2i32);
                            { let __ix2012 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2012) as usize].set_hh_lh(0i32); }
                            p = self.scan_toks(false, true);
                            { let __ix2013 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2014 = self.def_ref; self.mem[(__ix2013) as usize].set_hh_rh(__v2014); }
                        }
                    }
                }
            }
            5 => {
                // §1623
                {
                    self.get_x_token();
                    if (self.cur_cmd == 59i32) {
                        {
                            if (self.cur_chr <= 2i32) {
                                {
                                    p = self.cur_list.tail_field;
                                    self.do_extension();
                                    self.out_what(self.cur_list.tail_field);
                                    self.flush_node_list(self.cur_list.tail_field);
                                    self.cur_list.tail_field = p;
                                    self.mem[(p) as usize].set_hh_rh(0i32);
                                }
                            } else {
                                match self.cur_chr {
                                    9 => {
                                        {
                                            self.do_extension();
                                            if (self.obj_tab[(self.pdf_last_obj) as usize].int4 == 0i32) {
                                                self.pdf_error(1762i32, 1917i32);
                                            }
                                            self.pdf_write_obj(self.pdf_last_obj);
                                        }
                                    }
                                    11 => {
                                        {
                                            self.do_extension();
                                            self.pdf_cur_form = self.pdf_last_xform;
                                            self.pdf_ship_out(self.pdf_mem[((self.obj_tab[(self.pdf_last_xform) as usize].int4).wrapping_add(3i32)) as usize], false);
                                        }
                                    }
                                    13 => {
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
            6 => {
                // §1625
                if ((self.cur_list.mode_field).wrapping_abs() != 105i32) {
                    self.report_illegal_case();
                } else {
                    {
                        self.new_whatsit(5i32, 2i32);
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.cur_list.aux_field.set_hh_rh(0i32);
                        } else {
                            if (self.cur_val > 255i32) {
                                self.cur_list.aux_field.set_hh_rh(0i32);
                            } else {
                                { let __v2015 = self.cur_val; self.cur_list.aux_field.set_hh_rh(__v2015); }
                            }
                        }
                        { let __ix2016 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2017 = self.cur_list.aux_field.hh().rh(); self.mem[(__ix2016) as usize].set_hh_rh(__v2017); }
                        { let __ix2018 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2019 = self.norm_min(self.eqtb[((629069i32) - 1) as usize].int()); self.mem[(__ix2018) as usize].set_hh_b0(__v2019); }
                        { let __ix2020 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2021 = self.norm_min(self.eqtb[((629070i32) - 1) as usize].int()); self.mem[(__ix2020) as usize].set_hh_b1(__v2021); }
                    }
                }
            }
            15 => {
                // §1558
                {
                    self.check_pdfoutput(1824i32, true);
                    if self.scan_keyword(1783i32) {
                        {
                            self.pdf_last_annot = self.pdf_new_objnum();
                            // §469
                            {
                                self.get_x_token();
                                if (self.cur_cmd != 10i32) {
                                    self.back_input();
                                }
                            }
                        }
                    } else {
                        // §1558
                        {
                            if self.scan_keyword(1784i32) {
                                {
                                    self.scan_int();
                                    k = self.cur_val;
                                    if (((k <= 0i32) || (k > self.obj_ptr)) || (self.obj_tab[(k) as usize].int4 != 0i32)) {
                                        self.pdf_error(1762i32, 1825i32);
                                    }
                                }
                            } else {
                                k = self.pdf_new_objnum();
                            }
                            self.new_annot_whatsit(15i32, 7i32);
                            { let __ix2022 = (self.cur_list.tail_field).wrapping_add(6i32); self.mem[(__ix2022) as usize].set_int(k); }
                            self.scan_pdf_ext_toks();
                            { let __ix2023 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2024 = self.def_ref; self.mem[(__ix2023) as usize].set_hh_lh(__v2024); }
                            self.pdf_last_annot = k;
                        }
                    }
                }
            }
            25 => {
                // §1579
                {
                    self.check_pdfoutput(1855i32, false);
                    self.scan_pdf_ext_toks();
                    if (self.eqtb[((629079i32) - 1) as usize].int() > 0i32) {
                        self.pdf_catalog_toks = self.concat_tokens(self.pdf_catalog_toks, self.def_ref);
                    }
                    if self.scan_keyword(1856i32) {
                        {
                            if (self.pdf_catalog_openaction != 0i32) {
                                self.pdf_error(1762i32, 1857i32);
                            } else {
                                {
                                    p = self.scan_action();
                                    self.pdf_new_obj(0i32, 0i32, 1i32);
                                    if (self.eqtb[((629079i32) - 1) as usize].int() > 0i32) {
                                        self.pdf_catalog_openaction = self.obj_ptr;
                                    }
                                    self.write_action(p);
                                    self.pdf_end_obj();
                                    {
                                        if (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                                            {
                                                if (self.mem[(p) as usize].hh().b0() == 3i32) {
                                                    self.delete_token_ref(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                                } else {
                                                    {
                                                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() != 0i32) {
                                                            self.delete_token_ref(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                                        }
                                                        if (self.mem[(p) as usize].hh().b0() == 0i32) {
                                                            self.delete_token_ref(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                                        } else {
                                                            if ((((((self.mem[(p) as usize].hh().b1()) != 0) && ((1i32) != 0))) as i32) == 1i32) {
                                                                self.delete_token_ref(self.mem[(p) as usize].hh().rh());
                                                            }
                                                        }
                                                        if ((((((self.mem[(p) as usize].hh().b1()) != 0) && ((2i32) != 0))) as i32) == 2i32) {
                                                            self.delete_token_ref(self.mem[((p).wrapping_add(3i32)) as usize].hh().rh());
                                                        }
                                                    }
                                                }
                                                self.free_node(p, 4i32);
                                            }
                                        } else {
                                            { let __v2025 = (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh()).wrapping_sub(1i32); self.mem[((p).wrapping_add(2i32)) as usize].set_hh_rh(__v2025); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            19 => {
                // §1565
                {
                    self.check_pdfoutput(1833i32, true);
                    q = self.cur_list.tail_field;
                    self.new_whatsit(19i32, 7i32);
                    if self.scan_keyword(1813i32) {
                        {
                            self.scan_int();
                            if (self.cur_val <= 0i32) {
                                self.pdf_error(1762i32, 1834i32);
                            }
                            { let __ix2026 = (self.cur_list.tail_field).wrapping_add(6i32); let __v2027 = self.cur_val; self.mem[(__ix2026) as usize].set_hh_rh(__v2027); }
                            j = 6i32;
                        }
                    } else {
                        {
                            { let __ix2028 = (self.cur_list.tail_field).wrapping_add(6i32); self.mem[(__ix2028) as usize].set_hh_rh(0i32); }
                            j = 5i32;
                        }
                    }
                    if self.scan_keyword(1194i32) {
                        {
                            self.scan_int();
                            if (self.cur_val <= 0i32) {
                                self.pdf_error(1762i32, 1816i32);
                            }
                            if (self.cur_val > 268435455i32) {
                                self.pdf_error(1762i32, 1031i32);
                            }
                            { let __ix2029 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2030 = self.cur_val; self.mem[(__ix2029) as usize].set_hh_rh(__v2030); }
                            { let __ix2031 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2031) as usize].set_hh_b1(0i32); }
                        }
                    } else {
                        if self.scan_keyword(1815i32) {
                            {
                                self.scan_pdf_ext_toks();
                                { let __ix2032 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2033 = self.def_ref; self.mem[(__ix2032) as usize].set_hh_rh(__v2033); }
                                { let __ix2034 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2034) as usize].set_hh_b1(1i32); }
                            }
                        } else {
                            self.pdf_error(1762i32, 1817i32);
                        }
                    }
                    if self.scan_keyword(1835i32) {
                        {
                            { let __ix2035 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2035) as usize].set_hh_b0(0i32); }
                            if self.scan_keyword(1836i32) {
                                {
                                    self.scan_int();
                                    if (self.cur_val > 268435455i32) {
                                        self.pdf_error(1762i32, 1031i32);
                                    }
                                    { let __ix2036 = (self.cur_list.tail_field).wrapping_add(6i32); let __v2037 = self.cur_val; self.mem[(__ix2036) as usize].set_hh_lh(__v2037); }
                                }
                            } else {
                                { let __ix2038 = (self.cur_list.tail_field).wrapping_add(6i32); self.mem[(__ix2038) as usize].set_hh_lh(0i32); }
                            }
                        }
                    } else {
                        if self.scan_keyword(1837i32) {
                            { let __ix2039 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2039) as usize].set_hh_b0(5i32); }
                        } else {
                            if self.scan_keyword(1838i32) {
                                { let __ix2040 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2040) as usize].set_hh_b0(6i32); }
                            } else {
                                if self.scan_keyword(1839i32) {
                                    { let __ix2041 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2041) as usize].set_hh_b0(4i32); }
                                } else {
                                    if self.scan_keyword(1840i32) {
                                        { let __ix2042 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2042) as usize].set_hh_b0(2i32); }
                                    } else {
                                        if self.scan_keyword(1841i32) {
                                            { let __ix2043 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2043) as usize].set_hh_b0(3i32); }
                                        } else {
                                            if self.scan_keyword(1842i32) {
                                                { let __ix2044 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2044) as usize].set_hh_b0(7i32); }
                                            } else {
                                                if self.scan_keyword(1843i32) {
                                                    { let __ix2045 = (self.cur_list.tail_field).wrapping_add(5i32); self.mem[(__ix2045) as usize].set_hh_b0(1i32); }
                                                } else {
                                                    self.pdf_error(1762i32, 1844i32);
                                                }
                                            }
                                        }
                                    }
                                }
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
                    // §1565
                    if (self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().b0() == 7i32) {
                        {
                            self.scan_alt_rule();
                            { let __ix2046 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2047 = self.mem[((self.alt_rule).wrapping_add(1i32)) as usize].int(); self.mem[(__ix2046) as usize].set_int(__v2047); }
                            { let __ix2048 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2049 = self.mem[((self.alt_rule).wrapping_add(3i32)) as usize].int(); self.mem[(__ix2048) as usize].set_int(__v2049); }
                            { let __ix2050 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2051 = self.mem[((self.alt_rule).wrapping_add(2i32)) as usize].int(); self.mem[(__ix2050) as usize].set_int(__v2051); }
                        }
                    }
                    if (self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().b1() != 0i32) {
                        {
                            i = self.tokens_to_string(self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().rh());
                            k = self.find_obj(j, i, true);
                            self.flush_str(i);
                        }
                    } else {
                        k = self.find_obj(j, self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().rh(), false);
                    }
                    if ((k != 0i32) && (self.obj_tab[(k) as usize].int4 != 0i32)) {
                        {
                            self.warn_dest_dup(self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().rh(), self.mem[((self.cur_list.tail_field).wrapping_add(5i32)) as usize].hh().b1(), 1845i32, 1846i32);
                            self.flush_node_list(self.cur_list.tail_field);
                            self.cur_list.tail_field = q;
                            self.mem[(q) as usize].set_hh_rh(0i32);
                        }
                    }
                }
            }
            17 => {
                // §1561
                {
                    self.check_pdfoutput(1828i32, true);
                    if ((self.cur_list.mode_field).wrapping_abs() == 1i32) {
                        self.pdf_error(1762i32, 1829i32);
                    }
                    self.new_whatsit(17i32, 2i32);
                }
            }
            22 => {
                // §1569
                {
                    self.check_pdfoutput(1849i32, true);
                    self.new_whatsit(22i32, 2i32);
                }
            }
            27 => {
                // §1589
                {
                    self.check_pdfoutput(1201i32, true);
                    self.scan_font_ident();
                    k = self.cur_val;
                    if (k == 0i32) {
                        self.pdf_error(594i32, 873i32);
                    }
                    self.scan_pdf_ext_toks();
                    { let __v2052 = self.tokens_to_string(self.def_ref); self.pdf_font_attr[(k) as usize] = __v2052; }
                }
            }
            34 => {
                // §1535
                self.read_expand_font();
            }
            28 => {
                // §1588
                {
                    self.check_pdfoutput(1861i32, true);
                    self.pdf_include_chars();
                }
            }
            24 => {
                // §1578
                {
                    self.check_pdfoutput(1854i32, false);
                    self.scan_pdf_ext_toks();
                    if (self.eqtb[((629079i32) - 1) as usize].int() > 0i32) {
                        self.pdf_info_toks = self.concat_tokens(self.pdf_info_toks, self.def_ref);
                    }
                }
            }
            7 => {
                // §1538
                {
                    self.check_pdfoutput(1765i32, true);
                    if self.scan_keyword(1474i32) {
                        k = 8i32;
                    } else {
                        k = 7i32;
                    }
                    self.new_whatsit(k, 2i32);
                    if self.scan_keyword(890i32) {
                        { let __ix2053 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2053) as usize].set_hh_lh(2i32); }
                    } else {
                        if self.scan_keyword(889i32) {
                            { let __ix2054 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2054) as usize].set_hh_lh(1i32); }
                        } else {
                            { let __ix2055 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2055) as usize].set_hh_lh(0i32); }
                        }
                    }
                    if (k == 7i32) {
                        self.scan_pdf_ext_toks();
                    } else {
                        self.scan_pdf_ext_late_toks();
                    }
                    { let __ix2056 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2057 = self.def_ref; self.mem[(__ix2056) as usize].set_hh_rh(__v2057); }
                }
            }
            40 => {
                // §1539
                {
                    self.check_pdfoutput(1766i32, true);
                    self.scan_int();
                    if (self.cur_val >= self.colorstackused()) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1767i32);
                            }
                            self.print_int(((self.cur_val) as i64));
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1768i32;
                                self.help_line[(1i32) as usize] = 1769i32;
                                self.help_line[(0i32) as usize] = 1770i32;
                            }
                            self.error();
                            self.cur_val = 0i32;
                        }
                    }
                    if (self.cur_val < 0i32) {
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(264i32);
                                }
                                self.print(1771i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[(1i32) as usize] = 1769i32;
                                self.help_line[(0i32) as usize] = 1770i32;
                            }
                            self.error();
                            self.cur_val = 0i32;
                        }
                    }
                    if self.scan_keyword(1772i32) {
                        {
                            i = 0i32;
                            j = 3i32;
                        }
                    } else {
                        if self.scan_keyword(1773i32) {
                            {
                                i = 1i32;
                                j = 3i32;
                            }
                        } else {
                            if self.scan_keyword(1774i32) {
                                {
                                    i = 2i32;
                                    j = 2i32;
                                }
                            } else {
                                if self.scan_keyword(1775i32) {
                                    {
                                        i = 3i32;
                                        j = 2i32;
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
                            self.new_whatsit(40i32, j);
                            { let __ix2058 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2059 = self.cur_val; self.mem[(__ix2058) as usize].set_hh_rh(__v2059); }
                            { let __ix2060 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2060) as usize].set_hh_lh(i); }
                            if (i <= 1i32) {
                                {
                                    self.scan_pdf_ext_toks();
                                    { let __ix2061 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2062 = self.def_ref; self.mem[(__ix2061) as usize].set_hh_rh(__v2062); }
                                }
                            }
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
                                self.print(1776i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 1777i32;
                                self.help_line[(1i32) as usize] = 1778i32;
                                self.help_line[(0i32) as usize] = 1779i32;
                            }
                            self.error();
                        }
                    }
                }
            }
            41 => {
                // §1540
                {
                    self.check_pdfoutput(1132i32, true);
                    self.new_whatsit(41i32, 2i32);
                    self.scan_pdf_ext_toks();
                    { let __ix2063 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2064 = self.def_ref; self.mem[(__ix2063) as usize].set_hh_rh(__v2064); }
                }
            }
            42 => {
                // §1541
                {
                    self.check_pdfoutput(1780i32, true);
                    self.new_whatsit(42i32, 2i32);
                }
            }
            43 => {
                // §1542
                {
                    self.check_pdfoutput(1781i32, true);
                    self.new_whatsit(43i32, 2i32);
                }
            }
            29 => {
                // §1590
                {
                    self.check_pdfoutput(1862i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdfmapfile(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            30 => {
                // §1591
                {
                    self.check_pdfoutput(1863i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdfmapline(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            26 => {
                // §1580
                {
                    self.check_pdfoutput(1858i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdf_names_toks = self.concat_tokens(self.pdf_names_toks, self.def_ref);
                }
            }
            9 => {
                // §1544
                {
                    self.check_pdfoutput(1782i32, true);
                    if self.scan_keyword(1783i32) {
                        {
                            // §469
                            {
                                self.get_x_token();
                                if (self.cur_cmd != 10i32) {
                                    self.back_input();
                                }
                            }
                            // §1544
                            self.pdf_obj_count = (self.pdf_obj_count).wrapping_add(1i32);
                            self.pdf_create_obj(7i32, self.pdf_obj_count);
                            self.pdf_last_obj = self.obj_ptr;
                        }
                    } else {
                        {
                            k = (1i32).wrapping_neg();
                            if self.scan_keyword(1784i32) {
                                {
                                    self.scan_int();
                                    k = self.cur_val;
                                    if (((k <= 0i32) || (k > self.obj_ptr)) || (self.obj_tab[(k) as usize].int4 != 0i32)) {
                                        {
                                            self.pdf_warning(1782i32, 1785i32, true, true);
                                            self.pdf_retval = (1i32).wrapping_neg();
                                            k = (1i32).wrapping_neg();
                                        }
                                    }
                                }
                            }
                            if (k < 0i32) {
                                {
                                    self.pdf_obj_count = (self.pdf_obj_count).wrapping_add(1i32);
                                    self.pdf_create_obj(7i32, self.pdf_obj_count);
                                    k = self.obj_ptr;
                                }
                            }
                            self.obj_tab[(k) as usize].int4 = self.pdf_get_mem(4i32);
                            if self.scan_keyword(1022i32) {
                                {
                                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(1i32)) as usize] = 1i32;
                                    if self.scan_keyword(1786i32) {
                                        {
                                            self.scan_pdf_ext_toks();
                                            { let __ix2065 = (self.obj_tab[(k) as usize].int4).wrapping_add(2i32); let __v2066 = self.def_ref; self.pdf_mem[(__ix2065) as usize] = __v2066; }
                                        }
                                    } else {
                                        self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
                                    }
                                }
                            } else {
                                self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(1i32)) as usize] = 0i32;
                            }
                            if self.scan_keyword(878i32) {
                                self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] = 1i32;
                            } else {
                                self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] = 0i32;
                            }
                            self.scan_pdf_ext_toks();
                            { let __ix2067 = (self.obj_tab[(k) as usize].int4).wrapping_add(0i32); let __v2068 = self.def_ref; self.pdf_mem[(__ix2067) as usize] = __v2068; }
                            self.pdf_last_obj = k;
                        }
                    }
                }
            }
            18 => {
                // §1563
                {
                    self.check_pdfoutput(1830i32, true);
                    if self.scan_keyword(1786i32) {
                        {
                            self.scan_pdf_ext_toks();
                            r = self.def_ref;
                        }
                    } else {
                        r = 0i32;
                    }
                    p = self.scan_action();
                    if self.scan_keyword(537i32) {
                        {
                            self.scan_int();
                            i = self.cur_val;
                        }
                    } else {
                        i = 0i32;
                    }
                    self.scan_pdf_ext_toks();
                    q = self.def_ref;
                    self.pdf_new_obj(0i32, 0i32, 1i32);
                    j = self.obj_ptr;
                    self.write_action(p);
                    self.pdf_end_obj();
                    {
                        if (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                            {
                                if (self.mem[(p) as usize].hh().b0() == 3i32) {
                                    self.delete_token_ref(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                } else {
                                    {
                                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() != 0i32) {
                                            self.delete_token_ref(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
                                        }
                                        if (self.mem[(p) as usize].hh().b0() == 0i32) {
                                            self.delete_token_ref(self.mem[((p).wrapping_add(2i32)) as usize].hh().lh());
                                        } else {
                                            if ((((((self.mem[(p) as usize].hh().b1()) != 0) && ((1i32) != 0))) as i32) == 1i32) {
                                                self.delete_token_ref(self.mem[(p) as usize].hh().rh());
                                            }
                                        }
                                        if ((((((self.mem[(p) as usize].hh().b1()) != 0) && ((2i32) != 0))) as i32) == 2i32) {
                                            self.delete_token_ref(self.mem[((p).wrapping_add(3i32)) as usize].hh().rh());
                                        }
                                    }
                                }
                                self.free_node(p, 4i32);
                            }
                        } else {
                            { let __v2069 = (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh()).wrapping_sub(1i32); self.mem[((p).wrapping_add(2i32)) as usize].set_hh_rh(__v2069); }
                        }
                    }
                    self.pdf_create_obj(4i32, 0i32);
                    k = self.obj_ptr;
                    self.obj_tab[(k) as usize].int4 = self.pdf_get_mem(8i32);
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(6i32)) as usize] = j;
                    self.obj_tab[(k) as usize].int0 = i;
                    self.pdf_new_obj(0i32, 0i32, 1i32);
                    { let __a2070_0 = self.tokens_to_string(q); self.pdf_print_str_ln(__a2070_0) };
                    self.flush_str(self.last_tokens_string);
                    self.delete_token_ref(q);
                    self.pdf_end_obj();
                    { let __ix2071 = self.obj_tab[(k) as usize].int4; let __v2072 = self.obj_ptr; self.pdf_mem[(__ix2071) as usize] = __v2072; }
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(2i32)) as usize] = 0i32;
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] = 0i32;
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize] = 0i32;
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(5i32)) as usize] = 0i32;
                    { let __ix2073 = (self.obj_tab[(k) as usize].int4).wrapping_add(1i32); let __v2074 = self.pdf_parent_outline; self.pdf_mem[(__ix2073) as usize] = __v2074; }
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(7i32)) as usize] = r;
                    if (self.pdf_first_outline == 0i32) {
                        self.pdf_first_outline = k;
                    }
                    if (self.pdf_last_outline == 0i32) {
                        {
                            if (self.pdf_parent_outline != 0i32) {
                                self.pdf_mem[((self.obj_tab[(self.pdf_parent_outline) as usize].int4).wrapping_add(4i32)) as usize] = k;
                            }
                        }
                    } else {
                        {
                            self.pdf_mem[((self.obj_tab[(self.pdf_last_outline) as usize].int4).wrapping_add(3i32)) as usize] = k;
                            { let __ix2075 = (self.obj_tab[(k) as usize].int4).wrapping_add(2i32); let __v2076 = self.pdf_last_outline; self.pdf_mem[(__ix2075) as usize] = __v2076; }
                        }
                    }
                    self.pdf_last_outline = k;
                    if (self.obj_tab[(k) as usize].int0 != 0i32) {
                        {
                            self.pdf_parent_outline = k;
                            self.pdf_last_outline = 0i32;
                        }
                    } else {
                        if ((self.pdf_parent_outline != 0i32) && (self.outline_list_count(k) == (self.obj_tab[(self.pdf_parent_outline) as usize].int0).wrapping_abs())) {
                            {
                                j = self.pdf_last_outline;
                                loop {
                                    self.pdf_mem[((self.obj_tab[(self.pdf_parent_outline) as usize].int4).wrapping_add(5i32)) as usize] = j;
                                    j = self.pdf_parent_outline;
                                    self.pdf_parent_outline = self.pdf_mem[((self.obj_tab[(self.pdf_parent_outline) as usize].int4).wrapping_add(1i32)) as usize];
                                    if ((self.pdf_parent_outline == 0i32) || (self.outline_list_count(j) < (self.obj_tab[(self.pdf_parent_outline) as usize].int0).wrapping_abs())) { break; }
                                }
                                if (self.pdf_parent_outline == 0i32) {
                                    self.pdf_last_outline = self.pdf_first_outline;
                                } else {
                                    self.pdf_last_outline = self.pdf_mem[((self.obj_tab[(self.pdf_parent_outline) as usize].int4).wrapping_add(4i32)) as usize];
                                }
                                while (self.pdf_mem[((self.obj_tab[(self.pdf_last_outline) as usize].int4).wrapping_add(3i32)) as usize] != 0i32) {
                                    self.pdf_last_outline = self.pdf_mem[((self.obj_tab[(self.pdf_last_outline) as usize].int4).wrapping_add(3i32)) as usize];
                                }
                            }
                        }
                    }
                }
            }
            10 => {
                // §1546
                {
                    self.check_pdfoutput(1788i32, true);
                    self.scan_int();
                    self.pdf_check_obj(7i32, self.cur_val);
                    self.new_whatsit(10i32, 2i32);
                    { let __ix2077 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2078 = self.cur_val; self.mem[(__ix2077) as usize].set_hh_lh(__v2078); }
                }
            }
            12 => {
                // §1549
                {
                    self.check_pdfoutput(1792i32, true);
                    self.scan_int();
                    self.pdf_check_obj(8i32, self.cur_val);
                    self.new_whatsit(12i32, 5i32);
                    { let __ix2079 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2080 = self.cur_val; self.mem[(__ix2079) as usize].set_hh_lh(__v2080); }
                    { let __ix2081 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2082 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(0i32)) as usize]; self.mem[(__ix2081) as usize].set_int(__v2082); }
                    { let __ix2083 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2084 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(1i32)) as usize]; self.mem[(__ix2083) as usize].set_int(__v2084); }
                    { let __ix2085 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2086 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(2i32)) as usize]; self.mem[(__ix2085) as usize].set_int(__v2086); }
                }
            }
            14 => {
                // §1554
                {
                    self.check_pdfoutput(1808i32, true);
                    self.scan_int();
                    self.pdf_check_obj(9i32, self.cur_val);
                    self.new_whatsit(14i32, 5i32);
                    { let __ix2087 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2088 = self.cur_val; self.mem[(__ix2087) as usize].set_hh_lh(__v2088); }
                    { let __ix2089 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2090 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(0i32)) as usize]; self.mem[(__ix2089) as usize].set_int(__v2090); }
                    { let __ix2091 = (self.cur_list.tail_field).wrapping_add(2i32); let __v2092 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(1i32)) as usize]; self.mem[(__ix2091) as usize].set_int(__v2092); }
                    { let __ix2093 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2094 = self.pdf_mem[((self.obj_tab[(self.cur_val) as usize].int4).wrapping_add(2i32)) as usize]; self.mem[(__ix2093) as usize].set_int(__v2094); }
                }
            }
            23 => {
                // §1576
                {
                    self.new_whatsit(23i32, 2i32);
                }
            }
            36 => {
                // §1572
                {
                    self.check_pdfoutput(1850i32, true);
                    self.new_whatsit(36i32, 2i32);
                }
            }
            38 => {
                // §1575
                {
                    self.check_pdfoutput(1853i32, true);
                    self.new_whatsit(38i32, 2i32);
                    self.scan_int();
                    { let __ix2095 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2096 = self.fix_int(self.cur_val, 0i32, 1000i32); self.mem[(__ix2095) as usize].set_int(__v2096); }
                }
            }
            37 => {
                // §1574
                {
                    self.check_pdfoutput(1852i32, true);
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix2097 = self.cur_list.tail_field; let __v2098 = self.new_snap_node(37i32); self.mem[(__ix2097) as usize].set_hh_rh(__v2098); }
                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                    }
                }
            }
            16 => {
                // §1560
                {
                    self.check_pdfoutput(1826i32, true);
                    if ((self.cur_list.mode_field).wrapping_abs() == 1i32) {
                        self.pdf_error(1762i32, 1827i32);
                    }
                    k = self.pdf_new_objnum();
                    self.new_annot_whatsit(16i32, 7i32);
                    { let __ix2099 = (self.cur_list.tail_field).wrapping_add(5i32); let __v2100 = self.scan_action(); self.mem[(__ix2099) as usize].set_hh_rh(__v2100); }
                    { let __ix2101 = (self.cur_list.tail_field).wrapping_add(6i32); self.mem[(__ix2101) as usize].set_int(k); }
                    self.pdf_last_link = k;
                }
            }
            21 => {
                // §1568
                {
                    self.check_pdfoutput(1848i32, true);
                    self.new_annot_whatsit(21i32, 7i32);
                    self.scan_thread_id();
                }
            }
            20 => {
                // §1567
                {
                    self.check_pdfoutput(1847i32, true);
                    self.new_annot_whatsit(20i32, 7i32);
                    self.scan_thread_id();
                }
            }
            31 => {
                // §1581
                {
                    self.check_pdfoutput(1859i32, false);
                    self.scan_pdf_ext_toks();
                    if (self.eqtb[((629079i32) - 1) as usize].int() > 0i32) {
                        self.pdf_trailer_toks = self.concat_tokens(self.pdf_trailer_toks, self.def_ref);
                    }
                }
            }
            32 => {
                // §1582
                {
                    self.check_pdfoutput(1860i32, false);
                    self.scan_pdf_ext_toks();
                    if (self.eqtb[((629079i32) - 1) as usize].int() > 0i32) {
                        self.pdf_trailer_id_toks = self.concat_tokens(self.pdf_trailer_id_toks, self.def_ref);
                    }
                }
            }
            11 => {
                // §1548
                {
                    self.check_pdfoutput(1789i32, true);
                    self.pdf_xform_count = (self.pdf_xform_count).wrapping_add(1i32);
                    self.pdf_create_obj(8i32, self.pdf_xform_count);
                    k = self.obj_ptr;
                    self.obj_tab[(k) as usize].int4 = self.pdf_get_mem(6i32);
                    if self.scan_keyword(1786i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __ix2102 = (self.obj_tab[(k) as usize].int4).wrapping_add(4i32); let __v2103 = self.def_ref; self.pdf_mem[(__ix2102) as usize] = __v2103; }
                        }
                    } else {
                        self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize] = 0i32;
                    }
                    if self.scan_keyword(1790i32) {
                        {
                            self.scan_pdf_ext_toks();
                            { let __ix2104 = (self.obj_tab[(k) as usize].int4).wrapping_add(5i32); let __v2105 = self.def_ref; self.pdf_mem[(__ix2104) as usize] = __v2105; }
                        }
                    } else {
                        self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(5i32)) as usize] = 0i32;
                    }
                    self.scan_register_num();
                    if (self.cur_val < 256i32) {
                        p = self.eqtb[(((627433i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                    } else {
                        {
                            self.find_sa_element(4i32, self.cur_val, false);
                            if (self.cur_ptr == 0i32) {
                                p = 0i32;
                            } else {
                                p = self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().rh();
                            }
                        }
                    }
                    if (p == 0i32) {
                        self.pdf_error(1762i32, 1791i32);
                    }
                    { let __ix2106 = (self.obj_tab[(k) as usize].int4).wrapping_add(0i32); let __v2107 = self.mem[((p).wrapping_add(1i32)) as usize].int(); self.pdf_mem[(__ix2106) as usize] = __v2107; }
                    { let __ix2108 = (self.obj_tab[(k) as usize].int4).wrapping_add(1i32); let __v2109 = self.mem[((p).wrapping_add(3i32)) as usize].int(); self.pdf_mem[(__ix2108) as usize] = __v2109; }
                    { let __ix2110 = (self.obj_tab[(k) as usize].int4).wrapping_add(2i32); let __v2111 = self.mem[((p).wrapping_add(2i32)) as usize].int(); self.pdf_mem[(__ix2110) as usize] = __v2111; }
                    self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] = p;
                    if (self.cur_val < 256i32) {
                        { let __ix2112 = (627433i32).wrapping_add(self.cur_val); self.eqtb[((__ix2112) - 1) as usize].set_hh_rh(0i32); }
                    } else {
                        {
                            self.find_sa_element(4i32, self.cur_val, false);
                            if (self.cur_ptr != 0i32) {
                                {
                                    { let __ix2113 = (self.cur_ptr).wrapping_add(1i32); self.mem[(__ix2113) as usize].set_hh_rh(0i32); }
                                    { let __ix2114 = (self.cur_ptr).wrapping_add(1i32); let __v2115 = (self.mem[((self.cur_ptr).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix2114) as usize].set_hh_lh(__v2115); }
                                    self.delete_sa_ref(self.cur_ptr);
                                }
                            }
                        }
                    }
                    self.pdf_last_xform = k;
                }
            }
            13 => {
                // §1553
                {
                    self.check_pdfoutput(1807i32, true);
                    self.check_pdfversion();
                    self.scan_image();
                }
            }
            33 => {
                // §1586
                {
                    { let mut __f0 = ::core::mem::take(&mut self.epochseconds); let mut __f1 = ::core::mem::take(&mut self.microseconds); let __r = self.seconds_and_micros(&mut __f0, &mut __f1); self.epochseconds = __f0; self.microseconds = __f1; __r };
                }
            }
            35 => {
                // §1585
                {
                    self.scan_int();
                    if (self.cur_val < 0i32) {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                    self.random_seed = self.cur_val;
                    self.init_randoms(self.random_seed);
                }
            }
            39 => {
                // §1592
                {
                    self.glyph_to_unicode();
                }
            }
            44 => {
                // §1593
                {
                    self.check_pdfoutput(1864i32, true);
                    self.scan_font_ident();
                    k = self.cur_val;
                    if (k == 0i32) {
                        self.pdf_error(594i32, 873i32);
                    }
                    { let __v2116 = true; self.pdf_font_nobuiltin_tounicode[(k) as usize] = __v2116; }
                }
            }
            45 => {
                // §1594
                {
                    self.check_pdfoutput(1865i32, true);
                    self.new_whatsit(45i32, 2i32);
                }
            }
            46 => {
                // §1595
                {
                    self.check_pdfoutput(1866i32, true);
                    self.new_whatsit(46i32, 2i32);
                }
            }
            47 => {
                // §1596
                {
                    self.check_pdfoutput(1867i32, true);
                    self.new_whatsit(47i32, 2i32);
                }
            }
            48 => {
                // §1597
                {
                    self.check_pdfoutput(1868i32, true);
                    self.new_whatsit(48i32, 2i32);
                }
            }
            49 => {
                // §1598
                {
                    self.check_pdfoutput(1869i32, true);
                    self.new_whatsit(49i32, 2i32);
                }
            }
            50 => {
                // §1599
                {
                    self.check_pdfoutput(1870i32, true);
                    self.scan_pdf_ext_toks();
                    self.pdf_space_font_name = self.tokens_to_string(self.def_ref);
                    self.delete_token_ref(self.def_ref);
                }
            }
            _ => {
                // §1528
                self.confusion(1762i32);
            }
        }
    }

    /// The \.{\\language} extension is somewhat different.
    /// We need a subroutine that comes into play when a character of
    /// a non-`clang` language is being appended to the current paragraph.
    /// @<Declare action...
    // §1624
    pub fn fix_language(&mut self) {
        let mut l: ASCII_code = 0; // §1624
        if (self.eqtb[((629068i32) - 1) as usize].int() <= 0i32) {
            l = 0i32;
        } else {
            if (self.eqtb[((629068i32) - 1) as usize].int() > 255i32) {
                l = 0i32;
            } else {
                l = self.eqtb[((629068i32) - 1) as usize].int();
            }
        }
        if (l != self.cur_list.aux_field.hh().rh()) {
            {
                self.new_whatsit(5i32, 2i32);
                { let __ix2117 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2117) as usize].set_hh_rh(l); }
                self.cur_list.aux_field.set_hh_rh(l);
                { let __ix2118 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2119 = self.norm_min(self.eqtb[((629069i32) - 1) as usize].int()); self.mem[(__ix2118) as usize].set_hh_b0(__v2119); }
                { let __ix2120 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2121 = self.norm_min(self.eqtb[((629070i32) - 1) as usize].int()); self.mem[(__ix2120) as usize].set_hh_b1(__v2121); }
            }
        }
    }

    /// @<Declare the procedure called `handle_right_brace`
    // §1246
    pub fn handle_right_brace(&mut self) {
        let mut p: halfword = 0; // §1246
        let mut q: halfword = 0; // §1246
        let mut d: scaled = 0; // §1246
        let mut f: i32 = 0; // §1246
        match self.cur_group {
            1 => {
                self.unsave();
            }
            0 => {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(1456i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[(1i32) as usize] = 1457i32;
                        self.help_line[(0i32) as usize] = 1458i32;
                    }
                    self.error();
                }
            }
            14 | 15 | 16 => {
                self.extra_right_brace();
            }
            2 => {
                // §1263
                self.package(0i32);
            }
            3 => {
                {
                    self.adjust_tail = 4999994i32;
                    self.pre_adjust_tail = 4999985i32;
                    self.package(0i32);
                }
            }
            4 => {
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 0i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    {
                        self.end_graf();
                        self.package(0i32);
                    }
                }
            }
            5 => {
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 0i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    {
                        self.end_graf();
                        self.package(4i32);
                    }
                }
            }
            11 => {
                // §1278
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 1i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    {
                        self.end_graf();
                        q = self.eqtb[((626638i32) - 1) as usize].hh().rh();
                        { let __v2122 = (self.mem[(q) as usize].hh().rh()).wrapping_add(1i32); self.mem[(q) as usize].set_hh_rh(__v2122); }
                        d = self.eqtb[((629646i32) - 1) as usize].int();
                        f = self.eqtb[((629060i32) - 1) as usize].int();
                        self.unsave();
                        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                        p = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), 0i32, 1i32, 1073741823i32);
                        self.pop_nest();
                        if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() < 255i32) {
                            {
                                {
                                    self.prev_tail = self.cur_list.tail_field;
                                    { let __ix2123 = self.cur_list.tail_field; let __v2124 = self.get_node(5i32); self.mem[(__ix2123) as usize].set_hh_rh(__v2124); }
                                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                }
                                { let __ix2125 = self.cur_list.tail_field; self.mem[(__ix2125) as usize].set_hh_b0(3i32); }
                                { let __ix2126 = self.cur_list.tail_field; let __v2127 = (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int()).wrapping_add(0i32); self.mem[(__ix2126) as usize].set_hh_b1(__v2127); }
                                { let __ix2128 = (self.cur_list.tail_field).wrapping_add(3i32); let __v2129 = (self.mem[((p).wrapping_add(3i32)) as usize].int()).wrapping_add(self.mem[((p).wrapping_add(2i32)) as usize].int()); self.mem[(__ix2128) as usize].set_int(__v2129); }
                                { let __ix2130 = (self.cur_list.tail_field).wrapping_add(4i32); let __v2131 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(__ix2130) as usize].set_hh_lh(__v2131); }
                                { let __ix2132 = (self.cur_list.tail_field).wrapping_add(4i32); self.mem[(__ix2132) as usize].set_hh_rh(q); }
                                { let __ix2133 = (self.cur_list.tail_field).wrapping_add(2i32); self.mem[(__ix2133) as usize].set_int(d); }
                                { let __ix2134 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2134) as usize].set_int(f); }
                            }
                        } else {
                            {
                                {
                                    self.prev_tail = self.cur_list.tail_field;
                                    { let __ix2135 = self.cur_list.tail_field; let __v2136 = self.get_node(2i32); self.mem[(__ix2135) as usize].set_hh_rh(__v2136); }
                                    self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                }
                                { let __ix2137 = self.cur_list.tail_field; self.mem[(__ix2137) as usize].set_hh_b0(5i32); }
                                { let __ix2138 = self.cur_list.tail_field; let __v2139 = self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(); self.mem[(__ix2138) as usize].set_hh_b1(__v2139); }
                                { let __ix2140 = (self.cur_list.tail_field).wrapping_add(1i32); let __v2141 = self.mem[((p).wrapping_add(5i32)) as usize].hh().rh(); self.mem[(__ix2140) as usize].set_int(__v2141); }
                                self.delete_glue_ref(q);
                            }
                        }
                        self.free_node(p, 7i32);
                        if (self.nest_ptr == 0i32) {
                            self.build_page();
                        }
                    }
                }
            }
            8 => {
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 1i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    // §1203
                    {
                        while (((self.cur_input.state_field == 0i32) && (self.cur_input.loc_field == 0i32)) && (self.cur_input.index_field == 3i32)) {
                            self.end_token_list();
                        }
                        if (((self.cur_input.state_field != 0i32) || (self.cur_input.loc_field != 0i32)) || (self.cur_input.index_field != 6i32)) {
                            // §1204
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(679i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 1424i32;
                                    self.help_line[(0i32) as usize] = 1425i32;
                                }
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_input.loc_field == 0i32) { break; }
                                }
                            }
                        }
                        // §1203
                        self.output_can_end = true;
                        self.end_token_list();
                        self.output_can_end = false;
                        self.end_graf();
                        self.unsave();
                        self.output_active = false;
                        self.insert_penalties = 0i32;
                        // §1205
                        if (self.eqtb[((627688i32) - 1) as usize].hh().rh() != 0i32) {
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(1426i32);
                                }
                                self.print_esc(425i32);
                                self.print_int(((255i32) as i64));
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[(2i32) as usize] = 1427i32;
                                    self.help_line[(1i32) as usize] = 1428i32;
                                    self.help_line[(0i32) as usize] = 1429i32;
                                }
                                self.box_error(255i32);
                            }
                        }
                        // §1203
                        if (self.cur_list.tail_field != self.cur_list.head_field) {
                            {
                                { let __ix2142 = self.page_tail; let __v2143 = self.mem[(self.cur_list.head_field) as usize].hh().rh(); self.mem[(__ix2142) as usize].set_hh_rh(__v2143); }
                                self.page_tail = self.cur_list.tail_field;
                            }
                        }
                        if (self.mem[(4999997i32) as usize].hh().rh() != 0i32) {
                            {
                                if (self.mem[(4999998i32) as usize].hh().rh() == 0i32) {
                                    self.nest[(0i32) as usize].tail_field = self.page_tail;
                                }
                                { let __ix2144 = self.page_tail; let __v2145 = self.mem[(4999998i32) as usize].hh().rh(); self.mem[(__ix2144) as usize].set_hh_rh(__v2145); }
                                { let __v2146 = self.mem[(4999997i32) as usize].hh().rh(); self.mem[(4999998i32) as usize].set_hh_rh(__v2146); }
                                self.mem[(4999997i32) as usize].set_hh_rh(0i32);
                                self.page_tail = 4999997i32;
                            }
                        }
                        self.flush_node_list(self.disc_ptr[((2i32) - 1) as usize]);
                        self.disc_ptr[((2i32) - 1) as usize] = 0i32;
                        self.pop_nest();
                        self.build_page();
                    }
                }
            }
            10 => {
                // §1296
                self.build_discretionary();
            }
            6 => {
                // §1310
                {
                    self.back_input();
                    self.cur_tok = 619610i32;
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
                    self.print_esc(1308i32);
                    self.print(713i32);
                    {
                        self.help_ptr = 1i32;
                        self.help_line[(0i32) as usize] = 1540i32;
                    }
                    self.ins_error();
                }
            }
            7 => {
                // §1311
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 1i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    {
                        self.end_graf();
                        self.unsave();
                        self.align_peek();
                    }
                }
            }
            12 => {
                // §1346
                if ((self.eqtb[((629077i32) - 1) as usize].int() > 0i32) && (self.cur_list.mode_field == 105i32)) {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = 4i32;
                    }
                } else {
                    {
                        self.end_graf();
                        self.unsave();
                        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                        p = self.vpackage(self.mem[(self.cur_list.head_field) as usize].hh().rh(), self.save_stack[((self.save_ptr).wrapping_add(1i32)) as usize].int(), self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(), 1073741823i32);
                        self.pop_nest();
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2147 = self.cur_list.tail_field; let __v2148 = self.new_noad(); self.mem[(__ix2147) as usize].set_hh_rh(__v2148); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                        { let __ix2149 = self.cur_list.tail_field; self.mem[(__ix2149) as usize].set_hh_b0(29i32); }
                        { let __ix2150 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2150) as usize].set_hh_rh(2i32); }
                        { let __ix2151 = (self.cur_list.tail_field).wrapping_add(1i32); self.mem[(__ix2151) as usize].set_hh_lh(p); }
                    }
                }
            }
            13 => {
                // §1351
                self.build_choices();
            }
            9 => {
                // §1364
                {
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    { let __ix2152 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); self.mem[(__ix2152) as usize].set_hh_rh(3i32); }
                    p = self.fin_mlist(0i32);
                    { let __ix2153 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); self.mem[(__ix2153) as usize].set_hh_lh(p); }
                    if (p != 0i32) {
                        if (self.mem[(p) as usize].hh().rh() == 0i32) {
                            if (self.mem[(p) as usize].hh().b0() == 16i32) {
                                {
                                    if (self.mem[((p).wrapping_add(3i32)) as usize].hh().rh() == 0i32) {
                                        if (self.mem[((p).wrapping_add(2i32)) as usize].hh().rh() == 0i32) {
                                            {
                                                { let __ix2154 = self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int(); let __v2155 = self.mem[((p).wrapping_add(1i32)) as usize].hh(); self.mem[(__ix2154) as usize].set_hh(__v2155); }
                                                self.free_node(p, 4i32);
                                            }
                                        }
                                    }
                                }
                            } else {
                                if (self.mem[(p) as usize].hh().b0() == 28i32) {
                                    if (self.save_stack[((self.save_ptr).wrapping_add(0i32)) as usize].int() == (self.cur_list.tail_field).wrapping_add(1i32)) {
                                        if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 16i32) {
                                            // §1365
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
                // §1246
                self.confusion(1459i32);
            }
        }
    }

    /// We shall concentrate first on the inner loop of `main_control`, deferring
    /// consideration of the other cases until later.
    // §1207
    pub fn main_control(&mut self) {
        let mut t: i32 = 0; // §1207
        let mut tmp_k1: halfword = 0; // §1207
        let mut tmp_k2: halfword = 0; // §1207
        // goto labels: L60, reswitch, L70, L80, L90, L91, L92, L100, L101, L110, L111, L112, L95, L120, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if (self.eqtb[((627165i32) - 1) as usize].hh().rh() != 0i32) {
                    self.begin_token_list(self.eqtb[((627165i32) - 1) as usize].hh().rh(), 12i32);
                }
            }
            if __goto_1 <= 1 { // L60
                self.get_x_token();
            }
            if __goto_1 <= 2 { // reswitch
                if (self.interrupt != 0i32) {
                    // §1208
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
                if (self.eqtb[((629054i32) - 1) as usize].int() > 0i32) {
                    self.show_cur_cmd_chr();
                }
                // §1207
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
                            if ((((self.cur_cmd == 11i32) || (self.cur_cmd == 12i32)) || (self.cur_cmd == 68i32)) || (self.cur_cmd == 16i32)) {
                                self.cancel_boundary = true;
                            }
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                    115 => {
                        if ((self.cur_list.aux_field.hh().lh() == 1000i32) || (self.eqtb[((629101i32) - 1) as usize].int() > 0i32)) {
                            { __goto_1 = 14; continue 'l_dispatch_1; }
                        } else {
                            self.app_space();
                        }
                    }
                    169 | 273 => {
                        { __goto_1 = 14; continue 'l_dispatch_1; }
                    }
                    1 | 105 | 209 | 11 | 219 | 274 => {
                        // §1223
                    }
                    40 | 144 | 248 => {
                        {
                            if (self.cur_chr == 0i32) {
                                {
                                    // §432
                                    loop {
                                        self.get_x_token();
                                        if (self.cur_cmd != 10i32) { break; }
                                    }
                                    // §1223
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            } else {
                                {
                                    t = self.scanner_status;
                                    self.scanner_status = 0i32;
                                    self.get_next();
                                    self.scanner_status = t;
                                    if (self.cur_cs < 514i32) {
                                        self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                                    } else {
                                        self.cur_cs = self.prim_lookup(self.hash[((self.cur_cs) - 514) as usize].rh());
                                    }
                                    if (self.cur_cs != 0i32) {
                                        {
                                            self.cur_cmd = self.eqtb[(((615526i32).wrapping_add(self.cur_cs)) - 1) as usize].hh().b0();
                                            self.cur_chr = self.eqtb[(((615526i32).wrapping_add(self.cur_cs)) - 1) as usize].hh().rh();
                                            self.cur_tok = (619621i32).wrapping_add(self.cur_cs);
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
                        // §1234
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2156 = self.cur_list.tail_field; let __v2157 = self.scan_rule_spec(); self.mem[(__ix2156) as usize].set_hh_rh(__v2157); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            if ((self.cur_list.mode_field).wrapping_abs() == 1i32) {
                                { let __v2158 = self.eqtb[((629672i32) - 1) as usize].int(); self.cur_list.aux_field.set_int(__v2158); }
                            } else {
                                if ((self.cur_list.mode_field).wrapping_abs() == 105i32) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                }
                            }
                        }
                    }
                    28 | 131 | 235 | 237 => {
                        // §1235
                        self.append_glue();
                    }
                    30 | 134 | 238 | 239 => {
                        self.append_kern();
                    }
                    2 | 106 => {
                        // §1241
                        self.new_save_level(1i32);
                    }
                    62 | 166 | 270 => {
                        self.new_save_level(14i32);
                    }
                    63 | 167 | 271 => {
                        if (self.cur_group == 14i32) {
                            self.unsave();
                        } else {
                            self.off_save();
                        }
                    }
                    3 | 107 | 211 => {
                        // §1245
                        self.handle_right_brace();
                    }
                    22 | 127 | 231 => {
                        // §1251
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
                        // §1268
                        self.new_graf((self.cur_chr > 0i32));
                    }
                    12 | 13 | 17 | 69 | 4 | 24 | 36 | 46 | 48 | 27 | 34 | 65 | 66 => {
                        {
                            self.back_input();
                            self.new_graf(true);
                        }
                    }
                    148 | 252 => {
                        // §1270
                        if (self.cur_chr != 2i32) {
                            self.indent_in_hmode();
                        }
                    }
                    14 => {
                        // §1272
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
                            if (self.cur_list.mode_field == 1i32) {
                                self.build_page();
                            }
                        }
                    }
                    119 | 132 | 141 | 129 | 137 => {
                        self.head_for_vmode();
                    }
                    38 | 142 | 246 | 143 | 247 => {
                        // §1275
                        self.begin_insert_or_adjust();
                    }
                    19 | 123 | 227 => {
                        self.make_mark();
                    }
                    43 | 147 | 251 => {
                        // §1280
                        self.append_penalty();
                    }
                    26 | 130 | 234 => {
                        // §1282
                        self.delete_last();
                    }
                    25 | 128 | 232 => {
                        // §1287
                        self.unpackage();
                    }
                    149 => {
                        // §1290
                        self.append_italic_correction();
                    }
                    253 => {
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2159 = self.cur_list.tail_field; let __v2160 = self.new_kern(0i32); self.mem[(__ix2159) as usize].set_hh_rh(__v2160); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                    152 | 256 => {
                        // §1294
                        self.append_discretionary();
                    }
                    150 => {
                        // §1300
                        self.make_accent();
                    }
                    6 | 110 | 214 | 5 | 109 | 213 => {
                        // §1304
                        self.align_error();
                    }
                    35 | 139 | 243 => {
                        self.no_align_error();
                    }
                    64 | 168 | 272 => {
                        self.omit_error();
                    }
                    33 => {
                        // §1308
                        self.init_align();
                    }
                    138 => {
                        // §1703
                        if (self.cur_chr > 0i32) {
                            {
                                if self.eTeX_enabled((self.eqtb[((629126i32) - 1) as usize].int() > 0i32), self.cur_cmd, self.cur_chr) {
                                    {
                                        self.prev_tail = self.cur_list.tail_field;
                                        { let __ix2161 = self.cur_list.tail_field; let __v2162 = self.new_math(0i32, self.cur_chr); self.mem[(__ix2161) as usize].set_hh_rh(__v2162); }
                                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                    }
                                }
                            }
                        } else {
                            // §1308
                            self.init_align();
                        }
                    }
                    241 => {
                        if self.privileged() {
                            if (self.cur_group == 15i32) {
                                self.init_align();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    10 | 114 => {
                        if ((self.eqtb[((629077i32) - 1) as usize].int() > 1i32) && (self.cur_list.mode_field == 105i32)) {
                            {
                                self.back_input();
                                self.cur_tok = self.par_token;
                                self.back_input();
                                self.cur_input.index_field = 4i32;
                            }
                        } else {
                            self.do_endv();
                        }
                    }
                    68 | 172 | 276 => {
                        // §1312
                        self.cs_error();
                    }
                    108 => {
                        // §1315
                        self.init_math();
                    }
                    257 => {
                        // §1318
                        if self.privileged() {
                            if (self.cur_group == 15i32) {
                                self.start_eq_no();
                            } else {
                                self.off_save();
                            }
                        }
                    }
                    210 => {
                        // §1328
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2163 = self.cur_list.tail_field; let __v2164 = self.new_noad(); self.mem[(__ix2163) as usize].set_hh_rh(__v2164); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            self.back_input();
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    220 | 221 | 277 => {
                        // §1332
                        self.set_math_char((self.eqtb[(((628762i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32));
                    }
                    225 => {
                        {
                            self.scan_char_num();
                            self.cur_chr = self.cur_val;
                            self.set_math_char((self.eqtb[(((628762i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh()).wrapping_sub(0i32));
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
                        // §1336
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2165 = self.cur_list.tail_field; let __v2166 = self.new_noad(); self.mem[(__ix2165) as usize].set_hh_rh(__v2166); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix2167 = self.cur_list.tail_field; let __v2168 = self.cur_chr; self.mem[(__ix2167) as usize].set_hh_b0(__v2168); }
                            self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
                        }
                    }
                    260 => {
                        self.math_limit_switch();
                    }
                    275 => {
                        // §1340
                        self.math_radical();
                    }
                    254 | 255 => {
                        // §1342
                        self.math_ac();
                    }
                    265 => {
                        // §1345
                        {
                            self.scan_spec(12i32, false);
                            self.normal_paragraph();
                            self.push_nest();
                            self.cur_list.mode_field = (1i32).wrapping_neg();
                            { let __v2169 = self.eqtb[((629672i32) - 1) as usize].int(); self.cur_list.aux_field.set_int(__v2169); }
                            if (self.eqtb[((627164i32) - 1) as usize].hh().rh() != 0i32) {
                                self.begin_token_list(self.eqtb[((627164i32) - 1) as usize].hh().rh(), 11i32);
                            }
                        }
                    }
                    262 => {
                        // §1349
                        {
                            self.prev_tail = self.cur_list.tail_field;
                            { let __ix2170 = self.cur_list.tail_field; let __v2171 = self.new_style(self.cur_chr); self.mem[(__ix2170) as usize].set_hh_rh(__v2171); }
                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                        }
                    }
                    264 => {
                        {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2172 = self.cur_list.tail_field; let __v2173 = self.new_glue(0i32); self.mem[(__ix2172) as usize].set_hh_rh(__v2173); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                            { let __ix2174 = self.cur_list.tail_field; self.mem[(__ix2174) as usize].set_hh_b1(98i32); }
                        }
                    }
                    263 => {
                        self.append_choices();
                    }
                    217 | 216 => {
                        // §1353
                        self.sub_sup();
                    }
                    261 => {
                        // §1358
                        self.math_fraction();
                    }
                    258 => {
                        // §1368
                        self.math_left_right();
                    }
                    212 => {
                        // §1371
                        if (self.cur_group == 15i32) {
                            self.after_math();
                        } else {
                            self.off_save();
                        }
                    }
                    72 | 176 | 280 | 73 | 177 | 281 | 74 | 178 | 282 | 75 | 179 | 283 | 76 | 180 | 284 | 77 | 181 | 285 | 78 | 182 | 286 | 79 | 183 | 287 | 80 | 184 | 288 | 81 | 185 | 289 | 82 | 186 | 290 | 83 | 187 | 291 | 84 | 188 | 292 | 85 | 189 | 293 | 86 | 190 | 294 | 87 | 191 | 295 | 88 | 192 | 296 | 89 | 193 | 297 | 102 | 206 | 310 | 103 | 207 | 311 | 90 | 194 | 298 | 91 | 195 | 299 | 92 | 196 | 300 | 93 | 197 | 301 | 94 | 198 | 302 | 95 | 199 | 303 | 96 | 200 | 304 | 97 | 201 | 305 | 98 | 202 | 306 | 99 | 203 | 307 | 100 | 204 | 308 | 101 | 205 | 309 => {
                        // §1388
                        self.prefixed_command();
                    }
                    41 | 145 | 249 => {
                        // §1446
                        {
                            self.get_token();
                            self.after_token = self.cur_tok;
                        }
                    }
                    42 | 146 | 250 => {
                        // §1449
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
                        // §1452
                        self.open_or_close_in();
                    }
                    59 | 163 | 267 => {
                        // §1454
                        self.issue_message();
                    }
                    58 | 162 | 266 => {
                        // §1463
                        self.shift_case();
                    }
                    20 | 124 | 228 => {
                        // §1468
                        self.show_whatever();
                    }
                    60 | 164 | 268 => {
                        // §1527
                        self.do_extension();
                    }
                    _ => {}
                }
                // §1207
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 3 { // L70
                self.main_s = self.eqtb[(((628506i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                // §1211
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v2175 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2175); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v2176 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2176); }
                        }
                    }
                }
                self.save_tail = 0i32;
                self.main_f = self.eqtb[((627689i32) - 1) as usize].hh().rh();
                self.bchar = self.font_bchar[(self.main_f) as usize];
                self.false_bchar = self.font_false_bchar[(self.main_f) as usize];
                if (self.cur_list.mode_field > 0i32) {
                    if (self.eqtb[((629068i32) - 1) as usize].int() != self.cur_list.aux_field.hh().rh()) {
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
                            { let __ix2177 = self.lig_stack; self.mem[(__ix2177) as usize].set_hh_rh(0i32); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                { let __ix2178 = self.lig_stack; let __v2179 = self.main_f; self.mem[(__ix2178) as usize].set_hh_b0(__v2179); }
                self.cur_l = (self.cur_chr).wrapping_add(0i32);
                { let __ix2180 = self.lig_stack; let __v2181 = self.cur_l; self.mem[(__ix2180) as usize].set_hh_b1(__v2181); }
                self.cur_q = self.cur_list.tail_field;
                tmp_k1 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                // §1217
                if (tmp_k1 != 0i32) {
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
                                                { let __ix2182 = self.main_p; self.mem[(__ix2182) as usize].set_hh_b1(2i32); }
                                                self.lft_hit = false;
                                            }
                                        }
                                        if self.rt_hit {
                                            if (self.lig_stack == 0i32) {
                                                {
                                                    { let __ix2183 = self.main_p; let __v2184 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix2183) as usize].set_hh_b1(__v2184); }
                                                    self.rt_hit = false;
                                                }
                                            }
                                        }
                                        if (self.eqtb[((629102i32) - 1) as usize].int() > 0i32) {
                                            tmp_k2 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                                        } else {
                                            tmp_k2 = 0i32;
                                        }
                                        if (tmp_k2 == 0i32) {
                                            {
                                                { let __ix2185 = self.cur_q; let __v2186 = self.main_p; self.mem[(__ix2185) as usize].set_hh_rh(__v2186); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        } else {
                                            {
                                                { let __ix2187 = self.cur_q; self.mem[(__ix2187) as usize].set_hh_rh(tmp_k2); }
                                                { let __v2188 = self.main_p; self.mem[(tmp_k2) as usize].set_hh_rh(__v2188); }
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
                                                { let __ix2189 = self.cur_list.tail_field; let __v2190 = self.new_disc(); self.mem[(__ix2189) as usize].set_hh_rh(__v2190); }
                                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.save_tail = self.cur_list.tail_field;
                        if ((((((!(self.cur_list.tail_field >= self.hi_mem_min)) && (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 7i32)) && (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == 0i32)) && (self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().lh() == 0i32)) && (self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().rh() == 0i32)) && (self.mem[(self.prev_tail) as usize].hh().rh() == self.cur_list.tail_field)) {
                            {
                                {
                                    { let __ix2191 = self.prev_tail; self.mem[(__ix2191) as usize].set_hh_rh(tmp_k1); }
                                    { let __v2192 = self.cur_list.tail_field; self.mem[(tmp_k1) as usize].set_hh_rh(__v2192); }
                                    self.prev_tail = tmp_k1;
                                }
                            }
                        } else {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2193 = self.cur_list.tail_field; self.mem[(__ix2193) as usize].set_hh_rh(tmp_k1); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                        }
                        { __goto_1 = 5; continue 'l_dispatch_1; }
                    }
                }
                // §1211
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
                    // §1212
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
                                        { let __ix2194 = self.main_p; self.mem[(__ix2194) as usize].set_hh_b1(2i32); }
                                        self.lft_hit = false;
                                    }
                                }
                                if self.rt_hit {
                                    if (self.lig_stack == 0i32) {
                                        {
                                            { let __ix2195 = self.main_p; let __v2196 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix2195) as usize].set_hh_b1(__v2196); }
                                            self.rt_hit = false;
                                        }
                                    }
                                }
                                if (self.eqtb[((629102i32) - 1) as usize].int() > 0i32) {
                                    tmp_k2 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                                } else {
                                    tmp_k2 = 0i32;
                                }
                                if (tmp_k2 == 0i32) {
                                    {
                                        { let __ix2197 = self.cur_q; let __v2198 = self.main_p; self.mem[(__ix2197) as usize].set_hh_rh(__v2198); }
                                        self.cur_list.tail_field = self.main_p;
                                        self.ligature_present = false;
                                    }
                                } else {
                                    {
                                        { let __ix2199 = self.cur_q; self.mem[(__ix2199) as usize].set_hh_rh(tmp_k2); }
                                        { let __v2200 = self.main_p; self.mem[(tmp_k2) as usize].set_hh_rh(__v2200); }
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
                                        { let __ix2201 = self.cur_list.tail_field; let __v2202 = self.new_disc(); self.mem[(__ix2201) as usize].set_hh_rh(__v2202); }
                                        self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 5 { // L90
                // §1211
                if (self.lig_stack == 0i32) {
                    // §1213
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
                            { let __ix2203 = self.lig_stack; let __v2204 = self.avail; self.mem[(__ix2203) as usize].set_hh_rh(__v2204); }
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
                            { let __ix2205 = self.lig_stack; let __v2206 = self.avail; self.mem[(__ix2205) as usize].set_hh_rh(__v2206); }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        { __goto_1 = 1; continue 'l_dispatch_1; }
                    }
                }
                { let __ix2207 = self.cur_list.tail_field; let __v2208 = self.lig_stack; self.mem[(__ix2207) as usize].set_hh_rh(__v2208); }
                self.cur_list.tail_field = self.lig_stack;
            }
            if __goto_1 <= 8 { // L100
                // §1211
                self.get_next();
                // §1215
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
                self.main_s = self.eqtb[(((628506i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                { let __v2209 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2209); }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            { let __v2210 = self.main_s; self.cur_list.aux_field.set_hh_lh(__v2210); }
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
                            { let __ix2211 = self.lig_stack; self.mem[(__ix2211) as usize].set_hh_rh(0i32); }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                { let __ix2212 = self.lig_stack; let __v2213 = self.main_f; self.mem[(__ix2212) as usize].set_hh_b0(__v2213); }
                self.cur_r = (self.cur_chr).wrapping_add(0i32);
                { let __ix2214 = self.lig_stack; let __v2215 = self.cur_r; self.mem[(__ix2214) as usize].set_hh_b1(__v2215); }
                if (self.cur_r == self.false_bchar) {
                    self.cur_r = 256i32;
                }
            }
            if __goto_1 <= 10 { // L110
                // §1211
                tmp_k1 = self.get_auto_kern(self.main_f, self.cur_l, self.cur_r);
                // §1217
                if (tmp_k1 != 0i32) {
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
                                                { let __ix2216 = self.main_p; self.mem[(__ix2216) as usize].set_hh_b1(2i32); }
                                                self.lft_hit = false;
                                            }
                                        }
                                        if self.rt_hit {
                                            if (self.lig_stack == 0i32) {
                                                {
                                                    { let __ix2217 = self.main_p; let __v2218 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix2217) as usize].set_hh_b1(__v2218); }
                                                    self.rt_hit = false;
                                                }
                                            }
                                        }
                                        if (self.eqtb[((629102i32) - 1) as usize].int() > 0i32) {
                                            tmp_k2 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                                        } else {
                                            tmp_k2 = 0i32;
                                        }
                                        if (tmp_k2 == 0i32) {
                                            {
                                                { let __ix2219 = self.cur_q; let __v2220 = self.main_p; self.mem[(__ix2219) as usize].set_hh_rh(__v2220); }
                                                self.cur_list.tail_field = self.main_p;
                                                self.ligature_present = false;
                                            }
                                        } else {
                                            {
                                                { let __ix2221 = self.cur_q; self.mem[(__ix2221) as usize].set_hh_rh(tmp_k2); }
                                                { let __v2222 = self.main_p; self.mem[(tmp_k2) as usize].set_hh_rh(__v2222); }
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
                                                { let __ix2223 = self.cur_list.tail_field; let __v2224 = self.new_disc(); self.mem[(__ix2223) as usize].set_hh_rh(__v2224); }
                                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.save_tail = self.cur_list.tail_field;
                        if ((((((!(self.cur_list.tail_field >= self.hi_mem_min)) && (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 7i32)) && (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == 0i32)) && (self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().lh() == 0i32)) && (self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().rh() == 0i32)) && (self.mem[(self.prev_tail) as usize].hh().rh() == self.cur_list.tail_field)) {
                            {
                                {
                                    { let __ix2225 = self.prev_tail; self.mem[(__ix2225) as usize].set_hh_rh(tmp_k1); }
                                    { let __v2226 = self.cur_list.tail_field; self.mem[(tmp_k1) as usize].set_hh_rh(__v2226); }
                                    self.prev_tail = tmp_k1;
                                }
                            }
                        } else {
                            {
                                self.prev_tail = self.cur_list.tail_field;
                                { let __ix2227 = self.cur_list.tail_field; self.mem[(__ix2227) as usize].set_hh_rh(tmp_k1); }
                                self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                            }
                        }
                        { __goto_1 = 5; continue 'l_dispatch_1; }
                    }
                }
                // §1216
                if (((self.main_i.b2()).wrapping_sub(0i32) % 4i32) != 1i32) {
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
                        // §1218
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
                                                            { let __ix2228 = self.main_p; self.mem[(__ix2228) as usize].set_hh_b1(2i32); }
                                                            self.lft_hit = false;
                                                        }
                                                    }
                                                    if self.rt_hit {
                                                        if (self.lig_stack == 0i32) {
                                                            {
                                                                { let __ix2229 = self.main_p; let __v2230 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix2229) as usize].set_hh_b1(__v2230); }
                                                                self.rt_hit = false;
                                                            }
                                                        }
                                                    }
                                                    if (self.eqtb[((629102i32) - 1) as usize].int() > 0i32) {
                                                        tmp_k2 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                                                    } else {
                                                        tmp_k2 = 0i32;
                                                    }
                                                    if (tmp_k2 == 0i32) {
                                                        {
                                                            { let __ix2231 = self.cur_q; let __v2232 = self.main_p; self.mem[(__ix2231) as usize].set_hh_rh(__v2232); }
                                                            self.cur_list.tail_field = self.main_p;
                                                            self.ligature_present = false;
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix2233 = self.cur_q; self.mem[(__ix2233) as usize].set_hh_rh(tmp_k2); }
                                                            { let __v2234 = self.main_p; self.mem[(tmp_k2) as usize].set_hh_rh(__v2234); }
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
                                                            { let __ix2235 = self.cur_list.tail_field; let __v2236 = self.new_disc(); self.mem[(__ix2235) as usize].set_hh_rh(__v2236); }
                                                            self.cur_list.tail_field = self.mem[(self.cur_list.tail_field) as usize].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        self.prev_tail = self.cur_list.tail_field;
                                        { let __ix2237 = self.cur_list.tail_field; let __v2238 = self.new_kern(self.font_info[(((self.kern_base[(self.main_f) as usize]).wrapping_add((256i32).wrapping_mul(self.main_j.b2()))).wrapping_add(self.main_j.b3())) as usize].int()); self.mem[(__ix2237) as usize].set_hh_rh(__v2238); }
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
                                                    { let __ix2239 = (self.lig_stack).wrapping_add(1i32); let __v2240 = self.main_p; self.mem[(__ix2239) as usize].set_hh_rh(__v2240); }
                                                }
                                            } else {
                                                { let __ix2241 = self.lig_stack; let __v2242 = self.cur_r; self.mem[(__ix2241) as usize].set_hh_b1(__v2242); }
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    {
                                        self.cur_r = self.main_j.b3();
                                        self.main_p = self.lig_stack;
                                        self.lig_stack = self.new_lig_item(self.cur_r);
                                        { let __ix2243 = self.lig_stack; let __v2244 = self.main_p; self.mem[(__ix2243) as usize].set_hh_rh(__v2244); }
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
                                                                { let __ix2245 = self.main_p; self.mem[(__ix2245) as usize].set_hh_b1(2i32); }
                                                                self.lft_hit = false;
                                                            }
                                                        }
                                                        if false {
                                                            if (self.lig_stack == 0i32) {
                                                                {
                                                                    { let __ix2246 = self.main_p; let __v2247 = (self.mem[(self.main_p) as usize].hh().b1()).wrapping_add(1i32); self.mem[(__ix2246) as usize].set_hh_b1(__v2247); }
                                                                    self.rt_hit = false;
                                                                }
                                                            }
                                                        }
                                                        if (self.eqtb[((629102i32) - 1) as usize].int() > 0i32) {
                                                            tmp_k2 = self.get_auto_kern(self.main_f, 256i32, self.cur_l);
                                                        } else {
                                                            tmp_k2 = 0i32;
                                                        }
                                                        if (tmp_k2 == 0i32) {
                                                            {
                                                                { let __ix2248 = self.cur_q; let __v2249 = self.main_p; self.mem[(__ix2248) as usize].set_hh_rh(__v2249); }
                                                                self.cur_list.tail_field = self.main_p;
                                                                self.ligature_present = false;
                                                            }
                                                        } else {
                                                            {
                                                                { let __ix2250 = self.cur_q; self.mem[(__ix2250) as usize].set_hh_rh(tmp_k2); }
                                                                { let __v2251 = self.main_p; self.mem[(tmp_k2) as usize].set_hh_rh(__v2251); }
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
                                                                { let __ix2252 = self.cur_list.tail_field; let __v2253 = self.new_disc(); self.mem[(__ix2252) as usize].set_hh_rh(__v2253); }
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
                // §1216
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
                // §1211
                self.main_p = self.mem[((self.lig_stack).wrapping_add(1i32)) as usize].hh().rh();
                // §1214
                if (self.main_p > 0i32) {
                    {
                        self.prev_tail = self.cur_list.tail_field;
                        { let __ix2254 = self.cur_list.tail_field; let __v2255 = self.main_p; self.mem[(__ix2254) as usize].set_hh_rh(__v2255); }
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
                // §1207
                if (self.eqtb[((626640i32) - 1) as usize].hh().rh() == 0i32) {
                    // §1219
                    {
                        // §1220
                        {
                            self.main_p = self.font_glue[(self.eqtb[((627689i32) - 1) as usize].hh().rh()) as usize];
                            if (self.main_p == 0i32) {
                                {
                                    self.main_p = self.new_spec(0i32);
                                    self.main_k = (self.param_base[(self.eqtb[((627689i32) - 1) as usize].hh().rh()) as usize]).wrapping_add(2i32);
                                    { let __ix2256 = (self.main_p).wrapping_add(1i32); let __v2257 = self.font_info[(self.main_k) as usize].int(); self.mem[(__ix2256) as usize].set_int(__v2257); }
                                    { let __ix2258 = (self.main_p).wrapping_add(2i32); let __v2259 = self.font_info[((self.main_k).wrapping_add(1i32)) as usize].int(); self.mem[(__ix2258) as usize].set_int(__v2259); }
                                    { let __ix2260 = (self.main_p).wrapping_add(3i32); let __v2261 = self.font_info[((self.main_k).wrapping_add(2i32)) as usize].int(); self.mem[(__ix2260) as usize].set_int(__v2261); }
                                    { let __ix2262 = self.eqtb[((627689i32) - 1) as usize].hh().rh(); let __v2263 = self.main_p; self.font_glue[(__ix2262) as usize] = __v2263; }
                                }
                            }
                        }
                        // §1219
                        self.temp_ptr = self.new_glue(self.main_p);
                    }
                } else {
                    self.temp_ptr = self.new_param_glue(12i32);
                }
                if (self.eqtb[((629101i32) - 1) as usize].int() > 0i32) {
                    self.adjust_interword_glue(self.cur_list.tail_field, self.temp_ptr);
                }
                { let __ix2264 = self.cur_list.tail_field; let __v2265 = self.temp_ptr; self.mem[(__ix2264) as usize].set_hh_rh(__v2265); }
                self.cur_list.tail_field = self.temp_ptr;
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 15 { // exit
                // §1207
            }
            break 'l_dispatch_1;
        }
    }

    /// The `error` routine calls on `give_err_help` if help is requested from
    /// the `err_help` parameter.
    // §1462
    pub fn give_err_help(&mut self) {
        self.token_show(self.eqtb[((627167i32) - 1) as usize].hh().rh());
    }

    /// Here is the only place we use `pack_buffered_name`. This part of the program
    /// becomes active when a ``virgin'' \TeX\ is trying to get going, just after
    /// the preliminary initialization, or when the user is substituting another
    /// format file by typing `\.\&' after the initial `\.{**}' prompt.  The buffer
    /// contains the first line of input in `buffer[loc..(last-1)]`, where
    /// `loc<last` and `buffer[loc]<>" "`.
    /// @<Declare the function called `open_fmt_file`
    // §550
    pub fn open_fmt_file(&mut self) -> bool {
        let mut open_fmt_file: bool = false;
        let mut j: i32 = 0; // §550
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
    // §1481
    pub fn load_fmt_file(&mut self) -> bool {
        let mut load_fmt_file: bool = false;
        let mut j: i32 = 0; // §1481
        let mut k: i32 = 0; // §1481
        let mut p: halfword = 0; // §1481
        let mut q: halfword = 0; // §1481
        let mut x: i32 = 0; // §1481
        let mut w: four_quarters = four_quarters::default(); // §1481
        'l_exit_f: {
            'l_L6666_f: {
                // §1486
                x = self.fmt_file.buf.int();
                if (x != 399034618i32) {
                    break 'l_L6666_f;
                }
                // §1891
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
                                    self.xord[(k) as usize] = x;
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
                                    self.xchr[(k) as usize] = ((x) as u8);
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
                                    self.xprn[(k) as usize] = (x != 0i32);
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        if self.eight_bit_p {
                            {
                                let __for_end_7 = 255i32;
                                k = 0i32;
                                while k <= __for_end_7 {
                                    { let __v2266 = true; self.xprn[(k) as usize] = __v2266; }
                                    k = k.wrapping_add(1);
                                }
                            }
                        }
                    }
                }
                // §1655
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
                        // §1813
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 2061i32;
                    }
                } else {
                    // §1655
                    {
                        // §1812
                        self.max_reg_num = 255i32;
                        self.max_reg_help_line = 789i32;
                    }
                }
                // §1486
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
                if (x != 4999999i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 629929i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 522749i32) {
                    break 'l_L6666_f;
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    x = self.fmt_file.buf.int();
                }
                if (x != 8191i32) {
                    break 'l_L6666_f;
                }
                // §1488
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
                // §1490
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
                        let __for_end_5 = 5i32;
                        k = 0i32;
                        while k <= __for_end_5 {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > self.lo_mem_max)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.sa_root[(k) as usize] = x;
                                }
                            }
                            k = k.wrapping_add(1);
                        }
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
                                { let __v2267 = self.fmt_file.buf; self.mem[(k) as usize] = __v2267; }
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
                            { let __v2268 = self.fmt_file.buf; self.mem[(k) as usize] = __v2268; }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                if (mem_min < (2i32).wrapping_neg()) {
                    {
                        p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
                        q = (mem_min).wrapping_add(1i32);
                        { let __ix2269 = mem_min; self.mem[(__ix2269) as usize].set_hh_rh(0i32); }
                        { let __ix2270 = mem_min; self.mem[(__ix2270) as usize].set_hh_lh(0i32); }
                        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(q);
                        { let __ix2271 = (self.rover).wrapping_add(1i32); self.mem[(__ix2271) as usize].set_hh_lh(q); }
                        { let __v2272 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v2272); }
                        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(p);
                        self.mem[(q) as usize].set_hh_rh(268435455i32);
                        self.mem[(q) as usize].set_hh_lh(((0i32).wrapping_neg()).wrapping_sub(q));
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < (self.lo_mem_max).wrapping_add(1i32)) || (x > 4999985i32)) {
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
                    if ((x < 0i32) || (x > 4999999i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.avail = x;
                    }
                }
                self.mem_end = 4999999i32;
                {
                    let __for_end_4 = self.mem_end;
                    k = self.hi_mem_min;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2273 = self.fmt_file.buf; self.mem[(k) as usize] = __v2273; }
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
                // §1495
                k = 1i32;
                loop {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1i32) || ((k).wrapping_add(x) > 629930i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2274 = self.fmt_file.buf; self.eqtb[((j) - 1) as usize] = __v2274; }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || ((k).wrapping_add(x) > 629930i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            { let __v2275 = self.eqtb[(((k).wrapping_sub(1i32)) - 1) as usize]; self.eqtb[((j) - 1) as usize] = __v2275; }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    if (k > 629929i32) { break; }
                }
                // §1492
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.par_loc = self.fmt_file.buf.int();
                }
                self.par_token = (4095i32).wrapping_add(self.par_loc);
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 514i32) || (x > 615514i32)) {
                        break 'l_L6666_f;
                    } else {
                        self.write_loc = x;
                    }
                }
                // §1497
                {
                    let __for_end_4 = 2100i32;
                    p = 0i32;
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2276 = self.fmt_file.buf.hh(); self.prim[(p) as usize] = __v2276; }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 514i32) || (x > 615514i32)) {
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
                        { let __v2277 = self.fmt_file.buf.hh(); self.hash[((p) - 514) as usize] = __v2277; }
                    }
                    if (p == self.hash_used) { break; }
                }
                {
                    let __for_end_4 = 626626i32;
                    p = (self.hash_used).wrapping_add(1i32);
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            { let __v2278 = self.fmt_file.buf.hh(); self.hash[((p) - 514) as usize] = __v2278; }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.cs_count = self.fmt_file.buf.int();
                }
                // §1499
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
                            { let __v2279 = self.fmt_file.buf; self.font_info[(k) as usize] = __v2279; }
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
                self.make_pdftex_banner();
                {
                    let __for_end_4 = self.font_ptr;
                    k = 0i32;
                    while k <= __for_end_4 {
                        // §1501
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2280 = self.fmt_file.buf.qqqq(); self.font_check[(k) as usize] = __v2280; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2281 = self.fmt_file.buf.int(); self.font_size[(k) as usize] = __v2281; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2282 = self.fmt_file.buf.int(); self.font_dsize[(k) as usize] = __v2282; }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < 0i32) || (x > 268435455i32)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_params[(k) as usize] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2283 = self.fmt_file.buf.int(); self.hyphen_char[(k) as usize] = __v2283; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2284 = self.fmt_file.buf.int(); self.skew_char[(k) as usize] = __v2284; }
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
                                { let __v2285 = self.fmt_file.buf.int(); self.char_base[(k) as usize] = __v2285; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2286 = self.fmt_file.buf.int(); self.width_base[(k) as usize] = __v2286; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2287 = self.fmt_file.buf.int(); self.height_base[(k) as usize] = __v2287; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2288 = self.fmt_file.buf.int(); self.depth_base[(k) as usize] = __v2288; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2289 = self.fmt_file.buf.int(); self.italic_base[(k) as usize] = __v2289; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2290 = self.fmt_file.buf.int(); self.lig_kern_base[(k) as usize] = __v2290; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2291 = self.fmt_file.buf.int(); self.kern_base[(k) as usize] = __v2291; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2292 = self.fmt_file.buf.int(); self.exten_base[(k) as usize] = __v2292; }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                { let __v2293 = self.fmt_file.buf.int(); self.param_base[(k) as usize] = __v2293; }
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
                // §1503
                {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || (x > 8191i32)) {
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
                                if ((x < 0i32) || (x > 8191i32)) {
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
                                if ((x < 0i32) || (x > 268435455i32)) {
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
                            { let __v2294 = self.fmt_file.buf.hh(); self.trie[(k) as usize] = __v2294; }
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
                                if ((x < 0i32) || (x > 65535i32)) {
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
                // §1505
                {
                    self.undumpimagemeta(self.eqtb[((629088i32) - 1) as usize].int(), self.eqtb[((629089i32) - 1) as usize].int(), self.eqtb[((629092i32) - 1) as usize].int());
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        self.pdf_mem_size = self.fmt_file.buf.int();
                    }
                    { let __n2295 = ((self.pdf_mem_size) as usize) + 1; self.pdf_mem.resize(__n2295, 0); }
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
                                    { let __v2296 = self.fmt_file.buf.int(); self.pdf_mem[(k) as usize] = __v2296; }
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
                                    self.obj_tab[(k) as usize].int0 = self.fmt_file.buf.int();
                                }
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[(k) as usize].int1 = self.fmt_file.buf.int();
                                }
                                self.obj_tab[(k) as usize].int2 = (((1i32).wrapping_neg()) as i64);
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[(k) as usize].int3 = self.fmt_file.buf.int();
                                }
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    self.obj_tab[(k) as usize].int4 = self.fmt_file.buf.int();
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
                        { let __v2297 = self.fmt_file.buf.int(); self.head_tab[((7i32) - 1) as usize] = __v2297; }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v2298 = self.fmt_file.buf.int(); self.head_tab[((8i32) - 1) as usize] = __v2298; }
                    }
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        { let __v2299 = self.fmt_file.buf.int(); self.head_tab[((9i32) - 1) as usize] = __v2299; }
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
                // §1507
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
                // §1481
                { let __v2300 = self.eqtb[((629672i32) - 1) as usize].int(); self.cur_list.aux_field.set_int(__v2300); }
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
    // §1513
    pub fn close_files_and_terminate(&mut self) {
        let mut a: i32 = 0; // §1513
        let mut b: i32 = 0; // §1513
        let mut c: i32 = 0; // §1513
        let mut i: i32 = 0; // §1513
        let mut j: i32 = 0; // §1513
        let mut k: i32 = 0; // §1513
        let mut l: i32 = 0; // §1513
        let mut is_root: bool = false; // §1513
        let mut is_names: bool = false; // §1513
        let mut root: i32 = 0; // §1513
        let mut outlines: i32 = 0; // §1513
        let mut threads: i32 = 0; // §1513
        let mut names_tree: i32 = 0; // §1513
        let mut dests: i32 = 0; // §1513
        let mut xref_offset_width: i32 = 0; // §1513
        let mut names_head: i32 = 0; // §1513
        let mut names_tail: i32 = 0; // §1513
        // §1626
        {
            let __for_end_2 = 15i32;
            k = 0i32;
            while k <= __for_end_2 {
                if self.write_open[(k) as usize] {
                    { let mut __f0 = ::core::mem::take(&mut self.write_file[(k) as usize]); let __r = self.a_close(&mut __f0); self.write_file[(k) as usize] = __f0; __r };
                }
                k = k.wrapping_add(1);
            }
        }
        // §1513
        self.eqtb[((629067i32) - 1) as usize].set_int((1i32).wrapping_neg());
        if (self.eqtb[((629049i32) - 1) as usize].int() > 0i32) {
            // §1514
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
                        let __w3 = 615000i32;
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                        crate::system::wr_str(&mut self.log_file, " multiletter control sequences out of ");
                        crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                        crate::system::wr_str(&mut self.log_file, "+0");
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
                        let __w1 = 8191i32;
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
        // §1513
        if (!self.fixed_pdfoutput_set) {
            self.fix_pdfoutput();
        }
        if (self.fixed_pdfoutput > 0i32) {
            {
                if (self.history == 3i32) {
                    {
                        self.remove_pdffile();
                        {
                            if (self.interaction == 3i32) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1703i32);
                        }
                    }
                } else {
                    {
                        // §794
                        if (self.total_pages == 0i32) {
                            {
                                self.print_nl(998i32);
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
                                                if ((self.total_pages % 6i32) != 0i32) {
                                                    self.obj_tab[(self.pdf_last_pages) as usize].int0 = (self.total_pages % 6i32);
                                                }
                                                self.flush_jbig2_page0_objects();
                                                // §799
                                                k = self.head_tab[((1i32) - 1) as usize];
                                                while (self.obj_tab[(k) as usize].int4 == 0i32) {
                                                    {
                                                        self.pdf_warning(1192i32, 1199i32, true, false);
                                                        self.print_int(((self.obj_tab[(k) as usize].int0) as i64));
                                                        self.print(1200i32);
                                                        self.print_ln();
                                                        self.print_ln();
                                                        k = self.obj_tab[(k) as usize].int1;
                                                    }
                                                }
                                                self.head_tab[((1i32) - 1) as usize] = k;
                                                // §800
                                                k = self.head_tab[((1i32) - 1) as usize];
                                                l = 0i32;
                                                loop {
                                                    i = self.obj_tab[(k) as usize].int1;
                                                    self.obj_tab[(k) as usize].int1 = l;
                                                    l = k;
                                                    k = i;
                                                    if (k == 0i32) { break; }
                                                }
                                                self.head_tab[((1i32) - 1) as usize] = l;
                                                k = self.head_tab[((2i32) - 1) as usize];
                                                self.pages_tail = k;
                                                l = 0i32;
                                                loop {
                                                    i = self.obj_tab[(k) as usize].int1;
                                                    self.obj_tab[(k) as usize].int1 = l;
                                                    l = k;
                                                    k = i;
                                                    if (k == 0i32) { break; }
                                                }
                                                self.head_tab[((2i32) - 1) as usize] = l;
                                                // §796
                                                k = self.head_tab[((5i32) - 1) as usize];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_fix_dest(k);
                                                        k = self.obj_tab[(k) as usize].int1;
                                                    }
                                                }
                                                // §798
                                                k = self.head_tab[((6i32) - 1) as usize];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_fix_struct_dest(k);
                                                        k = self.obj_tab[(k) as usize].int1;
                                                    }
                                                }
                                                // §801
                                                {
                                                    let __for_end_12 = self.font_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if ((self.font_used[(k) as usize] && self.hasfmentry(k)) && (self.pdf_font_num[(k) as usize] < 0i32)) {
                                                            {
                                                                i = (self.pdf_font_num[(k) as usize]).wrapping_neg();
                                                                self.pdfassert((self.pdf_font_num[(i) as usize] > 0i32));
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
                                                                if (((self.str_start[((self.pdf_font_attr[(i) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(i) as usize]) as usize]) == 0i32) && ((self.str_start[((self.pdf_font_attr[(k) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(k) as usize]) as usize]) != 0i32)) {
                                                                    { let __v2301 = self.pdf_font_attr[(k) as usize]; self.pdf_font_attr[(i) as usize] = __v2301; }
                                                                } else {
                                                                    if (((self.str_start[((self.pdf_font_attr[(k) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(k) as usize]) as usize]) == 0i32) && ((self.str_start[((self.pdf_font_attr[(i) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(i) as usize]) as usize]) != 0i32)) {
                                                                        { let __v2302 = self.pdf_font_attr[(i) as usize]; self.pdf_font_attr[(k) as usize] = __v2302; }
                                                                    } else {
                                                                        if ((((self.str_start[((self.pdf_font_attr[(i) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(i) as usize]) as usize]) != 0i32) && ((self.str_start[((self.pdf_font_attr[(k) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pdf_font_attr[(k) as usize]) as usize]) != 0i32)) && (!self.str_eq_str(self.pdf_font_attr[(i) as usize], self.pdf_font_attr[(k) as usize]))) {
                                                                            {
                                                                                self.pdf_warning(1201i32, 1202i32, true, false);
                                                                                self.print_font_identifier(i);
                                                                                self.print(1203i32);
                                                                                self.print_font_identifier(k);
                                                                                self.print(1204i32);
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
                                                self.fixed_gen_tounicode = self.eqtb[((629104i32) - 1) as usize].int();
                                                k = self.head_tab[((3i32) - 1) as usize];
                                                while (k != 0i32) {
                                                    {
                                                        self.f = self.obj_tab[(k) as usize].int0;
                                                        self.pdfassert((self.pdf_font_num[(self.f) as usize] > 0i32));
                                                        self.do_pdf_font(k, self.f);
                                                        k = self.obj_tab[(k) as usize].int1;
                                                    }
                                                }
                                                self.write_fontstuff();
                                                // §802
                                                a = (self.sys_obj_ptr).wrapping_add(1i32);
                                                l = self.head_tab[((2i32) - 1) as usize];
                                                k = self.head_tab[((1i32) - 1) as usize];
                                                b = 0i32;
                                                loop {
                                                    i = 0i32;
                                                    c = 0i32;
                                                    if (self.obj_tab[(l) as usize].int1 == 0i32) {
                                                        is_root = true;
                                                    } else {
                                                        is_root = false;
                                                    }
                                                    loop {
                                                        if (!is_root) {
                                                            {
                                                                if ((i % 6i32) == 0i32) {
                                                                    {
                                                                        self.pdf_last_pages = self.pdf_new_objnum();
                                                                        if (c == 0i32) {
                                                                            c = self.pdf_last_pages;
                                                                        }
                                                                        self.obj_tab[(self.pages_tail) as usize].int1 = self.pdf_last_pages;
                                                                        self.pages_tail = self.pdf_last_pages;
                                                                        self.obj_tab[(self.pdf_last_pages) as usize].int1 = 0i32;
                                                                        self.obj_tab[(self.pdf_last_pages) as usize].int0 = self.obj_tab[(l) as usize].int0;
                                                                    }
                                                                } else {
                                                                    self.obj_tab[(self.pdf_last_pages) as usize].int0 = (self.obj_tab[(self.pdf_last_pages) as usize].int0).wrapping_add(self.obj_tab[(l) as usize].int0);
                                                                }
                                                            }
                                                        }
                                                        // §803
                                                        self.pdf_begin_dict(l, 1i32);
                                                        {
                                                            self.pdf_print(1205i32);
                                                            {
                                                                {
                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_os_get_os_buf(1i32);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_flush();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                {
                                                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                        self.pdf_int_entry_ln(1187i32, self.obj_tab[(l) as usize].int0);
                                                        if (!is_root) {
                                                            self.pdf_indirect_ln(1161i32, self.pdf_last_pages);
                                                        }
                                                        self.pdf_print(1206i32);
                                                        j = 0i32;
                                                        loop {
                                                            self.pdf_print_int(((k) as i64));
                                                            self.pdf_print(1146i32);
                                                            k = self.obj_tab[(k) as usize].int1;
                                                            j = (j).wrapping_add(1i32);
                                                            if (((((l < a) && (j == self.obj_tab[(l) as usize].int0)) || (k == 0i32)) || ((k == b) && (b != 0i32))) || (j == 6i32)) { break; }
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
                                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_flush();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                {
                                                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                        if (k == 0i32) {
                                                            {
                                                                k = self.head_tab[((2i32) - 1) as usize];
                                                                self.head_tab[((2i32) - 1) as usize] = 0i32;
                                                            }
                                                        }
                                                        if (is_root && (self.eqtb[((627168i32) - 1) as usize].hh().rh() != 0i32)) {
                                                            self.pdf_print_toks_ln(self.eqtb[((627168i32) - 1) as usize].hh().rh());
                                                        }
                                                        self.pdf_end_dict();
                                                        // §802
                                                        i = (i).wrapping_add(1i32);
                                                        l = self.obj_tab[(l) as usize].int1;
                                                        if (l == c) { break; }
                                                    }
                                                    b = c;
                                                    if (l == 0i32) {
                                                        break 'l_done_f;
                                                    }
                                                    if false { break; }
                                                }
                                            }
                                            // §788
                                            if (self.pdf_first_outline != 0i32) {
                                                {
                                                    self.pdf_new_dict(0i32, 0i32, 1i32);
                                                    outlines = self.obj_ptr;
                                                    l = self.pdf_first_outline;
                                                    k = 0i32;
                                                    loop {
                                                        k = (k).wrapping_add(1i32);
                                                        a = self.open_subentries(l);
                                                        if (self.obj_tab[(l) as usize].int0 > 0i32) {
                                                            k = (k).wrapping_add(a);
                                                        }
                                                        { let __ix2303 = (self.obj_tab[(l) as usize].int4).wrapping_add(1i32); let __v2304 = self.obj_ptr; self.pdf_mem[(__ix2303) as usize] = __v2304; }
                                                        l = self.pdf_mem[((self.obj_tab[(l) as usize].int4).wrapping_add(3i32)) as usize];
                                                        if (l == 0i32) { break; }
                                                    }
                                                    {
                                                        self.pdf_print(1184i32);
                                                        {
                                                            {
                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_os_get_os_buf(1i32);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                        self.overflow(1004i32, pdf_op_buf_size);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_flush();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            {
                                                                self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                    self.pdf_indirect_ln(1185i32, self.pdf_first_outline);
                                                    self.pdf_indirect_ln(1186i32, self.pdf_last_outline);
                                                    self.pdf_int_entry_ln(1187i32, k);
                                                    self.pdf_end_dict();
                                                    // §789
                                                    k = self.head_tab[((4i32) - 1) as usize];
                                                    while (k != 0i32) {
                                                        {
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(1i32)) as usize] == self.pdf_parent_outline) {
                                                                {
                                                                    if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(2i32)) as usize] == 0i32) {
                                                                        self.pdf_first_outline = k;
                                                                    }
                                                                    if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] == 0i32) {
                                                                        self.pdf_last_outline = k;
                                                                    }
                                                                }
                                                            }
                                                            self.pdf_begin_dict(k, 1i32);
                                                            self.pdf_indirect_ln(1188i32, self.pdf_mem[(self.obj_tab[(k) as usize].int4) as usize]);
                                                            self.pdf_indirect_ln(65i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(6i32)) as usize]);
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(1i32)) as usize] != 0i32) {
                                                                self.pdf_indirect_ln(1161i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(1i32)) as usize]);
                                                            }
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(2i32)) as usize] != 0i32) {
                                                                self.pdf_indirect_ln(1189i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(2i32)) as usize]);
                                                            }
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize] != 0i32) {
                                                                self.pdf_indirect_ln(1190i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(3i32)) as usize]);
                                                            }
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize] != 0i32) {
                                                                self.pdf_indirect_ln(1185i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(4i32)) as usize]);
                                                            }
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(5i32)) as usize] != 0i32) {
                                                                self.pdf_indirect_ln(1186i32, self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(5i32)) as usize]);
                                                            }
                                                            if (self.obj_tab[(k) as usize].int0 != 0i32) {
                                                                self.pdf_int_entry_ln(1187i32, self.obj_tab[(k) as usize].int0);
                                                            }
                                                            if (self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(7i32)) as usize] != 0i32) {
                                                                {
                                                                    self.pdf_print_toks_ln(self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(7i32)) as usize]);
                                                                    {
                                                                        self.delete_token_ref(self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(7i32)) as usize]);
                                                                        self.pdf_mem[((self.obj_tab[(k) as usize].int4).wrapping_add(7i32)) as usize] = 0i32;
                                                                    }
                                                                }
                                                            }
                                                            self.pdf_end_dict();
                                                            k = self.obj_tab[(k) as usize].int1;
                                                        }
                                                    }
                                                }
                                            } else {
                                                // §788
                                                outlines = 0i32;
                                            }
                                            // §804
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
                                                    self.pdf_create_obj(0i32, 0i32);
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
                                                            self.obj_tab[(names_tail) as usize].int1 = l;
                                                            names_tail = l;
                                                        }
                                                    }
                                                    self.obj_tab[(names_tail) as usize].int1 = 0i32;
                                                    // §805
                                                    self.pdf_begin_dict(l, 1i32);
                                                    j = 0i32;
                                                    if is_names {
                                                        {
                                                            self.obj_tab[(l) as usize].int0 = self.dest_names[(k) as usize].objname;
                                                            self.pdf_print(1208i32);
                                                            loop {
                                                                self.pdf_print_str(self.dest_names[(k) as usize].objname);
                                                                {
                                                                    {
                                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_os_get_os_buf(1i32);
                                                                        } else {
                                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                self.overflow(1004i32, pdf_op_buf_size);
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
                                                                self.pdf_print_int(((self.dest_names[(k) as usize].objnum) as i64));
                                                                self.pdf_print(1146i32);
                                                                j = (j).wrapping_add(1i32);
                                                                k = (k).wrapping_add(1i32);
                                                                if ((j == 6i32) || (k == self.pdf_dest_names_ptr)) { break; }
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
                                                                                self.overflow(1004i32, pdf_op_buf_size);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_flush();
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                            self.obj_tab[(l) as usize].int4 = self.dest_names[((k).wrapping_sub(1i32)) as usize].objname;
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
                                                            self.obj_tab[(l) as usize].int0 = self.obj_tab[(k) as usize].int0;
                                                            self.pdf_print(1206i32);
                                                            loop {
                                                                self.pdf_print_int(((k) as i64));
                                                                self.pdf_print(1146i32);
                                                                j = (j).wrapping_add(1i32);
                                                                self.obj_tab[(l) as usize].int4 = self.obj_tab[(k) as usize].int4;
                                                                k = self.obj_tab[(k) as usize].int1;
                                                                if (((j == 6i32) || (k == b)) || (self.obj_tab[(k) as usize].int1 == 0i32)) { break; }
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
                                                                                self.overflow(1004i32, pdf_op_buf_size);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_flush();
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                            if (k == b) {
                                                                b = 0i32;
                                                            }
                                                        }
                                                    }
                                                    self.pdf_print(1209i32);
                                                    self.pdf_print_str(self.obj_tab[(l) as usize].int0);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
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
                                                    self.pdf_print_str(self.obj_tab[(l) as usize].int4);
                                                    {
                                                        self.pdf_print(93i32);
                                                        {
                                                            {
                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_os_get_os_buf(1i32);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                        self.overflow(1004i32, pdf_op_buf_size);
                                                                    } else {
                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                            self.pdf_flush();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            {
                                                                self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                    self.pdf_end_dict();
                                                    if (b == 0i32) { break; }
                                                }
                                                // §804
                                                if (k == l) {
                                                    {
                                                        dests = l;
                                                        break 'l_done1_f;
                                                    }
                                                }
                                                if false { break; }
                                            }
                                        }
                                        if ((dests != 0i32) || (self.pdf_names_toks != 0i32)) {
                                            {
                                                self.pdf_new_dict(0i32, 0i32, 1i32);
                                                if (dests != 0i32) {
                                                    self.pdf_indirect_ln(1207i32, dests);
                                                }
                                                if (self.pdf_names_toks != 0i32) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_names_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_names_toks);
                                                            self.pdf_names_toks = 0i32;
                                                        }
                                                    }
                                                }
                                                self.pdf_end_dict();
                                                names_tree = self.obj_ptr;
                                            }
                                        } else {
                                            names_tree = 0i32;
                                        }
                                        // §790
                                        if (self.head_tab[((10i32) - 1) as usize] != 0i32) {
                                            {
                                                self.pdf_new_obj(0i32, 0i32, 1i32);
                                                threads = self.obj_ptr;
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1004i32, pdf_op_buf_size);
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
                                                k = self.head_tab[((10i32) - 1) as usize];
                                                while (k != 0i32) {
                                                    {
                                                        self.pdf_print_int(((k) as i64));
                                                        self.pdf_print(1146i32);
                                                        k = self.obj_tab[(k) as usize].int1;
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
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_end_obj();
                                                k = self.head_tab[((10i32) - 1) as usize];
                                                while (k != 0i32) {
                                                    {
                                                        self.out_thread(k);
                                                        k = self.obj_tab[(k) as usize].int1;
                                                    }
                                                }
                                            }
                                        } else {
                                            threads = 0i32;
                                        }
                                        // §806
                                        self.pdf_new_dict(0i32, 0i32, 1i32);
                                        root = self.obj_ptr;
                                        {
                                            self.pdf_print(1210i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.pdf_indirect_ln(1211i32, self.pdf_last_pages);
                                        if (threads != 0i32) {
                                            self.pdf_indirect_ln(1212i32, threads);
                                        }
                                        if (outlines != 0i32) {
                                            self.pdf_indirect_ln(1213i32, outlines);
                                        }
                                        if (names_tree != 0i32) {
                                            self.pdf_indirect_ln(1214i32, names_tree);
                                        }
                                        if (self.pdf_catalog_toks != 0i32) {
                                            {
                                                self.pdf_print_toks_ln(self.pdf_catalog_toks);
                                                {
                                                    self.delete_token_ref(self.pdf_catalog_toks);
                                                    self.pdf_catalog_toks = 0i32;
                                                }
                                            }
                                        }
                                        if (self.pdf_catalog_openaction != 0i32) {
                                            self.pdf_indirect_ln(1215i32, self.pdf_catalog_openaction);
                                        }
                                        self.pdf_end_dict();
                                        // §794
                                        if (self.eqtb[((629113i32) - 1) as usize].int() == 0i32) {
                                            self.pdf_print_info();
                                        }
                                        if self.pdf_os_enable {
                                            {
                                                self.pdf_os_switch(true);
                                                self.pdf_os_write_objstream();
                                                self.pdf_flush();
                                                self.pdf_os_switch(false);
                                                // §814
                                                self.pdf_new_dict(0i32, 0i32, 0i32);
                                                if (((self.obj_tab[(self.sys_obj_ptr) as usize].int2) as f64 / (((256i32) as i64)) as f64) > ((16777215i32) as f64)) {
                                                    xref_offset_width = 5i32;
                                                } else {
                                                    if (self.obj_tab[(self.sys_obj_ptr) as usize].int2 > ((16777215i32) as i64)) {
                                                        xref_offset_width = 4i32;
                                                    } else {
                                                        if (self.obj_tab[(self.sys_obj_ptr) as usize].int2 > ((65535i32) as i64)) {
                                                            xref_offset_width = 3i32;
                                                        } else {
                                                            xref_offset_width = 2i32;
                                                        }
                                                    }
                                                }
                                                // §812
                                                l = 0i32;
                                                self.obj_tab[(l) as usize].int2 = (((2i32).wrapping_neg()) as i64);
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if (!(self.obj_tab[(k) as usize].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                            {
                                                                self.obj_tab[(l) as usize].int1 = k;
                                                                l = k;
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.obj_tab[(l) as usize].int1 = 0i32;
                                                // §814
                                                {
                                                    self.pdf_print(1232i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1233i32);
                                                self.pdf_print_int((((self.obj_ptr).wrapping_add(1i32)) as i64));
                                                {
                                                    self.pdf_print(93i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_int_entry_ln(1234i32, (self.obj_ptr).wrapping_add(1i32));
                                                self.pdf_print(1235i32);
                                                self.pdf_print_int(((xref_offset_width) as i64));
                                                {
                                                    self.pdf_print(1236i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_indirect_ln(1237i32, root);
                                                if (self.eqtb[((629113i32) - 1) as usize].int() == 0i32) {
                                                    self.pdf_indirect_ln(1238i32, (self.obj_ptr).wrapping_sub(1i32));
                                                }
                                                if (self.pdf_trailer_toks != 0i32) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_trailer_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_trailer_toks);
                                                            self.pdf_trailer_toks = 0i32;
                                                        }
                                                    }
                                                }
                                                if (self.pdf_trailer_id_toks != 0i32) {
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
                                                                self.overflow(1004i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                self.pdf_begin_stream();
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 0i32;
                                                    while k <= __for_end_12 {
                                                        {
                                                            if (!(self.obj_tab[(k) as usize].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                                {
                                                                    {
                                                                        {
                                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_os_get_os_buf(1i32);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                    self.overflow(1004i32, pdf_op_buf_size);
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
                                                                    self.pdf_out_bytes(((self.obj_tab[(k) as usize].int1) as i64), xref_offset_width);
                                                                    {
                                                                        {
                                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                self.pdf_os_get_os_buf(1i32);
                                                                            } else {
                                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                    self.overflow(1004i32, pdf_op_buf_size);
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
                                                                    if (self.obj_tab[(k) as usize].int3 == (1i32).wrapping_neg()) {
                                                                        {
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1004i32, pdf_op_buf_size);
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
                                                                            self.pdf_out_bytes(self.obj_tab[(k) as usize].int2, xref_offset_width);
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1004i32, pdf_op_buf_size);
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
                                                                                            self.overflow(1004i32, pdf_op_buf_size);
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
                                                                            self.pdf_out_bytes(self.obj_tab[(k) as usize].int2, xref_offset_width);
                                                                            {
                                                                                {
                                                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                        self.pdf_os_get_os_buf(1i32);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                                                        } else {
                                                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                                self.pdf_flush();
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                {
                                                                                    self.pdf_buf_set(self.pdf_ptr, self.obj_tab[(k) as usize].int3);
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
                                                // §794
                                                self.pdf_flush();
                                            }
                                        } else {
                                            {
                                                // §812
                                                l = 0i32;
                                                self.obj_tab[(l) as usize].int2 = (((2i32).wrapping_neg()) as i64);
                                                {
                                                    let __for_end_12 = self.sys_obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        if (!(self.obj_tab[(k) as usize].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                            {
                                                                self.obj_tab[(l) as usize].int1 = k;
                                                                l = k;
                                                            }
                                                        }
                                                        k = k.wrapping_add(1);
                                                    }
                                                }
                                                self.obj_tab[(l) as usize].int1 = 0i32;
                                                // §813
                                                self.pdf_save_offset = (self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64));
                                                {
                                                    self.pdf_print(1227i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1228i32);
                                                {
                                                    self.pdf_print_int((((self.obj_ptr).wrapping_add(1i32)) as i64));
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print_fw_int(((self.obj_tab[(0i32) as usize].int1) as i64), 10i32);
                                                {
                                                    self.pdf_print(1229i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                {
                                                    let __for_end_12 = self.obj_ptr;
                                                    k = 1i32;
                                                    while k <= __for_end_12 {
                                                        {
                                                            if (!(self.obj_tab[(k) as usize].int2 > (((1i32).wrapping_neg()) as i64))) {
                                                                {
                                                                    self.pdf_print_fw_int(((self.obj_tab[(k) as usize].int1) as i64), 10i32);
                                                                    {
                                                                        self.pdf_print(1230i32);
                                                                        {
                                                                            {
                                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_os_get_os_buf(1i32);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                        self.overflow(1004i32, pdf_op_buf_size);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                            self.pdf_flush();
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            {
                                                                                self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    self.pdf_print_fw_int(self.obj_tab[(k) as usize].int2, 10i32);
                                                                    {
                                                                        self.pdf_print(1231i32);
                                                                        {
                                                                            {
                                                                                if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                    self.pdf_os_get_os_buf(1i32);
                                                                                } else {
                                                                                    if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                                        self.overflow(1004i32, pdf_op_buf_size);
                                                                                    } else {
                                                                                        if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                                            self.pdf_flush();
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            {
                                                                                self.pdf_buf_set(self.pdf_ptr, 10i32);
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
                                        // §815
                                        if (!self.pdf_os_enable) {
                                            {
                                                {
                                                    self.pdf_print(1239i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.pdf_print(1240i32);
                                                self.pdf_int_entry_ln(1234i32, (self.sys_obj_ptr).wrapping_add(1i32));
                                                self.pdf_indirect_ln(1237i32, root);
                                                if (self.eqtb[((629113i32) - 1) as usize].int() == 0i32) {
                                                    self.pdf_indirect_ln(1238i32, self.sys_obj_ptr);
                                                }
                                                if (self.pdf_trailer_toks != 0i32) {
                                                    {
                                                        self.pdf_print_toks_ln(self.pdf_trailer_toks);
                                                        {
                                                            self.delete_token_ref(self.pdf_trailer_toks);
                                                            self.pdf_trailer_toks = 0i32;
                                                        }
                                                    }
                                                }
                                                if (self.pdf_trailer_id_toks != 0i32) {
                                                    self.print_ID_alt(self.pdf_trailer_id_toks);
                                                } else {
                                                    self.print_ID(self.output_file_name);
                                                }
                                                {
                                                    self.pdf_print(1241i32);
                                                    {
                                                        {
                                                            if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_os_get_os_buf(1i32);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                    self.overflow(1004i32, pdf_op_buf_size);
                                                                } else {
                                                                    if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                        self.pdf_flush();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        {
                                            self.pdf_print(1242i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        if self.pdf_os_enable {
                                            {
                                                self.pdf_print_int(self.obj_tab[(self.sys_obj_ptr) as usize].int2);
                                                {
                                                    {
                                                        if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                            self.pdf_os_get_os_buf(1i32);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                                self.overflow(1004i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, 10i32);
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
                                                                self.overflow(1004i32, pdf_op_buf_size);
                                                            } else {
                                                                if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                    self.pdf_flush();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    {
                                                        self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                        {
                                            self.pdf_print(1243i32);
                                            {
                                                {
                                                    if (self.pdf_os_mode && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                        self.pdf_os_get_os_buf(1i32);
                                                    } else {
                                                        if ((!self.pdf_os_mode) && (1i32 > self.pdf_buf_size)) {
                                                            self.overflow(1004i32, pdf_op_buf_size);
                                                        } else {
                                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                                self.pdf_flush();
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        // §794
                                        self.pdf_flush();
                                        self.print_nl(999i32);
                                        self.print_file_name(0i32, self.output_file_name, 0i32);
                                        self.print(288i32);
                                        self.print_int(((self.total_pages) as i64));
                                        self.print(1000i32);
                                        if (self.total_pages != 1i32) {
                                            self.print_char(115i32);
                                        }
                                        self.print(1001i32);
                                        self.print_int((self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64)));
                                        self.print(1002i32);
                                    }
                                }
                                self.libpdffinish();
                                if (self.fixed_pdf_draftmode == 0i32) {
                                    { let mut __f0 = ::core::mem::take(&mut self.pdf_file); let __r = self.b_close(&mut __f0); self.pdf_file = __f0; __r };
                                } else {
                                    self.pdf_warning(0i32, 1191i32, true, true);
                                }
                            }
                        }
                        // §1513
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
                // §670
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
                    self.print_nl(998i32);
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
                        self.dvi_four(self.eqtb[((629035i32) - 1) as usize].int());
                        self.dvi_four(self.max_v);
                        self.dvi_four(self.max_h);
                        {
                            { let __ix2305 = self.dvi_ptr; let __v2306 = (self.max_push / 256i32); self.dvi_buf[(__ix2305) as usize] = __v2306; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        {
                            { let __ix2307 = self.dvi_ptr; let __v2308 = (self.max_push % 256i32); self.dvi_buf[(__ix2307) as usize] = __v2308; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        {
                            { let __ix2309 = self.dvi_ptr; let __v2310 = ((self.total_pages / 256i32) % 256i32); self.dvi_buf[(__ix2309) as usize] = __v2310; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        {
                            { let __ix2311 = self.dvi_ptr; let __v2312 = (self.total_pages % 256i32); self.dvi_buf[(__ix2311) as usize] = __v2312; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        // §671
                        while (self.font_ptr > 0i32) {
                            {
                                if self.font_used[(self.font_ptr) as usize] {
                                    self.dvi_font_def(self.font_ptr);
                                }
                                self.font_ptr = (self.font_ptr).wrapping_sub(1i32);
                            }
                        }
                        // §670
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
                        // §626
                        if (self.dvi_limit == self.half_buf) {
                            self.write_dvi(self.half_buf, (dvi_buf_size).wrapping_sub(1i32));
                        }
                        if (self.dvi_ptr > 0i32) {
                            self.write_dvi(0i32, (self.dvi_ptr).wrapping_sub(1i32));
                        }
                        // §670
                        self.print_nl(999i32);
                        self.slow_print(self.output_file_name);
                        self.print(288i32);
                        self.print_int(((self.total_pages) as i64));
                        self.print(1000i32);
                        if (self.total_pages != 1i32) {
                            self.print_char(115i32);
                        }
                        self.print(1001i32);
                        self.print_int((((self.dvi_offset).wrapping_add(self.dvi_ptr)) as i64));
                        self.print(1002i32);
                        { let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.b_close(&mut __f0); self.dvi_file = __f0; __r };
                    }
                }
            }
        }
        // §1513
        if self.log_opened {
            {
                {
                    crate::system::wr_ln(&mut self.log_file);
                }
                { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_close(&mut __f0); self.log_file = __f0; __r };
                self.selector = (self.selector).wrapping_sub(2i32);
                if (self.selector == 17i32) {
                    {
                        self.print_nl(1704i32);
                        self.slow_print(self.log_name);
                        self.print_char(46i32);
                    }
                }
            }
        }
        self.print_ln();
    }

    /// We get to the `final_cleanup` routine when \.{\\end} or \.{\\dump} has
    /// been scanned and `its_all_over`\kern-2pt.
    /// @<Last-minute...
    // §1515
    pub fn final_cleanup(&mut self) {
        let mut c: small_number = 0; // §1515
        'l_exit_f: {
            c = self.cur_chr;
            if (c != 1i32) {
                self.eqtb[((629067i32) - 1) as usize].set_int((1i32).wrapping_neg());
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
                    self.print(1705i32);
                    self.open_parens = (self.open_parens).wrapping_sub(1i32);
                }
            }
            if (self.cur_level > 1i32) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1706i32);
                    self.print(1707i32);
                    self.print_int((((self.cur_level).wrapping_sub(1i32)) as i64));
                    self.print_char(41i32);
                    if (self.eTeX_mode == 1i32) {
                        self.show_save_groups();
                    }
                }
            }
            while (self.cond_ptr != 0i32) {
                {
                    self.print_nl(40i32);
                    self.print_esc(1706i32);
                    self.print(1708i32);
                    self.print_cmd_chr(108i32, self.cur_if);
                    if (self.if_line != 0i32) {
                        {
                            self.print(1709i32);
                            self.print_int(((self.if_line) as i64));
                        }
                    }
                    self.print(1710i32);
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
                            self.print_nl(1711i32);
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
                    if (self.sa_root[(6i32) as usize] != 0i32) {
                        if self.do_marks(3i32, 0i32, self.sa_root[(6i32) as usize]) {
                            self.sa_root[(6i32) as usize] = 0i32;
                        }
                    }
                    {
                        let __for_end_5 = 3i32;
                        c = 2i32;
                        while c <= __for_end_5 {
                            self.flush_node_list(self.disc_ptr[((c) - 1) as usize]);
                            c = c.wrapping_add(1);
                        }
                    }
                    if (self.last_glue != 268435455i32) {
                        self.delete_glue_ref(self.last_glue);
                    }
                    self.store_fmt_file();
                    break 'l_exit_f;
                    self.print_nl(1712i32);
                    break 'l_exit_f;
                }
            }
        }
    }

    /// @<Last-minute...
    // §1516
    pub fn init_prim(&mut self) {
        self.no_new_control_sequence = false;
        self.first = 0i32;
        // §244
        self.primitive(389i32, 75i32, 626628i32);
        self.primitive(390i32, 75i32, 626629i32);
        self.primitive(391i32, 75i32, 626630i32);
        self.primitive(392i32, 75i32, 626631i32);
        self.primitive(393i32, 75i32, 626632i32);
        self.primitive(394i32, 75i32, 626633i32);
        self.primitive(395i32, 75i32, 626634i32);
        self.primitive(396i32, 75i32, 626635i32);
        self.primitive(397i32, 75i32, 626636i32);
        self.primitive(398i32, 75i32, 626637i32);
        self.primitive(399i32, 75i32, 626638i32);
        self.primitive(400i32, 75i32, 626639i32);
        self.primitive(401i32, 75i32, 626640i32);
        self.primitive(402i32, 75i32, 626641i32);
        self.primitive(403i32, 75i32, 626642i32);
        self.primitive(404i32, 76i32, 626643i32);
        self.primitive(405i32, 76i32, 626644i32);
        self.primitive(406i32, 76i32, 626645i32);
        // §248
        self.primitive(410i32, 72i32, 627159i32);
        self.primitive(411i32, 72i32, 627160i32);
        self.primitive(412i32, 72i32, 627161i32);
        self.primitive(413i32, 72i32, 627162i32);
        self.primitive(414i32, 72i32, 627163i32);
        self.primitive(415i32, 72i32, 627164i32);
        self.primitive(416i32, 72i32, 627165i32);
        self.primitive(417i32, 72i32, 627166i32);
        self.primitive(418i32, 72i32, 627167i32);
        self.primitive(419i32, 72i32, 627168i32);
        self.primitive(420i32, 72i32, 627169i32);
        self.primitive(421i32, 72i32, 627170i32);
        self.primitive(422i32, 72i32, 627171i32);
        // §256
        self.primitive(436i32, 73i32, 629018i32);
        self.primitive(437i32, 73i32, 629019i32);
        self.primitive(438i32, 73i32, 629020i32);
        self.primitive(439i32, 73i32, 629021i32);
        self.primitive(440i32, 73i32, 629022i32);
        self.primitive(441i32, 73i32, 629023i32);
        self.primitive(442i32, 73i32, 629024i32);
        self.primitive(443i32, 73i32, 629025i32);
        self.primitive(444i32, 73i32, 629026i32);
        self.primitive(445i32, 73i32, 629027i32);
        self.primitive(446i32, 73i32, 629028i32);
        self.primitive(447i32, 73i32, 629029i32);
        self.primitive(448i32, 73i32, 629030i32);
        self.primitive(449i32, 73i32, 629031i32);
        self.primitive(450i32, 73i32, 629032i32);
        self.primitive(451i32, 73i32, 629033i32);
        self.primitive(452i32, 73i32, 629034i32);
        self.primitive(453i32, 73i32, 629035i32);
        self.primitive(454i32, 73i32, 629036i32);
        self.primitive(455i32, 73i32, 629037i32);
        self.primitive(456i32, 73i32, 629038i32);
        self.primitive(457i32, 73i32, 629039i32);
        self.primitive(458i32, 73i32, 629040i32);
        self.primitive(459i32, 73i32, 629041i32);
        self.primitive(460i32, 73i32, 629042i32);
        self.primitive(461i32, 73i32, 629043i32);
        self.primitive(462i32, 73i32, 629044i32);
        self.primitive(463i32, 73i32, 629045i32);
        self.primitive(464i32, 73i32, 629046i32);
        self.primitive(465i32, 73i32, 629047i32);
        self.primitive(466i32, 73i32, 629048i32);
        self.primitive(467i32, 73i32, 629049i32);
        self.primitive(468i32, 73i32, 629050i32);
        self.primitive(469i32, 73i32, 629051i32);
        self.primitive(470i32, 73i32, 629052i32);
        self.primitive(471i32, 73i32, 629053i32);
        self.primitive(472i32, 73i32, 629054i32);
        self.primitive(473i32, 73i32, 629055i32);
        self.primitive(474i32, 73i32, 629056i32);
        self.primitive(475i32, 73i32, 629057i32);
        self.primitive(476i32, 73i32, 629058i32);
        self.primitive(477i32, 73i32, 629059i32);
        self.primitive(478i32, 73i32, 629060i32);
        self.primitive(479i32, 73i32, 629061i32);
        self.primitive(480i32, 73i32, 629062i32);
        self.primitive(481i32, 73i32, 629063i32);
        self.primitive(482i32, 73i32, 629064i32);
        self.primitive(483i32, 73i32, 629065i32);
        self.primitive(484i32, 73i32, 629066i32);
        self.primitive(485i32, 73i32, 629067i32);
        self.primitive(486i32, 73i32, 629068i32);
        self.primitive(487i32, 73i32, 629069i32);
        self.primitive(488i32, 73i32, 629070i32);
        self.primitive(489i32, 73i32, 629071i32);
        self.primitive(490i32, 73i32, 629072i32);
        if self.mltex_p {
            {
                self.mltex_enabled_p = true;
                if false {
                    self.primitive(491i32, 73i32, 629073i32);
                }
                self.primitive(492i32, 73i32, 629074i32);
                self.primitive(493i32, 73i32, 629075i32);
            }
        }
        self.primitive(494i32, 73i32, 629076i32);
        self.primitive(535i32, 103i32, 0i32);
        self.primitive(495i32, 73i32, 629077i32);
        self.primitive(496i32, 73i32, 629078i32);
        self.primitive(497i32, 73i32, 629079i32);
        self.primitive(498i32, 73i32, 629080i32);
        self.primitive(499i32, 73i32, 629100i32);
        self.primitive(500i32, 73i32, 629081i32);
        self.primitive(501i32, 73i32, 629082i32);
        self.primitive(502i32, 73i32, 629083i32);
        self.primitive(503i32, 73i32, 629084i32);
        self.primitive(504i32, 73i32, 629085i32);
        self.primitive(536i32, 73i32, 629089i32);
        self.primitive(505i32, 73i32, 629086i32);
        self.primitive(506i32, 73i32, 629087i32);
        self.primitive(507i32, 73i32, 629088i32);
        self.primitive(508i32, 73i32, 629089i32);
        self.primitive(509i32, 73i32, 629090i32);
        self.primitive(510i32, 73i32, 629091i32);
        self.primitive(511i32, 73i32, 629092i32);
        self.primitive(512i32, 73i32, 629093i32);
        self.primitive(513i32, 73i32, 629094i32);
        self.primitive(514i32, 73i32, 629095i32);
        self.primitive(515i32, 73i32, 629096i32);
        self.primitive(516i32, 73i32, 629097i32);
        self.primitive(517i32, 73i32, 629098i32);
        self.primitive(518i32, 73i32, 629099i32);
        self.primitive(519i32, 73i32, 629101i32);
        self.primitive(520i32, 73i32, 629102i32);
        self.primitive(521i32, 73i32, 629103i32);
        self.primitive(522i32, 73i32, 629104i32);
        self.primitive(523i32, 73i32, 629105i32);
        self.primitive(524i32, 73i32, 629106i32);
        self.primitive(525i32, 73i32, 629107i32);
        self.primitive(526i32, 73i32, 629108i32);
        self.primitive(527i32, 73i32, 629109i32);
        self.primitive(528i32, 73i32, 629110i32);
        self.primitive(529i32, 73i32, 629111i32);
        self.primitive(530i32, 73i32, 629112i32);
        self.primitive(531i32, 73i32, 629113i32);
        self.primitive(532i32, 73i32, 629114i32);
        self.primitive(533i32, 73i32, 629115i32);
        // §266
        self.primitive(539i32, 74i32, 629640i32);
        self.primitive(540i32, 74i32, 629641i32);
        self.primitive(541i32, 74i32, 629642i32);
        self.primitive(542i32, 74i32, 629643i32);
        self.primitive(543i32, 74i32, 629644i32);
        self.primitive(544i32, 74i32, 629645i32);
        self.primitive(545i32, 74i32, 629646i32);
        self.primitive(546i32, 74i32, 629647i32);
        self.primitive(547i32, 74i32, 629648i32);
        self.primitive(548i32, 74i32, 629649i32);
        self.primitive(549i32, 74i32, 629650i32);
        self.primitive(550i32, 74i32, 629651i32);
        self.primitive(551i32, 74i32, 629652i32);
        self.primitive(552i32, 74i32, 629653i32);
        self.primitive(553i32, 74i32, 629654i32);
        self.primitive(554i32, 74i32, 629655i32);
        self.primitive(555i32, 74i32, 629656i32);
        self.primitive(556i32, 74i32, 629657i32);
        self.primitive(557i32, 74i32, 629658i32);
        self.primitive(558i32, 74i32, 629659i32);
        self.primitive(559i32, 74i32, 629660i32);
        self.primitive(560i32, 74i32, 629661i32);
        self.primitive(561i32, 74i32, 629662i32);
        self.primitive(562i32, 74i32, 629663i32);
        self.primitive(563i32, 74i32, 629664i32);
        self.primitive(564i32, 74i32, 629665i32);
        self.primitive(565i32, 74i32, 629666i32);
        self.primitive(566i32, 74i32, 629667i32);
        self.primitive(567i32, 74i32, 629668i32);
        self.primitive(568i32, 74i32, 629669i32);
        self.primitive(569i32, 74i32, 629670i32);
        self.primitive(570i32, 74i32, 629671i32);
        self.primitive(571i32, 74i32, 629672i32);
        self.primitive(572i32, 74i32, 629673i32);
        // §287
        self.primitive(32i32, 64i32, 0i32);
        self.primitive(47i32, 44i32, 0i32);
        self.primitive(584i32, 45i32, 0i32);
        self.primitive(585i32, 90i32, 0i32);
        self.primitive(586i32, 40i32, 0i32);
        self.primitive(587i32, 41i32, 0i32);
        self.primitive(588i32, 61i32, 0i32);
        self.primitive(589i32, 16i32, 0i32);
        self.primitive(580i32, 110i32, 0i32);
        self.primitive(590i32, 15i32, 0i32);
        self.primitive(591i32, 92i32, 0i32);
        self.primitive(581i32, 67i32, 0i32);
        self.primitive(592i32, 62i32, 0i32);
        self.hash[((615516i32) - 514) as usize].set_rh(592i32);
        { let __v2313 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((615516i32) - 1) as usize] = __v2313; }
        self.primitive(593i32, 105i32, 0i32);
        self.primitive(594i32, 88i32, 0i32);
        self.primitive(595i32, 101i32, 0i32);
        self.primitive(596i32, 102i32, 0i32);
        self.primitive(597i32, 77i32, 0i32);
        self.primitive(598i32, 32i32, 0i32);
        self.primitive(599i32, 36i32, 0i32);
        self.primitive(600i32, 39i32, 0i32);
        self.primitive(340i32, 37i32, 0i32);
        self.primitive(363i32, 18i32, 0i32);
        self.primitive(601i32, 46i32, 0i32);
        self.primitive(602i32, 17i32, 0i32);
        self.primitive(603i32, 54i32, 0i32);
        self.primitive(604i32, 91i32, 0i32);
        self.primitive(605i32, 34i32, 0i32);
        self.primitive(606i32, 65i32, 0i32);
        self.primitive(607i32, 106i32, 0i32);
        self.primitive(577i32, 106i32, 1i32);
        self.primitive(345i32, 55i32, 0i32);
        self.primitive(608i32, 63i32, 0i32);
        self.primitive(609i32, 84i32, 627158i32);
        self.primitive(610i32, 42i32, 0i32);
        self.primitive(611i32, 80i32, 0i32);
        self.primitive(612i32, 66i32, 0i32);
        self.primitive(613i32, 96i32, 0i32);
        self.primitive(614i32, 0i32, 256i32);
        self.hash[((615521i32) - 514) as usize].set_rh(614i32);
        { let __v2314 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((615521i32) - 1) as usize] = __v2314; }
        self.primitive(615i32, 98i32, 0i32);
        self.primitive(616i32, 112i32, 0i32);
        self.primitive(423i32, 71i32, 0i32);
        self.primitive(364i32, 38i32, 0i32);
        self.primitive(617i32, 33i32, 0i32);
        self.primitive(618i32, 56i32, 0i32);
        self.primitive(619i32, 35i32, 0i32);
        // §356
        self.primitive(682i32, 13i32, 256i32);
        self.par_loc = self.cur_val;
        self.par_token = (4095i32).wrapping_add(self.par_loc);
        // §402
        self.primitive(716i32, 107i32, 0i32);
        self.primitive(717i32, 107i32, 1i32);
        // §410
        self.primitive(718i32, 113i32, 0i32);
        self.primitive(719i32, 113i32, 1i32);
        self.primitive(720i32, 113i32, 2i32);
        self.primitive(721i32, 113i32, 3i32);
        self.primitive(722i32, 113i32, 4i32);
        // §437
        self.primitive(537i32, 89i32, 0i32);
        self.primitive(574i32, 89i32, 1i32);
        self.primitive(408i32, 89i32, 2i32);
        self.primitive(409i32, 89i32, 3i32);
        // §442
        self.primitive(755i32, 79i32, 105i32);
        self.primitive(756i32, 79i32, 1i32);
        self.primitive(757i32, 82i32, 0i32);
        self.primitive(758i32, 82i32, 1i32);
        self.primitive(759i32, 83i32, 1i32);
        self.primitive(760i32, 83i32, 3i32);
        self.primitive(761i32, 83i32, 2i32);
        self.primitive(762i32, 70i32, 0i32);
        self.primitive(763i32, 70i32, 1i32);
        self.primitive(764i32, 70i32, 2i32);
        self.primitive(765i32, 70i32, 4i32);
        self.primitive(766i32, 70i32, 5i32);
        self.primitive(767i32, 70i32, 6i32);
        self.primitive(768i32, 70i32, 7i32);
        self.primitive(769i32, 70i32, 8i32);
        self.primitive(770i32, 70i32, 9i32);
        self.primitive(771i32, 70i32, 10i32);
        self.primitive(772i32, 70i32, 11i32);
        self.primitive(773i32, 70i32, 12i32);
        self.primitive(774i32, 70i32, 13i32);
        self.primitive(775i32, 70i32, 14i32);
        self.primitive(776i32, 70i32, 15i32);
        self.primitive(777i32, 70i32, 16i32);
        self.primitive(778i32, 70i32, 17i32);
        self.primitive(779i32, 70i32, 18i32);
        self.primitive(780i32, 70i32, 19i32);
        // §494
        self.primitive(839i32, 111i32, 0i32);
        self.primitive(840i32, 111i32, 1i32);
        self.primitive(841i32, 111i32, 2i32);
        self.primitive(842i32, 111i32, 3i32);
        self.primitive(843i32, 111i32, 4i32);
        self.primitive(844i32, 111i32, 6i32);
        self.primitive(845i32, 111i32, 7i32);
        self.primitive(846i32, 111i32, 8i32);
        self.primitive(847i32, 111i32, 9i32);
        self.primitive(848i32, 111i32, 10i32);
        self.primitive(849i32, 111i32, 11i32);
        self.primitive(850i32, 111i32, 12i32);
        self.primitive(851i32, 111i32, 16i32);
        self.primitive(852i32, 111i32, 17i32);
        self.primitive(853i32, 111i32, 13i32);
        self.primitive(854i32, 111i32, 14i32);
        self.primitive(855i32, 111i32, 15i32);
        self.primitive(856i32, 111i32, 20i32);
        self.primitive(857i32, 111i32, 21i32);
        self.primitive(858i32, 111i32, 22i32);
        self.primitive(859i32, 111i32, 23i32);
        self.primitive(860i32, 111i32, 24i32);
        self.primitive(861i32, 111i32, 25i32);
        self.primitive(862i32, 111i32, 26i32);
        self.primitive(863i32, 111i32, 27i32);
        self.primitive(864i32, 111i32, 28i32);
        self.primitive(865i32, 111i32, 18i32);
        self.primitive(866i32, 111i32, 19i32);
        self.primitive(867i32, 111i32, 29i32);
        self.primitive(868i32, 111i32, 30i32);
        self.primitive(869i32, 111i32, 33i32);
        self.primitive(870i32, 111i32, 31i32);
        self.primitive(871i32, 111i32, 32i32);
        // §513
        self.primitive(911i32, 108i32, 0i32);
        self.primitive(912i32, 108i32, 1i32);
        self.primitive(913i32, 108i32, 2i32);
        self.primitive(914i32, 108i32, 3i32);
        self.primitive(915i32, 108i32, 4i32);
        self.primitive(916i32, 108i32, 5i32);
        self.primitive(917i32, 108i32, 6i32);
        self.primitive(918i32, 108i32, 7i32);
        self.primitive(919i32, 108i32, 8i32);
        self.primitive(920i32, 108i32, 9i32);
        self.primitive(921i32, 108i32, 10i32);
        self.primitive(922i32, 108i32, 11i32);
        self.primitive(923i32, 108i32, 12i32);
        self.primitive(924i32, 108i32, 13i32);
        self.primitive(925i32, 108i32, 14i32);
        self.primitive(926i32, 108i32, 15i32);
        self.primitive(927i32, 108i32, 16i32);
        self.primitive(928i32, 108i32, 21i32);
        // §517
        self.primitive(930i32, 109i32, 2i32);
        self.hash[((615518i32) - 514) as usize].set_rh(930i32);
        { let __v2315 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((615518i32) - 1) as usize] = __v2315; }
        self.primitive(931i32, 109i32, 4i32);
        self.primitive(932i32, 109i32, 3i32);
        // §579
        self.primitive(959i32, 87i32, 0i32);
        self.hash[((617626i32) - 514) as usize].set_rh(959i32);
        { let __v2316 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((617626i32) - 1) as usize] = __v2316; }
        // §956
        self.primitive(1307i32, 4i32, 256i32);
        self.primitive(1308i32, 5i32, 257i32);
        self.hash[((615515i32) - 514) as usize].set_rh(1308i32);
        { let __v2317 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((615515i32) - 1) as usize] = __v2317; }
        self.primitive(1309i32, 5i32, 258i32);
        self.hash[((615519i32) - 514) as usize].set_rh(1310i32);
        self.hash[((615520i32) - 514) as usize].set_rh(1310i32);
        self.eqtb[((615520i32) - 1) as usize].set_hh_b0(9i32);
        self.eqtb[((615520i32) - 1) as usize].set_hh_rh(4999988i32);
        self.eqtb[((615520i32) - 1) as usize].set_hh_b1(1i32);
        { let __v2318 = self.eqtb[((615520i32) - 1) as usize]; self.eqtb[((615519i32) - 1) as usize] = __v2318; }
        self.eqtb[((615519i32) - 1) as usize].set_hh_b0(118i32);
        // §1160
        self.primitive(1385i32, 81i32, 0i32);
        self.primitive(1386i32, 81i32, 1i32);
        self.primitive(1387i32, 81i32, 2i32);
        self.primitive(1388i32, 81i32, 3i32);
        self.primitive(1389i32, 81i32, 4i32);
        self.primitive(1390i32, 81i32, 5i32);
        self.primitive(1391i32, 81i32, 6i32);
        self.primitive(1392i32, 81i32, 7i32);
        // §1230
        self.primitive(353i32, 14i32, 0i32);
        self.primitive(1438i32, 14i32, 1i32);
        // §1236
        self.primitive(1439i32, 26i32, 4i32);
        self.primitive(1440i32, 26i32, 0i32);
        self.primitive(1441i32, 26i32, 1i32);
        self.primitive(1442i32, 26i32, 2i32);
        self.primitive(1443i32, 26i32, 3i32);
        self.primitive(1444i32, 27i32, 4i32);
        self.primitive(1445i32, 27i32, 0i32);
        self.primitive(1446i32, 27i32, 1i32);
        self.primitive(1447i32, 27i32, 2i32);
        self.primitive(1448i32, 27i32, 3i32);
        self.primitive(346i32, 28i32, 5i32);
        self.primitive(324i32, 29i32, 1i32);
        self.primitive(352i32, 30i32, 99i32);
        // §1249
        self.primitive(1466i32, 21i32, 1i32);
        self.primitive(1467i32, 21i32, 0i32);
        self.primitive(1468i32, 22i32, 1i32);
        self.primitive(1469i32, 22i32, 0i32);
        self.primitive(425i32, 20i32, 0i32);
        self.primitive(1470i32, 20i32, 1i32);
        self.primitive(1471i32, 20i32, 2i32);
        self.primitive(1380i32, 20i32, 3i32);
        self.primitive(1472i32, 20i32, 4i32);
        self.primitive(1382i32, 20i32, 5i32);
        self.primitive(1473i32, 20i32, 109i32);
        self.primitive(1474i32, 31i32, 99i32);
        self.primitive(1475i32, 31i32, 100i32);
        self.primitive(1476i32, 31i32, 101i32);
        self.primitive(1477i32, 31i32, 102i32);
        // §1266
        self.primitive(1493i32, 43i32, 1i32);
        self.primitive(1494i32, 43i32, 0i32);
        self.primitive(1495i32, 43i32, 2i32);
        // §1285
        self.primitive(1505i32, 25i32, 12i32);
        self.primitive(1506i32, 25i32, 11i32);
        self.primitive(1507i32, 25i32, 10i32);
        self.primitive(1508i32, 23i32, 0i32);
        self.primitive(1509i32, 23i32, 1i32);
        self.primitive(1510i32, 24i32, 0i32);
        self.primitive(1511i32, 24i32, 1i32);
        // §1292
        self.primitive(45i32, 47i32, 1i32);
        self.primitive(361i32, 47i32, 0i32);
        // §1319
        self.primitive(1542i32, 48i32, 0i32);
        self.primitive(1543i32, 48i32, 1i32);
        // §1334
        self.primitive(1274i32, 50i32, 16i32);
        self.primitive(1275i32, 50i32, 17i32);
        self.primitive(1276i32, 50i32, 18i32);
        self.primitive(1277i32, 50i32, 19i32);
        self.primitive(1278i32, 50i32, 20i32);
        self.primitive(1279i32, 50i32, 21i32);
        self.primitive(1280i32, 50i32, 22i32);
        self.primitive(1281i32, 50i32, 23i32);
        self.primitive(1283i32, 50i32, 26i32);
        self.primitive(1282i32, 50i32, 27i32);
        self.primitive(1544i32, 51i32, 0i32);
        self.primitive(1287i32, 51i32, 1i32);
        self.primitive(1288i32, 51i32, 2i32);
        // §1347
        self.primitive(1269i32, 53i32, 0i32);
        self.primitive(1270i32, 53i32, 2i32);
        self.primitive(1271i32, 53i32, 4i32);
        self.primitive(1272i32, 53i32, 6i32);
        // §1356
        self.primitive(1562i32, 52i32, 0i32);
        self.primitive(1563i32, 52i32, 1i32);
        self.primitive(1564i32, 52i32, 2i32);
        self.primitive(1565i32, 52i32, 3i32);
        self.primitive(1566i32, 52i32, 4i32);
        self.primitive(1567i32, 52i32, 5i32);
        // §1366
        self.primitive(1284i32, 49i32, 30i32);
        self.primitive(1285i32, 49i32, 31i32);
        self.hash[((615517i32) - 514) as usize].set_rh(1285i32);
        { let __v2319 = self.eqtb[((self.cur_val) - 1) as usize]; self.eqtb[((615517i32) - 1) as usize] = __v2319; }
        // §1386
        self.primitive(1587i32, 93i32, 1i32);
        self.primitive(1588i32, 93i32, 2i32);
        self.primitive(1589i32, 93i32, 4i32);
        self.primitive(1590i32, 97i32, 0i32);
        self.primitive(1591i32, 97i32, 1i32);
        self.primitive(1592i32, 97i32, 2i32);
        self.primitive(1593i32, 97i32, 3i32);
        // §1397
        self.primitive(1610i32, 94i32, 0i32);
        self.primitive(1611i32, 94i32, 1i32);
        // §1400
        self.primitive(1612i32, 95i32, 0i32);
        self.primitive(1613i32, 95i32, 1i32);
        self.primitive(1614i32, 95i32, 2i32);
        self.primitive(1615i32, 95i32, 3i32);
        self.primitive(1616i32, 95i32, 4i32);
        self.primitive(1617i32, 95i32, 5i32);
        self.primitive(1618i32, 95i32, 6i32);
        // §1408
        self.primitive(431i32, 85i32, 627738i32);
        self.primitive(435i32, 85i32, 628762i32);
        self.primitive(432i32, 85i32, 627994i32);
        self.primitive(433i32, 85i32, 628250i32);
        self.primitive(434i32, 85i32, 628506i32);
        self.primitive(538i32, 85i32, 629384i32);
        self.primitive(428i32, 86i32, 627690i32);
        self.primitive(429i32, 86i32, 627706i32);
        self.primitive(430i32, 86i32, 627722i32);
        // §1428
        self.primitive(1354i32, 99i32, 0i32);
        self.primitive(1366i32, 99i32, 1i32);
        // §1432
        self.primitive(1636i32, 78i32, 0i32);
        self.primitive(1637i32, 78i32, 1i32);
        self.primitive(1638i32, 78i32, 2i32);
        self.primitive(1639i32, 78i32, 3i32);
        self.primitive(1640i32, 78i32, 4i32);
        self.primitive(1641i32, 78i32, 5i32);
        self.primitive(1642i32, 78i32, 7i32);
        self.primitive(1643i32, 78i32, 8i32);
        self.primitive(1644i32, 78i32, 9i32);
        self.primitive(1645i32, 78i32, 10i32);
        self.primitive(1646i32, 78i32, 11i32);
        self.primitive(1647i32, 78i32, 6i32);
        // §1440
        self.primitive(276i32, 100i32, 0i32);
        self.primitive(277i32, 100i32, 1i32);
        self.primitive(278i32, 100i32, 2i32);
        self.primitive(1655i32, 100i32, 3i32);
        // §1450
        self.primitive(1656i32, 60i32, 1i32);
        self.primitive(1657i32, 60i32, 0i32);
        // §1455
        self.primitive(1658i32, 58i32, 0i32);
        self.primitive(1659i32, 58i32, 1i32);
        // §1464
        self.primitive(1665i32, 57i32, 627994i32);
        self.primitive(1666i32, 57i32, 628250i32);
        // §1469
        self.primitive(1667i32, 19i32, 0i32);
        self.primitive(1668i32, 19i32, 1i32);
        self.primitive(1669i32, 19i32, 2i32);
        self.primitive(1670i32, 19i32, 3i32);
        // §1524
        self.primitive(1714i32, 59i32, 0i32);
        self.primitive(678i32, 59i32, 1i32);
        self.write_loc = self.cur_val;
        self.primitive(1715i32, 59i32, 2i32);
        self.primitive(1716i32, 59i32, 3i32);
        self.primitive(1717i32, 59i32, 5i32);
        self.primitive(1718i32, 59i32, 6i32);
        self.primitive(1719i32, 59i32, 7i32);
        self.primitive(1131i32, 59i32, 40i32);
        self.primitive(1720i32, 59i32, 41i32);
        self.primitive(1721i32, 59i32, 42i32);
        self.primitive(1722i32, 59i32, 43i32);
        self.primitive(1723i32, 59i32, 9i32);
        self.primitive(1724i32, 59i32, 10i32);
        self.primitive(1725i32, 59i32, 11i32);
        self.primitive(1726i32, 59i32, 12i32);
        self.primitive(1727i32, 59i32, 13i32);
        self.primitive(1728i32, 59i32, 14i32);
        self.primitive(1729i32, 59i32, 15i32);
        self.primitive(1730i32, 59i32, 16i32);
        self.primitive(1731i32, 59i32, 17i32);
        self.primitive(1732i32, 59i32, 18i32);
        self.primitive(1733i32, 59i32, 19i32);
        self.primitive(1734i32, 59i32, 20i32);
        self.primitive(1735i32, 59i32, 21i32);
        self.primitive(1736i32, 59i32, 22i32);
        self.primitive(1737i32, 59i32, 23i32);
        self.primitive(1738i32, 59i32, 36i32);
        self.primitive(1739i32, 59i32, 37i32);
        self.primitive(1740i32, 59i32, 38i32);
        self.primitive(1741i32, 59i32, 24i32);
        self.primitive(1742i32, 59i32, 25i32);
        self.primitive(1743i32, 59i32, 26i32);
        self.primitive(1744i32, 59i32, 28i32);
        self.primitive(1745i32, 59i32, 27i32);
        self.primitive(1746i32, 59i32, 29i32);
        self.primitive(1747i32, 59i32, 30i32);
        self.primitive(1748i32, 59i32, 31i32);
        self.primitive(1749i32, 59i32, 32i32);
        self.primitive(1750i32, 59i32, 33i32);
        self.primitive(1751i32, 59i32, 35i32);
        self.primitive(1752i32, 59i32, 34i32);
        self.primitive(1753i32, 59i32, 39i32);
        self.primitive(1754i32, 59i32, 44i32);
        self.primitive(1755i32, 59i32, 45i32);
        self.primitive(1756i32, 59i32, 46i32);
        self.primitive(1757i32, 59i32, 47i32);
        self.primitive(1758i32, 59i32, 48i32);
        self.primitive(1759i32, 59i32, 49i32);
        self.primitive(1760i32, 59i32, 50i32);
        // §1882
        self.primitive(2070i32, 73i32, 629127i32);
        // §1516
        self.no_new_control_sequence = true;
    }

}
