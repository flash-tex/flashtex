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
    /// When calculating the natural width, `w`, of the final line preceding
    /// the display, we may have to copy all or part of its hlist.  We copy,
    /// however, only those parts of the original list that are relevant for the
    /// computation of `pre_display_size`.
    /// @<Declare subprocedures for `init_math`
    // §1544
    pub fn just_copy(&mut self, mut p: halfword, mut h: halfword, mut t: halfword) {
        let mut r: halfword = 0; // §1544
        let mut words: i32 = 0; // §1544
        while (p != (268435455i32).wrapping_neg()) {
            {
                'l_not_found_f: {
                    'l_found_f: {
                        words = 1i32;
                        if (p >= self.hi_mem_min) {
                            r = self.get_avail();
                        } else {
                            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                hlist_node | vlist_node => {
                                    {
                                        r = self.get_node(box_node_size);
                                        // §1711
                                        {
                                            let __v1418 = self.mem
                                                [crate::ix::U(((p).wrapping_add(7i32)) as usize)]
                                            .hh()
                                            .lh();
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(7i32)) as usize)]
                                            .set_hh_lh(__v1418);
                                        }
                                        {
                                            let __v1419 = self.mem
                                                [crate::ix::U(((p).wrapping_add(7i32)) as usize)]
                                            .hh()
                                            .rh();
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(7i32)) as usize)]
                                            .set_hh_rh(__v1419);
                                        }
                                        // §1544
                                        {
                                            let __v1420 = self.mem
                                                [crate::ix::U(((p).wrapping_add(6i32)) as usize)];
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(6i32)) as usize)] =
                                                __v1420;
                                        }
                                        {
                                            let __v1421 = self.mem
                                                [crate::ix::U(((p).wrapping_add(5i32)) as usize)];
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(5i32)) as usize)] =
                                                __v1421;
                                        }
                                        words = 5i32;
                                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)]
                                            .set_hh_rh((268435455i32).wrapping_neg());
                                    }
                                }
                                rule_node => {
                                    r = self.get_node(rule_node_size);
                                    words = rule_node_size;
                                }
                                ligature_node => {
                                    r = self.get_avail();
                                    {
                                        let __v1422 = self.mem
                                            [crate::ix::U(((p).wrapping_add(1i32)) as usize)];
                                        self.mem[crate::ix::U((r) as usize)] = __v1422;
                                    }
                                    break 'l_found_f;
                                }
                                kern_node | math_node => {
                                    words = medium_node_size;
                                    r = self.get_node(words);
                                }
                                glue_node => {
                                    {
                                        r = self.get_node(medium_node_size);
                                        {
                                            let __ix1423 = self.mem
                                                [crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                            .hh()
                                            .lh();
                                            let __v1424 = (self.mem[crate::ix::U(
                                                (self.mem[crate::ix::U(
                                                    ((p).wrapping_add(1i32)) as usize,
                                                )]
                                                .hh()
                                                .lh())
                                                    as usize,
                                            )]
                                            .hh()
                                            .rh())
                                            .wrapping_add(1i32);
                                            self.mem[crate::ix::U((__ix1423) as usize)]
                                                .set_hh_rh(__v1424);
                                        }
                                        // §1713
                                        {
                                            let __v1425 = self.mem
                                                [crate::ix::U(((p).wrapping_add(2i32)) as usize)]
                                            .hh()
                                            .lh();
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                            .set_hh_lh(__v1425);
                                        }
                                        {
                                            let __v1426 = self.mem
                                                [crate::ix::U(((p).wrapping_add(2i32)) as usize)]
                                            .hh()
                                            .rh();
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                            .set_hh_rh(__v1426);
                                        }
                                        // §1544
                                        {
                                            let __v1427 = self.mem
                                                [crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                            .hh()
                                            .lh();
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                                            .set_hh_lh(__v1427);
                                        }
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                                            .set_hh_rh((268435455i32).wrapping_neg());
                                    }
                                }
                                whatsit_node => {
                                    // §1417
                                    match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                        open_node => {
                                            r = self.get_node(open_node_size);
                                            words = open_node_size;
                                        }
                                        write_node | special_node | latespecial_node => {
                                            r = self.get_node(write_node_size);
                                            {
                                                let __ix1428 = self.mem[crate::ix::U(
                                                    ((p).wrapping_add(1i32)) as usize,
                                                )]
                                                .hh()
                                                .rh();
                                                let __v1429 = (self.mem[crate::ix::U(
                                                    (self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .hh()
                                                    .rh())
                                                        as usize,
                                                )]
                                                .hh()
                                                .lh())
                                                .wrapping_add(1i32);
                                                self.mem[crate::ix::U((__ix1428) as usize)]
                                                    .set_hh_lh(__v1429);
                                            }
                                            words = write_node_size;
                                        }
                                        close_node | language_node => {
                                            r = self.get_node(small_node_size);
                                            words = small_node_size;
                                        }
                                        native_word_node | native_word_node_AT => {
                                            words = self.mem
                                                [crate::ix::U(((p).wrapping_add(4i32)) as usize)]
                                            .qqqq()
                                            .b0();
                                            r = self.get_node(words);
                                            while (words > 0i32) {
                                                {
                                                    words = (words).wrapping_sub(1i32);
                                                    {
                                                        let __v1430 = self.mem[crate::ix::U(
                                                            ((p).wrapping_add(words)) as usize,
                                                        )];
                                                        self.mem[crate::ix::U(
                                                            ((r).wrapping_add(words)) as usize,
                                                        )] = __v1430;
                                                    }
                                                }
                                            }
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(5i32)) as usize)]
                                            .set_int(null_ptr);
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(4i32)) as usize)]
                                            .set_qqqq_b3(0i32);
                                            self.copy_native_glyph_info(p, r);
                                        }
                                        glyph_node => {
                                            r = self.get_node(glyph_node_size);
                                            words = glyph_node_size;
                                        }
                                        pic_node | pdf_node => {
                                            words = (pic_node_size).wrapping_add(
                                                ((self.mem[crate::ix::U(
                                                    ((p).wrapping_add(4i32)) as usize,
                                                )]
                                                .hh()
                                                .b0())
                                                .wrapping_add(7i32)
                                                    / 8i32),
                                            );
                                            r = self.get_node(words);
                                        }
                                        pdf_save_pos_node => {
                                            r = self.get_node(small_node_size);
                                        }
                                        _ => {
                                            self.confusion(66758i32);
                                        }
                                    }
                                }
                                _ => {
                                    // §1544
                                    break 'l_not_found_f;
                                }
                            }
                        }
                        while (words > 0i32) {
                            {
                                words = (words).wrapping_sub(1i32);
                                {
                                    let __v1431 =
                                        self.mem[crate::ix::U(((p).wrapping_add(words)) as usize)];
                                    self.mem[crate::ix::U(((r).wrapping_add(words)) as usize)] =
                                        __v1431;
                                }
                            }
                        }
                    }
                    self.mem[crate::ix::U((h) as usize)].set_hh_rh(r);
                    h = r;
                }
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        self.mem[crate::ix::U((h) as usize)].set_hh_rh(t);
    }

    /// @<Declare subprocedures for `init_math`
    // §1549
    pub fn just_reverse(&mut self, mut p: halfword) {
        let mut l: halfword = 0; // §1549
        let mut t: halfword = 0; // §1549
        let mut q: halfword = 0; // §1549
        let mut m: halfword = 0; // §1549
        let mut n: halfword = 0; // §1549
        'l_done_f: {
            m = (268435455i32).wrapping_neg();
            n = (268435455i32).wrapping_neg();
            if (self.mem[crate::ix::U((temp_head) as usize)].hh().rh()
                == (268435455i32).wrapping_neg())
            {
                {
                    self.just_copy(
                        self.mem[crate::ix::U((p) as usize)].hh().rh(),
                        temp_head,
                        (268435455i32).wrapping_neg(),
                    );
                    q = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                }
            } else {
                {
                    q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                    self.flush_node_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
                }
            }
            t = self.new_edge(self.cur_dir, 0i32);
            l = t;
            self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
            while (q != (268435455i32).wrapping_neg()) {
                if (q >= self.hi_mem_min) {
                    loop {
                        p = q;
                        q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                        l = p;
                        if (!(q >= self.hi_mem_min)) {
                            break;
                        }
                    }
                } else {
                    {
                        p = q;
                        q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == math_node) {
                            // §1550
                            if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh()
                                    != ((L_code).wrapping_mul(
                                        (self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code),
                                    ))
                                    .wrapping_add(3i32))
                                {
                                    {
                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                        self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                    }
                                } else {
                                    {
                                        {
                                            self.temp_ptr = self.LR_ptr;
                                            self.LR_ptr = self.mem
                                                [crate::ix::U((self.temp_ptr) as usize)]
                                            .hh()
                                            .rh();
                                            {
                                                {
                                                    let __ix1432 = self.temp_ptr;
                                                    let __v1433 = self.avail;
                                                    self.mem[crate::ix::U((__ix1432) as usize)]
                                                        .set_hh_rh(__v1433);
                                                }
                                                self.avail = self.temp_ptr;
                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                            }
                                        }
                                        if (n > (268435455i32).wrapping_neg()) {
                                            {
                                                n = (n).wrapping_sub(1i32);
                                                {
                                                    let __v1434 = (self.mem
                                                        [crate::ix::U((p) as usize)]
                                                    .hh()
                                                    .b1())
                                                    .wrapping_sub(1i32);
                                                    self.mem[crate::ix::U((p) as usize)]
                                                        .set_hh_b1(__v1434);
                                                }
                                            }
                                        } else {
                                            {
                                                if (m > (268435455i32).wrapping_neg()) {
                                                    m = (m).wrapping_sub(1i32);
                                                } else {
                                                    {
                                                        {
                                                            let __v1435 = self.mem[crate::ix::U(
                                                                ((p).wrapping_add(1i32)) as usize,
                                                            )]
                                                            .int();
                                                            self.mem[crate::ix::U(
                                                                ((t).wrapping_add(1i32)) as usize,
                                                            )]
                                                            .set_int(__v1435);
                                                        }
                                                        self.mem[crate::ix::U((t) as usize)]
                                                            .set_hh_rh(q);
                                                        self.free_node(p, medium_node_size);
                                                        break 'l_done_f;
                                                    }
                                                }
                                                self.mem[crate::ix::U((p) as usize)]
                                                    .set_hh_b0(kern_node);
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    {
                                        self.temp_ptr = self.get_avail();
                                        {
                                            let __ix1436 = self.temp_ptr;
                                            let __v1437 = ((L_code).wrapping_mul(
                                                (self.mem[crate::ix::U((p) as usize)].hh().b1()
                                                    / L_code),
                                            ))
                                            .wrapping_add(3i32);
                                            self.mem[crate::ix::U((__ix1436) as usize)]
                                                .set_hh_lh(__v1437);
                                        }
                                        {
                                            let __ix1438 = self.temp_ptr;
                                            let __v1439 = self.LR_ptr;
                                            self.mem[crate::ix::U((__ix1438) as usize)]
                                                .set_hh_rh(__v1439);
                                        }
                                        self.LR_ptr = self.temp_ptr;
                                    }
                                    if ((n > (268435455i32).wrapping_neg())
                                        || ((self.mem[crate::ix::U((p) as usize)].hh().b1()
                                            / R_code)
                                            != self.cur_dir))
                                    {
                                        {
                                            n = (n).wrapping_add(1i32);
                                            {
                                                let __v1440 = (self.mem
                                                    [crate::ix::U((p) as usize)]
                                                .hh()
                                                .b1())
                                                .wrapping_add(1i32);
                                                self.mem[crate::ix::U((p) as usize)]
                                                    .set_hh_b1(__v1440);
                                            }
                                        }
                                    } else {
                                        {
                                            self.mem[crate::ix::U((p) as usize)]
                                                .set_hh_b0(kern_node);
                                            m = (m).wrapping_add(1i32);
                                        }
                                    }
                                }
                            }
                        }
                        // §1549
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                        l = p;
                    }
                }
            }
            break 'l_done_f;
            {
                let __v1441 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v1441);
            }
            self.mem[crate::ix::U((t) as usize)].set_hh_rh(q);
            self.free_node(p, small_node_size);
        }
        self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(l);
    }

    /// @<Declare act...
    // §1192
    pub fn init_math(&mut self) {
        let mut w: scaled = 0; // §1192
        let mut j: halfword = 0; // §1192
        let mut x: i32 = 0; // §1192
        let mut l: scaled = 0; // §1192
        let mut s: scaled = 0; // §1192
        let mut p: halfword = 0; // §1192
        let mut q: halfword = 0; // §1192
        let mut f: internal_font_number = 0; // §1192
        let mut n: i32 = 0; // §1192
        let mut v: scaled = 0; // §1192
        let mut d: scaled = 0; // §1192
        self.get_token();
        if ((self.cur_cmd == math_shift) && (self.cur_list.mode_field > 0i32)) {
            // §1199
            {
                j = (268435455i32).wrapping_neg();
                w = (1073741823i32).wrapping_neg();
                if (self.cur_list.head_field == self.cur_list.tail_field) {
                    // §1543
                    {
                        self.pop_nest();
                        // §1542
                        if (self.cur_list.eTeX_aux_field == (268435455i32).wrapping_neg()) {
                            x = 0i32;
                        } else {
                            if (self.mem[crate::ix::U((self.cur_list.eTeX_aux_field) as usize)]
                                .hh()
                                .lh()
                                >= R_code)
                            {
                                x = (1i32).wrapping_neg();
                            } else {
                                x = 1i32;
                            }
                        }
                    }
                } else {
                    // §1199
                    {
                        'l_done_f: {
                            self.line_break(true);
                            // §1545
                            if (self.eTeX_mode == 1i32) {
                                // §1551
                                {
                                    if (self.eqtb[crate::ix::U(((1205772i32) - 1) as usize)]
                                        .hh()
                                        .rh()
                                        == zero_glue)
                                    {
                                        j = self.new_kern(0i32);
                                    } else {
                                        j = self.new_param_glue(right_skip_code);
                                    }
                                    if (self.eqtb[crate::ix::U(((1205771i32) - 1) as usize)]
                                        .hh()
                                        .rh()
                                        == zero_glue)
                                    {
                                        p = self.new_kern(0i32);
                                    } else {
                                        p = self.new_param_glue(left_skip_code);
                                    }
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(j);
                                    j = self.new_null_box();
                                    {
                                        let __v1442 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(1i32)) as usize,
                                        )]
                                        .int();
                                        self.mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)]
                                            .set_int(__v1442);
                                    }
                                    {
                                        let __v1443 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(4i32)) as usize,
                                        )]
                                        .int();
                                        self.mem[crate::ix::U(((j).wrapping_add(4i32)) as usize)]
                                            .set_int(__v1443);
                                    }
                                    self.mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)]
                                        .set_hh_rh(p);
                                    {
                                        let __v1444 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(5i32)) as usize,
                                        )]
                                        .hh()
                                        .b1();
                                        self.mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)]
                                            .set_hh_b1(__v1444);
                                    }
                                    {
                                        let __v1445 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(5i32)) as usize,
                                        )]
                                        .hh()
                                        .b0();
                                        self.mem[crate::ix::U(((j).wrapping_add(5i32)) as usize)]
                                            .set_hh_b0(__v1445);
                                    }
                                    {
                                        let __v1446 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(6i32)) as usize,
                                        )]
                                        .gr();
                                        self.mem[crate::ix::U(((j).wrapping_add(6i32)) as usize)]
                                            .set_gr(__v1446);
                                    }
                                }
                            }
                            // §1545
                            v = self.mem
                                [crate::ix::U(((self.just_box).wrapping_add(4i32)) as usize)]
                            .int();
                            // §1542
                            if (self.cur_list.eTeX_aux_field == (268435455i32).wrapping_neg()) {
                                x = 0i32;
                            } else {
                                if (self.mem[crate::ix::U((self.cur_list.eTeX_aux_field) as usize)]
                                    .hh()
                                    .lh()
                                    >= R_code)
                                {
                                    x = (1i32).wrapping_neg();
                                } else {
                                    x = 1i32;
                                }
                            }
                            // §1545
                            if (x >= 0i32) {
                                {
                                    p = self.mem[crate::ix::U(
                                        ((self.just_box).wrapping_add(5i32)) as usize,
                                    )]
                                    .hh()
                                    .rh();
                                    self.mem[crate::ix::U((temp_head) as usize)]
                                        .set_hh_rh((268435455i32).wrapping_neg());
                                }
                            } else {
                                {
                                    v = ((v).wrapping_neg()).wrapping_sub(
                                        self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(1i32)) as usize,
                                        )]
                                        .int(),
                                    );
                                    p = self.new_math(0i32, begin_L_code);
                                    self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(p);
                                    {
                                        let __a1447_0 = self.mem[crate::ix::U(
                                            ((self.just_box).wrapping_add(5i32)) as usize,
                                        )]
                                        .hh()
                                        .rh();
                                        let __a1447_1 = p;
                                        let __a1447_2 = self.new_math(0i32, end_L_code);
                                        self.just_copy(__a1447_0, __a1447_1, __a1447_2)
                                    };
                                    self.cur_dir = right_to_left;
                                }
                            }
                            v = (v).wrapping_add(
                                (2i32).wrapping_mul(
                                    self.font_info[crate::ix::U(
                                        ((quad_code).wrapping_add(
                                            self.param_base[crate::ix::U(
                                                (self.eqtb
                                                    [crate::ix::U(((cur_font_loc) - 1) as usize)]
                                                .hh()
                                                .rh())
                                                    as usize,
                                            )],
                                        )) as usize,
                                    )]
                                    .int(),
                                ),
                            );
                            if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                                // §1520
                                {
                                    self.temp_ptr = self.get_avail();
                                    {
                                        let __ix1448 = self.temp_ptr;
                                        self.mem[crate::ix::U((__ix1448) as usize)]
                                            .set_hh_lh(before);
                                    }
                                    {
                                        let __ix1449 = self.temp_ptr;
                                        let __v1450 = self.LR_ptr;
                                        self.mem[crate::ix::U((__ix1449) as usize)]
                                            .set_hh_rh(__v1450);
                                    }
                                    self.LR_ptr = self.temp_ptr;
                                }
                            }
                            // §1200
                            while (p != (268435455i32).wrapping_neg()) {
                                {
                                    // goto labels: reswitch, found, not_found
                                    let mut __goto_1: i32 = 0;
                                    'l_dispatch_1: loop {
                                        if __goto_1 <= 0 {
                                            // §1201
                                            if (p >= self.hi_mem_min) {
                                                {
                                                    f = self.mem[crate::ix::U((p) as usize)]
                                                        .hh()
                                                        .b0();
                                                    d = self.font_info[crate::ix::U(
                                                        ((self.width_base
                                                            [crate::ix::U((f) as usize)])
                                                        .wrapping_add(
                                                            self.font_info[crate::ix::U(
                                                                ((self.char_base
                                                                    [crate::ix::U((f) as usize)])
                                                                .wrapping_add(
                                                                    self.mem[crate::ix::U(
                                                                        (p) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .b1(),
                                                                ))
                                                                    as usize,
                                                            )]
                                                            .qqqq()
                                                            .b0(),
                                                        ))
                                                            as usize,
                                                    )]
                                                    .int();
                                                    {
                                                        __goto_1 = 1;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                                hlist_node | vlist_node | rule_node => {
                                                    d = self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .int();
                                                    {
                                                        __goto_1 = 1;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                                ligature_node => {
                                                    // §692
                                                    {
                                                        {
                                                            let __v1451 = self.mem[crate::ix::U(
                                                                ((p).wrapping_add(1i32)) as usize,
                                                            )];
                                                            self.mem[crate::ix::U(
                                                                (lig_trick) as usize,
                                                            )] = __v1451;
                                                        }
                                                        {
                                                            let __v1452 = self.mem
                                                                [crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .rh();
                                                            self.mem[crate::ix::U(
                                                                (lig_trick) as usize,
                                                            )]
                                                            .set_hh_rh(__v1452);
                                                        }
                                                        p = lig_trick;
                                                        self.xtx_ligature_present = true;
                                                        {
                                                            __goto_1 = 0;
                                                            continue 'l_dispatch_1;
                                                        }
                                                    }
                                                }
                                                kern_node => {
                                                    // §1201
                                                    d = self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .int();
                                                }
                                                margin_kern_node => {
                                                    d = self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .int();
                                                }
                                                math_node => {
                                                    // §1547
                                                    {
                                                        d = self.mem[crate::ix::U(
                                                            ((p).wrapping_add(1i32)) as usize,
                                                        )]
                                                        .int();
                                                        if (self.eqtb[crate::ix::U(
                                                            ((7892339i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 0i32)
                                                        {
                                                            // §1548
                                                            if (((self.mem
                                                                [crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1())
                                                                % 2)
                                                                != 0)
                                                            {
                                                                {
                                                                    if (self.mem[crate::ix::U(
                                                                        (self.LR_ptr) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .lh()
                                                                        == ((L_code).wrapping_mul(
                                                                            (self.mem
                                                                                [crate::ix::U(
                                                                                    (p) as usize,
                                                                                )]
                                                                            .hh()
                                                                            .b1()
                                                                                / L_code),
                                                                        ))
                                                                        .wrapping_add(3i32))
                                                                    {
                                                                        {
                                                                            self.temp_ptr =
                                                                                self.LR_ptr;
                                                                            self.LR_ptr = self.mem
                                                                                [crate::ix::U(
                                                                                    (self.temp_ptr)
                                                                                        as usize,
                                                                                )]
                                                                            .hh()
                                                                            .rh();
                                                                            {
                                                                                {
                                                                                    let __ix1453 = self.temp_ptr;
                                                                                    let __v1454 =
                                                                                        self.avail;
                                                                                    self.mem[crate::ix::U((__ix1453) as usize)].set_hh_rh(__v1454);
                                                                                }
                                                                                self.avail =
                                                                                    self.temp_ptr;
                                                                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                            }
                                                                        }
                                                                    } else {
                                                                        if (self.mem[crate::ix::U(
                                                                            (p) as usize,
                                                                        )]
                                                                        .hh()
                                                                        .b1()
                                                                            > L_code)
                                                                        {
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
                                                                        self.temp_ptr =
                                                                            self.get_avail();
                                                                        {
                                                                            let __ix1455 =
                                                                                self.temp_ptr;
                                                                            let __v1456 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32);
                                                                            self.mem[crate::ix::U(
                                                                                (__ix1455) as usize,
                                                                            )]
                                                                            .set_hh_lh(__v1456);
                                                                        }
                                                                        {
                                                                            let __ix1457 =
                                                                                self.temp_ptr;
                                                                            let __v1458 =
                                                                                self.LR_ptr;
                                                                            self.mem[crate::ix::U(
                                                                                (__ix1457) as usize,
                                                                            )]
                                                                            .set_hh_rh(__v1458);
                                                                        }
                                                                        self.LR_ptr = self.temp_ptr;
                                                                    }
                                                                    if ((self.mem[crate::ix::U(
                                                                        (p) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .b1()
                                                                        / R_code)
                                                                        != self.cur_dir)
                                                                    {
                                                                        {
                                                                            self.just_reverse(p);
                                                                            p = temp_head;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            // §1547
                                                            if (self.mem
                                                                [crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1()
                                                                >= L_code)
                                                            {
                                                                {
                                                                    w = max_dimen;
                                                                    break 'l_done_f;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                edge_node => {
                                                    d = self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .int();
                                                    self.cur_dir = self.mem
                                                        [crate::ix::U((p) as usize)]
                                                    .hh()
                                                    .b1();
                                                }
                                                glue_node => {
                                                    // §1202
                                                    {
                                                        q = self.mem[crate::ix::U(
                                                            ((p).wrapping_add(1i32)) as usize,
                                                        )]
                                                        .hh()
                                                        .lh();
                                                        d = self.mem[crate::ix::U(
                                                            ((q).wrapping_add(1i32)) as usize,
                                                        )]
                                                        .int();
                                                        if (self.mem[crate::ix::U(
                                                            ((self.just_box).wrapping_add(5i32))
                                                                as usize,
                                                        )]
                                                        .hh()
                                                        .b0()
                                                            == stretching)
                                                        {
                                                            {
                                                                if ((self.mem[crate::ix::U(
                                                                    ((self.just_box)
                                                                        .wrapping_add(5i32))
                                                                        as usize,
                                                                )]
                                                                .hh()
                                                                .b1()
                                                                    == self.mem[crate::ix::U(
                                                                        (q) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .b0())
                                                                    && (self.mem[crate::ix::U(
                                                                        ((q).wrapping_add(2i32))
                                                                            as usize,
                                                                    )]
                                                                    .int()
                                                                        != 0i32))
                                                                {
                                                                    v = max_dimen;
                                                                }
                                                            }
                                                        } else {
                                                            if (self.mem[crate::ix::U(
                                                                ((self.just_box).wrapping_add(5i32))
                                                                    as usize,
                                                            )]
                                                            .hh()
                                                            .b0()
                                                                == shrinking)
                                                            {
                                                                {
                                                                    if ((self.mem[crate::ix::U(
                                                                        ((self.just_box)
                                                                            .wrapping_add(5i32))
                                                                            as usize,
                                                                    )]
                                                                    .hh()
                                                                    .b1()
                                                                        == self.mem[crate::ix::U(
                                                                            (q) as usize,
                                                                        )]
                                                                        .hh()
                                                                        .b1())
                                                                        && (self.mem[crate::ix::U(
                                                                            ((q).wrapping_add(3i32))
                                                                                as usize,
                                                                        )]
                                                                        .int()
                                                                            != 0i32))
                                                                    {
                                                                        v = max_dimen;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if (self.mem[crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1()
                                                            >= a_leaders)
                                                        {
                                                            {
                                                                __goto_1 = 1;
                                                                continue 'l_dispatch_1;
                                                            }
                                                        }
                                                    }
                                                }
                                                whatsit_node => {
                                                    // §1421
                                                    if (((((self.mem
                                                        [crate::ix::U((p) as usize)]
                                                    .hh()
                                                    .b1()
                                                        >= native_word_node)
                                                        && (self.mem
                                                            [crate::ix::U((p) as usize)]
                                                        .hh()
                                                        .b1()
                                                            <= native_word_node_AT))
                                                        || (self.mem[crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1()
                                                            == glyph_node))
                                                        || (self.mem[crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1()
                                                            == pic_node))
                                                        || (self.mem[crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .b1()
                                                            == pdf_node))
                                                    {
                                                        {
                                                            d = self.mem[crate::ix::U(
                                                                ((p).wrapping_add(1i32)) as usize,
                                                            )]
                                                            .int();
                                                            {
                                                                __goto_1 = 1;
                                                                continue 'l_dispatch_1;
                                                            }
                                                        }
                                                    } else {
                                                        d = 0i32;
                                                    }
                                                }
                                                _ => {
                                                    // §1201
                                                    d = 0i32;
                                                }
                                            }
                                            // §1200
                                            if (v < max_dimen) {
                                                v = (v).wrapping_add(d);
                                            }
                                            {
                                                __goto_1 = 2;
                                                continue 'l_dispatch_1;
                                            }
                                        }
                                        if __goto_1 <= 1 {
                                            // found
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
                                        if __goto_1 <= 2 {
                                            // not_found
                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                        }
                                        break 'l_dispatch_1;
                                    }
                                }
                            }
                        }
                        if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                            // §1546
                            {
                                while (self.LR_ptr != (268435455i32).wrapping_neg()) {
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = self.mem
                                            [crate::ix::U((self.temp_ptr) as usize)]
                                        .hh()
                                        .rh();
                                        {
                                            {
                                                let __ix1459 = self.temp_ptr;
                                                let __v1460 = self.avail;
                                                self.mem[crate::ix::U((__ix1459) as usize)]
                                                    .set_hh_rh(__v1460);
                                            }
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
                        self.flush_node_list(
                            self.mem[crate::ix::U((temp_head) as usize)].hh().rh(),
                        );
                    }
                }
                // §1203
                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)]
                    .hh()
                    .rh()
                    == (268435455i32).wrapping_neg())
                {
                    if ((self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() != 0i32)
                        && (((self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int() >= 0i32)
                            && ((self.cur_list.pg_field).wrapping_add(2i32)
                                > self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int()))
                            || ((self.cur_list.pg_field).wrapping_add(1i32)
                                < (self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].int())
                                    .wrapping_neg())))
                    {
                        {
                            l = (self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int())
                                .wrapping_sub(
                                    (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int())
                                        .wrapping_abs(),
                                );
                            if (self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int() > 0i32) {
                                s = self.eqtb[crate::ix::U(((9006737i32) - 1) as usize)].int();
                            } else {
                                s = 0i32;
                            }
                        }
                    } else {
                        {
                            l = self.eqtb[crate::ix::U(((9006723i32) - 1) as usize)].int();
                            s = 0i32;
                        }
                    }
                } else {
                    {
                        n = self.mem[crate::ix::U(
                            (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )]
                        .hh()
                        .lh();
                        if ((self.cur_list.pg_field).wrapping_add(2i32) >= n) {
                            p = (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)]
                                .hh()
                                .rh())
                            .wrapping_add((2i32).wrapping_mul(n));
                        } else {
                            p = (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)]
                                .hh()
                                .rh())
                            .wrapping_add(
                                (2i32).wrapping_mul((self.cur_list.pg_field).wrapping_add(2i32)),
                            );
                        }
                        s = self.mem[crate::ix::U(((p).wrapping_sub(1i32)) as usize)].int();
                        l = self.mem[crate::ix::U((p) as usize)].int();
                    }
                }
                // §1199
                self.push_math(math_shift_group);
                self.cur_list.mode_field = mmode;
                self.eq_word_define(7892308i32, (1i32).wrapping_neg());
                self.eq_word_define(9006733i32, w);
                self.cur_list.eTeX_aux_field = j;
                if (self.eTeX_mode == 1i32) {
                    self.eq_word_define(7892330i32, x);
                }
                self.eq_word_define(9006734i32, l);
                self.eq_word_define(9006735i32, s);
                if (self.eqtb[crate::ix::U(((every_display_loc) - 1) as usize)]
                    .hh()
                    .rh()
                    != (268435455i32).wrapping_neg())
                {
                    self.begin_token_list(
                        self.eqtb[crate::ix::U(((every_display_loc) - 1) as usize)]
                            .hh()
                            .rh(),
                        every_display_text,
                    );
                }
                if (self.nest_ptr == 1i32) {
                    self.build_page();
                }
            }
        } else {
            // §1192
            {
                self.back_input();
                // §1193
                {
                    self.push_math(math_shift_group);
                    self.eq_word_define(7892308i32, (1i32).wrapping_neg());
                    if (self.eqtb[crate::ix::U(((every_math_loc) - 1) as usize)]
                        .hh()
                        .rh()
                        != (268435455i32).wrapping_neg())
                    {
                        self.begin_token_list(
                            self.eqtb[crate::ix::U(((every_math_loc) - 1) as usize)]
                                .hh()
                                .rh(),
                            every_math_text,
                        );
                    }
                }
            }
        }
    }

    /// When \TeX\ is in display math mode, `cur_group=math_shift_group`,
    /// so it is not necessary for the `start_eq_no` procedure to test for
    /// this condition.
    /// @<Declare act...
    // §1196
    pub fn start_eq_no(&mut self) {
        {
            let __ix1461 = (self.save_ptr).wrapping_add(0i32);
            let __v1462 = self.cur_chr;
            self.save_stack[crate::ix::U((__ix1461) as usize)].set_int(__v1462);
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        // §1193
        {
            self.push_math(math_shift_group);
            self.eq_word_define(7892308i32, (1i32).wrapping_neg());
            if (self.eqtb[crate::ix::U(((every_math_loc) - 1) as usize)]
                .hh()
                .rh()
                != (268435455i32).wrapping_neg())
            {
                self.begin_token_list(
                    self.eqtb[crate::ix::U(((every_math_loc) - 1) as usize)]
                        .hh()
                        .rh(),
                    every_math_text,
                );
            }
        }
    }

    /// Recall that the `nucleus`, `subscr`, and `supscr` fields in a noad are
    /// broken down into subfields called `math_type` and either `info` or
    /// `(fam,character)`. The job of `scan_math` is to figure out what to place
    /// in one of these principal fields; it looks at the subformula that
    /// comes next in the input, and places an encoding of that subformula
    /// into a given word of `mem`.
    // §1205
    pub fn scan_math(&mut self, mut p: halfword) {
        let mut c: i32 = 0; // §1205
                            // goto labels: restart, reswitch, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                loop {
                    // §438
                    self.get_x_token();
                    if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                        break;
                    }
                }
            }
            if __goto_1 <= 1 {
                // reswitch
                // §1205
                match self.cur_cmd {
                    letter | other_char | char_given => {
                        {
                            c = self.eqtb[crate::ix::U(
                                (((math_code_base).wrapping_add(self.cur_chr)) - 1) as usize,
                            )]
                            .hh()
                            .rh();
                            if (self.math_char_field(c) == active_math_char) {
                                {
                                    // §1206
                                    {
                                        self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                        self.cur_cmd = self.eqtb
                                            [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                        .hh()
                                        .b0();
                                        self.cur_chr = self.eqtb
                                            [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                        .hh()
                                        .rh();
                                        self.x_token();
                                        self.back_input();
                                    }
                                    // §1205
                                    {
                                        __goto_1 = 0;
                                        continue 'l_dispatch_1;
                                    }
                                }
                            }
                        }
                    }
                    char_num => {
                        self.scan_char_num();
                        self.cur_chr = self.cur_val;
                        self.cur_cmd = char_given;
                        {
                            __goto_1 = 1;
                            continue 'l_dispatch_1;
                        }
                    }
                    math_char_num => {
                        if (self.cur_chr == 2i32) {
                            {
                                self.scan_math_class_int();
                                c = self.set_class_field(self.cur_val);
                                self.scan_math_fam_int();
                                c = (c).wrapping_add(self.set_family_field(self.cur_val));
                                self.scan_usv_num();
                                c = (c).wrapping_add(self.cur_val);
                            }
                        } else {
                            if (self.cur_chr == 1i32) {
                                {
                                    self.scan_xetex_math_char_int();
                                    c = self.cur_val;
                                }
                            } else {
                                {
                                    self.scan_fifteen_bit_int();
                                    c = ((self.set_class_field((self.cur_val / 4096i32)))
                                        .wrapping_add(self.set_family_field(
                                            ((self.cur_val % 4096i32) / 256i32),
                                        )))
                                    .wrapping_add((self.cur_val % 256i32));
                                }
                            }
                        }
                    }
                    math_given => {
                        c = ((self.set_class_field((self.cur_chr / 4096i32))).wrapping_add(
                            self.set_family_field(((self.cur_chr % 4096i32) / 256i32)),
                        ))
                        .wrapping_add((self.cur_chr % 256i32));
                    }
                    XeTeX_math_given => {
                        c = self.cur_chr;
                    }
                    delim_num => {
                        if (self.cur_chr == 1i32) {
                            {
                                self.scan_math_class_int();
                                c = self.set_class_field(self.cur_val);
                                self.scan_math_fam_int();
                                c = (c).wrapping_add(self.set_family_field(self.cur_val));
                                self.scan_usv_num();
                                c = (c).wrapping_add(self.cur_val);
                            }
                        } else {
                            {
                                self.scan_delimiter_int();
                                c = (self.cur_val / 4096i32);
                                c = ((self.set_class_field((c / 4096i32)))
                                    .wrapping_add(self.set_family_field(((c % 4096i32) / 256i32))))
                                .wrapping_add((c % 256i32));
                            }
                        }
                    }
                    _ => {
                        // §1207
                        {
                            self.back_input();
                            self.scan_left_brace();
                            {
                                let __ix1463 = (self.save_ptr).wrapping_add(0i32);
                                self.save_stack[crate::ix::U((__ix1463) as usize)].set_int(p);
                            }
                            self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                            self.push_math(math_group);
                            {
                                __goto_1 = 2;
                                continue 'l_dispatch_1;
                            }
                        }
                    }
                }
                // §1205
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(math_char);
                self.mem[crate::ix::U((p) as usize)].set_hh_b1((c % 65536i32));
                if ((self.math_class_field(c) == 7i32)
                    && ((self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int() >= 0i32)
                        && (self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int()
                            < number_math_families)))
                {
                    {
                        let __v1464 = self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int();
                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1464);
                    }
                } else {
                    {
                        let __v1465 = self.math_fam_field(c);
                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1465);
                    }
                }
                {
                    let __v1466 = (self.mem[crate::ix::U((p) as usize)].hh().b0())
                        .wrapping_add((self.math_char_field(c) / 65536i32).wrapping_mul(256i32));
                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1466);
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
    // §1209
    pub fn set_math_char(&mut self, mut c: i32) {
        let mut p: halfword = 0; // §1209
        let mut ch: UnicodeScalar = 0; // §1209
        if (self.math_char_field(c) == active_math_char) {
            // §1206
            {
                self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                self.cur_cmd = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)]
                    .hh()
                    .b0();
                self.cur_chr = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)]
                    .hh()
                    .rh();
                self.x_token();
                self.back_input();
            }
        } else {
            // §1209
            {
                p = self.new_noad();
                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(math_char);
                ch = self.math_char_field(c);
                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                    .set_hh_b1((ch % 65536i32));
                {
                    let __v1467 = self.math_fam_field(c);
                    self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b0(__v1467);
                }
                if (self.math_class_field(c) == 7i32) {
                    {
                        if ((self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int()
                                < number_math_families))
                        {
                            {
                                let __v1468 =
                                    self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int();
                                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                    .set_hh_b0(__v1468);
                            }
                        }
                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(ord_noad);
                    }
                } else {
                    {
                        let __v1469 = (ord_noad).wrapping_add(self.math_class_field(c));
                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v1469);
                    }
                }
                {
                    let __v1470 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                        .hh()
                        .b0())
                    .wrapping_add((ch / 65536i32).wrapping_mul(256i32));
                    self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_b0(__v1470);
                }
                {
                    let __ix1471 = self.cur_list.tail_field;
                    self.mem[crate::ix::U((__ix1471) as usize)].set_hh_rh(p);
                }
                self.cur_list.tail_field = p;
            }
        }
    }

    /// @<Declare act...
    // §1213
    pub fn math_limit_switch(&mut self) {
        'l_exit_f: {
            if (self.cur_list.head_field != self.cur_list.tail_field) {
                if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .b0()
                    == op_noad)
                {
                    {
                        {
                            let __ix1472 = self.cur_list.tail_field;
                            let __v1473 = self.cur_chr;
                            self.mem[crate::ix::U((__ix1472) as usize)].set_hh_b1(__v1473);
                        }
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (self.interaction == error_stop_mode) {}
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(65544i32);
                }
                self.print(66559i32);
            }
            {
                self.help_ptr = 1i32;
                self.help_line[crate::ix::U((0i32) as usize)] = 66560i32;
            }
            self.error();
        }
    }

    /// Delimiter fields of noads are filled in by the `scan_delimiter` routine.
    /// The first parameter of this procedure is the `mem` address where the
    /// delimiter is to be placed; the second tells if this delimiter follows
    /// \.{\\radical} or not.
    /// @<Declare act...
    // §1214
    pub fn scan_delimiter(&mut self, mut p: halfword, mut r: bool) {
        if r {
            {
                if (self.cur_chr == 1i32) {
                    {
                        self.cur_val1 = 1073741824i32;
                        self.scan_math_fam_int();
                        self.cur_val1 =
                            (self.cur_val1).wrapping_add((self.cur_val).wrapping_mul(2097152i32));
                        self.scan_usv_num();
                        self.cur_val = (self.cur_val1).wrapping_add(self.cur_val);
                    }
                } else {
                    self.scan_delimiter_int();
                }
            }
        } else {
            {
                // §438
                loop {
                    self.get_x_token();
                    if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                        break;
                    }
                }
                // §1214
                match self.cur_cmd {
                    letter | other_char => {
                        self.cur_val = self.eqtb[crate::ix::U(
                            (((del_code_base).wrapping_add(self.cur_chr)) - 1) as usize,
                        )]
                        .int();
                    }
                    delim_num => {
                        if (self.cur_chr == 1i32) {
                            {
                                self.cur_val1 = 1073741824i32;
                                self.scan_math_class_int();
                                self.scan_math_fam_int();
                                self.cur_val1 = (self.cur_val1)
                                    .wrapping_add((self.cur_val).wrapping_mul(2097152i32));
                                self.scan_usv_num();
                                self.cur_val = (self.cur_val1).wrapping_add(self.cur_val);
                            }
                        } else {
                            self.scan_delimiter_int();
                        }
                    }
                    _ => {
                        self.cur_val = (1i32).wrapping_neg();
                    }
                }
            }
        }
        if (self.cur_val < 0i32) {
            {
                // §1215
                {
                    {
                        if (self.interaction == error_stop_mode) {}
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66561i32);
                    }
                    {
                        self.help_ptr = 6i32;
                        self.help_line[crate::ix::U((5i32) as usize)] = 66562i32;
                        self.help_line[crate::ix::U((4i32) as usize)] = 66563i32;
                        self.help_line[crate::ix::U((3i32) as usize)] = 66564i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66565i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66566i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66567i32;
                    }
                    self.back_error();
                    self.cur_val = 0i32;
                }
            }
        }
        // §1214
        if (self.cur_val >= 1073741824i32) {
            {
                {
                    let __v1474 = (((self.cur_val % 2097152i32) / 65536i32).wrapping_mul(256i32))
                        .wrapping_add(((self.cur_val / 2097152i32) % 256i32));
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b0(__v1474);
                }
                {
                    let __v1475 = (self.cur_val % 65536i32);
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b1(__v1475);
                }
                self.mem[crate::ix::U((p) as usize)].set_qqqq_b2(0i32);
                self.mem[crate::ix::U((p) as usize)].set_qqqq_b3(0i32);
            }
        } else {
            {
                {
                    let __v1476 = ((self.cur_val / 1048576i32) % 16i32);
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b0(__v1476);
                }
                {
                    let __v1477 = ((self.cur_val / 4096i32) % 256i32);
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b1(__v1477);
                }
                {
                    let __v1478 = ((self.cur_val / 256i32) % 16i32);
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b2(__v1478);
                }
                {
                    let __v1479 = (self.cur_val % 256i32);
                    self.mem[crate::ix::U((p) as usize)].set_qqqq_b3(__v1479);
                }
            }
        }
    }

    /// @<Declare act...
    // §1217
    pub fn math_radical(&mut self) {
        {
            {
                let __ix1480 = self.cur_list.tail_field;
                let __v1481 = self.get_node(radical_noad_size);
                self.mem[crate::ix::U((__ix1480) as usize)].set_hh_rh(__v1481);
            }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                .hh()
                .rh();
        }
        {
            let __ix1482 = self.cur_list.tail_field;
            self.mem[crate::ix::U((__ix1482) as usize)].set_hh_b0(radical_noad);
        }
        {
            let __ix1483 = self.cur_list.tail_field;
            self.mem[crate::ix::U((__ix1483) as usize)].set_hh_b1(normal);
        }
        {
            let __ix1484 = (self.cur_list.tail_field).wrapping_add(1i32);
            let __v1485 = self.empty_field;
            self.mem[crate::ix::U((__ix1484) as usize)].set_hh(__v1485);
        }
        {
            let __ix1486 = (self.cur_list.tail_field).wrapping_add(3i32);
            let __v1487 = self.empty_field;
            self.mem[crate::ix::U((__ix1486) as usize)].set_hh(__v1487);
        }
        {
            let __ix1488 = (self.cur_list.tail_field).wrapping_add(2i32);
            let __v1489 = self.empty_field;
            self.mem[crate::ix::U((__ix1488) as usize)].set_hh(__v1489);
        }
        self.scan_delimiter((self.cur_list.tail_field).wrapping_add(4i32), true);
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// @<Declare act...
    // §1219
    pub fn math_ac(&mut self) {
        let mut c: i32 = 0; // §1219
        if (self.cur_cmd == accent) {
            // §1220
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66570i32);
                }
                self.print_esc(65830i32);
                self.print(66571i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66572i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66573i32;
                }
                self.error();
            }
        }
        // §1219
        {
            {
                let __ix1490 = self.cur_list.tail_field;
                let __v1491 = self.get_node(accent_noad_size);
                self.mem[crate::ix::U((__ix1490) as usize)].set_hh_rh(__v1491);
            }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                .hh()
                .rh();
        }
        {
            let __ix1492 = self.cur_list.tail_field;
            self.mem[crate::ix::U((__ix1492) as usize)].set_hh_b0(accent_noad);
        }
        {
            let __ix1493 = self.cur_list.tail_field;
            self.mem[crate::ix::U((__ix1493) as usize)].set_hh_b1(normal);
        }
        {
            let __ix1494 = (self.cur_list.tail_field).wrapping_add(1i32);
            let __v1495 = self.empty_field;
            self.mem[crate::ix::U((__ix1494) as usize)].set_hh(__v1495);
        }
        {
            let __ix1496 = (self.cur_list.tail_field).wrapping_add(3i32);
            let __v1497 = self.empty_field;
            self.mem[crate::ix::U((__ix1496) as usize)].set_hh(__v1497);
        }
        {
            let __ix1498 = (self.cur_list.tail_field).wrapping_add(2i32);
            let __v1499 = self.empty_field;
            self.mem[crate::ix::U((__ix1498) as usize)].set_hh(__v1499);
        }
        {
            let __ix1500 = (self.cur_list.tail_field).wrapping_add(4i32);
            self.mem[crate::ix::U((__ix1500) as usize)].set_hh_rh(math_char);
        }
        if (self.cur_chr == 1i32) {
            {
                if self.scan_keyword(66568i32) {
                    {
                        let __ix1501 = self.cur_list.tail_field;
                        self.mem[crate::ix::U((__ix1501) as usize)].set_hh_b1(fixed_acc);
                    }
                } else {
                    if self.scan_keyword(66569i32) {
                        {
                            if self.scan_keyword(66568i32) {
                                {
                                    let __ix1502 = self.cur_list.tail_field;
                                    self.mem[crate::ix::U((__ix1502) as usize)].set_hh_b1(3i32);
                                }
                            } else {
                                {
                                    let __ix1503 = self.cur_list.tail_field;
                                    self.mem[crate::ix::U((__ix1503) as usize)]
                                        .set_hh_b1(bottom_acc);
                                }
                            }
                        }
                    }
                }
                self.scan_math_class_int();
                c = self.set_class_field(self.cur_val);
                self.scan_math_fam_int();
                c = (c).wrapping_add(self.set_family_field(self.cur_val));
                self.scan_usv_num();
                self.cur_val = (self.cur_val).wrapping_add(c);
            }
        } else {
            {
                self.scan_fifteen_bit_int();
                self.cur_val = ((self.set_class_field((self.cur_val / 4096i32)))
                    .wrapping_add(self.set_family_field(((self.cur_val % 4096i32) / 256i32))))
                .wrapping_add((self.cur_val % 256i32));
            }
        }
        {
            let __ix1504 = (self.cur_list.tail_field).wrapping_add(4i32);
            let __v1505 = (self.cur_val % 65536i32);
            self.mem[crate::ix::U((__ix1504) as usize)].set_hh_b1(__v1505);
        }
        if ((self.math_class_field(self.cur_val) == 7i32)
            && ((self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int() >= 0i32)
                && (self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int()
                    < number_math_families)))
        {
            {
                let __ix1506 = (self.cur_list.tail_field).wrapping_add(4i32);
                let __v1507 = self.eqtb[crate::ix::U(((7892308i32) - 1) as usize)].int();
                self.mem[crate::ix::U((__ix1506) as usize)].set_hh_b0(__v1507);
            }
        } else {
            {
                let __ix1508 = (self.cur_list.tail_field).wrapping_add(4i32);
                let __v1509 = self.math_fam_field(self.cur_val);
                self.mem[crate::ix::U((__ix1508) as usize)].set_hh_b0(__v1509);
            }
        }
        {
            let __ix1510 = (self.cur_list.tail_field).wrapping_add(4i32);
            let __v1511 = (self.mem
                [crate::ix::U(((self.cur_list.tail_field).wrapping_add(4i32)) as usize)]
            .hh()
            .b0())
            .wrapping_add((self.math_char_field(self.cur_val) / 65536i32).wrapping_mul(256i32));
            self.mem[crate::ix::U((__ix1510) as usize)].set_hh_b0(__v1511);
        }
        self.scan_math((self.cur_list.tail_field).wrapping_add(1i32));
    }

    /// The routine that scans the four mlists of a \.{\\mathchoice} is very
    /// much like the routine that builds discretionary nodes.
    /// @<Declare act...
    // §1226
    pub fn append_choices(&mut self) {
        {
            {
                let __ix1512 = self.cur_list.tail_field;
                let __v1513 = self.new_choice();
                self.mem[crate::ix::U((__ix1512) as usize)].set_hh_rh(__v1513);
            }
            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                .hh()
                .rh();
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        {
            let __ix1514 = (self.save_ptr).wrapping_sub(1i32);
            self.save_stack[crate::ix::U((__ix1514) as usize)].set_int(0i32);
        }
        self.push_math(math_choice_group);
        self.scan_left_brace();
    }

    /// At the end of a math formula or subformula, the `fin_mlist` routine is
    /// called upon to return a pointer to the newly completed mlist, and to
    /// pop the nest back to the enclosing semantic level. The parameter to
    /// `fin_mlist`, if not null, points to a `right_noad` that ends the
    /// current mlist; this `right_noad` has not yet been appended.
    /// @<Declare the function called `fin_mlist`
    // §1238
    pub fn fin_mlist(&mut self, mut p: halfword) -> halfword {
        let mut fin_mlist: halfword = 0;
        let mut q: halfword = 0; // §1238
        if (self.cur_list.aux_field.int() != (268435455i32).wrapping_neg()) {
            // §1239
            {
                {
                    let __ix1515 = (self.cur_list.aux_field.int()).wrapping_add(3i32);
                    self.mem[crate::ix::U((__ix1515) as usize)].set_hh_rh(sub_mlist);
                }
                {
                    let __ix1516 = (self.cur_list.aux_field.int()).wrapping_add(3i32);
                    let __v1517 = self.mem[crate::ix::U((self.cur_list.head_field) as usize)]
                        .hh()
                        .rh();
                    self.mem[crate::ix::U((__ix1516) as usize)].set_hh_lh(__v1517);
                }
                if (p == (268435455i32).wrapping_neg()) {
                    q = self.cur_list.aux_field.int();
                } else {
                    {
                        q = self.mem[crate::ix::U(
                            ((self.cur_list.aux_field.int()).wrapping_add(2i32)) as usize,
                        )]
                        .hh()
                        .lh();
                        if ((self.mem[crate::ix::U((q) as usize)].hh().b0() != left_noad)
                            || (self.cur_list.eTeX_aux_field == (268435455i32).wrapping_neg()))
                        {
                            self.confusion(66280i32);
                        }
                        {
                            let __ix1518 = (self.cur_list.aux_field.int()).wrapping_add(2i32);
                            let __v1519 = self.mem
                                [crate::ix::U((self.cur_list.eTeX_aux_field) as usize)]
                            .hh()
                            .rh();
                            self.mem[crate::ix::U((__ix1518) as usize)].set_hh_lh(__v1519);
                        }
                        {
                            let __ix1520 = self.cur_list.eTeX_aux_field;
                            let __v1521 = self.cur_list.aux_field.int();
                            self.mem[crate::ix::U((__ix1520) as usize)].set_hh_rh(__v1521);
                        }
                        {
                            let __ix1522 = self.cur_list.aux_field.int();
                            self.mem[crate::ix::U((__ix1522) as usize)].set_hh_rh(p);
                        }
                    }
                }
            }
        } else {
            // §1238
            {
                {
                    let __ix1523 = self.cur_list.tail_field;
                    self.mem[crate::ix::U((__ix1523) as usize)].set_hh_rh(p);
                }
                q = self.mem[crate::ix::U((self.cur_list.head_field) as usize)]
                    .hh()
                    .rh();
            }
        }
        self.pop_nest();
        fin_mlist = q;
        fin_mlist
    }

    /// @<Declare act...
    // §1228
    pub fn build_choices(&mut self) {
        let mut p: halfword = 0; // §1228
        'l_exit_f: {
            self.unsave();
            p = self.fin_mlist((268435455i32).wrapping_neg());
            match self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()
            {
                0 => {
                    let __ix1524 = (self.cur_list.tail_field).wrapping_add(1i32);
                    self.mem[crate::ix::U((__ix1524) as usize)].set_hh_lh(p);
                }
                1 => {
                    let __ix1525 = (self.cur_list.tail_field).wrapping_add(1i32);
                    self.mem[crate::ix::U((__ix1525) as usize)].set_hh_rh(p);
                }
                2 => {
                    let __ix1526 = (self.cur_list.tail_field).wrapping_add(2i32);
                    self.mem[crate::ix::U((__ix1526) as usize)].set_hh_lh(p);
                }
                3 => {
                    {
                        let __ix1527 = (self.cur_list.tail_field).wrapping_add(2i32);
                        self.mem[crate::ix::U((__ix1527) as usize)].set_hh_rh(p);
                    }
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    break 'l_exit_f;
                }
                _ => {}
            }
            {
                let __ix1528 = (self.save_ptr).wrapping_sub(1i32);
                let __v1529 = (self.save_stack
                    [crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)]
                .int())
                .wrapping_add(1i32);
                self.save_stack[crate::ix::U((__ix1528) as usize)].set_int(__v1529);
            }
            self.push_math(math_choice_group);
            self.scan_left_brace();
        }
    }

    /// @<Declare act...
    // §1230
    pub fn sub_sup(&mut self) {
        let mut t: small_number = 0; // §1230
        let mut p: halfword = 0; // §1230
        t = empty;
        p = (268435455i32).wrapping_neg();
        if (self.cur_list.tail_field != self.cur_list.head_field) {
            if ((self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                .hh()
                .b0()
                >= ord_noad)
                && (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .b0()
                    < left_noad))
            {
                {
                    p = (((self.cur_list.tail_field).wrapping_add(2i32))
                        .wrapping_add(self.cur_cmd))
                    .wrapping_sub(7i32);
                    t = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
        }
        if ((p == (268435455i32).wrapping_neg()) || (t != empty)) {
            // §1231
            {
                {
                    {
                        let __ix1530 = self.cur_list.tail_field;
                        let __v1531 = self.new_noad();
                        self.mem[crate::ix::U((__ix1530) as usize)].set_hh_rh(__v1531);
                    }
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                p = (((self.cur_list.tail_field).wrapping_add(2i32)).wrapping_add(self.cur_cmd))
                    .wrapping_sub(7i32);
                if (t != empty) {
                    {
                        if (self.cur_cmd == sup_mark) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66574i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66575i32;
                                }
                            }
                        } else {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66576i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66577i32;
                                }
                            }
                        }
                        self.error();
                    }
                }
            }
        }
        // §1230
        self.scan_math(p);
    }

    /// @<Declare act...
    // §1235
    pub fn math_fraction(&mut self) {
        let mut c: small_number = 0; // §1235
        c = self.cur_chr;
        if (self.cur_list.aux_field.int() != (268435455i32).wrapping_neg()) {
            // §1237
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
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66584i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66585i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66586i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66587i32;
                }
                self.error();
            }
        } else {
            // §1235
            {
                {
                    let __v1532 = self.get_node(fraction_noad_size);
                    self.cur_list.aux_field.set_int(__v1532);
                }
                {
                    let __ix1533 = self.cur_list.aux_field.int();
                    self.mem[crate::ix::U((__ix1533) as usize)].set_hh_b0(fraction_noad);
                }
                {
                    let __ix1534 = self.cur_list.aux_field.int();
                    self.mem[crate::ix::U((__ix1534) as usize)].set_hh_b1(normal);
                }
                {
                    let __ix1535 = (self.cur_list.aux_field.int()).wrapping_add(2i32);
                    self.mem[crate::ix::U((__ix1535) as usize)].set_hh_rh(sub_mlist);
                }
                {
                    let __ix1536 = (self.cur_list.aux_field.int()).wrapping_add(2i32);
                    let __v1537 = self.mem[crate::ix::U((self.cur_list.head_field) as usize)]
                        .hh()
                        .rh();
                    self.mem[crate::ix::U((__ix1536) as usize)].set_hh_lh(__v1537);
                }
                {
                    let __ix1538 = (self.cur_list.aux_field.int()).wrapping_add(3i32);
                    let __v1539 = self.empty_field;
                    self.mem[crate::ix::U((__ix1538) as usize)].set_hh(__v1539);
                }
                {
                    let __ix1540 = (self.cur_list.aux_field.int()).wrapping_add(4i32);
                    let __v1541 = self.null_delimiter;
                    self.mem[crate::ix::U((__ix1540) as usize)].set_qqqq(__v1541);
                }
                {
                    let __ix1542 = (self.cur_list.aux_field.int()).wrapping_add(5i32);
                    let __v1543 = self.null_delimiter;
                    self.mem[crate::ix::U((__ix1542) as usize)].set_qqqq(__v1543);
                }
                {
                    let __ix1544 = self.cur_list.head_field;
                    self.mem[crate::ix::U((__ix1544) as usize)]
                        .set_hh_rh((268435455i32).wrapping_neg());
                }
                self.cur_list.tail_field = self.cur_list.head_field;
                // §1236
                if (c >= delimited_code) {
                    {
                        self.scan_delimiter(
                            (self.cur_list.aux_field.int()).wrapping_add(4i32),
                            false,
                        );
                        self.scan_delimiter(
                            (self.cur_list.aux_field.int()).wrapping_add(5i32),
                            false,
                        );
                    }
                }
                match (c % delimited_code) {
                    above_code => {
                        self.scan_dimen(false, false, false);
                        {
                            let __ix1545 = (self.cur_list.aux_field.int()).wrapping_add(1i32);
                            let __v1546 = self.cur_val;
                            self.mem[crate::ix::U((__ix1545) as usize)].set_int(__v1546);
                        }
                    }
                    over_code => {
                        let __ix1547 = (self.cur_list.aux_field.int()).wrapping_add(1i32);
                        self.mem[crate::ix::U((__ix1547) as usize)].set_int(default_code);
                    }
                    atop_code => {
                        let __ix1548 = (self.cur_list.aux_field.int()).wrapping_add(1i32);
                        self.mem[crate::ix::U((__ix1548) as usize)].set_int(0i32);
                    }
                    _ => {}
                }
            }
        }
    }

    /// @<Declare act...
    // §1245
    pub fn math_left_right(&mut self) {
        let mut t: small_number = 0; // §1245
        let mut p: halfword = 0; // §1245
        let mut q: halfword = 0; // §1245
        t = self.cur_chr;
        if ((t != left_noad) && (self.cur_group != math_left_group)) {
            // §1246
            {
                if (self.cur_group == math_shift_group) {
                    {
                        self.scan_delimiter(garbage, false);
                        {
                            if (self.interaction == error_stop_mode) {}
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66159i32);
                        }
                        if (t == middle_noad) {
                            {
                                self.print_esc(66281i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66588i32;
                                }
                            }
                        } else {
                            {
                                self.print_esc(66280i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66589i32;
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
            // §1245
            {
                p = self.new_noad();
                self.mem[crate::ix::U((p) as usize)].set_hh_b0(t);
                self.scan_delimiter((p).wrapping_add(1i32), false);
                if (t == middle_noad) {
                    {
                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(right_noad);
                        self.mem[crate::ix::U((p) as usize)].set_hh_b1(middle_noad);
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
                        {
                            let __ix1549 = self.cur_list.head_field;
                            self.mem[crate::ix::U((__ix1549) as usize)].set_hh_rh(q);
                        }
                        self.cur_list.tail_field = p;
                        self.cur_list.eTeX_aux_field = p;
                    }
                } else {
                    {
                        {
                            {
                                let __ix1550 = self.cur_list.tail_field;
                                let __v1551 = self.new_noad();
                                self.mem[crate::ix::U((__ix1550) as usize)].set_hh_rh(__v1551);
                            }
                            self.cur_list.tail_field = self.mem
                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                            .hh()
                            .rh();
                        }
                        {
                            let __ix1552 = self.cur_list.tail_field;
                            self.mem[crate::ix::U((__ix1552) as usize)].set_hh_b0(inner_noad);
                        }
                        {
                            let __ix1553 = (self.cur_list.tail_field).wrapping_add(1i32);
                            self.mem[crate::ix::U((__ix1553) as usize)].set_hh_rh(sub_mlist);
                        }
                        {
                            let __ix1554 = (self.cur_list.tail_field).wrapping_add(1i32);
                            self.mem[crate::ix::U((__ix1554) as usize)].set_hh_lh(q);
                        }
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
    // §1555
    pub fn app_display(&mut self, mut j: halfword, mut b: halfword, mut d: scaled) {
        let mut z: scaled = 0; // §1555
        let mut s: scaled = 0; // §1555
        let mut e: scaled = 0; // §1555
        let mut x: i32 = 0; // §1555
        let mut p: halfword = 0; // §1555
        let mut q: halfword = 0; // §1555
        let mut r: halfword = 0; // §1555
        let mut t: halfword = 0; // §1555
        let mut u: halfword = 0; // §1555
        s = self.eqtb[crate::ix::U(((9006735i32) - 1) as usize)].int();
        x = self.eqtb[crate::ix::U(((7892330i32) - 1) as usize)].int();
        if (x == 0i32) {
            self.mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int((s).wrapping_add(d));
        } else {
            {
                z = self.eqtb[crate::ix::U(((9006734i32) - 1) as usize)].int();
                p = b;
                // §1556
                if (x > 0i32) {
                    e = ((z).wrapping_sub(d)).wrapping_sub(
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(),
                    );
                } else {
                    {
                        e = d;
                        d = ((z).wrapping_sub(e)).wrapping_sub(
                            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(),
                        );
                    }
                }
                if (j != (268435455i32).wrapping_neg()) {
                    {
                        b = self.copy_node_list(j);
                        {
                            let __v1555 =
                                self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                            self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)]
                                .set_int(__v1555);
                        }
                        {
                            let __v1556 =
                                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                            self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)]
                                .set_int(__v1556);
                        }
                        s = (s).wrapping_sub(
                            self.mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].int(),
                        );
                        d = (d).wrapping_add(s);
                        e = (((e).wrapping_add(
                            self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int(),
                        ))
                        .wrapping_sub(z))
                        .wrapping_sub(s);
                    }
                }
                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == dlist) {
                    q = p;
                } else {
                    {
                        r = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)]
                            .hh()
                            .rh();
                        self.free_node(p, box_node_size);
                        if (r == (268435455i32).wrapping_neg()) {
                            self.confusion(66920i32);
                        }
                        if (x > 0i32) {
                            {
                                p = r;
                                loop {
                                    q = r;
                                    r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    if (r == (268435455i32).wrapping_neg()) {
                                        break;
                                    }
                                }
                            }
                        } else {
                            {
                                p = (268435455i32).wrapping_neg();
                                q = r;
                                loop {
                                    t = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    self.mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                                    p = r;
                                    r = t;
                                    if (r == (268435455i32).wrapping_neg()) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                // §1557
                if (j == (268435455i32).wrapping_neg()) {
                    {
                        r = self.new_kern(0i32);
                        t = self.new_kern(0i32);
                    }
                } else {
                    {
                        r = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)]
                            .hh()
                            .rh();
                        t = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                u = self.new_math(0i32, end_M_code);
                if (self.mem[crate::ix::U((t) as usize)].hh().b0() == glue_node) {
                    {
                        j = self.new_skip_param(right_skip_code);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(j);
                        self.mem[crate::ix::U((j) as usize)].set_hh_rh(u);
                        j = self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)]
                            .hh()
                            .lh();
                        {
                            let __ix1557 = self.temp_ptr;
                            let __v1558 = self.mem[crate::ix::U((j) as usize)].hh().b0();
                            self.mem[crate::ix::U((__ix1557) as usize)].set_hh_b0(__v1558);
                        }
                        {
                            let __ix1559 = self.temp_ptr;
                            let __v1560 = self.mem[crate::ix::U((j) as usize)].hh().b1();
                            self.mem[crate::ix::U((__ix1559) as usize)].set_hh_b1(__v1560);
                        }
                        {
                            let __ix1561 = (self.temp_ptr).wrapping_add(1i32);
                            let __v1562 = (e).wrapping_sub(
                                self.mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)].int(),
                            );
                            self.mem[crate::ix::U((__ix1561) as usize)].set_int(__v1562);
                        }
                        {
                            let __ix1563 = (self.temp_ptr).wrapping_add(2i32);
                            let __v1564 =
                                (self.mem[crate::ix::U(((j).wrapping_add(2i32)) as usize)].int())
                                    .wrapping_neg();
                            self.mem[crate::ix::U((__ix1563) as usize)].set_int(__v1564);
                        }
                        {
                            let __ix1565 = (self.temp_ptr).wrapping_add(3i32);
                            let __v1566 =
                                (self.mem[crate::ix::U(((j).wrapping_add(3i32)) as usize)].int())
                                    .wrapping_neg();
                            self.mem[crate::ix::U((__ix1565) as usize)].set_int(__v1566);
                        }
                        self.mem[crate::ix::U((u) as usize)].set_hh_rh(t);
                    }
                } else {
                    {
                        self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(e);
                        self.mem[crate::ix::U((t) as usize)].set_hh_rh(u);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(t);
                    }
                }
                u = self.new_math(0i32, begin_M_code);
                if (self.mem[crate::ix::U((r) as usize)].hh().b0() == glue_node) {
                    {
                        j = self.new_skip_param(left_skip_code);
                        self.mem[crate::ix::U((u) as usize)].set_hh_rh(j);
                        self.mem[crate::ix::U((j) as usize)].set_hh_rh(p);
                        j = self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                            .hh()
                            .lh();
                        {
                            let __ix1567 = self.temp_ptr;
                            let __v1568 = self.mem[crate::ix::U((j) as usize)].hh().b0();
                            self.mem[crate::ix::U((__ix1567) as usize)].set_hh_b0(__v1568);
                        }
                        {
                            let __ix1569 = self.temp_ptr;
                            let __v1570 = self.mem[crate::ix::U((j) as usize)].hh().b1();
                            self.mem[crate::ix::U((__ix1569) as usize)].set_hh_b1(__v1570);
                        }
                        {
                            let __ix1571 = (self.temp_ptr).wrapping_add(1i32);
                            let __v1572 = (d).wrapping_sub(
                                self.mem[crate::ix::U(((j).wrapping_add(1i32)) as usize)].int(),
                            );
                            self.mem[crate::ix::U((__ix1571) as usize)].set_int(__v1572);
                        }
                        {
                            let __ix1573 = (self.temp_ptr).wrapping_add(2i32);
                            let __v1574 =
                                (self.mem[crate::ix::U(((j).wrapping_add(2i32)) as usize)].int())
                                    .wrapping_neg();
                            self.mem[crate::ix::U((__ix1573) as usize)].set_int(__v1574);
                        }
                        {
                            let __ix1575 = (self.temp_ptr).wrapping_add(3i32);
                            let __v1576 =
                                (self.mem[crate::ix::U(((j).wrapping_add(3i32)) as usize)].int())
                                    .wrapping_neg();
                            self.mem[crate::ix::U((__ix1575) as usize)].set_int(__v1576);
                        }
                        self.mem[crate::ix::U((r) as usize)].set_hh_rh(u);
                    }
                } else {
                    {
                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(d);
                        self.mem[crate::ix::U((r) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((u) as usize)].set_hh_rh(r);
                        if (j == (268435455i32).wrapping_neg()) {
                            {
                                b = self.hpack(u, 0i32, additional);
                                self.mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)]
                                    .set_int(s);
                            }
                        } else {
                            self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(u);
                        }
                    }
                }
            }
        }
        // §1555
        self.append_to_vlist(b);
    }

    /// @<Declare act...
    // §1248
    pub fn after_math(&mut self) {
        let mut l: bool = false; // §1248
        let mut danger: bool = false; // §1248
        let mut m: i32 = 0; // §1248
        let mut p: halfword = 0; // §1248
        let mut a: halfword = 0; // §1248
        let mut b: halfword = 0; // §1252
        let mut w: scaled = 0; // §1252
        let mut z: scaled = 0; // §1252
        let mut e: scaled = 0; // §1252
        let mut q: scaled = 0; // §1252
        let mut d: scaled = 0; // §1252
        let mut s: scaled = 0; // §1252
        let mut g1: small_number = 0; // §1252
        let mut g2: small_number = 0; // §1252
        let mut r: halfword = 0; // §1252
        let mut t: halfword = 0; // §1252
        let mut pre_t: halfword = 0; // §1252
        let mut j: halfword = 0; // §1552
        danger = false;
        // §1553
        if (self.cur_list.mode_field == mmode) {
            j = self.cur_list.eTeX_aux_field;
        }
        // §1249
        if ((((self.font_params[crate::ix::U(
            (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                .hh()
                .rh()) as usize,
        )] < total_mathsy_params)
            && (!((self.font_area[crate::ix::U(
                (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                    .hh()
                    .rh()) as usize,
            )] == otgr_font_flag)
                && self.isOpenTypeMathFont(
                    self.font_layout_engine[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )],
                ))))
            || ((self.font_params[crate::ix::U(
                (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                    .hh()
                    .rh()) as usize,
            )] < total_mathsy_params)
                && (!((self.font_area[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] == otgr_font_flag)
                    && self.isOpenTypeMathFont(
                        self.font_layout_engine[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )],
                    )))))
            || ((self.font_params[crate::ix::U(
                (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                    .hh()
                    .rh()) as usize,
            )] < total_mathsy_params)
                && (!((self.font_area[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] == otgr_font_flag)
                    && self.isOpenTypeMathFont(
                        self.font_layout_engine[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )],
                    )))))
        {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66590i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 66591i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66592i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66593i32;
                }
                self.error();
                self.flush_math();
                danger = true;
            }
        } else {
            if ((((self.font_params[crate::ix::U(
                (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                    .hh()
                    .rh()) as usize,
            )] < total_mathex_params)
                && (!((self.font_area[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] == otgr_font_flag)
                    && self.isOpenTypeMathFont(
                        self.font_layout_engine[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )],
                    ))))
                || ((self.font_params[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] < total_mathex_params)
                    && (!((self.font_area[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] == otgr_font_flag)
                        && self.isOpenTypeMathFont(
                            self.font_layout_engine[crate::ix::U(
                                (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )],
                        )))))
                || ((self.font_params[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] < total_mathex_params)
                    && (!((self.font_area[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] == otgr_font_flag)
                        && self.isOpenTypeMathFont(
                            self.font_layout_engine[crate::ix::U(
                                (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )],
                        )))))
            {
                {
                    {
                        if (self.interaction == error_stop_mode) {}
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66594i32);
                    }
                    {
                        self.help_ptr = 3i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66595i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66596i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66597i32;
                    }
                    self.error();
                    self.flush_math();
                    danger = true;
                }
            }
        }
        // §1248
        m = self.cur_list.mode_field;
        l = false;
        p = self.fin_mlist((268435455i32).wrapping_neg());
        if (self.cur_list.mode_field == (m).wrapping_neg()) {
            {
                // §1251
                {
                    self.get_x_token();
                    if (self.cur_cmd != math_shift) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {}
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
                // §1248
                self.cur_mlist = p;
                self.cur_style = text_style;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                a = self.hpack(
                    self.mem[crate::ix::U((temp_head) as usize)].hh().rh(),
                    0i32,
                    additional,
                );
                self.mem[crate::ix::U((a) as usize)].set_hh_b1(dlist);
                self.unsave();
                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)]
                    .int()
                    == 1i32)
                {
                    l = true;
                }
                danger = false;
                // §1553
                if (self.cur_list.mode_field == mmode) {
                    j = self.cur_list.eTeX_aux_field;
                }
                // §1249
                if ((((self.font_params[crate::ix::U(
                    (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] < total_mathsy_params)
                    && (!((self.font_area[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] == otgr_font_flag)
                        && self.isOpenTypeMathFont(
                            self.font_layout_engine[crate::ix::U(
                                (self.eqtb[crate::ix::U(((1206826i32) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )],
                        ))))
                    || ((self.font_params[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] < total_mathsy_params)
                        && (!((self.font_area[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )] == otgr_font_flag)
                            && self.isOpenTypeMathFont(
                                self.font_layout_engine[crate::ix::U(
                                    (self.eqtb[crate::ix::U(((1207082i32) - 1) as usize)]
                                        .hh()
                                        .rh()) as usize,
                                )],
                            )))))
                    || ((self.font_params[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] < total_mathsy_params)
                        && (!((self.font_area[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )] == otgr_font_flag)
                            && self.isOpenTypeMathFont(
                                self.font_layout_engine[crate::ix::U(
                                    (self.eqtb[crate::ix::U(((1207338i32) - 1) as usize)]
                                        .hh()
                                        .rh()) as usize,
                                )],
                            )))))
                {
                    {
                        {
                            if (self.interaction == error_stop_mode) {}
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66590i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66591i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66592i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66593i32;
                        }
                        self.error();
                        self.flush_math();
                        danger = true;
                    }
                } else {
                    if ((((self.font_params[crate::ix::U(
                        (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] < total_mathex_params)
                        && (!((self.font_area[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )] == otgr_font_flag)
                            && self.isOpenTypeMathFont(
                                self.font_layout_engine[crate::ix::U(
                                    (self.eqtb[crate::ix::U(((1206827i32) - 1) as usize)]
                                        .hh()
                                        .rh()) as usize,
                                )],
                            ))))
                        || ((self.font_params[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )] < total_mathex_params)
                            && (!((self.font_area[crate::ix::U(
                                (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )] == otgr_font_flag)
                                && self.isOpenTypeMathFont(
                                    self.font_layout_engine[crate::ix::U(
                                        (self.eqtb[crate::ix::U(((1207083i32) - 1) as usize)]
                                            .hh()
                                            .rh()) as usize,
                                    )],
                                )))))
                        || ((self.font_params[crate::ix::U(
                            (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                                .hh()
                                .rh()) as usize,
                        )] < total_mathex_params)
                            && (!((self.font_area[crate::ix::U(
                                (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )] == otgr_font_flag)
                                && self.isOpenTypeMathFont(
                                    self.font_layout_engine[crate::ix::U(
                                        (self.eqtb[crate::ix::U(((1207339i32) - 1) as usize)]
                                            .hh()
                                            .rh()) as usize,
                                    )],
                                )))))
                    {
                        {
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66594i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 66595i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66596i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66597i32;
                            }
                            self.error();
                            self.flush_math();
                            danger = true;
                        }
                    }
                }
                // §1248
                m = self.cur_list.mode_field;
                p = self.fin_mlist((268435455i32).wrapping_neg());
            }
        } else {
            a = (268435455i32).wrapping_neg();
        }
        if (m < 0i32) {
            // §1250
            {
                {
                    {
                        let __ix1577 = self.cur_list.tail_field;
                        let __v1578 = self.new_math(
                            self.eqtb[crate::ix::U(((9006721i32) - 1) as usize)].int(),
                            before,
                        );
                        self.mem[crate::ix::U((__ix1577) as usize)].set_hh_rh(__v1578);
                    }
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                self.cur_mlist = p;
                self.cur_style = text_style;
                self.mlist_penalties = (self.cur_list.mode_field > 0i32);
                self.mlist_to_hlist();
                {
                    let __ix1579 = self.cur_list.tail_field;
                    let __v1580 = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                    self.mem[crate::ix::U((__ix1579) as usize)].set_hh_rh(__v1580);
                }
                while (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh()
                    != (268435455i32).wrapping_neg())
                {
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                {
                    {
                        let __ix1581 = self.cur_list.tail_field;
                        let __v1582 = self.new_math(
                            self.eqtb[crate::ix::U(((9006721i32) - 1) as usize)].int(),
                            after,
                        );
                        self.mem[crate::ix::U((__ix1581) as usize)].set_hh_rh(__v1582);
                    }
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                self.cur_list.aux_field.set_hh_lh(1000i32);
                self.unsave();
            }
        } else {
            // §1248
            {
                if (a == (268435455i32).wrapping_neg()) {
                    // §1251
                    {
                        self.get_x_token();
                        if (self.cur_cmd != math_shift) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
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
                // §1253
                self.cur_mlist = p;
                self.cur_style = display_style;
                self.mlist_penalties = false;
                self.mlist_to_hlist();
                p = self.mem[crate::ix::U((temp_head) as usize)].hh().rh();
                self.adjust_tail = adjust_head;
                self.pre_adjust_tail = pre_adjust_head;
                b = self.hpack(p, 0i32, additional);
                p = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)]
                    .hh()
                    .rh();
                t = self.adjust_tail;
                self.adjust_tail = (268435455i32).wrapping_neg();
                pre_t = self.pre_adjust_tail;
                self.pre_adjust_tail = (268435455i32).wrapping_neg();
                w = self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int();
                z = self.eqtb[crate::ix::U(((9006734i32) - 1) as usize)].int();
                s = self.eqtb[crate::ix::U(((9006735i32) - 1) as usize)].int();
                if (self.eqtb[crate::ix::U(((7892330i32) - 1) as usize)].int() < 0i32) {
                    s = ((s).wrapping_neg()).wrapping_sub(z);
                }
                if ((a == (268435455i32).wrapping_neg()) || danger) {
                    {
                        e = 0i32;
                        q = 0i32;
                    }
                } else {
                    {
                        e = self.mem[crate::ix::U(((a).wrapping_add(1i32)) as usize)].int();
                        q = (e).wrapping_add(self.math_quad(text_size));
                    }
                }
                if ((w).wrapping_add(q) > z) {
                    // §1255
                    {
                        if ((e != 0i32)
                            && ((((((w).wrapping_sub(
                                self.total_shrink[crate::ix::U((normal) as usize)],
                            ))
                            .wrapping_add(q)
                                <= z)
                                || (self.total_shrink[crate::ix::U((fil) as usize)] != 0i32))
                                || (self.total_shrink[crate::ix::U((fill) as usize)] != 0i32))
                                || (self.total_shrink[crate::ix::U((filll) as usize)] != 0i32)))
                        {
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
                        w = self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int();
                    }
                }
                // §1256
                self.mem[crate::ix::U((b) as usize)].set_hh_b1(dlist);
                d = self.half((z).wrapping_sub(w));
                if ((e > 0i32) && (d < (2i32).wrapping_mul(e))) {
                    {
                        d = self.half(((z).wrapping_sub(w)).wrapping_sub(e));
                        if (p != (268435455i32).wrapping_neg()) {
                            if (!(p >= self.hi_mem_min)) {
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) {
                                    d = 0i32;
                                }
                            }
                        }
                    }
                }
                // §1257
                {
                    {
                        let __ix1583 = self.cur_list.tail_field;
                        let __v1584 = self.new_penalty(
                            self.eqtb[crate::ix::U(((7892275i32) - 1) as usize)].int(),
                        );
                        self.mem[crate::ix::U((__ix1583) as usize)].set_hh_rh(__v1584);
                    }
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                if (((d).wrapping_add(s)
                    <= self.eqtb[crate::ix::U(((9006733i32) - 1) as usize)].int())
                    || l)
                {
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
                            {
                                let __ix1585 = self.cur_list.tail_field;
                                let __v1586 = self.new_penalty(inf_penalty);
                                self.mem[crate::ix::U((__ix1585) as usize)].set_hh_rh(__v1586);
                            }
                            self.cur_list.tail_field = self.mem
                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                            .hh()
                            .rh();
                        }
                    }
                } else {
                    {
                        {
                            let __ix1587 = self.cur_list.tail_field;
                            let __v1588 = self.new_param_glue(g1);
                            self.mem[crate::ix::U((__ix1587) as usize)].set_hh_rh(__v1588);
                        }
                        self.cur_list.tail_field = self.mem
                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                        .hh()
                        .rh();
                    }
                }
                // §1258
                if (e != 0i32) {
                    {
                        r = self.new_kern((((z).wrapping_sub(w)).wrapping_sub(e)).wrapping_sub(d));
                        if l {
                            {
                                self.mem[crate::ix::U((a) as usize)].set_hh_rh(r);
                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(b);
                                b = a;
                                d = 0i32;
                            }
                        } else {
                            {
                                self.mem[crate::ix::U((b) as usize)].set_hh_rh(r);
                                self.mem[crate::ix::U((r) as usize)].set_hh_rh(a);
                            }
                        }
                        b = self.hpack(b, 0i32, additional);
                    }
                }
                self.app_display(j, b, d);
                // §1259
                if (((a != (268435455i32).wrapping_neg()) && (e == 0i32)) && (!l)) {
                    {
                        {
                            {
                                let __ix1589 = self.cur_list.tail_field;
                                let __v1590 = self.new_penalty(inf_penalty);
                                self.mem[crate::ix::U((__ix1589) as usize)].set_hh_rh(__v1590);
                            }
                            self.cur_list.tail_field = self.mem
                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                            .hh()
                            .rh();
                        }
                        self.app_display(
                            j,
                            a,
                            (z).wrapping_sub(
                                self.mem[crate::ix::U(((a).wrapping_add(1i32)) as usize)].int(),
                            ),
                        );
                        g2 = 0i32;
                    }
                }
                if (t != adjust_head) {
                    {
                        {
                            let __ix1591 = self.cur_list.tail_field;
                            let __v1592 = self.mem[crate::ix::U((adjust_head) as usize)].hh().rh();
                            self.mem[crate::ix::U((__ix1591) as usize)].set_hh_rh(__v1592);
                        }
                        self.cur_list.tail_field = t;
                    }
                }
                if (pre_t != pre_adjust_head) {
                    {
                        {
                            let __ix1593 = self.cur_list.tail_field;
                            let __v1594 =
                                self.mem[crate::ix::U((pre_adjust_head) as usize)].hh().rh();
                            self.mem[crate::ix::U((__ix1593) as usize)].set_hh_rh(__v1594);
                        }
                        self.cur_list.tail_field = pre_t;
                    }
                }
                {
                    {
                        let __ix1595 = self.cur_list.tail_field;
                        let __v1596 = self.new_penalty(
                            self.eqtb[crate::ix::U(((7892276i32) - 1) as usize)].int(),
                        );
                        self.mem[crate::ix::U((__ix1595) as usize)].set_hh_rh(__v1596);
                    }
                    self.cur_list.tail_field = self.mem
                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                    .hh()
                    .rh();
                }
                if (g2 > 0i32) {
                    {
                        {
                            let __ix1597 = self.cur_list.tail_field;
                            let __v1598 = self.new_param_glue(g2);
                            self.mem[crate::ix::U((__ix1597) as usize)].set_hh_rh(__v1598);
                        }
                        self.cur_list.tail_field = self.mem
                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                        .hh()
                        .rh();
                    }
                }
                // §1554
                self.flush_node_list(j);
                // §1253
                self.resume_after_display();
            }
        }
    }

    /// @<Declare act...
    // §1254
    pub fn resume_after_display(&mut self) {
        if (self.cur_group != math_shift_group) {
            self.confusion(66601i32);
        }
        self.unsave();
        self.cur_list.pg_field = (self.cur_list.pg_field).wrapping_add(3i32);
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
        {
            let __v1599 = self.cur_lang;
            self.cur_list.aux_field.set_hh_rh(__v1599);
        }
        self.cur_list.pg_field = ((((self
            .norm_min(self.eqtb[crate::ix::U(((7892315i32) - 1) as usize)].int()))
        .wrapping_mul(64i32))
        .wrapping_add(self.norm_min(self.eqtb[crate::ix::U(((7892316i32) - 1) as usize)].int())))
        .wrapping_mul(65536i32))
        .wrapping_add(self.cur_lang);
        // §477
        {
            self.get_x_token();
            if (self.cur_cmd != spacer) {
                self.back_input();
            }
        }
        // §1254
        if (self.nest_ptr == 1i32) {
            self.build_page();
        }
    }

    /// When a control sequence is to be defined, by \.{\\def} or \.{\\let} or
    /// something similar, the `get_r_token` routine will substitute a special
    /// control sequence for a token that is not redefinable.
    /// @<Declare subprocedures for `prefixed_command`
    // §1269
    pub fn get_r_token(&mut self) {
        'l_restart_b: loop {
            loop {
                self.get_token();
                if (self.cur_tok != space_token) {
                    break;
                }
            }
            if (((self.cur_cs == 0i32) || (self.cur_cs > eqtb_top))
                || ((self.cur_cs > frozen_control_sequence) && (self.cur_cs <= eqtb_size)))
            {
                {
                    {
                        if (self.interaction == error_stop_mode) {}
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66619i32);
                    }
                    {
                        self.help_ptr = 5i32;
                        self.help_line[crate::ix::U((4i32) as usize)] = 66620i32;
                        self.help_line[crate::ix::U((3i32) as usize)] = 66621i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66622i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66623i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66624i32;
                    }
                    if (self.cur_cs == 0i32) {
                        self.back_input();
                    }
                    self.cur_tok = 34749081i32;
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
    // §1283
    pub fn trap_zero_glue(&mut self) {
        if (((self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int() == 0i32)
            && (self.mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()
                == 0i32))
            && (self.mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int() == 0i32))
        {
            {
                {
                    let __v1600 =
                        (self.mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(1i32);
                    self.mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v1600);
                }
                self.delete_glue_ref(self.cur_val);
                self.cur_val = zero_glue;
            }
        }
    }

    /// We use the fact that `register<advance<multiply<divide`.
    /// @<Declare subprocedures for `prefixed_command`
    // §1290
    pub fn do_register_command(&mut self, mut a: small_number) {
        let mut l: halfword = 0; // §1290
        let mut q: halfword = 0; // §1290
        let mut r: halfword = 0; // §1290
        let mut s: halfword = 0; // §1290
        let mut p: i32 = 0; // §1290
        let mut e: bool = false; // §1290
        let mut w: i32 = 0; // §1290
        'l_exit_f: {
            'l_found_f: {
                q = self.cur_cmd;
                e = false;
                // §1291
                {
                    if (q != register) {
                        {
                            self.get_x_token();
                            if ((self.cur_cmd >= assign_int) && (self.cur_cmd <= assign_mu_glue)) {
                                {
                                    l = self.cur_chr;
                                    p = (self.cur_cmd).wrapping_sub(74i32);
                                    break 'l_found_f;
                                }
                            }
                            if (self.cur_cmd != register) {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {}
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66025i32);
                                    }
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print(66026i32);
                                    self.print_cmd_chr(q, 0i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66657i32;
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
                            p = (self.mem[crate::ix::U((l) as usize)].hh().b0() / 64i32);
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
                                        l = (self.cur_val).wrapping_add(7892352i32);
                                    }
                                    dimen_val => {
                                        l = (self.cur_val).wrapping_add(9006743i32);
                                    }
                                    glue_val => {
                                        l = (self.cur_val).wrapping_add(1205783i32);
                                    }
                                    mu_val => {
                                        l = (self.cur_val).wrapping_add(1206039i32);
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
                    w = self.mem[crate::ix::U(((l).wrapping_add(2i32)) as usize)].int();
                } else {
                    w = self.eqtb[crate::ix::U(((l) - 1) as usize)].int();
                }
            } else {
                if e {
                    s = self.mem[crate::ix::U(((l).wrapping_add(1i32)) as usize)]
                        .hh()
                        .rh();
                } else {
                    s = self.eqtb[crate::ix::U(((l) - 1) as usize)].hh().rh();
                }
            }
            // §1290
            if (q == register) {
                self.scan_optional_equals();
            } else {
                if self.scan_keyword(66653i32) {}
            }
            self.arith_error = false;
            if (q < multiply) {
                // §1292
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
                            // §1293
                            {
                                q = self.new_spec(self.cur_val);
                                r = s;
                                self.delete_glue_ref(self.cur_val);
                                {
                                    let __v1601 = (self.mem
                                        [crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                                    .int())
                                    .wrapping_add(
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                                            .int(),
                                    );
                                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                                        .set_int(__v1601);
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()
                                    == 0i32)
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(normal);
                                }
                                if (self.mem[crate::ix::U((q) as usize)].hh().b0()
                                    == self.mem[crate::ix::U((r) as usize)].hh().b0())
                                {
                                    {
                                        let __v1602 = (self.mem
                                            [crate::ix::U(((q).wrapping_add(2i32)) as usize)]
                                        .int())
                                        .wrapping_add(
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                            .int(),
                                        );
                                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)]
                                            .set_int(__v1602);
                                    }
                                } else {
                                    if ((self.mem[crate::ix::U((q) as usize)].hh().b0()
                                        < self.mem[crate::ix::U((r) as usize)].hh().b0())
                                        && (self.mem
                                            [crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                        .int()
                                            != 0i32))
                                    {
                                        {
                                            {
                                                let __v1603 = self.mem[crate::ix::U(
                                                    ((r).wrapping_add(2i32)) as usize,
                                                )]
                                                .int();
                                                self.mem[crate::ix::U(
                                                    ((q).wrapping_add(2i32)) as usize,
                                                )]
                                                .set_int(__v1603);
                                            }
                                            {
                                                let __v1604 =
                                                    self.mem[crate::ix::U((r) as usize)].hh().b0();
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_b0(__v1604);
                                            }
                                        }
                                    }
                                }
                                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()
                                    == 0i32)
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(normal);
                                }
                                if (self.mem[crate::ix::U((q) as usize)].hh().b1()
                                    == self.mem[crate::ix::U((r) as usize)].hh().b1())
                                {
                                    {
                                        let __v1605 = (self.mem
                                            [crate::ix::U(((q).wrapping_add(3i32)) as usize)]
                                        .int())
                                        .wrapping_add(
                                            self.mem
                                                [crate::ix::U(((r).wrapping_add(3i32)) as usize)]
                                            .int(),
                                        );
                                        self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)]
                                            .set_int(__v1605);
                                    }
                                } else {
                                    if ((self.mem[crate::ix::U((q) as usize)].hh().b1()
                                        < self.mem[crate::ix::U((r) as usize)].hh().b1())
                                        && (self.mem
                                            [crate::ix::U(((r).wrapping_add(3i32)) as usize)]
                                        .int()
                                            != 0i32))
                                    {
                                        {
                                            {
                                                let __v1606 = self.mem[crate::ix::U(
                                                    ((r).wrapping_add(3i32)) as usize,
                                                )]
                                                .int();
                                                self.mem[crate::ix::U(
                                                    ((q).wrapping_add(3i32)) as usize,
                                                )]
                                                .set_int(__v1606);
                                            }
                                            {
                                                let __v1607 =
                                                    self.mem[crate::ix::U((r) as usize)].hh().b1();
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_b1(__v1607);
                                            }
                                        }
                                    }
                                }
                                self.cur_val = q;
                            }
                        }
                    }
                }
            } else {
                // §1294
                {
                    self.scan_int();
                    if (p < glue_val) {
                        if (q == multiply) {
                            if (p == int_val) {
                                self.cur_val =
                                    self.mult_and_add(w, self.cur_val, 0i32, 2147483647i32);
                            } else {
                                self.cur_val =
                                    self.mult_and_add(w, self.cur_val, 0i32, 1073741823i32);
                            }
                        } else {
                            self.cur_val = self.x_over_n(w, self.cur_val);
                        }
                    } else {
                        {
                            r = self.new_spec(s);
                            if (q == multiply) {
                                {
                                    {
                                        let __v1608 = self.mult_and_add(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(1i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                            0i32,
                                            1073741823i32,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                                            .set_int(__v1608);
                                    }
                                    {
                                        let __v1609 = self.mult_and_add(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(2i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                            0i32,
                                            1073741823i32,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                            .set_int(__v1609);
                                    }
                                    {
                                        let __v1610 = self.mult_and_add(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(3i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                            0i32,
                                            1073741823i32,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)]
                                            .set_int(__v1610);
                                    }
                                }
                            } else {
                                {
                                    {
                                        let __v1611 = self.x_over_n(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(1i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)]
                                            .set_int(__v1611);
                                    }
                                    {
                                        let __v1612 = self.x_over_n(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(2i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)]
                                            .set_int(__v1612);
                                    }
                                    {
                                        let __v1613 = self.x_over_n(
                                            self.mem
                                                [crate::ix::U(((s).wrapping_add(3i32)) as usize)]
                                            .int(),
                                            self.cur_val,
                                        );
                                        self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)]
                                            .set_int(__v1613);
                                    }
                                }
                            }
                            self.cur_val = r;
                        }
                    }
                }
            }
            // §1290
            if self.arith_error {
                {
                    {
                        if (self.interaction == error_stop_mode) {}
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66654i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66655i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66656i32;
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
    // §1297
    pub fn alter_aux(&mut self) {
        let mut c: halfword = 0; // §1297
        if (self.cur_chr != (self.cur_list.mode_field).wrapping_abs()) {
            self.report_illegal_case();
        } else {
            {
                c = self.cur_chr;
                self.scan_optional_equals();
                if (c == vmode) {
                    {
                        self.scan_dimen(false, false, false);
                        {
                            let __v1614 = self.cur_val;
                            self.cur_list.aux_field.set_int(__v1614);
                        }
                    }
                } else {
                    {
                        self.scan_int();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32767i32)) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66660i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66661i32;
                                }
                                self.int_error(self.cur_val);
                            }
                        } else {
                            {
                                let __v1615 = self.cur_val;
                                self.cur_list.aux_field.set_hh_lh(__v1615);
                            }
                        }
                    }
                }
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1298
    pub fn alter_prev_graf(&mut self) {
        let mut p: i32 = 0; // §1298
        {
            let __ix1616 = self.nest_ptr;
            let __v1617 = self.cur_list;
            self.nest[crate::ix::U((__ix1616) as usize)] = __v1617;
        }
        p = self.nest_ptr;
        while ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() != vmode) {
            p = (p).wrapping_sub(1i32);
        }
        self.scan_optional_equals();
        self.scan_int();
        if (self.cur_val < 0i32) {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66384i32);
                }
                self.print_esc(65846i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66662i32;
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
    // §1299
    pub fn alter_page_so_far(&mut self) {
        let mut c: i32 = 0; // §1299
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        {
            let __v1618 = self.cur_val;
            self.page_so_far[crate::ix::U((c) as usize)] = __v1618;
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1300
    pub fn alter_integer(&mut self) {
        let mut c: small_number = 0; // §1300
        c = self.cur_chr;
        self.scan_optional_equals();
        self.scan_int();
        if (c == 0i32) {
            self.dead_cycles = self.cur_val;
        } else {
            // §1506
            if (c == 2i32) {
                {
                    if ((self.cur_val < batch_mode) || (self.cur_val > error_stop_mode)) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66895i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66896i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66897i32;
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
                // §1300
                self.insert_penalties = self.cur_val;
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1301
    pub fn alter_box_dimen(&mut self) {
        let mut c: small_number = 0; // §1301
        let mut b: halfword = 0; // §1301
        c = self.cur_chr;
        self.scan_register_num();
        if (self.cur_val < 256i32) {
            b = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)]
                .hh()
                .rh();
        } else {
            {
                self.find_sa_element(box_val, self.cur_val, false);
                if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                    b = (268435455i32).wrapping_neg();
                } else {
                    b = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)]
                        .hh()
                        .rh();
                }
            }
        }
        self.scan_optional_equals();
        self.scan_dimen(false, false, false);
        if (b != (268435455i32).wrapping_neg()) {
            {
                let __v1619 = self.cur_val;
                self.mem[crate::ix::U(((b).wrapping_add(c)) as usize)].set_int(__v1619);
            }
        }
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1311
    pub fn new_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §1311
        let mut s: scaled = 0; // §1311
        let mut f: internal_font_number = 0; // §1311
        let mut t: str_number = 0; // §1311
        let mut old_setting: i32 = 0; // §1311
        'l_common_ending_f: {
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            self.get_r_token();
            u = self.cur_cs;
            if (u >= hash_base) {
                t = self.hash[crate::ix::U(((u) - 1179650) as usize)].rh();
            } else {
                if (u >= single_base) {
                    if (u == null_cs) {
                        t = 66668i32;
                    } else {
                        t = (u).wrapping_sub(1114113i32);
                    }
                } else {
                    {
                        old_setting = self.selector;
                        self.selector = new_string;
                        self.print(66668i32);
                        self.print((u).wrapping_sub(1i32));
                        self.selector = old_setting;
                        {
                            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                self.overflow(
                                    65539i32,
                                    (pool_size).wrapping_sub(self.init_pool_ptr),
                                );
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
            // §1312
            self.name_in_progress = true;
            if self.scan_keyword(66669i32) {
                // §1313
                {
                    self.scan_dimen(false, false, false);
                    s = self.cur_val;
                    if ((s <= 0i32) || (s >= 134217728i32)) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66671i32);
                            }
                            self.print_scaled(s);
                            self.print(66672i32);
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66673i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66674i32;
                            }
                            self.error();
                            s = (10i32).wrapping_mul(unity);
                        }
                    }
                }
            } else {
                // §1312
                if self.scan_keyword(66670i32) {
                    {
                        self.scan_int();
                        s = (self.cur_val).wrapping_neg();
                        if ((self.cur_val <= 0i32) || (self.cur_val > 32768i32)) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65872i32);
                                }
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 65873i32;
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
            // §1314
            {
                let __for_end_3 = self.font_ptr;
                f = 1i32;
                while f <= __for_end_3 {
                    {
                        if (self
                            .str_eq_str(self.font_name[crate::ix::U((f) as usize)], self.cur_name)
                            && (((self.cur_area == 65626i32)
                                && ((self.font_area[crate::ix::U((f) as usize)]
                                    == aat_font_flag)
                                    || (self.font_area[crate::ix::U((f) as usize)]
                                        == otgr_font_flag)))
                                || self.str_eq_str(
                                    self.font_area[crate::ix::U((f) as usize)],
                                    self.cur_area,
                                )))
                        {
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
                                        if (self.font_size[crate::ix::U((f) as usize)]
                                            == self.xn_over_d(
                                                self.font_dsize[crate::ix::U((f) as usize)],
                                                (s).wrapping_neg(),
                                                1000i32,
                                            ))
                                        {
                                            if (!self.arith_error) {
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.append_str(self.cur_area);
                        self.append_str(self.cur_name);
                        self.append_str(self.cur_ext);
                        if {
                            let __a1620_0 = self.font_name[crate::ix::U((f) as usize)];
                            let __a1620_1 = self.make_string();
                            self.str_eq_str(__a1620_0, __a1620_1)
                        } {
                            {
                                {
                                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                    self.pool_ptr = self.str_start[crate::ix::U(
                                        ((self.str_ptr).wrapping_sub(65536i32)) as usize,
                                    )];
                                }
                                if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag)
                                    || (self.font_area[crate::ix::U((f) as usize)]
                                        == otgr_font_flag))
                                {
                                    {
                                        if (s > 0i32) {
                                            {
                                                if (s == self.font_size[crate::ix::U((f) as usize)])
                                                {
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        } else {
                                            if (self.font_size[crate::ix::U((f) as usize)]
                                                == self.xn_over_d(
                                                    self.font_dsize[crate::ix::U((f) as usize)],
                                                    (s).wrapping_neg(),
                                                    1000i32,
                                                ))
                                            {
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                self.pool_ptr = self.str_start[crate::ix::U(
                                    ((self.str_ptr).wrapping_sub(65536i32)) as usize,
                                )];
                            }
                        }
                    }
                    f = f.wrapping_add(1);
                }
            }
            // §1311
            f = self.read_font_info(u, self.cur_name, self.cur_area, s);
        }
        if (a >= 4i32) {
            self.geq_define(u, set_font, f);
        } else {
            self.eq_define(u, set_font, f);
        }
        {
            let __v1621 = self.eqtb[crate::ix::U(((u) - 1) as usize)];
            self.eqtb[crate::ix::U((((font_id_base).wrapping_add(f)) - 1) as usize)] = __v1621;
        }
        self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 1179650) as usize)].set_rh(t);
    }

    /// @<Declare subprocedures for `prefixed_command`
    // §1319
    pub fn new_interaction(&mut self) {
        self.print_ln();
        self.interaction = self.cur_chr;
        // §79
        if (self.interaction == batch_mode) {
            self.selector = no_print;
        } else {
            self.selector = term_only;
        }
        // §1319
        if self.log_opened {
            self.selector = (self.selector).wrapping_add(2i32);
        }
    }

    /// If the user says, e.g., `\.{\\global\\global}', the redundancy is
    /// silently accepted.
    /// @<Declare act...
    // §1265
    pub fn prefixed_command(&mut self) {
        let mut a: small_number = 0; // §1265
        let mut f: internal_font_number = 0; // §1265
        let mut j: halfword = 0; // §1265
        let mut k: font_index = 0; // §1265
        let mut p: halfword = 0; // §1265
        let mut q: halfword = 0; // §1265
        let mut n: i32 = 0; // §1265
        let mut e: bool = false; // §1265
        'l_exit_f: {
            'l_done_f: {
                a = 0i32;
                while (self.cur_cmd == prefix) {
                    {
                        if (!(((a / self.cur_chr) % 2) != 0)) {
                            a = (a).wrapping_add(self.cur_chr);
                        }
                        // §438
                        loop {
                            self.get_x_token();
                            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                                break;
                            }
                        }
                        // §1265
                        if (self.cur_cmd <= max_non_prefixed_command) {
                            // §1266
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66611i32);
                                }
                                self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                self.print_char(39i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66612i32;
                                }
                                if (self.eTeX_mode == 1i32) {
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66613i32;
                                }
                                self.back_error();
                                break 'l_exit_f;
                            }
                        }
                        // §1265
                        if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() > 2i32) {
                            if (self.eTeX_mode == 1i32) {
                                self.show_cur_cmd_chr();
                            }
                        }
                    }
                }
                // §1267
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
                            if (self.interaction == error_stop_mode) {}
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66025i32);
                        }
                        self.print_esc(66603i32);
                        self.print(66614i32);
                        self.print_esc(66604i32);
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66615i32;
                        }
                        if (self.eTeX_mode == 1i32) {
                            {
                                self.help_line[crate::ix::U((0i32) as usize)] = 66616i32;
                                self.print(66614i32);
                                self.print_esc(66617i32);
                            }
                        }
                        self.print(66618i32);
                        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                        self.print_char(39i32);
                        self.error();
                    }
                }
                // §1268
                if (self.eqtb[crate::ix::U(((7892307i32) - 1) as usize)].int() != 0i32) {
                    if (self.eqtb[crate::ix::U(((7892307i32) - 1) as usize)].int() < 0i32) {
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
                // §1265
                match self.cur_cmd {
                    set_font => {
                        // §1271
                        if (a >= 4i32) {
                            self.geq_define(cur_font_loc, data, self.cur_chr);
                        } else {
                            self.eq_define(cur_font_loc, data, self.cur_chr);
                        }
                    }
                    def => {
                        // §1272
                        {
                            if (((((self.cur_chr) % 2) != 0) && (!(a >= 4i32)))
                                && (self.eqtb[crate::ix::U(((7892307i32) - 1) as usize)].int()
                                    >= 0i32))
                            {
                                a = (a).wrapping_add(4i32);
                            }
                            e = (self.cur_chr >= 2i32);
                            self.get_r_token();
                            p = self.cur_cs;
                            q = self.scan_toks(true, e);
                            if (j != 0i32) {
                                {
                                    q = self.get_avail();
                                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(j);
                                    {
                                        let __v1622 = self.mem
                                            [crate::ix::U((self.def_ref) as usize)]
                                        .hh()
                                        .rh();
                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1622);
                                    }
                                    {
                                        let __ix1623 = self.def_ref;
                                        self.mem[crate::ix::U((__ix1623) as usize)].set_hh_rh(q);
                                    }
                                }
                            }
                            if (a >= 4i32) {
                                self.geq_define(p, (call).wrapping_add((a % 4i32)), self.def_ref);
                            } else {
                                self.eq_define(p, (call).wrapping_add((a % 4i32)), self.def_ref);
                            }
                        }
                    }
                    let_ => {
                        // §1275
                        {
                            n = self.cur_chr;
                            self.get_r_token();
                            p = self.cur_cs;
                            if (n == normal) {
                                {
                                    loop {
                                        self.get_token();
                                        if (self.cur_cmd != spacer) {
                                            break;
                                        }
                                    }
                                    if (self.cur_tok == 25165885i32) {
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
                                {
                                    let __ix1624 = self.cur_chr;
                                    let __v1625 =
                                        (self.mem[crate::ix::U((self.cur_chr) as usize)].hh().lh())
                                            .wrapping_add(1i32);
                                    self.mem[crate::ix::U((__ix1624) as usize)].set_hh_lh(__v1625);
                                }
                            } else {
                                if ((self.cur_cmd == register) || (self.cur_cmd == toks_register)) {
                                    if ((self.cur_chr < mem_bot)
                                        || (self.cur_chr > lo_mem_stat_max))
                                    {
                                        {
                                            let __ix1626 = (self.cur_chr).wrapping_add(1i32);
                                            let __v1627 = (self.mem[crate::ix::U(
                                                ((self.cur_chr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .lh())
                                            .wrapping_add(1i32);
                                            self.mem[crate::ix::U((__ix1626) as usize)]
                                                .set_hh_lh(__v1627);
                                        }
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
                    shorthand_def => {
                        // §1278
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
                                    self.scan_usv_num();
                                    if (a >= 4i32) {
                                        self.geq_define(p, char_given, self.cur_val);
                                    } else {
                                        self.eq_define(p, char_given, self.cur_val);
                                    }
                                }
                                math_char_def_code => {
                                    self.scan_fifteen_bit_int();
                                    if (a >= 4i32) {
                                        self.geq_define(p, math_given, self.cur_val);
                                    } else {
                                        self.eq_define(p, math_given, self.cur_val);
                                    }
                                }
                                XeTeX_math_char_num_def_code => {
                                    self.scan_xetex_math_char_int();
                                    if (a >= 4i32) {
                                        self.geq_define(p, XeTeX_math_given, self.cur_val);
                                    } else {
                                        self.eq_define(p, XeTeX_math_given, self.cur_val);
                                    }
                                }
                                XeTeX_math_char_def_code => {
                                    self.scan_math_class_int();
                                    n = self.set_class_field(self.cur_val);
                                    self.scan_math_fam_int();
                                    n = (n).wrapping_add(self.set_family_field(self.cur_val));
                                    self.scan_usv_num();
                                    n = (n).wrapping_add(self.cur_val);
                                    if (a >= 4i32) {
                                        self.geq_define(p, XeTeX_math_given, n);
                                    } else {
                                        self.eq_define(p, XeTeX_math_given, n);
                                    }
                                }
                                _ => {
                                    self.scan_register_num();
                                    if (self.cur_val > 255i32) {
                                        {
                                            j = (n).wrapping_sub(2i32);
                                            if (j > mu_val) {
                                                j = tok_val;
                                            }
                                            self.find_sa_element(j, self.cur_val, true);
                                            {
                                                let __ix1628 = (self.cur_ptr).wrapping_add(1i32);
                                                let __v1629 = (self.mem[crate::ix::U(
                                                    ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                                )]
                                                .hh()
                                                .lh())
                                                .wrapping_add(1i32);
                                                self.mem[crate::ix::U((__ix1628) as usize)]
                                                    .set_hh_lh(__v1629);
                                            }
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
                                                    self.geq_define(
                                                        p,
                                                        assign_int,
                                                        (count_base).wrapping_add(self.cur_val),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        assign_int,
                                                        (count_base).wrapping_add(self.cur_val),
                                                    );
                                                }
                                            }
                                            dimen_def_code => {
                                                if (a >= 4i32) {
                                                    self.geq_define(
                                                        p,
                                                        assign_dimen,
                                                        (scaled_base).wrapping_add(self.cur_val),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        assign_dimen,
                                                        (scaled_base).wrapping_add(self.cur_val),
                                                    );
                                                }
                                            }
                                            skip_def_code => {
                                                if (a >= 4i32) {
                                                    self.geq_define(
                                                        p,
                                                        assign_glue,
                                                        (skip_base).wrapping_add(self.cur_val),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        assign_glue,
                                                        (skip_base).wrapping_add(self.cur_val),
                                                    );
                                                }
                                            }
                                            mu_skip_def_code => {
                                                if (a >= 4i32) {
                                                    self.geq_define(
                                                        p,
                                                        assign_mu_glue,
                                                        (mu_skip_base).wrapping_add(self.cur_val),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        assign_mu_glue,
                                                        (mu_skip_base).wrapping_add(self.cur_val),
                                                    );
                                                }
                                            }
                                            toks_def_code => {
                                                if (a >= 4i32) {
                                                    self.geq_define(
                                                        p,
                                                        assign_toks,
                                                        (toks_base).wrapping_add(self.cur_val),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        assign_toks,
                                                        (toks_base).wrapping_add(self.cur_val),
                                                    );
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    read_to_cs => {
                        // §1279
                        {
                            j = self.cur_chr;
                            self.scan_int();
                            n = self.cur_val;
                            if (!self.scan_keyword(66244i32)) {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {}
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66501i32);
                                    }
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66639i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66640i32;
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
                        // §1280
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
                            } else {
                                if (self.cur_chr == XeTeX_inter_char_loc) {
                                    {
                                        self.scan_char_class_not_ignored();
                                        self.cur_ptr = self.cur_val;
                                        self.scan_char_class_not_ignored();
                                        self.find_sa_element(
                                            inter_char_val,
                                            ((self.cur_ptr).wrapping_mul(char_class_limit))
                                                .wrapping_add(self.cur_val),
                                            true,
                                        );
                                        self.cur_chr = self.cur_ptr;
                                        e = true;
                                    }
                                }
                            }
                            p = self.cur_chr;
                            self.scan_optional_equals();
                            // §438
                            loop {
                                self.get_x_token();
                                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                                    break;
                                }
                            }
                            // §1280
                            if (self.cur_cmd != left_brace) {
                                // §1281
                                if ((self.cur_cmd == toks_register)
                                    || (self.cur_cmd == assign_toks))
                                {
                                    {
                                        if (self.cur_cmd == toks_register) {
                                            if (self.cur_chr == mem_bot) {
                                                {
                                                    self.scan_register_num();
                                                    if (self.cur_val < 256i32) {
                                                        q = self.eqtb[crate::ix::U(
                                                            (((toks_base)
                                                                .wrapping_add(self.cur_val))
                                                                - 1)
                                                                as usize,
                                                        )]
                                                        .hh()
                                                        .rh();
                                                    } else {
                                                        {
                                                            self.find_sa_element(
                                                                tok_val,
                                                                self.cur_val,
                                                                false,
                                                            );
                                                            if (self.cur_ptr
                                                                == (268435455i32).wrapping_neg())
                                                            {
                                                                q = (268435455i32).wrapping_neg();
                                                            } else {
                                                                q = self.mem[crate::ix::U(
                                                                    ((self.cur_ptr)
                                                                        .wrapping_add(1i32))
                                                                        as usize,
                                                                )]
                                                                .hh()
                                                                .rh();
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                q = self.mem[crate::ix::U(
                                                    ((self.cur_chr).wrapping_add(1i32)) as usize,
                                                )]
                                                .hh()
                                                .rh();
                                            }
                                        } else {
                                            if (self.cur_chr == XeTeX_inter_char_loc) {
                                                {
                                                    self.scan_char_class_not_ignored();
                                                    self.cur_ptr = self.cur_val;
                                                    self.scan_char_class_not_ignored();
                                                    self.find_sa_element(
                                                        inter_char_val,
                                                        ((self.cur_ptr)
                                                            .wrapping_mul(char_class_limit))
                                                        .wrapping_add(self.cur_val),
                                                        false,
                                                    );
                                                    if (self.cur_ptr
                                                        == (268435455i32).wrapping_neg())
                                                    {
                                                        q = (268435455i32).wrapping_neg();
                                                    } else {
                                                        q = self.mem[crate::ix::U(
                                                            ((self.cur_ptr).wrapping_add(1i32))
                                                                as usize,
                                                        )]
                                                        .hh()
                                                        .rh();
                                                    }
                                                }
                                            } else {
                                                q = self.eqtb
                                                    [crate::ix::U(((self.cur_chr) - 1) as usize)]
                                                .hh()
                                                .rh();
                                            }
                                        }
                                        if (q == (268435455i32).wrapping_neg()) {
                                            if e {
                                                if (a >= 4i32) {
                                                    self.gsa_def(p, (268435455i32).wrapping_neg());
                                                } else {
                                                    self.sa_def(p, (268435455i32).wrapping_neg());
                                                }
                                            } else {
                                                if (a >= 4i32) {
                                                    self.geq_define(
                                                        p,
                                                        undefined_cs,
                                                        (268435455i32).wrapping_neg(),
                                                    );
                                                } else {
                                                    self.eq_define(
                                                        p,
                                                        undefined_cs,
                                                        (268435455i32).wrapping_neg(),
                                                    );
                                                }
                                            }
                                        } else {
                                            {
                                                {
                                                    let __v1630 = (self.mem
                                                        [crate::ix::U((q) as usize)]
                                                    .hh()
                                                    .lh())
                                                    .wrapping_add(1i32);
                                                    self.mem[crate::ix::U((q) as usize)]
                                                        .set_hh_lh(__v1630);
                                                }
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
                            // §1280
                            self.back_input();
                            self.cur_cs = q;
                            q = self.scan_toks(false, false);
                            if (self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh()
                                == (268435455i32).wrapping_neg())
                            {
                                {
                                    if e {
                                        if (a >= 4i32) {
                                            self.gsa_def(p, (268435455i32).wrapping_neg());
                                        } else {
                                            self.sa_def(p, (268435455i32).wrapping_neg());
                                        }
                                    } else {
                                        if (a >= 4i32) {
                                            self.geq_define(
                                                p,
                                                undefined_cs,
                                                (268435455i32).wrapping_neg(),
                                            );
                                        } else {
                                            self.eq_define(
                                                p,
                                                undefined_cs,
                                                (268435455i32).wrapping_neg(),
                                            );
                                        }
                                    }
                                    {
                                        {
                                            let __ix1631 = self.def_ref;
                                            let __v1632 = self.avail;
                                            self.mem[crate::ix::U((__ix1631) as usize)]
                                                .set_hh_rh(__v1632);
                                        }
                                        self.avail = self.def_ref;
                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                    }
                                }
                            } else {
                                {
                                    if ((p == output_routine_loc) && (!e)) {
                                        {
                                            {
                                                let __v1633 = self.get_avail();
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_rh(__v1633);
                                            }
                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                            self.mem[crate::ix::U((q) as usize)]
                                                .set_hh_lh(4194429i32);
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((q) as usize)]
                                                .set_hh_lh(2097275i32);
                                            {
                                                let __v1634 = self.mem
                                                    [crate::ix::U((self.def_ref) as usize)]
                                                .hh()
                                                .rh();
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_rh(__v1634);
                                            }
                                            {
                                                let __ix1635 = self.def_ref;
                                                self.mem[crate::ix::U((__ix1635) as usize)]
                                                    .set_hh_rh(q);
                                            }
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
                        // §1282
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
                        p = self.cur_chr;
                        self.scan_optional_equals();
                        self.scan_dimen(false, false, false);
                        if (a >= 4i32) {
                            self.geq_word_define(p, self.cur_val);
                        } else {
                            self.eq_word_define(p, self.cur_val);
                        }
                    }
                    assign_glue | assign_mu_glue => {
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
                    XeTeX_def_code => {
                        // §1286
                        {
                            if (self.cur_chr == sf_code_base) {
                                {
                                    p = self.cur_chr;
                                    self.scan_usv_num();
                                    p = (p).wrapping_add(self.cur_val);
                                    n = (self.eqtb[crate::ix::U(
                                        (((sf_code_base).wrapping_add(self.cur_val)) - 1) as usize,
                                    )]
                                    .hh()
                                    .rh()
                                        % 65536i32);
                                    self.scan_optional_equals();
                                    self.scan_char_class();
                                    if (a >= 4i32) {
                                        self.geq_define(
                                            p,
                                            data,
                                            ((self.cur_val).wrapping_mul(65536i32)).wrapping_add(n),
                                        );
                                    } else {
                                        self.eq_define(
                                            p,
                                            data,
                                            ((self.cur_val).wrapping_mul(65536i32)).wrapping_add(n),
                                        );
                                    }
                                }
                            } else {
                                if (self.cur_chr == math_code_base) {
                                    {
                                        p = self.cur_chr;
                                        self.scan_usv_num();
                                        p = (p).wrapping_add(self.cur_val);
                                        self.scan_optional_equals();
                                        self.scan_xetex_math_char_int();
                                        if (a >= 4i32) {
                                            self.geq_define(p, data, self.cur_val);
                                        } else {
                                            self.eq_define(p, data, self.cur_val);
                                        }
                                    }
                                } else {
                                    if (self.cur_chr == 5664041i32) {
                                        {
                                            p = (self.cur_chr).wrapping_sub(1i32);
                                            self.scan_usv_num();
                                            p = (p).wrapping_add(self.cur_val);
                                            self.scan_optional_equals();
                                            self.scan_math_class_int();
                                            n = self.set_class_field(self.cur_val);
                                            self.scan_math_fam_int();
                                            n = (n)
                                                .wrapping_add(self.set_family_field(self.cur_val));
                                            self.scan_usv_num();
                                            n = (n).wrapping_add(self.cur_val);
                                            if (a >= 4i32) {
                                                self.geq_define(p, data, n);
                                            } else {
                                                self.eq_define(p, data, n);
                                            }
                                        }
                                    } else {
                                        if (self.cur_chr == del_code_base) {
                                            {
                                                p = self.cur_chr;
                                                self.scan_usv_num();
                                                p = (p).wrapping_add(self.cur_val);
                                                self.scan_optional_equals();
                                                self.scan_int();
                                                if (a >= 4i32) {
                                                    self.geq_word_define(p, self.cur_val);
                                                } else {
                                                    self.eq_word_define(p, self.cur_val);
                                                }
                                            }
                                        } else {
                                            {
                                                p = (self.cur_chr).wrapping_sub(1i32);
                                                self.scan_usv_num();
                                                p = (p).wrapping_add(self.cur_val);
                                                self.scan_optional_equals();
                                                n = 1073741824i32;
                                                self.scan_math_fam_int();
                                                n = (n).wrapping_add(
                                                    (self.cur_val).wrapping_mul(2097152i32),
                                                );
                                                self.scan_usv_num();
                                                n = (n).wrapping_add(self.cur_val);
                                                if (a >= 4i32) {
                                                    self.geq_word_define(p, n);
                                                } else {
                                                    self.eq_word_define(p, n);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    def_code => {
                        {
                            // §1287
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
                                            n = biggest_usv;
                                        }
                                    }
                                }
                            }
                            // §1286
                            p = self.cur_chr;
                            self.scan_usv_num();
                            p = (p).wrapping_add(self.cur_val);
                            self.scan_optional_equals();
                            self.scan_int();
                            if (((self.cur_val < 0i32) && (p < del_code_base))
                                || (self.cur_val > n))
                            {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {}
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66118i32);
                                    }
                                    self.print_int(self.cur_val);
                                    if (p < del_code_base) {
                                        self.print(66650i32);
                                    } else {
                                        self.print(66651i32);
                                    }
                                    self.print_int(n);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66652i32;
                                    }
                                    self.error();
                                    self.cur_val = 0i32;
                                }
                            }
                            if (p < math_code_base) {
                                {
                                    if (p >= sf_code_base) {
                                        {
                                            n = (self.eqtb[crate::ix::U(((p) - 1) as usize)]
                                                .hh()
                                                .rh()
                                                / 65536i32);
                                            if (a >= 4i32) {
                                                self.geq_define(
                                                    p,
                                                    data,
                                                    ((n).wrapping_mul(65536i32))
                                                        .wrapping_add(self.cur_val),
                                                );
                                            } else {
                                                self.eq_define(
                                                    p,
                                                    data,
                                                    ((n).wrapping_mul(65536i32))
                                                        .wrapping_add(self.cur_val),
                                                );
                                            }
                                        }
                                    } else {
                                        if (a >= 4i32) {
                                            self.geq_define(p, data, self.cur_val);
                                        } else {
                                            self.eq_define(p, data, self.cur_val);
                                        }
                                    }
                                }
                            } else {
                                if (p < del_code_base) {
                                    {
                                        if (self.cur_val == 32768i32) {
                                            self.cur_val = active_math_char;
                                        } else {
                                            self.cur_val = ((self
                                                .set_class_field((self.cur_val / 4096i32)))
                                            .wrapping_add(self.set_family_field(
                                                ((self.cur_val % 4096i32) / 256i32),
                                            )))
                                            .wrapping_add((self.cur_val % 256i32));
                                        }
                                        if (a >= 4i32) {
                                            self.geq_define(p, data, self.cur_val);
                                        } else {
                                            self.eq_define(p, data, self.cur_val);
                                        }
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
                    def_family => {
                        // §1288
                        {
                            p = self.cur_chr;
                            self.scan_math_fam_int();
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
                        // §1289
                        self.do_register_command(a);
                    }
                    set_box => {
                        // §1295
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
                                        if (self.interaction == error_stop_mode) {}
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66020i32);
                                    }
                                    self.print_esc(65852i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66658i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66659i32;
                                    }
                                    self.error();
                                }
                            }
                        }
                    }
                    set_aux => {
                        // §1296
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
                        // §1302
                        {
                            q = self.cur_chr;
                            self.scan_optional_equals();
                            self.scan_int();
                            n = self.cur_val;
                            if (n <= 0i32) {
                                p = (268435455i32).wrapping_neg();
                            } else {
                                if (q > par_shape_loc) {
                                    {
                                        n = (self.cur_val / 2i32).wrapping_add(1i32);
                                        p = self
                                            .get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(n);
                                        n = self.cur_val;
                                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                            .set_int(n);
                                        {
                                            let __for_end_10 =
                                                ((p).wrapping_add(n)).wrapping_add(1i32);
                                            j = (p).wrapping_add(2i32);
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_int();
                                                    {
                                                        let __v1636 = self.cur_val;
                                                        self.mem[crate::ix::U((j) as usize)]
                                                            .set_int(__v1636);
                                                    }
                                                }
                                                j = j.wrapping_add(1);
                                            }
                                        }
                                        if (!(((n) % 2) != 0)) {
                                            self.mem[crate::ix::U(
                                                (((p).wrapping_add(n)).wrapping_add(2i32)) as usize,
                                            )]
                                            .set_int(0i32);
                                        }
                                    }
                                } else {
                                    {
                                        p = self
                                            .get_node(((2i32).wrapping_mul(n)).wrapping_add(1i32));
                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(n);
                                        {
                                            let __for_end_10 = n;
                                            j = 1i32;
                                            while j <= __for_end_10 {
                                                {
                                                    self.scan_dimen(false, false, false);
                                                    {
                                                        let __v1637 = self.cur_val;
                                                        self.mem[crate::ix::U(
                                                            (((p).wrapping_add(
                                                                (2i32).wrapping_mul(j),
                                                            ))
                                                            .wrapping_sub(1i32))
                                                                as usize,
                                                        )]
                                                        .set_int(__v1637);
                                                    }
                                                    self.scan_dimen(false, false, false);
                                                    {
                                                        let __v1638 = self.cur_val;
                                                        self.mem[crate::ix::U(
                                                            ((p).wrapping_add(
                                                                (2i32).wrapping_mul(j),
                                                            ))
                                                                as usize,
                                                        )]
                                                        .set_int(__v1638);
                                                    }
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
                        // §1306
                        if (self.cur_chr == 1i32) {
                            {
                                self.new_patterns();
                                break 'l_done_f;
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66663i32);
                                }
                                self.help_ptr = 0i32;
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_cmd == right_brace) {
                                        break;
                                    }
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
                    assign_font_dimen => {
                        // §1307
                        {
                            self.find_font_dimen(true);
                            k = self.cur_val;
                            self.scan_optional_equals();
                            self.scan_dimen(false, false, false);
                            {
                                let __v1639 = self.cur_val;
                                self.font_info[crate::ix::U((k) as usize)].set_int(__v1639);
                            }
                        }
                    }
                    assign_font_int => {
                        n = self.cur_chr;
                        self.scan_font_ident();
                        f = self.cur_val;
                        if (n < lp_code_base) {
                            {
                                self.scan_optional_equals();
                                self.scan_int();
                                if (n == 0i32) {
                                    {
                                        let __v1640 = self.cur_val;
                                        self.hyphen_char[crate::ix::U((f) as usize)] = __v1640;
                                    }
                                } else {
                                    {
                                        let __v1641 = self.cur_val;
                                        self.skew_char[crate::ix::U((f) as usize)] = __v1641;
                                    }
                                }
                            }
                        } else {
                            {
                                if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag)
                                    || (self.font_area[crate::ix::U((f) as usize)]
                                        == otgr_font_flag))
                                {
                                    self.scan_glyph_number(f);
                                } else {
                                    self.scan_char_num();
                                }
                                p = self.cur_val;
                                self.scan_optional_equals();
                                self.scan_int();
                                match n {
                                    lp_code_base => {
                                        self.set_cp_code(f, p, left_side, self.cur_val);
                                    }
                                    rp_code_base => {
                                        self.set_cp_code(f, p, right_side, self.cur_val);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    def_font => {
                        // §1310
                        self.new_font(a);
                    }
                    set_interaction => {
                        // §1318
                        self.new_interaction();
                    }
                    _ => {
                        // §1265
                        self.confusion(66610i32);
                    }
                }
            }
            if (self.after_token != 0i32) {
                // §1323
                {
                    self.cur_tok = self.after_token;
                    self.back_input();
                    self.after_token = 0i32;
                }
            }
        }
        // §1265
    }

    /// Here is a procedure that might be called `Get the next non-blank non-relax
    /// non-call non-assignment token'.
    /// @<Declare act...
    // §1324
    pub fn do_assignments(&mut self) {
        'l_exit_f: {
            while true {
                {
                    // §438
                    loop {
                        self.get_x_token();
                        if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                            break;
                        }
                    }
                    // §1324
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
    // §1329
    pub fn open_or_close_in(&mut self) {
        let mut c: i32 = 0; // §1329
        let mut n: i32 = 0; // §1329
        let mut k: i32 = 0; // §1329
        c = self.cur_chr;
        self.scan_four_bit_int();
        n = self.cur_val;
        if (self.read_open[crate::ix::U((n) as usize)] != closed) {
            {
                {
                    let mut __f0 =
                        ::core::mem::take(&mut self.read_file[crate::ix::U((n) as usize)]);
                    let __r = self.u_close(&mut __f0);
                    self.read_file[crate::ix::U((n) as usize)] = __f0;
                    __r
                };
                self.read_open[crate::ix::U((n) as usize)] = closed;
            }
        }
        if (c != 0i32) {
            {
                self.scan_optional_equals();
                self.scan_file_name();
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                self.set_tex_input_type(false);
                if (self.kpse_in_name_ok() && {
                    let mut __f0 =
                        ::core::mem::take(&mut self.read_file[crate::ix::U((n) as usize)]);
                    let __r = self.u_open_in(
                        &mut __f0,
                        kpse_tex_format,
                        self.eqtb[crate::ix::U(((7892345i32) - 1) as usize)].int(),
                        self.eqtb[crate::ix::U(((7892346i32) - 1) as usize)].int(),
                    );
                    self.read_file[crate::ix::U((n) as usize)] = __f0;
                    __r
                }) {
                    {
                        self.make_utf16_name();
                        self.name_in_progress = true;
                        self.begin_name();
                        self.stop_at_space = false;
                        k = 0i32;
                        while ((k < self.name_length16)
                            && self.more_name(self.name_of_file16[crate::ix::U((k) as usize)]))
                        {
                            k = (k).wrapping_add(1i32);
                        }
                        self.stop_at_space = true;
                        self.end_name();
                        self.name_in_progress = false;
                        self.read_open[crate::ix::U((n) as usize)] = just_open;
                    }
                }
            }
        }
    }

    /// @<Declare act...
    // §1333
    pub fn issue_message(&mut self) {
        let mut old_setting: i32 = 0; // §1333
        let mut c: i32 = 0; // §1333
        let mut s: str_number = 0; // §1333
        c = self.cur_chr;
        {
            let __v1642 = self.scan_toks(false, true);
            self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v1642);
        }
        old_setting = self.selector;
        self.selector = new_string;
        self.token_show(self.def_ref);
        self.selector = old_setting;
        self.flush_list(self.def_ref);
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        if (c == 0i32) {
            // §1334
            {
                if ((self.term_offset).wrapping_add(self.length(s))
                    > (self.max_print_line).wrapping_sub(2i32))
                {
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
            // §1337
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65626i32);
                }
                self.print(s);
                if (self.eqtb[crate::ix::U(((err_help_loc) - 1) as usize)]
                    .hh()
                    .rh()
                    != (268435455i32).wrapping_neg())
                {
                    self.use_err_help = true;
                } else {
                    if self.long_help_seen {
                        {
                            self.help_ptr = 1i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66681i32;
                        }
                    } else {
                        {
                            if (self.interaction < error_stop_mode) {
                                self.long_help_seen = true;
                            }
                            {
                                self.help_ptr = 4i32;
                                self.help_line[crate::ix::U((3i32) as usize)] = 66682i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 66683i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66684i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66685i32;
                            }
                        }
                    }
                }
                self.error();
                self.use_err_help = false;
            }
        }
        // §1333
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr =
                self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
        }
    }

    /// @<Declare act...
    // §1342
    pub fn shift_case(&mut self) {
        let mut b: halfword = 0; // §1342
        let mut p: halfword = 0; // §1342
        let mut t: halfword = 0; // §1342
        let mut c: i32 = 0; // §1342
        b = self.cur_chr;
        p = self.scan_toks(false, false);
        p = self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh();
        while (p != (268435455i32).wrapping_neg()) {
            {
                // §1343
                t = self.mem[crate::ix::U((p) as usize)].hh().lh();
                if (t < 34668544i32) {
                    {
                        c = (t % max_char_val);
                        if (self.eqtb[crate::ix::U((((b).wrapping_add(c)) - 1) as usize)]
                            .hh()
                            .rh()
                            != 0i32)
                        {
                            {
                                let __v1643 = ((t).wrapping_sub(c)).wrapping_add(
                                    self.eqtb[crate::ix::U((((b).wrapping_add(c)) - 1) as usize)]
                                        .hh()
                                        .rh(),
                                );
                                self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v1643);
                            }
                        }
                    }
                }
                // §1342
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        self.begin_token_list(
            self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(),
            backed_up,
        );
        {
            {
                let __ix1644 = self.def_ref;
                let __v1645 = self.avail;
                self.mem[crate::ix::U((__ix1644) as usize)].set_hh_rh(__v1645);
            }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
    }

    /// @<Declare act...
    // §1347
    pub fn show_whatever(&mut self) {
        let mut p: halfword = 0; // §1347
        let mut t: small_number = 0; // §1347
        let mut m: i32 = 0; // §1347
        let mut l: i32 = 0; // §1347
        let mut n: i32 = 0; // §1347
        'l_common_ending_f: {
            match self.cur_chr {
                show_lists_code => {
                    {
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1347
                        self.begin_diagnostic();
                        self.show_activities();
                    }
                }
                show_box_code => {
                    // §1350
                    {
                        self.scan_register_num();
                        if (self.cur_val < 256i32) {
                            p = self.eqtb[crate::ix::U(
                                (((box_base).wrapping_add(self.cur_val)) - 1) as usize,
                            )]
                            .hh()
                            .rh();
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                    p = (268435455i32).wrapping_neg();
                                } else {
                                    p = self.mem[crate::ix::U(
                                        ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                    )]
                                    .hh()
                                    .rh();
                                }
                            }
                        }
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1350
                        self.begin_diagnostic();
                        self.print_nl(66701i32);
                        self.print_int(self.cur_val);
                        self.print_char(61i32);
                        if (p == (268435455i32).wrapping_neg()) {
                            self.print(65702i32);
                        } else {
                            self.show_box(p);
                        }
                    }
                }
                show_code => {
                    // §1348
                    {
                        self.get_token();
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1348
                        if (self.interaction == error_stop_mode) {}
                        self.print_nl(66697i32);
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
                    // §1487
                    {
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1487
                        self.begin_diagnostic();
                        self.show_save_groups();
                    }
                }
                show_ifs => {
                    // §1501
                    {
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1501
                        self.begin_diagnostic();
                        self.print_nl(65626i32);
                        self.print_ln();
                        if (self.cond_ptr == (268435455i32).wrapping_neg()) {
                            {
                                self.print_nl(65654i32);
                                self.print(66892i32);
                            }
                        } else {
                            {
                                p = self.cond_ptr;
                                n = 0i32;
                                loop {
                                    n = (n).wrapping_add(1i32);
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (p == (268435455i32).wrapping_neg()) {
                                        break;
                                    }
                                }
                                p = self.cond_ptr;
                                t = self.cur_if;
                                l = self.if_line;
                                m = self.if_limit;
                                loop {
                                    self.print_nl(66893i32);
                                    self.print_int(n);
                                    self.print(65593i32);
                                    self.print_cmd_chr(if_test, t);
                                    if (m == fi_code) {
                                        self.print_esc(66158i32);
                                    }
                                    if (l != 0i32) {
                                        {
                                            self.print(66891i32);
                                            self.print_int(l);
                                        }
                                    }
                                    n = (n).wrapping_sub(1i32);
                                    t = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    l = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                        .int();
                                    m = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    if (p == (268435455i32).wrapping_neg()) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // §1351
                    {
                        p = self.the_toks();
                        // §1702
                        if (((self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int() >= 0i32)
                            && (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int()
                                < no_print))
                            && self.write_open[crate::ix::U(
                                (self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int())
                                    as usize,
                            )])
                        {
                            self.selector =
                                self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].int();
                        }
                        // §1351
                        if (self.interaction == error_stop_mode) {}
                        self.print_nl(66697i32);
                        self.token_show(temp_head);
                        self.flush_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
                        break 'l_common_ending_f;
                    }
                }
            }
            // §1352
            self.end_diagnostic(true);
            {
                if (self.interaction == error_stop_mode) {}
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(65544i32);
                }
                self.print(66702i32);
            }
            if (self.selector == term_and_log) {
                if (self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int() <= 0i32) {
                    {
                        self.selector = term_only;
                        self.print(66703i32);
                        self.selector = term_and_log;
                    }
                }
            }
        }
        // §1347
        if (self.selector < no_print) {
            {
                self.print_ln();
                // §79
                if (self.interaction == batch_mode) {
                    self.selector = no_print;
                } else {
                    self.selector = term_only;
                }
                // §1347
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
                    if (self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int() > 0i32) {
                        {
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 66692i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66693i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66694i32;
                            }
                        }
                    } else {
                        {
                            {
                                self.help_ptr = 5i32;
                                self.help_line[crate::ix::U((4i32) as usize)] = 66692i32;
                                self.help_line[crate::ix::U((3i32) as usize)] = 66693i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 66694i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66695i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66696i32;
                            }
                        }
                    }
                }
                self.error();
            }
        }
    }

    /// @<Declare act...
    // §1356
    pub fn store_fmt_file(&mut self) {
        let mut j: i32 = 0; // §1356
        let mut k: i32 = 0; // §1356
        let mut l: i32 = 0; // §1356
        let mut p: halfword = 0; // §1356
        let mut q: halfword = 0; // §1356
        let mut x: i32 = 0; // §1356
        let mut w: four_quarters = four_quarters::default(); // §1356
                                                             // §1358
        if (self.save_ptr != 0i32) {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66705i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66706i32;
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
        // §1382
        self.selector = new_string;
        self.print(66723i32);
        self.print(self.job_name);
        self.print_char(32i32);
        self.print_int(self.eqtb[crate::ix::U(((7892287i32) - 1) as usize)].int());
        self.print_char(46i32);
        self.print_int(self.eqtb[crate::ix::U(((7892286i32) - 1) as usize)].int());
        self.print_char(46i32);
        self.print_int(self.eqtb[crate::ix::U(((7892285i32) - 1) as usize)].int());
        self.print_char(41i32);
        if (self.interaction == batch_mode) {
            self.selector = log_only;
        } else {
            self.selector = term_and_log;
        }
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.format_ident = self.make_string();
        self.pack_job_name(format_extension);
        while (!{
            let mut __f0 = ::core::mem::take(&mut self.fmt_file);
            let __r = self.w_open_out(&mut __f0);
            self.fmt_file = __f0;
            __r
        }) {
            self.prompt_file_name(66724i32, format_extension);
        }
        self.print_nl(66725i32);
        {
            let __a1646_0 = {
                let mut __f0 = ::core::mem::take(&mut self.fmt_file);
                let __r = self.w_make_name_string(&mut __f0);
                self.fmt_file = __f0;
                __r
            };
            self.print(__a1646_0)
        };
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr =
                self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
        }
        self.print_nl(65626i32);
        self.print(self.format_ident);
        // §1361
        {
            self.fmt_file.buf.set_int(173681847i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1464
        {
            {
                let __v1647 = self.eTeX_mode;
                self.fmt_file.buf.set_int(__v1647);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1569
        while (self.pseudo_files != (268435455i32).wrapping_neg()) {
            self.pseudo_close();
        }
        // §1361
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
            {
                let __v1648 = self.hash_high;
                self.fmt_file.buf.set_int(__v1648);
            }
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
        // §1363
        {
            {
                let __v1649 = self.pool_ptr;
                self.fmt_file.buf.set_int(__v1649);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1650 = self.str_ptr;
                self.fmt_file.buf.set_int(__v1650);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.str_ptr;
            k = too_big_char;
            while k <= __for_end_2 {
                {
                    {
                        let __v1651 =
                            self.str_start[crate::ix::U(((k).wrapping_sub(65536i32)) as usize)];
                        self.fmt_file.buf.set_int(__v1651);
                    }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        k = 0i32;
        while ((k).wrapping_add(4i32) < self.pool_ptr) {
            {
                {
                    let __v1652 = self.str_pool[crate::ix::U((k) as usize)];
                    w.set_b0(__v1652);
                }
                {
                    let __v1653 = self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
                    w.set_b1(__v1653);
                }
                {
                    let __v1654 = self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)];
                    w.set_b2(__v1654);
                }
                {
                    let __v1655 = self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)];
                    w.set_b3(__v1655);
                }
                {
                    self.fmt_file.buf.set_qqqq(w);
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = (k).wrapping_add(4i32);
            }
        }
        k = (self.pool_ptr).wrapping_sub(4i32);
        {
            let __v1656 = self.str_pool[crate::ix::U((k) as usize)];
            w.set_b0(__v1656);
        }
        {
            let __v1657 = self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
            w.set_b1(__v1657);
        }
        {
            let __v1658 = self.str_pool[crate::ix::U(((k).wrapping_add(2i32)) as usize)];
            w.set_b2(__v1658);
        }
        {
            let __v1659 = self.str_pool[crate::ix::U(((k).wrapping_add(3i32)) as usize)];
            w.set_b3(__v1659);
        }
        {
            self.fmt_file.buf.set_qqqq(w);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(self.str_ptr);
        self.print(66707i32);
        self.print_int(self.pool_ptr);
        // §1365
        self.sort_avail();
        self.var_used = 0i32;
        {
            {
                let __v1660 = self.lo_mem_max;
                self.fmt_file.buf.set_int(__v1660);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1661 = self.rover;
                self.fmt_file.buf.set_int(__v1661);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        if (self.eTeX_mode == 1i32) {
            {
                let __for_end_3 = inter_char_val;
                k = int_val;
                while k <= __for_end_3 {
                    {
                        {
                            let __v1662 = self.sa_root[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1662);
                        }
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
                        self.fmt_file.buf = self.mem[crate::ix::U((k) as usize)];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
            x = (((x).wrapping_add(q)).wrapping_add(2i32)).wrapping_sub(p);
            self.var_used = ((self.var_used).wrapping_add(q)).wrapping_sub(p);
            p = (q).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().lh());
            q = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                .hh()
                .rh();
            if (q == self.rover) {
                break;
            }
        }
        self.var_used = ((self.var_used).wrapping_add(self.lo_mem_max)).wrapping_sub(p);
        self.dyn_used = ((self.mem_end).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        {
            let __for_end_2 = self.lo_mem_max;
            k = p;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.mem[crate::ix::U((k) as usize)];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.lo_mem_max)).wrapping_add(1i32)).wrapping_sub(p);
        {
            {
                let __v1663 = self.hi_mem_min;
                self.fmt_file.buf.set_int(__v1663);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1664 = self.avail;
                self.fmt_file.buf.set_int(__v1664);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.mem_end;
            k = self.hi_mem_min;
            while k <= __for_end_2 {
                {
                    self.fmt_file.buf = self.mem[crate::ix::U((k) as usize)];
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        x = (((x).wrapping_add(self.mem_end)).wrapping_add(1i32)).wrapping_sub(self.hi_mem_min);
        p = self.avail;
        while (p != (268435455i32).wrapping_neg()) {
            {
                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        {
            {
                let __v1665 = self.var_used;
                self.fmt_file.buf.set_int(__v1665);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1666 = self.dyn_used;
                self.fmt_file.buf.set_int(__v1666);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(x);
        self.print(66708i32);
        self.print_int(self.var_used);
        self.print_char(38i32);
        self.print_int(self.dyn_used);
        // §1369
        k = active_base;
        loop {
            'l_done1_f: {
                'l_found1_f: {
                    j = k;
                    while (j < 7892263i32) {
                        {
                            if (((self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().rh()
                                == self.eqtb
                                    [crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                .hh()
                                .rh())
                                && (self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().b0()
                                    == self.eqtb
                                        [crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                    .hh()
                                    .b0()))
                                && (self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().b1()
                                    == self.eqtb
                                        [crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                    .hh()
                                    .b1()))
                            {
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
                while (j < 7892263i32) {
                    {
                        if (((self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().rh()
                            != self.eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                .hh()
                                .rh())
                            || (self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().b0()
                                != self.eqtb
                                    [crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                .hh()
                                .b0()))
                            || (self.eqtb[crate::ix::U(((j) - 1) as usize)].hh().b1()
                                != self.eqtb
                                    [crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                .hh()
                                .b1()))
                        {
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
                        self.fmt_file.buf = self.eqtb[crate::ix::U(((k) - 1) as usize)];
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
            if (k == int_base) {
                break;
            }
        }
        // §1370
        loop {
            'l_done2_f: {
                'l_found2_f: {
                    j = k;
                    while (j < eqtb_size) {
                        {
                            if (self.eqtb[crate::ix::U(((j) - 1) as usize)].int()
                                == self.eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                    .int())
                            {
                                break 'l_found2_f;
                            }
                            j = (j).wrapping_add(1i32);
                        }
                    }
                    l = 9006999i32;
                    break 'l_done2_f;
                }
                j = (j).wrapping_add(1i32);
                l = j;
                while (j < eqtb_size) {
                    {
                        if (self.eqtb[crate::ix::U(((j) - 1) as usize)].int()
                            != self.eqtb[crate::ix::U((((j).wrapping_add(1i32)) - 1) as usize)]
                                .int())
                        {
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
                        self.fmt_file.buf = self.eqtb[crate::ix::U(((k) - 1) as usize)];
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
            if (k > eqtb_size) {
                break;
            }
        }
        if (self.hash_high > 0i32) {
            {
                let __for_end_3 = (eqtb_size).wrapping_add(self.hash_high);
                k = 9006999i32;
                while k <= __for_end_3 {
                    {
                        self.fmt_file.buf = self.eqtb[crate::ix::U(((k) - 1) as usize)];
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    k = k.wrapping_add(1);
                }
            }
        }
        // §1367
        {
            {
                let __v1667 = self.par_loc;
                self.fmt_file.buf.set_int(__v1667);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1668 = self.write_loc;
                self.fmt_file.buf.set_int(__v1668);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        // §1372
        {
            let __for_end_2 = prim_size;
            p = 0i32;
            while p <= __for_end_2 {
                {
                    {
                        let __v1669 = self.prim[crate::ix::U((p) as usize)];
                        self.fmt_file.buf.set_hh(__v1669);
                    }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        {
            {
                let __v1670 = self.hash_used;
                self.fmt_file.buf.set_int(__v1670);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.cs_count = ((1194649i32).wrapping_sub(self.hash_used)).wrapping_add(self.hash_high);
        {
            let __for_end_2 = self.hash_used;
            p = hash_base;
            while p <= __for_end_2 {
                if (self.hash[crate::ix::U(((p) - 1179650) as usize)].rh() != 0i32) {
                    {
                        {
                            self.fmt_file.buf.set_int(p);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            {
                                let __v1671 = self.hash[crate::ix::U(((p) - 1179650) as usize)];
                                self.fmt_file.buf.set_hh(__v1671);
                            }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        self.cs_count = (self.cs_count).wrapping_add(1i32);
                    }
                }
                p = p.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 1205762i32;
            p = (self.hash_used).wrapping_add(1i32);
            while p <= __for_end_2 {
                {
                    {
                        let __v1672 = self.hash[crate::ix::U(((p) - 1179650) as usize)];
                        self.fmt_file.buf.set_hh(__v1672);
                    }
                    crate::system::put_word(&mut self.fmt_file);
                }
                p = p.wrapping_add(1);
            }
        }
        if (self.hash_high > 0i32) {
            {
                let __for_end_3 = (eqtb_size).wrapping_add(self.hash_high);
                p = 9006999i32;
                while p <= __for_end_3 {
                    {
                        {
                            let __v1673 = self.hash[crate::ix::U(((p) - 1179650) as usize)];
                            self.fmt_file.buf.set_hh(__v1673);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    p = p.wrapping_add(1);
                }
            }
        }
        {
            {
                let __v1674 = self.cs_count;
                self.fmt_file.buf.set_int(__v1674);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        self.print_ln();
        self.print_int(self.cs_count);
        self.print(66709i32);
        // §1374
        {
            {
                let __v1675 = self.fmem_ptr;
                self.fmt_file.buf.set_int(__v1675);
            }
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
            {
                let __v1676 = self.font_ptr;
                self.fmt_file.buf.set_int(__v1676);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.font_ptr;
            k = null_font;
            while k <= __for_end_2 {
                // §1376
                {
                    {
                        {
                            let __v1677 = self.font_check[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_qqqq(__v1677);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1678 = self.font_size[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1678);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1679 = self.font_dsize[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1679);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1680 = self.font_params[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1680);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1681 = self.hyphen_char[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1681);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1682 = self.skew_char[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1682);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1683 = self.font_name[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1683);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1684 = self.font_area[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1684);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1685 = self.font_bc[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1685);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1686 = self.font_ec[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1686);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1687 = self.char_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1687);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1688 = self.width_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1688);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1689 = self.height_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1689);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1690 = self.depth_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1690);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1691 = self.italic_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1691);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1692 = self.lig_kern_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1692);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1693 = self.kern_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1693);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1694 = self.exten_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1694);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1695 = self.param_base[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1695);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1696 = self.font_glue[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1696);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1697 = self.bchar_label[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1697);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1698 = self.font_bchar[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1698);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1699 = self.font_false_bchar[crate::ix::U((k) as usize)];
                            self.fmt_file.buf.set_int(__v1699);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    self.print_nl(66712i32);
                    self.print_esc(
                        self.hash
                            [crate::ix::U((((font_id_base).wrapping_add(k)) - 1179650) as usize)]
                        .rh(),
                    );
                    self.print_char(61i32);
                    if (((self.font_area[crate::ix::U((k) as usize)] == aat_font_flag)
                        || (self.font_area[crate::ix::U((k) as usize)] == otgr_font_flag))
                        || (self.font_mapping[crate::ix::U((k) as usize)] != 0i32))
                    {
                        {
                            self.print_file_name(
                                self.font_name[crate::ix::U((k) as usize)],
                                65626i32,
                                65626i32,
                            );
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66713i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[crate::ix::U((2i32) as usize)] = 66714i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66715i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66716i32;
                            }
                            self.error();
                        }
                    } else {
                        self.print_file_name(
                            self.font_name[crate::ix::U((k) as usize)],
                            self.font_area[crate::ix::U((k) as usize)],
                            65626i32,
                        );
                    }
                    if (self.font_size[crate::ix::U((k) as usize)]
                        != self.font_dsize[crate::ix::U((k) as usize)])
                    {
                        {
                            self.print(66121i32);
                            self.print_scaled(self.font_size[crate::ix::U((k) as usize)]);
                            self.print(65689i32);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        // §1374
        self.print_ln();
        self.print_int((self.fmem_ptr).wrapping_sub(7i32));
        self.print(66710i32);
        self.print_int((self.font_ptr).wrapping_sub(0i32));
        self.print(66711i32);
        if (self.font_ptr != 1i32) {
            self.print_char(115i32);
        }
        // §1378
        {
            {
                let __v1700 = self.hyph_count;
                self.fmt_file.buf.set_int(__v1700);
            }
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
                            {
                                let __v1701 = self.hyph_word[crate::ix::U((k) as usize)];
                                self.fmt_file.buf.set_int(__v1701);
                            }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            {
                                let __v1702 = self.hyph_list[crate::ix::U((k) as usize)];
                                self.fmt_file.buf.set_int(__v1702);
                            }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.print_int(self.hyph_count);
        self.print(66717i32);
        if (self.hyph_count != 1i32) {
            self.print_char(115i32);
        }
        if self.trie_not_ready {
            self.init_trie();
        }
        {
            {
                let __v1703 = self.trie_max;
                self.fmt_file.buf.set_int(__v1703);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1704 = self.hyph_start;
                self.fmt_file.buf.set_int(__v1704);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_max;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    {
                        let __v1705 = self.trie[crate::ix::U((k) as usize)];
                        self.fmt_file.buf.set_hh(__v1705);
                    }
                    crate::system::put_word(&mut self.fmt_file);
                }
                k = k.wrapping_add(1);
            }
        }
        {
            {
                let __v1706 = self.max_hyph_char;
                self.fmt_file.buf.set_int(__v1706);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1707 = self.trie_op_ptr;
                self.fmt_file.buf.set_int(__v1707);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            let __for_end_2 = self.trie_op_ptr;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    {
                        {
                            let __v1708 = self.hyf_distance[crate::ix::U(((k) - 1) as usize)];
                            self.fmt_file.buf.set_int(__v1708);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1709 = self.hyf_num[crate::ix::U(((k) - 1) as usize)];
                            self.fmt_file.buf.set_int(__v1709);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                    {
                        {
                            let __v1710 = self.hyf_next[crate::ix::U(((k) - 1) as usize)];
                            self.fmt_file.buf.set_int(__v1710);
                        }
                        crate::system::put_word(&mut self.fmt_file);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.print_nl(66718i32);
        self.print_int(self.trie_max);
        self.print(66719i32);
        self.print_int(self.trie_op_ptr);
        self.print(66720i32);
        if (self.trie_op_ptr != 1i32) {
            self.print_char(115i32);
        }
        self.print(66721i32);
        self.print_int(trie_op_size);
        {
            let __for_end_2 = 0i32;
            k = biggest_lang;
            while k >= __for_end_2 {
                if (self.trie_used[crate::ix::U((k) as usize)] > min_quarterword) {
                    {
                        self.print_nl(66185i32);
                        self.print_int(self.trie_used[crate::ix::U((k) as usize)]);
                        self.print(66722i32);
                        self.print_int(k);
                        {
                            self.fmt_file.buf.set_int(k);
                            crate::system::put_word(&mut self.fmt_file);
                        }
                        {
                            {
                                let __v1711 = self.trie_used[crate::ix::U((k) as usize)];
                                self.fmt_file.buf.set_int(__v1711);
                            }
                            crate::system::put_word(&mut self.fmt_file);
                        }
                    }
                }
                k = k.wrapping_sub(1);
            }
        }
        // §1380
        {
            {
                let __v1712 = self.interaction;
                self.fmt_file.buf.set_int(__v1712);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            {
                let __v1713 = self.format_ident;
                self.fmt_file.buf.set_int(__v1713);
            }
            crate::system::put_word(&mut self.fmt_file);
        }
        {
            self.fmt_file.buf.set_int(69069i32);
            crate::system::put_word(&mut self.fmt_file);
        }
        self.eqtb[crate::ix::U(((7892295i32) - 1) as usize)].set_int(0i32);
        // §1383
        {
            let mut __f0 = ::core::mem::take(&mut self.fmt_file);
            let __r = self.w_close(&mut __f0);
            self.fmt_file = __f0;
            __r
        };
    }

    /// Here is a subroutine that creates a whatsit node having a given `subtype`
    /// and a given number of words. It initializes only the first word of the whatsit,
    /// and appends it to the current list.
    /// @<Declare procedures needed in `do_extension`
    // §1404
    pub fn new_whatsit(&mut self, mut s: small_number, mut w: small_number) {
        let mut p: halfword = 0; // §1404
        p = self.get_node(w);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        {
            let __ix1714 = self.cur_list.tail_field;
            self.mem[crate::ix::U((__ix1714) as usize)].set_hh_rh(p);
        }
        self.cur_list.tail_field = p;
    }

    /// The next subroutine uses `cur_chr` to decide what sort of whatsit is
    /// involved, and also inserts a `write_stream` number.
    /// @<Declare procedures needed in `do_ext...
    // §1405
    pub fn new_write_whatsit(&mut self, mut w: small_number) {
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
        {
            let __ix1715 = (self.cur_list.tail_field).wrapping_add(1i32);
            let __v1716 = self.cur_val;
            self.mem[crate::ix::U((__ix1715) as usize)].set_hh_lh(__v1716);
        }
    }

    /// Load a picture file and handle following keywords.
    // §1445
    pub fn load_picture(&mut self, mut is_pdf: bool) {
        let mut pic_path: i32 = 0; // §1445
        let mut bounds: real_rect = real_rect::default(); // §1445
        let mut t: transform = transform::default(); // §1445
        let mut t2: transform = transform::default(); // §1445
        let mut corners: Vec<real_point> = vec![real_point::default(); 4]; // §1445
        let mut x_size_req: f64 = 0.0; // §1445
        let mut y_size_req: f64 = 0.0; // §1445
        let mut check_keywords: bool = false; // §1445
        let mut xmin: f64 = 0.0; // §1445
        let mut xmax: f64 = 0.0; // §1445
        let mut ymin: f64 = 0.0; // §1445
        let mut ymax: f64 = 0.0; // §1445
        let mut i: small_number = 0; // §1445
        let mut page: i32 = 0; // §1445
        let mut pdf_box_type: i32 = 0; // §1445
        let mut result: i32 = 0; // §1445
        self.scan_file_name();
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
        pdf_box_type = 0i32;
        page = 0i32;
        if is_pdf {
            {
                if self.scan_keyword(66422i32) {
                    {
                        self.scan_int();
                        page = self.cur_val;
                    }
                }
                pdf_box_type = pdfbox_none;
                if self.scan_keyword(66784i32) {
                    pdf_box_type = pdfbox_crop;
                } else {
                    if self.scan_keyword(66785i32) {
                        pdf_box_type = pdfbox_media;
                    } else {
                        if self.scan_keyword(66786i32) {
                            pdf_box_type = pdfbox_bleed;
                        } else {
                            if self.scan_keyword(66787i32) {
                                pdf_box_type = pdfbox_trim;
                            } else {
                                if self.scan_keyword(66788i32) {
                                    pdf_box_type = pdfbox_art;
                                }
                            }
                        }
                    }
                }
            }
        }
        if (pdf_box_type == pdfbox_none) {
            result = {
                let mut __f0 = ::core::mem::take(&mut pic_path);
                let mut __f1 = ::core::mem::take(&mut bounds);
                let __r = self.find_pic_file(&mut __f0, &mut __f1, pdfbox_crop, page);
                pic_path = __f0;
                bounds = __f1;
                __r
            };
        } else {
            result = {
                let mut __f0 = ::core::mem::take(&mut pic_path);
                let mut __f1 = ::core::mem::take(&mut bounds);
                let __r = self.find_pic_file(&mut __f0, &mut __f1, pdf_box_type, page);
                pic_path = __f0;
                bounds = __f1;
                __r
            };
        }
        {
            let mut __f0 = ::core::mem::take(&mut corners[crate::ix::U((0i32) as usize)]);
            let __r = self.setPoint(&mut __f0, bounds.x, bounds.y);
            corners[crate::ix::U((0i32) as usize)] = __f0;
            __r
        };
        {
            let mut __f0 = ::core::mem::take(&mut corners[crate::ix::U((1i32) as usize)]);
            let __r = self.setPoint(
                &mut __f0,
                corners[crate::ix::U((0i32) as usize)].x,
                (bounds.y + bounds.ht),
            );
            corners[crate::ix::U((1i32) as usize)] = __f0;
            __r
        };
        {
            let mut __f0 = ::core::mem::take(&mut corners[crate::ix::U((2i32) as usize)]);
            let __r = self.setPoint(
                &mut __f0,
                (bounds.x + bounds.wd),
                corners[crate::ix::U((1i32) as usize)].y,
            );
            corners[crate::ix::U((2i32) as usize)] = __f0;
            __r
        };
        {
            let mut __f0 = ::core::mem::take(&mut corners[crate::ix::U((3i32) as usize)]);
            let __r = self.setPoint(
                &mut __f0,
                corners[crate::ix::U((2i32) as usize)].x,
                corners[crate::ix::U((0i32) as usize)].y,
            );
            corners[crate::ix::U((3i32) as usize)] = __f0;
            __r
        };
        x_size_req = 0.0f64;
        y_size_req = 0.0f64;
        {
            let mut __f0 = ::core::mem::take(&mut t);
            let __r = self.make_identity(&mut __f0);
            t = __f0;
            __r
        };
        check_keywords = true;
        while check_keywords {
            {
                if self.scan_keyword(66670i32) {
                    {
                        self.scan_int();
                        if ((x_size_req == 0.0f64) && (y_size_req == 0.0f64)) {
                            {
                                {
                                    let mut __f0 = ::core::mem::take(&mut t2);
                                    let __r = self.make_scale(
                                        &mut __f0,
                                        (((self.cur_val) as f64) / 1000.0f64),
                                        (((self.cur_val) as f64) / 1000.0f64),
                                    );
                                    t2 = __f0;
                                    __r
                                };
                                {
                                    let __for_end_8 = 3i32;
                                    i = 0i32;
                                    while i <= __for_end_8 {
                                        {
                                            let mut __f0 = ::core::mem::take(
                                                &mut corners[crate::ix::U((i) as usize)],
                                            );
                                            let mut __f1 = ::core::mem::take(&mut t2);
                                            let __r = self.transform_point(&mut __f0, &mut __f1);
                                            corners[crate::ix::U((i) as usize)] = __f0;
                                            t2 = __f1;
                                            __r
                                        };
                                        i = i.wrapping_add(1);
                                    }
                                }
                                {
                                    let mut __f0 = ::core::mem::take(&mut t);
                                    let mut __f1 = ::core::mem::take(&mut t2);
                                    let __r = self.transform_concat(&mut __f0, &mut __f1);
                                    t = __f0;
                                    t2 = __f1;
                                    __r
                                };
                            }
                        }
                    }
                } else {
                    if self.scan_keyword(66789i32) {
                        {
                            self.scan_int();
                            if ((x_size_req == 0.0f64) && (y_size_req == 0.0f64)) {
                                {
                                    {
                                        let mut __f0 = ::core::mem::take(&mut t2);
                                        let __r = self.make_scale(
                                            &mut __f0,
                                            (((self.cur_val) as f64) / 1000.0f64),
                                            1.0f64,
                                        );
                                        t2 = __f0;
                                        __r
                                    };
                                    {
                                        let __for_end_9 = 3i32;
                                        i = 0i32;
                                        while i <= __for_end_9 {
                                            {
                                                let mut __f0 = ::core::mem::take(
                                                    &mut corners[crate::ix::U((i) as usize)],
                                                );
                                                let mut __f1 = ::core::mem::take(&mut t2);
                                                let __r =
                                                    self.transform_point(&mut __f0, &mut __f1);
                                                corners[crate::ix::U((i) as usize)] = __f0;
                                                t2 = __f1;
                                                __r
                                            };
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                    {
                                        let mut __f0 = ::core::mem::take(&mut t);
                                        let mut __f1 = ::core::mem::take(&mut t2);
                                        let __r = self.transform_concat(&mut __f0, &mut __f1);
                                        t = __f0;
                                        t2 = __f1;
                                        __r
                                    };
                                }
                            }
                        }
                    } else {
                        if self.scan_keyword(66790i32) {
                            {
                                self.scan_int();
                                if ((x_size_req == 0.0f64) && (y_size_req == 0.0f64)) {
                                    {
                                        {
                                            let mut __f0 = ::core::mem::take(&mut t2);
                                            let __r = self.make_scale(
                                                &mut __f0,
                                                1.0f64,
                                                (((self.cur_val) as f64) / 1000.0f64),
                                            );
                                            t2 = __f0;
                                            __r
                                        };
                                        {
                                            let __for_end_10 = 3i32;
                                            i = 0i32;
                                            while i <= __for_end_10 {
                                                {
                                                    let mut __f0 = ::core::mem::take(
                                                        &mut corners[crate::ix::U((i) as usize)],
                                                    );
                                                    let mut __f1 = ::core::mem::take(&mut t2);
                                                    let __r =
                                                        self.transform_point(&mut __f0, &mut __f1);
                                                    corners[crate::ix::U((i) as usize)] = __f0;
                                                    t2 = __f1;
                                                    __r
                                                };
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        {
                                            let mut __f0 = ::core::mem::take(&mut t);
                                            let mut __f1 = ::core::mem::take(&mut t2);
                                            let __r = self.transform_concat(&mut __f0, &mut __f1);
                                            t = __f0;
                                            t2 = __f1;
                                            __r
                                        };
                                    }
                                }
                            }
                        } else {
                            if self.scan_keyword(66084i32) {
                                {
                                    self.scan_dimen(false, false, false);
                                    if (self.cur_val <= 0i32) {
                                        {
                                            {
                                                if (self.interaction == error_stop_mode) {}
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(66791i32);
                                            }
                                            self.print(66792i32);
                                            self.print_scaled(self.cur_val);
                                            self.print(66793i32);
                                            {
                                                self.help_ptr = 2i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] =
                                                    66794i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] =
                                                    66795i32;
                                            }
                                            self.error();
                                        }
                                    } else {
                                        x_size_req = self.Fix2D(self.cur_val);
                                    }
                                }
                            } else {
                                if self.scan_keyword(66085i32) {
                                    {
                                        self.scan_dimen(false, false, false);
                                        if (self.cur_val <= 0i32) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {}
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66791i32);
                                                }
                                                self.print(66792i32);
                                                self.print_scaled(self.cur_val);
                                                self.print(66793i32);
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] =
                                                        66794i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] =
                                                        66795i32;
                                                }
                                                self.error();
                                            }
                                        } else {
                                            y_size_req = self.Fix2D(self.cur_val);
                                        }
                                    }
                                } else {
                                    if self.scan_keyword(66796i32) {
                                        {
                                            self.scan_decimal();
                                            if ((x_size_req != 0.0f64) || (y_size_req != 0.0f64)) {
                                                {
                                                    {
                                                        xmin = 1000000.0f64;
                                                        xmax = ((((xmin) as i32).wrapping_neg())
                                                            as f64);
                                                        ymin = xmin;
                                                        ymax = xmax;
                                                        {
                                                            let __for_end_14 = 3i32;
                                                            i = 0i32;
                                                            while i <= __for_end_14 {
                                                                {
                                                                    if (corners[crate::ix::U(
                                                                        (i) as usize,
                                                                    )]
                                                                    .x < xmin)
                                                                    {
                                                                        xmin = corners
                                                                            [crate::ix::U(
                                                                                (i) as usize,
                                                                            )]
                                                                        .x;
                                                                    }
                                                                    if (corners[crate::ix::U(
                                                                        (i) as usize,
                                                                    )]
                                                                    .x > xmax)
                                                                    {
                                                                        xmax = corners
                                                                            [crate::ix::U(
                                                                                (i) as usize,
                                                                            )]
                                                                        .x;
                                                                    }
                                                                    if (corners[crate::ix::U(
                                                                        (i) as usize,
                                                                    )]
                                                                    .y < ymin)
                                                                    {
                                                                        ymin = corners
                                                                            [crate::ix::U(
                                                                                (i) as usize,
                                                                            )]
                                                                        .y;
                                                                    }
                                                                    if (corners[crate::ix::U(
                                                                        (i) as usize,
                                                                    )]
                                                                    .y > ymax)
                                                                    {
                                                                        ymax = corners
                                                                            [crate::ix::U(
                                                                                (i) as usize,
                                                                            )]
                                                                        .y;
                                                                    }
                                                                }
                                                                i = i.wrapping_add(1);
                                                            }
                                                        }
                                                    }
                                                    if (x_size_req == 0.0f64) {
                                                        {
                                                            {
                                                                let mut __f0 =
                                                                    ::core::mem::take(&mut t2);
                                                                let __r = self.make_scale(
                                                                    &mut __f0,
                                                                    (y_size_req / (ymax - ymin)),
                                                                    (y_size_req / (ymax - ymin)),
                                                                );
                                                                t2 = __f0;
                                                                __r
                                                            };
                                                        }
                                                    } else {
                                                        if (y_size_req == 0.0f64) {
                                                            {
                                                                {
                                                                    let mut __f0 =
                                                                        ::core::mem::take(&mut t2);
                                                                    let __r = self.make_scale(
                                                                        &mut __f0,
                                                                        (x_size_req
                                                                            / (xmax - xmin)),
                                                                        (x_size_req
                                                                            / (xmax - xmin)),
                                                                    );
                                                                    t2 = __f0;
                                                                    __r
                                                                };
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    let mut __f0 =
                                                                        ::core::mem::take(&mut t2);
                                                                    let __r = self.make_scale(
                                                                        &mut __f0,
                                                                        (x_size_req
                                                                            / (xmax - xmin)),
                                                                        (y_size_req
                                                                            / (ymax - ymin)),
                                                                    );
                                                                    t2 = __f0;
                                                                    __r
                                                                };
                                                            }
                                                        }
                                                    }
                                                    {
                                                        let __for_end_13 = 3i32;
                                                        i = 0i32;
                                                        while i <= __for_end_13 {
                                                            {
                                                                let mut __f0 = ::core::mem::take(
                                                                    &mut corners[crate::ix::U(
                                                                        (i) as usize,
                                                                    )],
                                                                );
                                                                let mut __f1 =
                                                                    ::core::mem::take(&mut t2);
                                                                let __r = self.transform_point(
                                                                    &mut __f0, &mut __f1,
                                                                );
                                                                corners
                                                                    [crate::ix::U((i) as usize)] =
                                                                    __f0;
                                                                t2 = __f1;
                                                                __r
                                                            };
                                                            i = i.wrapping_add(1);
                                                        }
                                                    }
                                                    x_size_req = 0.0f64;
                                                    y_size_req = 0.0f64;
                                                    {
                                                        let mut __f0 = ::core::mem::take(&mut t);
                                                        let mut __f1 = ::core::mem::take(&mut t2);
                                                        let __r = self
                                                            .transform_concat(&mut __f0, &mut __f1);
                                                        t = __f0;
                                                        t2 = __f1;
                                                        __r
                                                    };
                                                }
                                            }
                                            {
                                                let mut __f0 = ::core::mem::take(&mut t2);
                                                let __a1 = ((self.Fix2D(self.cur_val)
                                                    * 3.141592653589793f64)
                                                    / 180.0f64);
                                                let __r = self.make_rotation(&mut __f0, __a1);
                                                t2 = __f0;
                                                __r
                                            };
                                            {
                                                let __for_end_11 = 3i32;
                                                i = 0i32;
                                                while i <= __for_end_11 {
                                                    {
                                                        let mut __f0 = ::core::mem::take(
                                                            &mut corners
                                                                [crate::ix::U((i) as usize)],
                                                        );
                                                        let mut __f1 = ::core::mem::take(&mut t2);
                                                        let __r = self
                                                            .transform_point(&mut __f0, &mut __f1);
                                                        corners[crate::ix::U((i) as usize)] = __f0;
                                                        t2 = __f1;
                                                        __r
                                                    };
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                            {
                                                xmin = 1000000.0f64;
                                                xmax = ((((xmin) as i32).wrapping_neg()) as f64);
                                                ymin = xmin;
                                                ymax = xmax;
                                                {
                                                    let __for_end_12 = 3i32;
                                                    i = 0i32;
                                                    while i <= __for_end_12 {
                                                        {
                                                            if (corners[crate::ix::U((i) as usize)]
                                                                .x
                                                                < xmin)
                                                            {
                                                                xmin = corners
                                                                    [crate::ix::U((i) as usize)]
                                                                .x;
                                                            }
                                                            if (corners[crate::ix::U((i) as usize)]
                                                                .x
                                                                > xmax)
                                                            {
                                                                xmax = corners
                                                                    [crate::ix::U((i) as usize)]
                                                                .x;
                                                            }
                                                            if (corners[crate::ix::U((i) as usize)]
                                                                .y
                                                                < ymin)
                                                            {
                                                                ymin = corners
                                                                    [crate::ix::U((i) as usize)]
                                                                .y;
                                                            }
                                                            if (corners[crate::ix::U((i) as usize)]
                                                                .y
                                                                > ymax)
                                                            {
                                                                ymax = corners
                                                                    [crate::ix::U((i) as usize)]
                                                                .y;
                                                            }
                                                        }
                                                        i = i.wrapping_add(1);
                                                    }
                                                }
                                            }
                                            {
                                                let mut __f0 = ::core::mem::take(
                                                    &mut corners[crate::ix::U((0i32) as usize)],
                                                );
                                                let __r = self.setPoint(&mut __f0, xmin, ymin);
                                                corners[crate::ix::U((0i32) as usize)] = __f0;
                                                __r
                                            };
                                            {
                                                let mut __f0 = ::core::mem::take(
                                                    &mut corners[crate::ix::U((1i32) as usize)],
                                                );
                                                let __r = self.setPoint(&mut __f0, xmin, ymax);
                                                corners[crate::ix::U((1i32) as usize)] = __f0;
                                                __r
                                            };
                                            {
                                                let mut __f0 = ::core::mem::take(
                                                    &mut corners[crate::ix::U((2i32) as usize)],
                                                );
                                                let __r = self.setPoint(&mut __f0, xmax, ymax);
                                                corners[crate::ix::U((2i32) as usize)] = __f0;
                                                __r
                                            };
                                            {
                                                let mut __f0 = ::core::mem::take(
                                                    &mut corners[crate::ix::U((3i32) as usize)],
                                                );
                                                let __r = self.setPoint(&mut __f0, xmax, ymin);
                                                corners[crate::ix::U((3i32) as usize)] = __f0;
                                                __r
                                            };
                                            {
                                                let mut __f0 = ::core::mem::take(&mut t);
                                                let mut __f1 = ::core::mem::take(&mut t2);
                                                let __r =
                                                    self.transform_concat(&mut __f0, &mut __f1);
                                                t = __f0;
                                                t2 = __f1;
                                                __r
                                            };
                                        }
                                    } else {
                                        check_keywords = false;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if ((x_size_req != 0.0f64) || (y_size_req != 0.0f64)) {
            {
                {
                    xmin = 1000000.0f64;
                    xmax = ((((xmin) as i32).wrapping_neg()) as f64);
                    ymin = xmin;
                    ymax = xmax;
                    {
                        let __for_end_5 = 3i32;
                        i = 0i32;
                        while i <= __for_end_5 {
                            {
                                if (corners[crate::ix::U((i) as usize)].x < xmin) {
                                    xmin = corners[crate::ix::U((i) as usize)].x;
                                }
                                if (corners[crate::ix::U((i) as usize)].x > xmax) {
                                    xmax = corners[crate::ix::U((i) as usize)].x;
                                }
                                if (corners[crate::ix::U((i) as usize)].y < ymin) {
                                    ymin = corners[crate::ix::U((i) as usize)].y;
                                }
                                if (corners[crate::ix::U((i) as usize)].y > ymax) {
                                    ymax = corners[crate::ix::U((i) as usize)].y;
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }
                if (x_size_req == 0.0f64) {
                    {
                        {
                            let mut __f0 = ::core::mem::take(&mut t2);
                            let __r = self.make_scale(
                                &mut __f0,
                                (y_size_req / (ymax - ymin)),
                                (y_size_req / (ymax - ymin)),
                            );
                            t2 = __f0;
                            __r
                        };
                    }
                } else {
                    if (y_size_req == 0.0f64) {
                        {
                            {
                                let mut __f0 = ::core::mem::take(&mut t2);
                                let __r = self.make_scale(
                                    &mut __f0,
                                    (x_size_req / (xmax - xmin)),
                                    (x_size_req / (xmax - xmin)),
                                );
                                t2 = __f0;
                                __r
                            };
                        }
                    } else {
                        {
                            {
                                let mut __f0 = ::core::mem::take(&mut t2);
                                let __r = self.make_scale(
                                    &mut __f0,
                                    (x_size_req / (xmax - xmin)),
                                    (y_size_req / (ymax - ymin)),
                                );
                                t2 = __f0;
                                __r
                            };
                        }
                    }
                }
                {
                    let __for_end_4 = 3i32;
                    i = 0i32;
                    while i <= __for_end_4 {
                        {
                            let mut __f0 =
                                ::core::mem::take(&mut corners[crate::ix::U((i) as usize)]);
                            let mut __f1 = ::core::mem::take(&mut t2);
                            let __r = self.transform_point(&mut __f0, &mut __f1);
                            corners[crate::ix::U((i) as usize)] = __f0;
                            t2 = __f1;
                            __r
                        };
                        i = i.wrapping_add(1);
                    }
                }
                x_size_req = 0.0f64;
                y_size_req = 0.0f64;
                {
                    let mut __f0 = ::core::mem::take(&mut t);
                    let mut __f1 = ::core::mem::take(&mut t2);
                    let __r = self.transform_concat(&mut __f0, &mut __f1);
                    t = __f0;
                    t2 = __f1;
                    __r
                };
            }
        }
        {
            xmin = 1000000.0f64;
            xmax = ((((xmin) as i32).wrapping_neg()) as f64);
            ymin = xmin;
            ymax = xmax;
            {
                let __for_end_3 = 3i32;
                i = 0i32;
                while i <= __for_end_3 {
                    {
                        if (corners[crate::ix::U((i) as usize)].x < xmin) {
                            xmin = corners[crate::ix::U((i) as usize)].x;
                        }
                        if (corners[crate::ix::U((i) as usize)].x > xmax) {
                            xmax = corners[crate::ix::U((i) as usize)].x;
                        }
                        if (corners[crate::ix::U((i) as usize)].y < ymin) {
                            ymin = corners[crate::ix::U((i) as usize)].y;
                        }
                        if (corners[crate::ix::U((i) as usize)].y > ymax) {
                            ymax = corners[crate::ix::U((i) as usize)].y;
                        }
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
        {
            let mut __f0 = ::core::mem::take(&mut t2);
            let __r = self.make_translation(
                &mut __f0,
                ((((((xmin) as i32).wrapping_neg()).wrapping_mul(72i32)) as f64) / 72.27f64),
                ((((((ymin) as i32).wrapping_neg()).wrapping_mul(72i32)) as f64) / 72.27f64),
            );
            t2 = __f0;
            __r
        };
        {
            let mut __f0 = ::core::mem::take(&mut t);
            let mut __f1 = ::core::mem::take(&mut t2);
            let __r = self.transform_concat(&mut __f0, &mut __f1);
            t = __f0;
            t2 = __f1;
            __r
        };
        if (result == 0i32) {
            {
                {
                    let __a1717_0 = pic_node;
                    let __a1717_1 = (pic_node_size)
                        .wrapping_add(((self.pic_path_len(pic_path)).wrapping_add(7i32) / 8i32));
                    self.new_whatsit(__a1717_0, __a1717_1)
                };
                if is_pdf {
                    {
                        {
                            let __ix1718 = self.cur_list.tail_field;
                            self.mem[crate::ix::U((__ix1718) as usize)].set_hh_b1(pdf_node);
                        }
                    }
                }
                {
                    let __ix1719 = (self.cur_list.tail_field).wrapping_add(4i32);
                    let __v1720 = self.pic_path_len(pic_path);
                    self.mem[crate::ix::U((__ix1719) as usize)].set_hh_b0(__v1720);
                }
                {
                    let __ix1721 = (self.cur_list.tail_field).wrapping_add(4i32);
                    self.mem[crate::ix::U((__ix1721) as usize)].set_hh_b1(page);
                }
                {
                    let __ix1722 = (self.cur_list.tail_field).wrapping_add(8i32);
                    self.mem[crate::ix::U((__ix1722) as usize)].set_hh_b0(pdf_box_type);
                }
                {
                    let __ix1723 = (self.cur_list.tail_field).wrapping_add(1i32);
                    let __v1724 = self.D2Fix((xmax - xmin));
                    self.mem[crate::ix::U((__ix1723) as usize)].set_int(__v1724);
                }
                {
                    let __ix1725 = (self.cur_list.tail_field).wrapping_add(3i32);
                    let __v1726 = self.D2Fix((ymax - ymin));
                    self.mem[crate::ix::U((__ix1725) as usize)].set_int(__v1726);
                }
                {
                    let __ix1727 = (self.cur_list.tail_field).wrapping_add(2i32);
                    self.mem[crate::ix::U((__ix1727) as usize)].set_int(0i32);
                }
                {
                    let __ix1728 = (self.cur_list.tail_field).wrapping_add(5i32);
                    let __v1729 = self.D2Fix(t.a);
                    self.mem[crate::ix::U((__ix1728) as usize)].set_hh_lh(__v1729);
                }
                {
                    let __ix1730 = (self.cur_list.tail_field).wrapping_add(5i32);
                    let __v1731 = self.D2Fix(t.b);
                    self.mem[crate::ix::U((__ix1730) as usize)].set_hh_rh(__v1731);
                }
                {
                    let __ix1732 = (self.cur_list.tail_field).wrapping_add(6i32);
                    let __v1733 = self.D2Fix(t.c);
                    self.mem[crate::ix::U((__ix1732) as usize)].set_hh_lh(__v1733);
                }
                {
                    let __ix1734 = (self.cur_list.tail_field).wrapping_add(6i32);
                    let __v1735 = self.D2Fix(t.d);
                    self.mem[crate::ix::U((__ix1734) as usize)].set_hh_rh(__v1735);
                }
                {
                    let __ix1736 = (self.cur_list.tail_field).wrapping_add(7i32);
                    let __v1737 = self.D2Fix(t.x);
                    self.mem[crate::ix::U((__ix1736) as usize)].set_hh_lh(__v1737);
                }
                {
                    let __ix1738 = (self.cur_list.tail_field).wrapping_add(7i32);
                    let __v1739 = self.D2Fix(t.y);
                    self.mem[crate::ix::U((__ix1738) as usize)].set_hh_rh(__v1739);
                }
                self.pic_path_to_mem(pic_path, (self.cur_list.tail_field).wrapping_add(9i32));
            }
        } else {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66797i32);
                }
                self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
                self.print(39i32);
                if (result == (43i32).wrapping_neg()) {
                    {
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66798i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66799i32;
                        }
                    }
                } else {
                    {
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66798i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66800i32;
                        }
                    }
                }
                self.error();
            }
        }
    }

    /// @<Declare procedures needed in `do_extension`
    // §1456
    pub fn scan_and_pack_name(&mut self) {
        self.scan_file_name();
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// @<Declare act...
    // §1403
    pub fn do_extension(&mut self) {
        let mut i: i32 = 0; // §1403
        let mut j: i32 = 0; // §1403
        let mut k: i32 = 0; // §1403
        let mut p: halfword = 0; // §1403
        let mut q: halfword = 0; // §1403
        let mut r: halfword = 0; // §1403
        match self.cur_chr {
            open_node => {
                // §1406
                {
                    self.new_write_whatsit(open_node_size);
                    self.scan_optional_equals();
                    self.scan_file_name();
                    {
                        let __ix1740 = (self.cur_list.tail_field).wrapping_add(1i32);
                        let __v1741 = self.cur_name;
                        self.mem[crate::ix::U((__ix1740) as usize)].set_hh_rh(__v1741);
                    }
                    {
                        let __ix1742 = (self.cur_list.tail_field).wrapping_add(2i32);
                        let __v1743 = self.cur_area;
                        self.mem[crate::ix::U((__ix1742) as usize)].set_hh_lh(__v1743);
                    }
                    {
                        let __ix1744 = (self.cur_list.tail_field).wrapping_add(2i32);
                        let __v1745 = self.cur_ext;
                        self.mem[crate::ix::U((__ix1744) as usize)].set_hh_rh(__v1745);
                    }
                }
            }
            write_node => {
                // §1407
                {
                    k = self.cur_cs;
                    self.new_write_whatsit(write_node_size);
                    self.cur_cs = k;
                    p = self.scan_toks(false, false);
                    {
                        let __ix1746 = (self.cur_list.tail_field).wrapping_add(1i32);
                        let __v1747 = self.def_ref;
                        self.mem[crate::ix::U((__ix1746) as usize)].set_hh_rh(__v1747);
                    }
                }
            }
            close_node => {
                // §1408
                {
                    self.new_write_whatsit(write_node_size);
                    {
                        let __ix1748 = (self.cur_list.tail_field).wrapping_add(1i32);
                        self.mem[crate::ix::U((__ix1748) as usize)]
                            .set_hh_rh((268435455i32).wrapping_neg());
                    }
                }
            }
            special_node => {
                // §1409
                {
                    if self.scan_keyword(66489i32) {
                        {
                            self.new_whatsit(latespecial_node, write_node_size);
                            {
                                let __ix1749 = (self.cur_list.tail_field).wrapping_add(1i32);
                                self.mem[crate::ix::U((__ix1749) as usize)]
                                    .set_hh_lh((268435455i32).wrapping_neg());
                            }
                            p = self.scan_toks(false, false);
                            {
                                let __ix1750 = (self.cur_list.tail_field).wrapping_add(1i32);
                                let __v1751 = self.def_ref;
                                self.mem[crate::ix::U((__ix1750) as usize)].set_hh_rh(__v1751);
                            }
                        }
                    } else {
                        {
                            self.new_whatsit(special_node, write_node_size);
                            {
                                let __ix1752 = (self.cur_list.tail_field).wrapping_add(1i32);
                                self.mem[crate::ix::U((__ix1752) as usize)]
                                    .set_hh_lh((268435455i32).wrapping_neg());
                            }
                            p = self.scan_toks(false, true);
                            {
                                let __ix1753 = (self.cur_list.tail_field).wrapping_add(1i32);
                                let __v1754 = self.def_ref;
                                self.mem[crate::ix::U((__ix1753) as usize)].set_hh_rh(__v1754);
                            }
                        }
                    }
                }
            }
            immediate_code => {
                // §1438
                {
                    self.get_x_token();
                    if ((self.cur_cmd == extension) && (self.cur_chr <= close_node)) {
                        {
                            p = self.cur_list.tail_field;
                            self.do_extension();
                            self.out_what(self.cur_list.tail_field);
                            self.flush_node_list(self.cur_list.tail_field);
                            self.cur_list.tail_field = p;
                            self.mem[crate::ix::U((p) as usize)]
                                .set_hh_rh((268435455i32).wrapping_neg());
                        }
                    } else {
                        self.back_input();
                    }
                }
            }
            set_language_code => {
                // §1440
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
                                {
                                    let __v1755 = self.cur_val;
                                    self.cur_list.aux_field.set_hh_rh(__v1755);
                                }
                            }
                        }
                        {
                            let __ix1756 = (self.cur_list.tail_field).wrapping_add(1i32);
                            let __v1757 = self.cur_list.aux_field.hh().rh();
                            self.mem[crate::ix::U((__ix1756) as usize)].set_hh_rh(__v1757);
                        }
                        {
                            let __ix1758 = (self.cur_list.tail_field).wrapping_add(1i32);
                            let __v1759 = self.norm_min(
                                self.eqtb[crate::ix::U(((7892315i32) - 1) as usize)].int(),
                            );
                            self.mem[crate::ix::U((__ix1758) as usize)].set_hh_b0(__v1759);
                        }
                        {
                            let __ix1760 = (self.cur_list.tail_field).wrapping_add(1i32);
                            let __v1761 = self.norm_min(
                                self.eqtb[crate::ix::U(((7892316i32) - 1) as usize)].int(),
                            );
                            self.mem[crate::ix::U((__ix1760) as usize)].set_hh_b1(__v1761);
                        }
                    }
                }
            }
            pdf_save_pos_node => {
                // §1450
                {
                    self.new_whatsit(pdf_save_pos_node, small_node_size);
                }
            }
            reset_timer_code => {
                // §1414
                {
                    {
                        let mut __f0 = ::core::mem::take(&mut self.epochseconds);
                        let mut __f1 = ::core::mem::take(&mut self.microseconds);
                        let __r = self.seconds_and_micros(&mut __f0, &mut __f1);
                        self.epochseconds = __f0;
                        self.microseconds = __f1;
                        __r
                    };
                }
            }
            set_random_seed_code => {
                // §1413
                {
                    self.scan_int();
                    if (self.cur_val < 0i32) {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                    self.random_seed = self.cur_val;
                    self.init_randoms(self.random_seed);
                }
            }
            pic_file_code => {
                // §1442
                if ((self.cur_list.mode_field).wrapping_abs() == mmode) {
                    self.report_illegal_case();
                } else {
                    self.load_picture(false);
                }
            }
            pdf_file_code => {
                // §1443
                if ((self.cur_list.mode_field).wrapping_abs() == mmode) {
                    self.report_illegal_case();
                } else {
                    self.load_picture(true);
                }
            }
            glyph_code => {
                // §1444
                {
                    if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                        {
                            self.back_input();
                            self.new_graf(true);
                        }
                    } else {
                        if ((self.cur_list.mode_field).wrapping_abs() == mmode) {
                            self.report_illegal_case();
                        } else {
                            {
                                if ((self.font_area[crate::ix::U(
                                    (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                                        .hh()
                                        .rh()) as usize,
                                )] == aat_font_flag)
                                    || (self.font_area[crate::ix::U(
                                        (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                                            .hh()
                                            .rh()) as usize,
                                    )] == otgr_font_flag))
                                {
                                    {
                                        self.new_whatsit(glyph_node, glyph_node_size);
                                        self.scan_int();
                                        if ((self.cur_val < 0i32) || (self.cur_val > 65535i32)) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {}
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66782i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] =
                                                        66783i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] =
                                                        65996i32;
                                                }
                                                self.int_error(self.cur_val);
                                                self.cur_val = 0i32;
                                            }
                                        }
                                        {
                                            let __ix1762 =
                                                (self.cur_list.tail_field).wrapping_add(4i32);
                                            let __v1763 = self.eqtb
                                                [crate::ix::U(((cur_font_loc) - 1) as usize)]
                                            .hh()
                                            .rh();
                                            self.mem[crate::ix::U((__ix1762) as usize)]
                                                .set_qqqq_b1(__v1763);
                                        }
                                        {
                                            let __ix1764 =
                                                (self.cur_list.tail_field).wrapping_add(4i32);
                                            let __v1765 = self.cur_val;
                                            self.mem[crate::ix::U((__ix1764) as usize)]
                                                .set_qqqq_b2(__v1765);
                                        }
                                        self.set_native_glyph_metrics(
                                            self.cur_list.tail_field,
                                            (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)]
                                                .int()
                                                > 0i32),
                                        );
                                    }
                                } else {
                                    self.not_native_font_error(
                                        extension,
                                        glyph_code,
                                        self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                                            .hh()
                                            .rh(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            XeTeX_input_encoding_extension_code => {
                // §1446
                {
                    self.scan_and_pack_name();
                    i = {
                        let mut __f0 = ::core::mem::take(&mut j);
                        let __r = self.get_encoding_mode_and_info(&mut __f0);
                        j = __f0;
                        __r
                    };
                    if (i == XeTeX_input_mode_auto) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66801i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66802i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66803i32;
                            }
                            self.error();
                        }
                    } else {
                        {
                            let mut __f0 = ::core::mem::take(
                                &mut self.input_file[crate::ix::U((self.in_open) as usize)],
                            );
                            let __r = self.set_input_file_encoding(&mut __f0, i, j);
                            self.input_file[crate::ix::U((self.in_open) as usize)] = __f0;
                            __r
                        };
                    }
                }
            }
            XeTeX_default_encoding_extension_code => {
                // §1447
                {
                    self.scan_and_pack_name();
                    i = {
                        let mut __f0 = ::core::mem::take(&mut j);
                        let __r = self.get_encoding_mode_and_info(&mut __f0);
                        j = __f0;
                        __r
                    };
                    self.eqtb[crate::ix::U(((7892345i32) - 1) as usize)].set_int(i);
                    self.eqtb[crate::ix::U(((7892346i32) - 1) as usize)].set_int(j);
                }
            }
            XeTeX_linebreak_locale_extension_code => {
                // §1448
                {
                    self.scan_file_name();
                    if (self.length(self.cur_name) == 0i32) {
                        self.eqtb[crate::ix::U(((7892336i32) - 1) as usize)].set_int(0i32);
                    } else {
                        {
                            let __v1766 = self.cur_name;
                            self.eqtb[crate::ix::U(((7892336i32) - 1) as usize)].set_int(__v1766);
                        }
                    }
                }
            }
            _ => {
                // §1403
                self.confusion(66752i32);
            }
        }
    }

    /// The \.{\\language} extension is somewhat different.
    /// We need a subroutine that comes into play when a character of
    /// a non-`clang` language is being appended to the current paragraph.
    /// @<Declare action...
    // §1439
    pub fn fix_language(&mut self) {
        let mut l: UTF16_code = 0; // §1439
        if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() <= 0i32) {
            l = 0i32;
        } else {
            if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int() > 255i32) {
                l = 0i32;
            } else {
                l = self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int();
            }
        }
        if (l != self.cur_list.aux_field.hh().rh()) {
            {
                self.new_whatsit(language_node, small_node_size);
                {
                    let __ix1767 = (self.cur_list.tail_field).wrapping_add(1i32);
                    self.mem[crate::ix::U((__ix1767) as usize)].set_hh_rh(l);
                }
                self.cur_list.aux_field.set_hh_rh(l);
                {
                    let __ix1768 = (self.cur_list.tail_field).wrapping_add(1i32);
                    let __v1769 =
                        self.norm_min(self.eqtb[crate::ix::U(((7892315i32) - 1) as usize)].int());
                    self.mem[crate::ix::U((__ix1768) as usize)].set_hh_b0(__v1769);
                }
                {
                    let __ix1770 = (self.cur_list.tail_field).wrapping_add(1i32);
                    let __v1771 =
                        self.norm_min(self.eqtb[crate::ix::U(((7892316i32) - 1) as usize)].int());
                    self.mem[crate::ix::U((__ix1770) as usize)].set_hh_b1(__v1771);
                }
            }
        }
    }

    /// @<Declare the procedure called `handle_right_brace`
    // §1122
    pub fn handle_right_brace(&mut self) {
        let mut p: halfword = 0; // §1122
        let mut q: halfword = 0; // §1122
        let mut d: scaled = 0; // §1122
        let mut f: i32 = 0; // §1122
        match self.cur_group {
            simple_group => {
                self.unsave();
            }
            bottom_level => {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66471i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66472i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66473i32;
                }
                self.error();
            }
            semi_simple_group | math_shift_group | math_left_group => {
                self.extra_right_brace();
            }
            hbox_group => {
                // §1139
                self.package(0i32);
            }
            adjusted_hbox_group => {
                self.adjust_tail = adjust_head;
                self.pre_adjust_tail = pre_adjust_head;
                self.package(0i32);
            }
            vbox_group => {
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 0i32)
                    && (self.cur_list.mode_field == hmode))
                {
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
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 0i32)
                    && (self.cur_list.mode_field == hmode))
                {
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
                // §1154
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 1i32)
                    && (self.cur_list.mode_field == hmode))
                {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    {
                        self.end_graf();
                        q = self.eqtb[crate::ix::U(((1205774i32) - 1) as usize)]
                            .hh()
                            .rh();
                        {
                            let __v1772 =
                                (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32);
                            self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v1772);
                        }
                        d = self.eqtb[crate::ix::U(((9006726i32) - 1) as usize)].int();
                        f = self.eqtb[crate::ix::U(((7892306i32) - 1) as usize)].int();
                        self.unsave();
                        self.save_ptr = (self.save_ptr).wrapping_sub(2i32);
                        p = self.vpackage(
                            self.mem[crate::ix::U((self.cur_list.head_field) as usize)]
                                .hh()
                                .rh(),
                            0i32,
                            additional,
                            max_dimen,
                        );
                        self.pop_nest();
                        if (self.save_stack
                            [crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)]
                        .int()
                            < 255i32)
                        {
                            {
                                {
                                    {
                                        let __ix1773 = self.cur_list.tail_field;
                                        let __v1774 = self.get_node(ins_node_size);
                                        self.mem[crate::ix::U((__ix1773) as usize)]
                                            .set_hh_rh(__v1774);
                                    }
                                    self.cur_list.tail_field = self.mem
                                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                                    .hh()
                                    .rh();
                                }
                                {
                                    let __ix1775 = self.cur_list.tail_field;
                                    self.mem[crate::ix::U((__ix1775) as usize)].set_hh_b0(ins_node);
                                }
                                {
                                    let __ix1776 = self.cur_list.tail_field;
                                    let __v1777 = self.save_stack[crate::ix::U(
                                        ((self.save_ptr).wrapping_add(0i32)) as usize,
                                    )]
                                    .int();
                                    self.mem[crate::ix::U((__ix1776) as usize)].set_hh_b1(__v1777);
                                }
                                {
                                    let __ix1778 = (self.cur_list.tail_field).wrapping_add(3i32);
                                    let __v1779 = (self.mem
                                        [crate::ix::U(((p).wrapping_add(3i32)) as usize)]
                                    .int())
                                    .wrapping_add(
                                        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)]
                                            .int(),
                                    );
                                    self.mem[crate::ix::U((__ix1778) as usize)].set_int(__v1779);
                                }
                                {
                                    let __ix1780 = (self.cur_list.tail_field).wrapping_add(4i32);
                                    let __v1781 = self.mem
                                        [crate::ix::U(((p).wrapping_add(5i32)) as usize)]
                                    .hh()
                                    .rh();
                                    self.mem[crate::ix::U((__ix1780) as usize)].set_hh_lh(__v1781);
                                }
                                {
                                    let __ix1782 = (self.cur_list.tail_field).wrapping_add(4i32);
                                    self.mem[crate::ix::U((__ix1782) as usize)].set_hh_rh(q);
                                }
                                {
                                    let __ix1783 = (self.cur_list.tail_field).wrapping_add(2i32);
                                    self.mem[crate::ix::U((__ix1783) as usize)].set_int(d);
                                }
                                {
                                    let __ix1784 = (self.cur_list.tail_field).wrapping_add(1i32);
                                    self.mem[crate::ix::U((__ix1784) as usize)].set_int(f);
                                }
                            }
                        } else {
                            {
                                {
                                    {
                                        let __ix1785 = self.cur_list.tail_field;
                                        let __v1786 = self.get_node(small_node_size);
                                        self.mem[crate::ix::U((__ix1785) as usize)]
                                            .set_hh_rh(__v1786);
                                    }
                                    self.cur_list.tail_field = self.mem
                                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                                    .hh()
                                    .rh();
                                }
                                {
                                    let __ix1787 = self.cur_list.tail_field;
                                    self.mem[crate::ix::U((__ix1787) as usize)]
                                        .set_hh_b0(adjust_node);
                                }
                                {
                                    let __ix1788 = self.cur_list.tail_field;
                                    let __v1789 = self.save_stack[crate::ix::U(
                                        ((self.save_ptr).wrapping_add(1i32)) as usize,
                                    )]
                                    .int();
                                    self.mem[crate::ix::U((__ix1788) as usize)].set_hh_b1(__v1789);
                                }
                                {
                                    let __ix1790 = (self.cur_list.tail_field).wrapping_add(1i32);
                                    let __v1791 = self.mem
                                        [crate::ix::U(((p).wrapping_add(5i32)) as usize)]
                                    .hh()
                                    .rh();
                                    self.mem[crate::ix::U((__ix1790) as usize)].set_int(__v1791);
                                }
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
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 1i32)
                    && (self.cur_list.mode_field == hmode))
                {
                    {
                        self.back_input();
                        self.cur_tok = self.par_token;
                        self.back_input();
                        self.cur_input.index_field = inserted;
                    }
                } else {
                    // §1080
                    {
                        while (((self.cur_input.state_field == token_list)
                            && (self.cur_input.loc_field == (268435455i32).wrapping_neg()))
                            && (self.cur_input.index_field == backed_up))
                        {
                            self.end_token_list();
                        }
                        if (((self.cur_input.state_field != token_list)
                            || (self.cur_input.loc_field != (268435455i32).wrapping_neg()))
                            || (self.cur_input.index_field != output_text))
                        {
                            // §1081
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65916i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66439i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66440i32;
                                }
                                self.error();
                                loop {
                                    self.get_token();
                                    if (self.cur_input.loc_field == (268435455i32).wrapping_neg()) {
                                        break;
                                    }
                                }
                            }
                        }
                        // §1080
                        self.output_can_end = true;
                        self.end_token_list();
                        self.output_can_end = false;
                        self.end_graf();
                        self.unsave();
                        self.output_active = false;
                        self.insert_penalties = 0i32;
                        // §1082
                        if (self.eqtb[crate::ix::U(((1206822i32) - 1) as usize)]
                            .hh()
                            .rh()
                            != (268435455i32).wrapping_neg())
                        {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66441i32);
                                }
                                self.print_esc(65701i32);
                                self.print_int(255i32);
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 66442i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66443i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66444i32;
                                }
                                self.box_error(255i32);
                            }
                        }
                        // §1080
                        if (self.cur_list.tail_field != self.cur_list.head_field) {
                            {
                                {
                                    let __ix1792 = self.page_tail;
                                    let __v1793 = self.mem
                                        [crate::ix::U((self.cur_list.head_field) as usize)]
                                    .hh()
                                    .rh();
                                    self.mem[crate::ix::U((__ix1792) as usize)].set_hh_rh(__v1793);
                                }
                                self.page_tail = self.cur_list.tail_field;
                            }
                        }
                        if (self.mem[crate::ix::U((page_head) as usize)].hh().rh()
                            != (268435455i32).wrapping_neg())
                        {
                            {
                                if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh()
                                    == (268435455i32).wrapping_neg())
                                {
                                    self.nest[crate::ix::U((0i32) as usize)].tail_field =
                                        self.page_tail;
                                }
                                {
                                    let __ix1794 = self.page_tail;
                                    let __v1795 =
                                        self.mem[crate::ix::U((contrib_head) as usize)].hh().rh();
                                    self.mem[crate::ix::U((__ix1794) as usize)].set_hh_rh(__v1795);
                                }
                                {
                                    let __v1796 =
                                        self.mem[crate::ix::U((page_head) as usize)].hh().rh();
                                    self.mem[crate::ix::U((contrib_head) as usize)]
                                        .set_hh_rh(__v1796);
                                }
                                self.mem[crate::ix::U((page_head) as usize)]
                                    .set_hh_rh((268435455i32).wrapping_neg());
                                self.page_tail = page_head;
                            }
                        }
                        self.flush_node_list(
                            self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)],
                        );
                        self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] =
                            (268435455i32).wrapping_neg();
                        self.pop_nest();
                        self.build_page();
                    }
                }
            }
            disc_group => {
                // §1172
                self.build_discretionary();
            }
            align_group => {
                // §1186
                {
                    self.back_input();
                    self.cur_tok = 34749082i32;
                    {
                        if (self.interaction == error_stop_mode) {}
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(65949i32);
                    }
                    self.print_esc(66320i32);
                    self.print(65950i32);
                    {
                        self.help_ptr = 1i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66554i32;
                    }
                    self.ins_error();
                }
            }
            no_align_group => {
                // §1187
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 1i32)
                    && (self.cur_list.mode_field == hmode))
                {
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
                // §1222
                if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)].int() > 0i32)
                    && (self.cur_list.mode_field == hmode))
                {
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
                        p = self.vpackage(
                            self.mem[crate::ix::U((self.cur_list.head_field) as usize)]
                                .hh()
                                .rh(),
                            self.save_stack
                                [crate::ix::U(((self.save_ptr).wrapping_add(1i32)) as usize)]
                            .int(),
                            self.save_stack
                                [crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)]
                            .int(),
                            max_dimen,
                        );
                        self.pop_nest();
                        {
                            {
                                let __ix1797 = self.cur_list.tail_field;
                                let __v1798 = self.new_noad();
                                self.mem[crate::ix::U((__ix1797) as usize)].set_hh_rh(__v1798);
                            }
                            self.cur_list.tail_field = self.mem
                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                            .hh()
                            .rh();
                        }
                        {
                            let __ix1799 = self.cur_list.tail_field;
                            self.mem[crate::ix::U((__ix1799) as usize)].set_hh_b0(vcenter_noad);
                        }
                        {
                            let __ix1800 = (self.cur_list.tail_field).wrapping_add(1i32);
                            self.mem[crate::ix::U((__ix1800) as usize)].set_hh_rh(sub_box);
                        }
                        {
                            let __ix1801 = (self.cur_list.tail_field).wrapping_add(1i32);
                            self.mem[crate::ix::U((__ix1801) as usize)].set_hh_lh(p);
                        }
                    }
                }
            }
            math_choice_group => {
                // §1227
                self.build_choices();
            }
            math_group => {
                // §1240
                {
                    self.unsave();
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                    {
                        let __ix1802 = self.save_stack
                            [crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)]
                        .int();
                        self.mem[crate::ix::U((__ix1802) as usize)].set_hh_rh(sub_mlist);
                    }
                    p = self.fin_mlist((268435455i32).wrapping_neg());
                    {
                        let __ix1803 = self.save_stack
                            [crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)]
                        .int();
                        self.mem[crate::ix::U((__ix1803) as usize)].set_hh_lh(p);
                    }
                    if (p != (268435455i32).wrapping_neg()) {
                        if (self.mem[crate::ix::U((p) as usize)].hh().rh()
                            == (268435455i32).wrapping_neg())
                        {
                            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == ord_noad) {
                                {
                                    if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)]
                                        .hh()
                                        .rh()
                                        == empty)
                                    {
                                        if (self.mem
                                            [crate::ix::U(((p).wrapping_add(2i32)) as usize)]
                                        .hh()
                                        .rh()
                                            == empty)
                                        {
                                            {
                                                {
                                                    let __ix1804 = self.save_stack[crate::ix::U(
                                                        ((self.save_ptr).wrapping_add(0i32))
                                                            as usize,
                                                    )]
                                                    .int();
                                                    let __v1805 = self.mem[crate::ix::U(
                                                        ((p).wrapping_add(1i32)) as usize,
                                                    )]
                                                    .hh();
                                                    self.mem[crate::ix::U((__ix1804) as usize)]
                                                        .set_hh(__v1805);
                                                }
                                                self.free_node(p, noad_size);
                                            }
                                        }
                                    }
                                }
                            } else {
                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == accent_noad) {
                                    if (self.save_stack[crate::ix::U(
                                        ((self.save_ptr).wrapping_add(0i32)) as usize,
                                    )]
                                    .int()
                                        == (self.cur_list.tail_field).wrapping_add(1i32))
                                    {
                                        if (self.mem
                                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                                        .hh()
                                        .b0()
                                            == ord_noad)
                                        {
                                            // §1241
                                            {
                                                q = self.cur_list.head_field;
                                                while (self.mem[crate::ix::U((q) as usize)]
                                                    .hh()
                                                    .rh()
                                                    != self.cur_list.tail_field)
                                                {
                                                    q = self.mem[crate::ix::U((q) as usize)]
                                                        .hh()
                                                        .rh();
                                                }
                                                self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
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
                // §1122
                self.confusion(66474i32);
            }
        }
    }

    /// We shall concentrate first on the inner loop of `main_control`, deferring
    /// consideration of the other cases until later.
    // §1084
    pub fn main_control(&mut self) {
        let mut t: i32 = 0; // §1084
                            // goto labels: L60, reswitch, L70, L80, L90, L91, L92, L100, L101, L110, L111, L112, L95, L120, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                if (self.eqtb[crate::ix::U(((every_job_loc) - 1) as usize)]
                    .hh()
                    .rh()
                    != (268435455i32).wrapping_neg())
                {
                    self.begin_token_list(
                        self.eqtb[crate::ix::U(((every_job_loc) - 1) as usize)]
                            .hh()
                            .rh(),
                        every_job_text,
                    );
                }
            }
            if __goto_1 <= 1 {
                // L60
                self.get_x_token();
            }
            if __goto_1 <= 2 {
                // reswitch
                if (self.interrupt != 0i32) {
                    // §1085
                    if self.OK_to_interrupt {
                        {
                            self.back_input();
                            {
                                if (self.interrupt != 0i32) {
                                    self.pause_for_instructions();
                                }
                            }
                            {
                                __goto_1 = 1;
                                continue 'l_dispatch_1;
                            }
                        }
                    }
                }
                if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() > 0i32) {
                    self.show_cur_cmd_chr();
                }
                // §1084
                match ((self.cur_list.mode_field).wrapping_abs()).wrapping_add(self.cur_cmd) {
                    116 | 117 | 173 => {
                        __goto_1 = 3;
                        continue 'l_dispatch_1;
                    }
                    121 => {
                        self.scan_usv_num();
                        self.cur_chr = self.cur_val;
                        {
                            __goto_1 = 3;
                            continue 'l_dispatch_1;
                        }
                    }
                    170 => {
                        self.get_x_token();
                        if ((((self.cur_cmd == letter) || (self.cur_cmd == other_char))
                            || (self.cur_cmd == char_given))
                            || (self.cur_cmd == char_num))
                        {
                            self.cancel_boundary = true;
                        }
                        {
                            __goto_1 = 2;
                            continue 'l_dispatch_1;
                        }
                    }
                    _ => {
                        {
                            if ((self.cur_list.mode_field).wrapping_abs() == hmode) {
                                if (((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int()
                                    > 0i32)
                                    && (self.space_class != char_class_ignored))
                                    && (self.prev_class != 4095i32))
                                {
                                    {
                                        self.prev_class = 4095i32;
                                        self.find_sa_element(
                                            inter_char_val,
                                            ((self.space_class).wrapping_mul(char_class_limit))
                                                .wrapping_add(4095i32),
                                            false,
                                        );
                                        if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                            && (self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh()
                                                != (268435455i32).wrapping_neg()))
                                        {
                                            {
                                                if (self.cur_cs == 0i32) {
                                                    {
                                                        if (self.cur_cmd == char_num) {
                                                            self.cur_cmd = other_char;
                                                        }
                                                        self.cur_tok = ((self.cur_cmd)
                                                            .wrapping_mul(max_char_val))
                                                        .wrapping_add(self.cur_chr);
                                                    }
                                                } else {
                                                    self.cur_tok =
                                                        (cs_token_flag).wrapping_add(self.cur_cs);
                                                }
                                                self.back_input();
                                                self.begin_token_list(
                                                    self.mem[crate::ix::U(
                                                        ((self.cur_ptr).wrapping_add(1i32))
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh(),
                                                    inter_char_text,
                                                );
                                                {
                                                    __goto_1 = 1;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            match ((self.cur_list.mode_field).wrapping_abs())
                                .wrapping_add(self.cur_cmd)
                            {
                                115 => {
                                    if (self.cur_list.aux_field.hh().lh() == 1000i32) {
                                        {
                                            __goto_1 = 14;
                                            continue 'l_dispatch_1;
                                        }
                                    } else {
                                        self.app_space();
                                    }
                                }
                                169 | 273 => {
                                    __goto_1 = 14;
                                    continue 'l_dispatch_1;
                                }
                                1 | 105 | 209 | 11 | 219 | 274 => {
                                    // §1099
                                }
                                40 | 144 | 248 => {
                                    {
                                        if (self.cur_chr == 0i32) {
                                            {
                                                // §440
                                                loop {
                                                    self.get_x_token();
                                                    if (self.cur_cmd != spacer) {
                                                        break;
                                                    }
                                                }
                                                // §1099
                                                {
                                                    __goto_1 = 2;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        } else {
                                            {
                                                t = self.scanner_status;
                                                self.scanner_status = normal;
                                                self.get_next();
                                                self.scanner_status = t;
                                                if (self.cur_cs < hash_base) {
                                                    self.cur_cs = self.prim_lookup(
                                                        (self.cur_cs).wrapping_sub(1114113i32),
                                                    );
                                                } else {
                                                    self.cur_cs = self.prim_lookup(
                                                        self.hash[crate::ix::U(
                                                            ((self.cur_cs) - 1179650) as usize,
                                                        )]
                                                        .rh(),
                                                    );
                                                }
                                                if (self.cur_cs != undefined_primitive) {
                                                    {
                                                        self.cur_cmd = self.eqtb[crate::ix::U(
                                                            (((prim_eqtb_base)
                                                                .wrapping_add(self.cur_cs))
                                                                - 1)
                                                                as usize,
                                                        )]
                                                        .hh()
                                                        .b0();
                                                        self.cur_chr = self.eqtb[crate::ix::U(
                                                            (((prim_eqtb_base)
                                                                .wrapping_add(self.cur_cs))
                                                                - 1)
                                                                as usize,
                                                        )]
                                                        .hh()
                                                        .rh();
                                                        self.cur_tok =
                                                            (34749093i32).wrapping_add(self.cur_cs);
                                                        {
                                                            __goto_1 = 2;
                                                            continue 'l_dispatch_1;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                15 => {
                                    if self.its_all_over() {
                                        {
                                            __goto_1 = 15;
                                            continue 'l_dispatch_1;
                                        }
                                    }
                                }
                                23 | 126 | 230 | 72 | 176 | 280 | 39 | 45 | 49 | 153 | 7 | 111
                                | 215 => {
                                    self.report_illegal_case();
                                }
                                8 | 112 | 9 | 113 | 18 | 122 | 70 | 174 | 71 | 175 | 51 | 155
                                | 16 | 120 | 50 | 154 | 53 | 157 | 67 | 171 | 54 | 158 | 55
                                | 159 | 57 | 161 | 56 | 160 | 31 | 135 | 52 | 156 | 29 | 133
                                | 47 | 151 | 218 | 222 | 223 | 236 | 233 | 242 | 245 => {
                                    self.insert_dollar_sign();
                                }
                                37 | 140 | 244 => {
                                    // §1110
                                    {
                                        {
                                            {
                                                let __ix1806 = self.cur_list.tail_field;
                                                let __v1807 = self.scan_rule_spec();
                                                self.mem[crate::ix::U((__ix1806) as usize)]
                                                    .set_hh_rh(__v1807);
                                            }
                                            self.cur_list.tail_field = self.mem
                                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                                            .hh()
                                            .rh();
                                        }
                                        if ((self.cur_list.mode_field).wrapping_abs() == vmode) {
                                            self.cur_list
                                                .aux_field
                                                .set_int((65536000i32).wrapping_neg());
                                        } else {
                                            if ((self.cur_list.mode_field).wrapping_abs() == hmode)
                                            {
                                                self.cur_list.aux_field.set_hh_lh(1000i32);
                                            }
                                        }
                                    }
                                }
                                28 | 131 | 235 | 237 => {
                                    // §1111
                                    self.append_glue();
                                }
                                30 | 134 | 238 | 239 => {
                                    self.append_kern();
                                }
                                2 | 106 => {
                                    // §1117
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
                                    // §1121
                                    self.handle_right_brace();
                                }
                                22 | 127 | 231 => {
                                    // §1127
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
                                    // §1144
                                    self.new_graf((self.cur_chr > 0i32));
                                }
                                12 | 13 | 17 | 69 | 4 | 24 | 36 | 46 | 48 | 27 | 34 | 65 | 66 => {
                                    self.back_input();
                                    self.new_graf(true);
                                }
                                148 | 252 => {
                                    // §1146
                                    self.indent_in_hmode();
                                }
                                14 => {
                                    // §1148
                                    {
                                        self.normal_paragraph();
                                        if (self.cur_list.mode_field > 0i32) {
                                            self.build_page();
                                        }
                                    }
                                }
                                118 => {
                                    if (self.align_state < 0i32) {
                                        self.off_save();
                                    }
                                    self.end_graf();
                                    if (self.cur_list.mode_field == vmode) {
                                        self.build_page();
                                    }
                                }
                                119 | 132 | 141 | 129 | 137 => {
                                    self.head_for_vmode();
                                }
                                38 | 142 | 246 | 143 | 247 => {
                                    // §1151
                                    self.begin_insert_or_adjust();
                                }
                                19 | 123 | 227 => {
                                    self.make_mark();
                                }
                                43 | 147 | 251 => {
                                    // §1156
                                    self.append_penalty();
                                }
                                26 | 130 | 234 => {
                                    // §1158
                                    self.delete_last();
                                }
                                25 | 128 | 232 => {
                                    // §1163
                                    self.unpackage();
                                }
                                149 => {
                                    // §1166
                                    self.append_italic_correction();
                                }
                                253 => {
                                    {
                                        let __ix1808 = self.cur_list.tail_field;
                                        let __v1809 = self.new_kern(0i32);
                                        self.mem[crate::ix::U((__ix1808) as usize)]
                                            .set_hh_rh(__v1809);
                                    }
                                    self.cur_list.tail_field = self.mem
                                        [crate::ix::U((self.cur_list.tail_field) as usize)]
                                    .hh()
                                    .rh();
                                }
                                152 | 256 => {
                                    // §1170
                                    self.append_discretionary();
                                }
                                150 => {
                                    // §1176
                                    self.make_accent();
                                }
                                6 | 110 | 214 | 5 | 109 | 213 => {
                                    // §1180
                                    self.align_error();
                                }
                                35 | 139 | 243 => {
                                    self.no_align_error();
                                }
                                64 | 168 | 272 => {
                                    self.omit_error();
                                }
                                33 => {
                                    // §1184
                                    self.init_align();
                                }
                                138 => {
                                    // §1513
                                    if (self.cur_chr > 0i32) {
                                        {
                                            if self.eTeX_enabled(
                                                (self.eqtb
                                                    [crate::ix::U(((7892339i32) - 1) as usize)]
                                                .int()
                                                    > 0i32),
                                                self.cur_cmd,
                                                self.cur_chr,
                                            ) {
                                                {
                                                    {
                                                        let __ix1810 = self.cur_list.tail_field;
                                                        let __v1811 =
                                                            self.new_math(0i32, self.cur_chr);
                                                        self.mem[crate::ix::U((__ix1810) as usize)]
                                                            .set_hh_rh(__v1811);
                                                    }
                                                    self.cur_list.tail_field = self.mem
                                                        [crate::ix::U(
                                                            (self.cur_list.tail_field) as usize,
                                                        )]
                                                    .hh()
                                                    .rh();
                                                }
                                            }
                                        }
                                    } else {
                                        // §1184
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
                                    if ((self.eqtb[crate::ix::U(((7892323i32) - 1) as usize)]
                                        .int()
                                        > 1i32)
                                        && (self.cur_list.mode_field == hmode))
                                    {
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
                                    // §1188
                                    self.cs_error();
                                }
                                108 => {
                                    // §1191
                                    self.init_math();
                                }
                                257 => {
                                    // §1194
                                    if self.privileged() {
                                        if (self.cur_group == math_shift_group) {
                                            self.start_eq_no();
                                        } else {
                                            self.off_save();
                                        }
                                    }
                                }
                                210 => {
                                    // §1204
                                    {
                                        {
                                            {
                                                let __ix1812 = self.cur_list.tail_field;
                                                let __v1813 = self.new_noad();
                                                self.mem[crate::ix::U((__ix1812) as usize)]
                                                    .set_hh_rh(__v1813);
                                            }
                                            self.cur_list.tail_field = self.mem
                                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                                            .hh()
                                            .rh();
                                        }
                                        self.back_input();
                                        self.scan_math(
                                            (self.cur_list.tail_field).wrapping_add(1i32),
                                        );
                                    }
                                }
                                220 | 221 | 277 => {
                                    // §1208
                                    self.set_math_char(
                                        self.eqtb[crate::ix::U(
                                            (((math_code_base).wrapping_add(self.cur_chr)) - 1)
                                                as usize,
                                        )]
                                        .hh()
                                        .rh(),
                                    );
                                }
                                225 => {
                                    self.scan_char_num();
                                    self.cur_chr = self.cur_val;
                                    self.set_math_char(
                                        self.eqtb[crate::ix::U(
                                            (((math_code_base).wrapping_add(self.cur_chr)) - 1)
                                                as usize,
                                        )]
                                        .hh()
                                        .rh(),
                                    );
                                }
                                226 => {
                                    if (self.cur_chr == 2i32) {
                                        {
                                            self.scan_math_class_int();
                                            t = self.set_class_field(self.cur_val);
                                            self.scan_math_fam_int();
                                            t = (t)
                                                .wrapping_add(self.set_family_field(self.cur_val));
                                            self.scan_usv_num();
                                            t = (t).wrapping_add(self.cur_val);
                                            self.set_math_char(t);
                                        }
                                    } else {
                                        if (self.cur_chr == 1i32) {
                                            {
                                                self.scan_xetex_math_char_int();
                                                self.set_math_char(self.cur_val);
                                            }
                                        } else {
                                            {
                                                self.scan_fifteen_bit_int();
                                                {
                                                    let __a1814_0 = ((self.set_class_field(
                                                        (self.cur_val / 4096i32),
                                                    ))
                                                    .wrapping_add(self.set_family_field(
                                                        ((self.cur_val % 4096i32) / 256i32),
                                                    )))
                                                    .wrapping_add((self.cur_val % 256i32));
                                                    self.set_math_char(__a1814_0)
                                                };
                                            }
                                        }
                                    }
                                }
                                278 => {
                                    {
                                        let __a1815_0 = ((self
                                            .set_class_field((self.cur_chr / 4096i32)))
                                        .wrapping_add(self.set_family_field(
                                            ((self.cur_chr % 4096i32) / 256i32),
                                        )))
                                        .wrapping_add((self.cur_chr % 256i32));
                                        self.set_math_char(__a1815_0)
                                    };
                                }
                                279 => {
                                    self.set_math_char(self.cur_chr);
                                }
                                224 => {
                                    if (self.cur_chr == 1i32) {
                                        {
                                            self.scan_math_class_int();
                                            t = self.set_class_field(self.cur_val);
                                            self.scan_math_fam_int();
                                            t = (t)
                                                .wrapping_add(self.set_family_field(self.cur_val));
                                            self.scan_usv_num();
                                            t = (t).wrapping_add(self.cur_val);
                                            self.set_math_char(t);
                                        }
                                    } else {
                                        {
                                            self.scan_delimiter_int();
                                            self.cur_val = (self.cur_val / 4096i32);
                                            {
                                                let __a1816_0 = ((self
                                                    .set_class_field((self.cur_val / 4096i32)))
                                                .wrapping_add(self.set_family_field(
                                                    ((self.cur_val % 4096i32) / 256i32),
                                                )))
                                                .wrapping_add((self.cur_val % 256i32));
                                                self.set_math_char(__a1816_0)
                                            };
                                        }
                                    }
                                }
                                259 => {
                                    // §1212
                                    {
                                        {
                                            {
                                                let __ix1817 = self.cur_list.tail_field;
                                                let __v1818 = self.new_noad();
                                                self.mem[crate::ix::U((__ix1817) as usize)]
                                                    .set_hh_rh(__v1818);
                                            }
                                            self.cur_list.tail_field = self.mem
                                                [crate::ix::U((self.cur_list.tail_field) as usize)]
                                            .hh()
                                            .rh();
                                        }
                                        {
                                            let __ix1819 = self.cur_list.tail_field;
                                            let __v1820 = self.cur_chr;
                                            self.mem[crate::ix::U((__ix1819) as usize)]
                                                .set_hh_b0(__v1820);
                                        }
                                        self.scan_math(
                                            (self.cur_list.tail_field).wrapping_add(1i32),
                                        );
                                    }
                                }
                                260 => {
                                    self.math_limit_switch();
                                }
                                275 => {
                                    // §1216
                                    self.math_radical();
                                }
                                254 | 255 => {
                                    // §1218
                                    self.math_ac();
                                }
                                265 => {
                                    // §1221
                                    {
                                        self.scan_spec(vcenter_group, false);
                                        self.normal_paragraph();
                                        self.push_nest();
                                        self.cur_list.mode_field = (1i32).wrapping_neg();
                                        self.cur_list
                                            .aux_field
                                            .set_int((65536000i32).wrapping_neg());
                                        if (self.eqtb
                                            [crate::ix::U(((every_vbox_loc) - 1) as usize)]
                                        .hh()
                                        .rh()
                                            != (268435455i32).wrapping_neg())
                                        {
                                            self.begin_token_list(
                                                self.eqtb
                                                    [crate::ix::U(((every_vbox_loc) - 1) as usize)]
                                                .hh()
                                                .rh(),
                                                every_vbox_text,
                                            );
                                        }
                                    }
                                }
                                262 => {
                                    // §1225
                                    {
                                        {
                                            let __ix1821 = self.cur_list.tail_field;
                                            let __v1822 = self.new_style(self.cur_chr);
                                            self.mem[crate::ix::U((__ix1821) as usize)]
                                                .set_hh_rh(__v1822);
                                        }
                                        self.cur_list.tail_field = self.mem
                                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                                        .hh()
                                        .rh();
                                    }
                                }
                                264 => {
                                    {
                                        {
                                            let __ix1823 = self.cur_list.tail_field;
                                            let __v1824 = self.new_glue(zero_glue);
                                            self.mem[crate::ix::U((__ix1823) as usize)]
                                                .set_hh_rh(__v1824);
                                        }
                                        self.cur_list.tail_field = self.mem
                                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                                        .hh()
                                        .rh();
                                    }
                                    {
                                        let __ix1825 = self.cur_list.tail_field;
                                        self.mem[crate::ix::U((__ix1825) as usize)]
                                            .set_hh_b1(cond_math_glue);
                                    }
                                }
                                263 => {
                                    self.append_choices();
                                }
                                217 | 216 => {
                                    // §1229
                                    self.sub_sup();
                                }
                                261 => {
                                    // §1234
                                    self.math_fraction();
                                }
                                258 => {
                                    // §1244
                                    self.math_left_right();
                                }
                                212 => {
                                    // §1247
                                    if (self.cur_group == math_shift_group) {
                                        self.after_math();
                                    } else {
                                        self.off_save();
                                    }
                                }
                                73 | 177 | 281 | 74 | 178 | 282 | 75 | 179 | 283 | 76 | 180
                                | 284 | 77 | 181 | 285 | 78 | 182 | 286 | 79 | 183 | 287 | 80
                                | 184 | 288 | 81 | 185 | 289 | 82 | 186 | 290 | 83 | 187 | 291
                                | 84 | 188 | 292 | 85 | 189 | 293 | 86 | 190 | 294 | 87 | 191
                                | 295 | 88 | 192 | 296 | 89 | 193 | 297 | 90 | 194 | 298 | 91
                                | 195 | 299 | 92 | 196 | 300 | 93 | 197 | 301 | 94 | 198 | 302
                                | 95 | 199 | 303 | 96 | 200 | 304 | 97 | 201 | 305 | 98 | 202
                                | 306 | 99 | 203 | 307 | 100 | 204 | 308 | 101 | 205 | 309
                                | 102 | 206 | 310 | 103 | 207 | 311 => {
                                    // §1264
                                    self.prefixed_command();
                                }
                                41 | 145 | 249 => {
                                    // §1322
                                    {
                                        self.get_token();
                                        self.after_token = self.cur_tok;
                                    }
                                }
                                42 | 146 | 250 => {
                                    // §1325
                                    {
                                        self.get_token();
                                        self.save_for_after(self.cur_tok);
                                    }
                                }
                                104 | 208 | 312 => {
                                    self.get_token();
                                    if (self.cur_cs > 0i32) {
                                        {
                                            self.par_loc = self.cur_cs;
                                            self.par_token = self.cur_tok;
                                        }
                                    }
                                }
                                61 | 165 | 269 => {
                                    // §1328
                                    self.open_or_close_in();
                                }
                                59 | 163 | 267 => {
                                    // §1330
                                    self.issue_message();
                                }
                                58 | 162 | 266 => {
                                    // §1339
                                    self.shift_case();
                                }
                                20 | 124 | 228 => {
                                    // §1344
                                    self.show_whatever();
                                }
                                60 | 164 | 268 => {
                                    // §1402
                                    self.do_extension();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // §1084
                {
                    __goto_1 = 1;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 3 {
                // L70
                self.prev_class = 4095i32;
                // §1088
                if ((self.font_area[crate::ix::U(
                    (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                        .hh()
                        .rh()) as usize,
                )] == aat_font_flag)
                    || (self.font_area[crate::ix::U(
                        (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                            .hh()
                            .rh()) as usize,
                    )] == otgr_font_flag))
                {
                    {
                        // goto labels: L71, L72
                        let mut __goto_2: i32 = 0;
                        'l_dispatch_2: loop {
                            if __goto_2 <= 0 {
                                if (self.cur_list.mode_field > 0i32) {
                                    if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int()
                                        != self.cur_list.aux_field.hh().rh())
                                    {
                                        self.fix_language();
                                    }
                                }
                                self.main_h = 0i32;
                                self.main_f = self.eqtb
                                    [crate::ix::U(((cur_font_loc) - 1) as usize)]
                                .hh()
                                .rh();
                                self.native_len = 0i32;
                            }
                            if __goto_2 <= 1 {
                                // L71
                                self.main_s = (self.eqtb[crate::ix::U(
                                    (((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize,
                                )]
                                .hh()
                                .rh()
                                    % 65536i32);
                                if (self.main_s == 1000i32) {
                                    self.cur_list.aux_field.set_hh_lh(1000i32);
                                } else {
                                    if (self.main_s < 1000i32) {
                                        {
                                            if (self.main_s > 0i32) {
                                                {
                                                    let __v1826 = self.main_s;
                                                    self.cur_list.aux_field.set_hh_lh(__v1826);
                                                }
                                            }
                                        }
                                    } else {
                                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                                            self.cur_list.aux_field.set_hh_lh(1000i32);
                                        } else {
                                            {
                                                let __v1827 = self.main_s;
                                                self.cur_list.aux_field.set_hh_lh(__v1827);
                                            }
                                        }
                                    }
                                }
                                self.cur_ptr = (268435455i32).wrapping_neg();
                                self.space_class = (self.eqtb[crate::ix::U(
                                    (((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize,
                                )]
                                .hh()
                                .rh()
                                    / 65536i32);
                                if ((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int()
                                    > 0i32)
                                    && (self.space_class != char_class_ignored))
                                {
                                    {
                                        if (self.prev_class == 4095i32) {
                                            {
                                                if ((self.cur_input.state_field != token_list)
                                                    || (self.cur_input.index_field
                                                        != backed_up_char))
                                                {
                                                    {
                                                        self.find_sa_element(
                                                            inter_char_val,
                                                            ((4095i32)
                                                                .wrapping_mul(char_class_limit))
                                                            .wrapping_add(self.space_class),
                                                            false,
                                                        );
                                                        if ((self.cur_ptr
                                                            != (268435455i32).wrapping_neg())
                                                            && (self.mem[crate::ix::U(
                                                                ((self.cur_ptr).wrapping_add(1i32))
                                                                    as usize,
                                                            )]
                                                            .hh()
                                                            .rh()
                                                                != (268435455i32).wrapping_neg()))
                                                        {
                                                            {
                                                                if (self.cur_cmd != letter) {
                                                                    self.cur_cmd = other_char;
                                                                }
                                                                self.cur_tok = ((self.cur_cmd)
                                                                    .wrapping_mul(max_char_val))
                                                                .wrapping_add(self.cur_chr);
                                                                self.back_input();
                                                                self.cur_input.index_field =
                                                                    backed_up_char;
                                                                self.begin_token_list(
                                                                    self.mem[crate::ix::U(
                                                                        ((self.cur_ptr)
                                                                            .wrapping_add(1i32))
                                                                            as usize,
                                                                    )]
                                                                    .hh()
                                                                    .rh(),
                                                                    inter_char_text,
                                                                );
                                                                {
                                                                    __goto_1 = 1;
                                                                    continue 'l_dispatch_1;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            {
                                                self.find_sa_element(
                                                    inter_char_val,
                                                    ((self.prev_class)
                                                        .wrapping_mul(char_class_limit))
                                                    .wrapping_add(self.space_class),
                                                    false,
                                                );
                                                if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                                    && (self.mem[crate::ix::U(
                                                        ((self.cur_ptr).wrapping_add(1i32))
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh()
                                                        != (268435455i32).wrapping_neg()))
                                                {
                                                    {
                                                        if (self.cur_cmd != letter) {
                                                            self.cur_cmd = other_char;
                                                        }
                                                        self.cur_tok = ((self.cur_cmd)
                                                            .wrapping_mul(max_char_val))
                                                        .wrapping_add(self.cur_chr);
                                                        self.back_input();
                                                        self.cur_input.index_field = backed_up_char;
                                                        self.begin_token_list(
                                                            self.mem[crate::ix::U(
                                                                ((self.cur_ptr).wrapping_add(1i32))
                                                                    as usize,
                                                            )]
                                                            .hh()
                                                            .rh(),
                                                            inter_char_text,
                                                        );
                                                        self.prev_class = 4095i32;
                                                        {
                                                            __goto_2 = 2;
                                                            continue 'l_dispatch_2;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.prev_class = self.space_class;
                                    }
                                }
                                if (self.cur_chr > 65535i32) {
                                    {
                                        while (self.native_text_size
                                            <= (self.native_len).wrapping_add(2i32))
                                        {
                                            {
                                                self.native_text_size =
                                                    (self.native_text_size).wrapping_add(128i32);
                                                self.native_text.resize_len(
                                                    ((self.native_text_size) as usize) + 1,
                                                );
                                            }
                                        }
                                        {
                                            {
                                                let __ix1828 = self.native_len;
                                                let __v1829 = ((self.cur_chr)
                                                    .wrapping_sub(65536i32)
                                                    / 1024i32)
                                                    .wrapping_add(55296i32);
                                                self.native_text
                                                    [crate::ix::U((__ix1828) as usize)] = __v1829;
                                            }
                                            self.native_len = (self.native_len).wrapping_add(1i32);
                                        }
                                        {
                                            {
                                                let __ix1830 = self.native_len;
                                                let __v1831 = ((self.cur_chr)
                                                    .wrapping_sub(65536i32)
                                                    % 1024i32)
                                                    .wrapping_add(56320i32);
                                                self.native_text
                                                    [crate::ix::U((__ix1830) as usize)] = __v1831;
                                            }
                                            self.native_len = (self.native_len).wrapping_add(1i32);
                                        }
                                    }
                                } else {
                                    {
                                        while (self.native_text_size
                                            <= (self.native_len).wrapping_add(1i32))
                                        {
                                            {
                                                self.native_text_size =
                                                    (self.native_text_size).wrapping_add(128i32);
                                                self.native_text.resize_len(
                                                    ((self.native_text_size) as usize) + 1,
                                                );
                                            }
                                        }
                                        {
                                            {
                                                let __ix1832 = self.native_len;
                                                let __v1833 = self.cur_chr;
                                                self.native_text
                                                    [crate::ix::U((__ix1832) as usize)] = __v1833;
                                            }
                                            self.native_len = (self.native_len).wrapping_add(1i32);
                                        }
                                    }
                                }
                                self.is_hyph = ((self.cur_chr
                                    == self.hyphen_char[crate::ix::U((self.main_f) as usize)])
                                    || ((self.eqtb[crate::ix::U(((7892340i32) - 1) as usize)]
                                        .int()
                                        > 0i32)
                                        && ((self.cur_chr == 8212i32)
                                            || (self.cur_chr == 8211i32))));
                                if ((self.main_h == 0i32) && self.is_hyph) {
                                    self.main_h = self.native_len;
                                }
                                self.get_next();
                                if (((self.cur_cmd == letter) || (self.cur_cmd == other_char))
                                    || (self.cur_cmd == char_given))
                                {
                                    {
                                        __goto_2 = 1;
                                        continue 'l_dispatch_2;
                                    }
                                }
                                self.x_token();
                                if (((self.cur_cmd == letter) || (self.cur_cmd == other_char))
                                    || (self.cur_cmd == char_given))
                                {
                                    {
                                        __goto_2 = 1;
                                        continue 'l_dispatch_2;
                                    }
                                }
                                if (self.cur_cmd == char_num) {
                                    {
                                        self.scan_usv_num();
                                        self.cur_chr = self.cur_val;
                                        {
                                            __goto_2 = 1;
                                            continue 'l_dispatch_2;
                                        }
                                    }
                                }
                                if (((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int()
                                    > 0i32)
                                    && (self.space_class != char_class_ignored))
                                    && (self.prev_class != 4095i32))
                                {
                                    {
                                        self.prev_class = 4095i32;
                                        self.find_sa_element(
                                            inter_char_val,
                                            ((self.space_class).wrapping_mul(char_class_limit))
                                                .wrapping_add(4095i32),
                                            false,
                                        );
                                        if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                            && (self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh()
                                                != (268435455i32).wrapping_neg()))
                                        {
                                            {
                                                if (self.cur_cs == 0i32) {
                                                    {
                                                        if (self.cur_cmd == char_num) {
                                                            self.cur_cmd = other_char;
                                                        }
                                                        self.cur_tok = ((self.cur_cmd)
                                                            .wrapping_mul(max_char_val))
                                                        .wrapping_add(self.cur_chr);
                                                    }
                                                } else {
                                                    self.cur_tok =
                                                        (cs_token_flag).wrapping_add(self.cur_cs);
                                                }
                                                self.back_input();
                                                self.begin_token_list(
                                                    self.mem[crate::ix::U(
                                                        ((self.cur_ptr).wrapping_add(1i32))
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh(),
                                                    inter_char_text,
                                                );
                                                {
                                                    __goto_2 = 2;
                                                    continue 'l_dispatch_2;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if __goto_2 <= 2 {
                                // L72
                                if (self.font_mapping[crate::ix::U((self.main_f) as usize)] != 0i32)
                                {
                                    {
                                        self.main_k = self.apply_mapping_native(
                                            self.font_mapping[crate::ix::U((self.main_f) as usize)],
                                            self.native_len,
                                        );
                                        self.native_len = 0i32;
                                        while (self.native_text_size
                                            <= (self.native_len).wrapping_add(self.main_k))
                                        {
                                            {
                                                self.native_text_size =
                                                    (self.native_text_size).wrapping_add(128i32);
                                                self.native_text.resize_len(
                                                    ((self.native_text_size) as usize) + 1,
                                                );
                                            }
                                        }
                                        self.main_h = 0i32;
                                        {
                                            let __for_end_10 = (self.main_k).wrapping_sub(1i32);
                                            self.main_p = 0i32;
                                            while self.main_p <= __for_end_10 {
                                                {
                                                    {
                                                        {
                                                            let __ix1834 = self.native_len;
                                                            let __v1835 = self.mapped_text
                                                                [crate::ix::U(
                                                                    (self.main_p) as usize,
                                                                )];
                                                            self.native_text[crate::ix::U(
                                                                (__ix1834) as usize,
                                                            )] = __v1835;
                                                        }
                                                        self.native_len =
                                                            (self.native_len).wrapping_add(1i32);
                                                    }
                                                    if ((self.main_h == 0i32)
                                                        && ((self.mapped_text[crate::ix::U(
                                                            (self.main_p) as usize,
                                                        )] == self.hyphen_char[crate::ix::U(
                                                            (self.main_f) as usize,
                                                        )]) || ((self.eqtb[crate::ix::U(
                                                            ((7892340i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 0i32)
                                                            && ((self.mapped_text[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )] == 8212i32)
                                                                || (self.mapped_text
                                                                    [crate::ix::U(
                                                                        (self.main_p) as usize,
                                                                    )]
                                                                    == 8211i32)))))
                                                    {
                                                        self.main_h = self.native_len;
                                                    }
                                                }
                                                self.main_p = self.main_p.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int()
                                    > 0i32)
                                {
                                    {
                                        self.temp_ptr = 0i32;
                                        while (self.temp_ptr < self.native_len) {
                                            {
                                                self.main_k = self.native_text
                                                    [crate::ix::U((self.temp_ptr) as usize)];
                                                self.temp_ptr = (self.temp_ptr).wrapping_add(1i32);
                                                if ((self.main_k >= 55296i32)
                                                    && (self.main_k < 56320i32))
                                                {
                                                    {
                                                        self.main_k = (65536i32).wrapping_add(
                                                            ((self.main_k).wrapping_sub(55296i32))
                                                                .wrapping_mul(1024i32),
                                                        );
                                                        self.main_k = ((self.main_k).wrapping_add(
                                                            self.native_text[crate::ix::U(
                                                                (self.temp_ptr) as usize,
                                                            )],
                                                        ))
                                                        .wrapping_sub(56320i32);
                                                        self.temp_ptr =
                                                            (self.temp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                if (self
                                                    .map_char_to_glyph(self.main_f, self.main_k)
                                                    == 0i32)
                                                {
                                                    self.char_warning(self.main_f, self.main_k);
                                                }
                                            }
                                        }
                                    }
                                }
                                self.main_k = self.native_len;
                                self.main_pp = self.cur_list.tail_field;
                                if (self.cur_list.mode_field == hmode) {
                                    {
                                        self.main_ppp = self.cur_list.head_field;
                                        while ((self.main_ppp != self.main_pp)
                                            && (self.mem[crate::ix::U((self.main_ppp) as usize)]
                                                .hh()
                                                .rh()
                                                != self.main_pp))
                                        {
                                            {
                                                if ((!(self.main_ppp >= self.hi_mem_min))
                                                    && (self.mem
                                                        [crate::ix::U((self.main_ppp) as usize)]
                                                    .hh()
                                                    .b0()
                                                        == disc_node))
                                                {
                                                    {
                                                        self.temp_ptr = self.main_ppp;
                                                        {
                                                            let __for_end_14 = self.mem
                                                                [crate::ix::U(
                                                                    (self.temp_ptr) as usize,
                                                                )]
                                                            .hh()
                                                            .b1();
                                                            self.main_p = 1i32;
                                                            while self.main_p <= __for_end_14 {
                                                                self.main_ppp = self.mem
                                                                    [crate::ix::U(
                                                                        (self.main_ppp) as usize,
                                                                    )]
                                                                .hh()
                                                                .rh();
                                                                self.main_p =
                                                                    self.main_p.wrapping_add(1);
                                                            }
                                                        }
                                                    }
                                                }
                                                if (self.main_ppp != self.main_pp) {
                                                    self.main_ppp = self.mem
                                                        [crate::ix::U((self.main_ppp) as usize)]
                                                    .hh()
                                                    .rh();
                                                }
                                            }
                                        }
                                        self.temp_ptr = 0i32;
                                        loop {
                                            if (self.main_h == 0i32) {
                                                self.main_h = self.main_k;
                                            }
                                            if ((((((((self.main_pp
                                                != (268435455i32).wrapping_neg())
                                                && (!(self.main_pp >= self.hi_mem_min)))
                                                && (self.mem
                                                    [crate::ix::U((self.main_pp) as usize)]
                                                .hh()
                                                .b0()
                                                    == whatsit_node))
                                                && ((self.mem
                                                    [crate::ix::U((self.main_pp) as usize)]
                                                .hh()
                                                .b1()
                                                    >= native_word_node)
                                                    && (self.mem[crate::ix::U(
                                                        (self.main_pp) as usize,
                                                    )]
                                                    .hh()
                                                    .b1()
                                                        <= native_word_node_AT)))
                                                && (self.mem[crate::ix::U(
                                                    ((self.main_pp).wrapping_add(4i32)) as usize,
                                                )]
                                                .qqqq()
                                                .b1()
                                                    == self.main_f))
                                                && (self.main_ppp != self.main_pp))
                                                && (!(self.main_ppp >= self.hi_mem_min)))
                                                && (self.mem
                                                    [crate::ix::U((self.main_ppp) as usize)]
                                                .hh()
                                                .b0()
                                                    != disc_node))
                                            {
                                                {
                                                    self.main_k = (self.main_h).wrapping_add(
                                                        self.mem[crate::ix::U(
                                                            ((self.main_pp).wrapping_add(4i32))
                                                                as usize,
                                                        )]
                                                        .qqqq()
                                                        .b2(),
                                                    );
                                                    while (self.native_text_size
                                                        <= (self.native_len)
                                                            .wrapping_add(self.main_k))
                                                    {
                                                        {
                                                            self.native_text_size = (self
                                                                .native_text_size)
                                                                .wrapping_add(128i32);
                                                            self.native_text.resize_len(
                                                                ((self.native_text_size) as usize)
                                                                    + 1,
                                                            );
                                                        }
                                                    }
                                                    self.save_native_len = self.native_len;
                                                    {
                                                        let __for_end_13 = (self.mem[crate::ix::U(
                                                            ((self.main_pp).wrapping_add(4i32))
                                                                as usize,
                                                        )]
                                                        .qqqq()
                                                        .b2())
                                                        .wrapping_sub(1i32);
                                                        self.main_p = 0i32;
                                                        while self.main_p <= __for_end_13 {
                                                            {
                                                                {
                                                                    let __ix1836 = self.native_len;
                                                                    let __v1837 = self
                                                                        .get_native_char(
                                                                            self.main_pp,
                                                                            self.main_p,
                                                                        );
                                                                    self.native_text
                                                                        [crate::ix::U(
                                                                            (__ix1836) as usize,
                                                                        )] = __v1837;
                                                                }
                                                                self.native_len = (self.native_len)
                                                                    .wrapping_add(1i32);
                                                            }
                                                            self.main_p =
                                                                self.main_p.wrapping_add(1);
                                                        }
                                                    }
                                                    {
                                                        let __for_end_13 =
                                                            (self.main_h).wrapping_sub(1i32);
                                                        self.main_p = 0i32;
                                                        while self.main_p <= __for_end_13 {
                                                            {
                                                                {
                                                                    let __ix1838 = self.native_len;
                                                                    let __v1839 = self.native_text
                                                                        [crate::ix::U(
                                                                            ((self.temp_ptr)
                                                                                .wrapping_add(
                                                                                    self.main_p,
                                                                                ))
                                                                                as usize,
                                                                        )];
                                                                    self.native_text
                                                                        [crate::ix::U(
                                                                            (__ix1838) as usize,
                                                                        )] = __v1839;
                                                                }
                                                                self.native_len = (self.native_len)
                                                                    .wrapping_add(1i32);
                                                            }
                                                            self.main_p =
                                                                self.main_p.wrapping_add(1);
                                                        }
                                                    }
                                                    self.do_locale_linebreaks(
                                                        self.save_native_len,
                                                        self.main_k,
                                                    );
                                                    self.native_len = self.save_native_len;
                                                    self.main_k = ((self.native_len)
                                                        .wrapping_sub(self.main_h))
                                                    .wrapping_sub(self.temp_ptr);
                                                    self.temp_ptr = self.main_h;
                                                    self.main_h = 0i32;
                                                    while (((self.main_h < self.main_k)
                                                        && (self.native_text[crate::ix::U(
                                                            ((self.temp_ptr)
                                                                .wrapping_add(self.main_h))
                                                                as usize,
                                                        )] != self.hyphen_char[crate::ix::U(
                                                            (self.main_f) as usize,
                                                        )]))
                                                        && ((!(self.eqtb[crate::ix::U(
                                                            ((7892340i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 0i32))
                                                            || ((self.native_text[crate::ix::U(
                                                                ((self.temp_ptr)
                                                                    .wrapping_add(self.main_h))
                                                                    as usize,
                                                            )] != 8212i32)
                                                                && (self.native_text
                                                                    [crate::ix::U(
                                                                        ((self.temp_ptr)
                                                                            .wrapping_add(
                                                                                self.main_h,
                                                                            ))
                                                                            as usize,
                                                                    )]
                                                                    != 8211i32))))
                                                    {
                                                        self.main_h =
                                                            (self.main_h).wrapping_add(1i32);
                                                    }
                                                    if (self.main_h < self.main_k) {
                                                        self.main_h =
                                                            (self.main_h).wrapping_add(1i32);
                                                    }
                                                    {
                                                        let __ix1840 = self.main_ppp;
                                                        let __v1841 = self.mem
                                                            [crate::ix::U((self.main_pp) as usize)]
                                                        .hh()
                                                        .rh();
                                                        self.mem[crate::ix::U((__ix1840) as usize)]
                                                            .set_hh_rh(__v1841);
                                                    }
                                                    {
                                                        let __ix1842 = self.main_pp;
                                                        self.mem[crate::ix::U((__ix1842) as usize)]
                                                            .set_hh_rh(
                                                                (268435455i32).wrapping_neg(),
                                                            );
                                                    }
                                                    self.flush_node_list(self.main_pp);
                                                    self.main_pp = self.cur_list.tail_field;
                                                    while (self.mem
                                                        [crate::ix::U((self.main_ppp) as usize)]
                                                    .hh()
                                                    .rh()
                                                        != self.main_pp)
                                                    {
                                                        self.main_ppp = self.mem[crate::ix::U(
                                                            (self.main_ppp) as usize,
                                                        )]
                                                        .hh()
                                                        .rh();
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.do_locale_linebreaks(
                                                        self.temp_ptr,
                                                        self.main_h,
                                                    );
                                                    self.temp_ptr =
                                                        (self.temp_ptr).wrapping_add(self.main_h);
                                                    self.main_k =
                                                        (self.main_k).wrapping_sub(self.main_h);
                                                    self.main_h = 0i32;
                                                    while (((self.main_h < self.main_k)
                                                        && (self.native_text[crate::ix::U(
                                                            ((self.temp_ptr)
                                                                .wrapping_add(self.main_h))
                                                                as usize,
                                                        )] != self.hyphen_char[crate::ix::U(
                                                            (self.main_f) as usize,
                                                        )]))
                                                        && ((!(self.eqtb[crate::ix::U(
                                                            ((7892340i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 0i32))
                                                            || ((self.native_text[crate::ix::U(
                                                                ((self.temp_ptr)
                                                                    .wrapping_add(self.main_h))
                                                                    as usize,
                                                            )] != 8212i32)
                                                                && (self.native_text
                                                                    [crate::ix::U(
                                                                        ((self.temp_ptr)
                                                                            .wrapping_add(
                                                                                self.main_h,
                                                                            ))
                                                                            as usize,
                                                                    )]
                                                                    != 8211i32))))
                                                    {
                                                        self.main_h =
                                                            (self.main_h).wrapping_add(1i32);
                                                    }
                                                    if (self.main_h < self.main_k) {
                                                        self.main_h =
                                                            (self.main_h).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            if ((self.main_k > 0i32) || self.is_hyph) {
                                                {
                                                    {
                                                        {
                                                            let __ix1843 = self.cur_list.tail_field;
                                                            let __v1844 = self.new_disc();
                                                            self.mem
                                                                [crate::ix::U((__ix1843) as usize)]
                                                            .set_hh_rh(__v1844);
                                                        }
                                                        self.cur_list.tail_field = self.mem
                                                            [crate::ix::U(
                                                                (self.cur_list.tail_field) as usize,
                                                            )]
                                                        .hh()
                                                        .rh();
                                                    }
                                                    self.main_pp = self.cur_list.tail_field;
                                                }
                                            }
                                            if (self.main_k == 0i32) {
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        self.main_ppp = self.cur_list.head_field;
                                        while ((self.main_ppp != self.main_pp)
                                            && (self.mem[crate::ix::U((self.main_ppp) as usize)]
                                                .hh()
                                                .rh()
                                                != self.main_pp))
                                        {
                                            {
                                                if ((!(self.main_ppp >= self.hi_mem_min))
                                                    && (self.mem
                                                        [crate::ix::U((self.main_ppp) as usize)]
                                                    .hh()
                                                    .b0()
                                                        == disc_node))
                                                {
                                                    {
                                                        self.temp_ptr = self.main_ppp;
                                                        {
                                                            let __for_end_14 = self.mem
                                                                [crate::ix::U(
                                                                    (self.temp_ptr) as usize,
                                                                )]
                                                            .hh()
                                                            .b1();
                                                            self.main_p = 1i32;
                                                            while self.main_p <= __for_end_14 {
                                                                self.main_ppp = self.mem
                                                                    [crate::ix::U(
                                                                        (self.main_ppp) as usize,
                                                                    )]
                                                                .hh()
                                                                .rh();
                                                                self.main_p =
                                                                    self.main_p.wrapping_add(1);
                                                            }
                                                        }
                                                    }
                                                }
                                                if (self.main_ppp != self.main_pp) {
                                                    self.main_ppp = self.mem
                                                        [crate::ix::U((self.main_ppp) as usize)]
                                                    .hh()
                                                    .rh();
                                                }
                                            }
                                        }
                                        if ((((((((self.main_pp
                                            != (268435455i32).wrapping_neg())
                                            && (!(self.main_pp >= self.hi_mem_min)))
                                            && (self.mem
                                                [crate::ix::U((self.main_pp) as usize)]
                                            .hh()
                                            .b0()
                                                == whatsit_node))
                                            && ((self.mem
                                                [crate::ix::U((self.main_pp) as usize)]
                                            .hh()
                                            .b1()
                                                >= native_word_node)
                                                && (self.mem
                                                    [crate::ix::U((self.main_pp) as usize)]
                                                .hh()
                                                .b1()
                                                    <= native_word_node_AT)))
                                            && (self.mem[crate::ix::U(
                                                ((self.main_pp).wrapping_add(4i32)) as usize,
                                            )]
                                            .qqqq()
                                            .b1()
                                                == self.main_f))
                                            && (self.main_ppp != self.main_pp))
                                            && (!(self.main_ppp >= self.hi_mem_min)))
                                            && (self.mem[crate::ix::U((self.main_ppp) as usize)]
                                                .hh()
                                                .b0()
                                                != disc_node))
                                        {
                                            {
                                                {
                                                    let __ix1845 = self.main_pp;
                                                    let __v1846 = self.new_native_word_node(
                                                        self.main_f,
                                                        (self.main_k).wrapping_add(
                                                            self.mem[crate::ix::U(
                                                                ((self.main_pp).wrapping_add(4i32))
                                                                    as usize,
                                                            )]
                                                            .qqqq()
                                                            .b2(),
                                                        ),
                                                    );
                                                    self.mem[crate::ix::U((__ix1845) as usize)]
                                                        .set_hh_rh(__v1846);
                                                }
                                                self.cur_list.tail_field = self.mem
                                                    [crate::ix::U((self.main_pp) as usize)]
                                                .hh()
                                                .rh();
                                                {
                                                    let __for_end_12 = (self.mem[crate::ix::U(
                                                        ((self.main_pp).wrapping_add(4i32))
                                                            as usize,
                                                    )]
                                                    .qqqq()
                                                    .b2())
                                                    .wrapping_sub(1i32);
                                                    self.main_p = 0i32;
                                                    while self.main_p <= __for_end_12 {
                                                        {
                                                            let __a1847_0 =
                                                                self.cur_list.tail_field;
                                                            let __a1847_1 = self.main_p;
                                                            let __a1847_2 = self.get_native_char(
                                                                self.main_pp,
                                                                self.main_p,
                                                            );
                                                            self.set_native_char(
                                                                __a1847_0, __a1847_1, __a1847_2,
                                                            )
                                                        };
                                                        self.main_p = self.main_p.wrapping_add(1);
                                                    }
                                                }
                                                {
                                                    let __for_end_12 =
                                                        (self.main_k).wrapping_sub(1i32);
                                                    self.main_p = 0i32;
                                                    while self.main_p <= __for_end_12 {
                                                        self.set_native_char(
                                                            self.cur_list.tail_field,
                                                            (self.main_p).wrapping_add(
                                                                self.mem[crate::ix::U(
                                                                    ((self.main_pp)
                                                                        .wrapping_add(4i32))
                                                                        as usize,
                                                                )]
                                                                .qqqq()
                                                                .b2(),
                                                            ),
                                                            self.native_text[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )],
                                                        );
                                                        self.main_p = self.main_p.wrapping_add(1);
                                                    }
                                                }
                                                self.set_native_metrics(
                                                    self.cur_list.tail_field,
                                                    (self.eqtb[crate::ix::U(
                                                        ((7892342i32) - 1) as usize,
                                                    )]
                                                    .int()
                                                        > 0i32),
                                                );
                                                self.main_p = self.cur_list.head_field;
                                                if (self.main_p != self.main_pp) {
                                                    while (self.mem
                                                        [crate::ix::U((self.main_p) as usize)]
                                                    .hh()
                                                    .rh()
                                                        != self.main_pp)
                                                    {
                                                        self.main_p = self.mem
                                                            [crate::ix::U((self.main_p) as usize)]
                                                        .hh()
                                                        .rh();
                                                    }
                                                }
                                                {
                                                    let __ix1848 = self.main_p;
                                                    let __v1849 = self.mem
                                                        [crate::ix::U((self.main_pp) as usize)]
                                                    .hh()
                                                    .rh();
                                                    self.mem[crate::ix::U((__ix1848) as usize)]
                                                        .set_hh_rh(__v1849);
                                                }
                                                {
                                                    let __ix1850 = self.main_pp;
                                                    self.mem[crate::ix::U((__ix1850) as usize)]
                                                        .set_hh_rh((268435455i32).wrapping_neg());
                                                }
                                                self.flush_node_list(self.main_pp);
                                            }
                                        } else {
                                            {
                                                {
                                                    let __ix1851 = self.main_pp;
                                                    let __v1852 = self.new_native_word_node(
                                                        self.main_f,
                                                        self.main_k,
                                                    );
                                                    self.mem[crate::ix::U((__ix1851) as usize)]
                                                        .set_hh_rh(__v1852);
                                                }
                                                self.cur_list.tail_field = self.mem
                                                    [crate::ix::U((self.main_pp) as usize)]
                                                .hh()
                                                .rh();
                                                {
                                                    let __for_end_12 =
                                                        (self.main_k).wrapping_sub(1i32);
                                                    self.main_p = 0i32;
                                                    while self.main_p <= __for_end_12 {
                                                        self.set_native_char(
                                                            self.cur_list.tail_field,
                                                            self.main_p,
                                                            self.native_text[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )],
                                                        );
                                                        self.main_p = self.main_p.wrapping_add(1);
                                                    }
                                                }
                                                self.set_native_metrics(
                                                    self.cur_list.tail_field,
                                                    (self.eqtb[crate::ix::U(
                                                        ((7892342i32) - 1) as usize,
                                                    )]
                                                    .int()
                                                        > 0i32),
                                                );
                                            }
                                        }
                                    }
                                }
                                if (self.eqtb[crate::ix::U(((7892348i32) - 1) as usize)].int()
                                    > 0i32)
                                {
                                    {
                                        self.main_p = self.cur_list.head_field;
                                        self.main_pp = (268435455i32).wrapping_neg();
                                        while (self.main_p != self.cur_list.tail_field) {
                                            {
                                                if ((((self.main_p
                                                    != (268435455i32).wrapping_neg())
                                                    && (!(self.main_p >= self.hi_mem_min)))
                                                    && (self.mem
                                                        [crate::ix::U((self.main_p) as usize)]
                                                    .hh()
                                                    .b0()
                                                        == whatsit_node))
                                                    && ((self.mem
                                                        [crate::ix::U((self.main_p) as usize)]
                                                    .hh()
                                                    .b1()
                                                        >= native_word_node)
                                                        && (self.mem[crate::ix::U(
                                                            (self.main_p) as usize,
                                                        )]
                                                        .hh()
                                                        .b1()
                                                            <= native_word_node_AT)))
                                                {
                                                    self.main_pp = self.main_p;
                                                }
                                                self.main_p = self.mem
                                                    [crate::ix::U((self.main_p) as usize)]
                                                .hh()
                                                .rh();
                                            }
                                        }
                                        if (self.main_pp != (268435455i32).wrapping_neg()) {
                                            {
                                                if (self.mem[crate::ix::U(
                                                    ((self.main_pp).wrapping_add(4i32)) as usize,
                                                )]
                                                .qqqq()
                                                .b1()
                                                    == self.main_f)
                                                {
                                                    {
                                                        self.main_p = self.mem
                                                            [crate::ix::U((self.main_pp) as usize)]
                                                        .hh()
                                                        .rh();
                                                        while ((!(self.main_p >= self.hi_mem_min))
                                                            && (((((self.mem[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )]
                                                            .hh()
                                                            .b0()
                                                                == penalty_node)
                                                                || (self.mem[crate::ix::U(
                                                                    (self.main_p) as usize,
                                                                )]
                                                                .hh()
                                                                .b0()
                                                                    == ins_node))
                                                                || (self.mem[crate::ix::U(
                                                                    (self.main_p) as usize,
                                                                )]
                                                                .hh()
                                                                .b0()
                                                                    == mark_node))
                                                                || (self.mem[crate::ix::U(
                                                                    (self.main_p) as usize,
                                                                )]
                                                                .hh()
                                                                .b0()
                                                                    == adjust_node))
                                                                || ((self.mem[crate::ix::U(
                                                                    (self.main_p) as usize,
                                                                )]
                                                                .hh()
                                                                .b0()
                                                                    == whatsit_node)
                                                                    && (self.mem[crate::ix::U(
                                                                        (self.main_p) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .b1()
                                                                        <= 4i32))))
                                                        {
                                                            self.main_p = self.mem[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )]
                                                            .hh()
                                                            .rh();
                                                        }
                                                        if ((!(self.main_p >= self.hi_mem_min))
                                                            && (self.mem[crate::ix::U(
                                                                (self.main_p) as usize,
                                                            )]
                                                            .hh()
                                                            .b0()
                                                                == glue_node))
                                                        {
                                                            {
                                                                self.main_ppp = self.mem
                                                                    [crate::ix::U(
                                                                        (self.main_p) as usize,
                                                                    )]
                                                                .hh()
                                                                .rh();
                                                                while ((!(self.main_ppp
                                                                    >= self.hi_mem_min))
                                                                    && (((((self.mem
                                                                        [crate::ix::U(
                                                                            (self.main_ppp)
                                                                                as usize,
                                                                        )]
                                                                    .hh()
                                                                    .b0()
                                                                        == penalty_node)
                                                                        || (self.mem
                                                                            [crate::ix::U(
                                                                                (self.main_ppp)
                                                                                    as usize,
                                                                            )]
                                                                        .hh()
                                                                        .b0()
                                                                            == ins_node))
                                                                        || (self.mem
                                                                            [crate::ix::U(
                                                                                (self.main_ppp)
                                                                                    as usize,
                                                                            )]
                                                                        .hh()
                                                                        .b0()
                                                                            == mark_node))
                                                                        || (self.mem
                                                                            [crate::ix::U(
                                                                                (self.main_ppp)
                                                                                    as usize,
                                                                            )]
                                                                        .hh()
                                                                        .b0()
                                                                            == adjust_node))
                                                                        || ((self.mem
                                                                            [crate::ix::U(
                                                                                (self.main_ppp)
                                                                                    as usize,
                                                                            )]
                                                                        .hh()
                                                                        .b0()
                                                                            == whatsit_node)
                                                                            && (self.mem
                                                                                [crate::ix::U(
                                                                                    (self.main_ppp)
                                                                                        as usize,
                                                                                )]
                                                                            .hh()
                                                                            .b1()
                                                                                <= 4i32))))
                                                                {
                                                                    self.main_ppp = self.mem
                                                                        [crate::ix::U(
                                                                            (self.main_ppp)
                                                                                as usize,
                                                                        )]
                                                                    .hh()
                                                                    .rh();
                                                                }
                                                                if (self.main_ppp
                                                                    == self.cur_list.tail_field)
                                                                {
                                                                    {
                                                                        self.temp_ptr = self.new_native_word_node(self.main_f, ((self.mem[crate::ix::U(((self.main_pp).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_add(1i32)).wrapping_add(self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(4i32)) as usize)].qqqq().b2()));
                                                                        self.main_k = 0i32;
                                                                        {
                                                                            let __for_end_18 = (self.mem[crate::ix::U(((self.main_pp).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                            t = 0i32;
                                                                            while t <= __for_end_18
                                                                            {
                                                                                {
                                                                                    {
                                                                                        let __a1853_0 = self.temp_ptr;
                                                                                        let __a1853_1 = self.main_k;
                                                                                        let __a1853_2 = self.get_native_char(self.main_pp, t);
                                                                                        self.set_native_char(__a1853_0, __a1853_1, __a1853_2)
                                                                                    };
                                                                                    self.main_k = (self.main_k).wrapping_add(1i32);
                                                                                }
                                                                                t = t.wrapping_add(
                                                                                    1,
                                                                                );
                                                                            }
                                                                        }
                                                                        self.set_native_char(
                                                                            self.temp_ptr,
                                                                            self.main_k,
                                                                            32i32,
                                                                        );
                                                                        self.main_k = (self.main_k)
                                                                            .wrapping_add(1i32);
                                                                        {
                                                                            let __for_end_18 = (self.mem[crate::ix::U(((self.cur_list.tail_field).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                            t = 0i32;
                                                                            while t <= __for_end_18
                                                                            {
                                                                                {
                                                                                    {
                                                                                        let __a1854_0 = self.temp_ptr;
                                                                                        let __a1854_1 = self.main_k;
                                                                                        let __a1854_2 = self.get_native_char(self.cur_list.tail_field, t);
                                                                                        self.set_native_char(__a1854_0, __a1854_1, __a1854_2)
                                                                                    };
                                                                                    self.main_k = (self.main_k).wrapping_add(1i32);
                                                                                }
                                                                                t = t.wrapping_add(
                                                                                    1,
                                                                                );
                                                                            }
                                                                        }
                                                                        self.set_native_metrics(
                                                                            self.temp_ptr,
                                                                            (self.eqtb
                                                                                [crate::ix::U(
                                                                                    ((7892342i32)
                                                                                        - 1)
                                                                                        as usize,
                                                                                )]
                                                                            .int()
                                                                                > 0i32),
                                                                        );
                                                                        t = ((self.mem
                                                                            [crate::ix::U(
                                                                                ((self.temp_ptr)
                                                                                    .wrapping_add(
                                                                                        1i32,
                                                                                    ))
                                                                                    as usize,
                                                                            )]
                                                                        .int())
                                                                        .wrapping_sub(
                                                                            self.mem[crate::ix::U(
                                                                                ((self.main_pp)
                                                                                    .wrapping_add(
                                                                                        1i32,
                                                                                    ))
                                                                                    as usize,
                                                                            )]
                                                                            .int(),
                                                                        ))
                                                                        .wrapping_sub(
                                                                            self.mem[crate::ix::U(
                                                                                ((self
                                                                                    .cur_list
                                                                                    .tail_field)
                                                                                    .wrapping_add(
                                                                                        1i32,
                                                                                    ))
                                                                                    as usize,
                                                                            )]
                                                                            .int(),
                                                                        );
                                                                        self.free_node(
                                                                            self.temp_ptr,
                                                                            self.mem[crate::ix::U(
                                                                                ((self.temp_ptr)
                                                                                    .wrapping_add(
                                                                                        4i32,
                                                                                    ))
                                                                                    as usize,
                                                                            )]
                                                                            .qqqq()
                                                                            .b0(),
                                                                        );
                                                                        if (t != self.mem[crate::ix::U(((self.font_glue[crate::ix::U((self.main_f) as usize)]).wrapping_add(1i32)) as usize)].int()) {
                                                                            {
                                                                                self.temp_ptr = self.new_kern((t).wrapping_sub(self.mem[crate::ix::U(((self.font_glue[crate::ix::U((self.main_f) as usize)]).wrapping_add(1i32)) as usize)].int()));
                                                                                { let __ix1855 = self.temp_ptr; self.mem[crate::ix::U((__ix1855) as usize)].set_hh_b1(space_adjustment); }
                                                                                { let __ix1856 = self.temp_ptr; let __v1857 = self.mem[crate::ix::U((self.main_p) as usize)].hh().rh(); self.mem[crate::ix::U((__ix1856) as usize)].set_hh_rh(__v1857); }
                                                                                { let __ix1858 = self.main_p; let __v1859 = self.temp_ptr; self.mem[crate::ix::U((__ix1858) as usize)].set_hh_rh(__v1859); }
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
                                if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                    {
                                        __goto_1 = 1;
                                        continue 'l_dispatch_1;
                                    }
                                } else {
                                    {
                                        __goto_1 = 2;
                                        continue 'l_dispatch_1;
                                    }
                                }
                            }
                            break 'l_dispatch_2;
                        }
                    }
                }
                self.main_s = (self.eqtb
                    [crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)]
                .hh()
                .rh()
                    % 65536i32);
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                {
                                    let __v1860 = self.main_s;
                                    self.cur_list.aux_field.set_hh_lh(__v1860);
                                }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            {
                                let __v1861 = self.main_s;
                                self.cur_list.aux_field.set_hh_lh(__v1861);
                            }
                        }
                    }
                }
                self.cur_ptr = (268435455i32).wrapping_neg();
                self.space_class = (self.eqtb
                    [crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)]
                .hh()
                .rh()
                    / 65536i32);
                if ((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int() > 0i32)
                    && (self.space_class != char_class_ignored))
                {
                    {
                        if (self.prev_class == 4095i32) {
                            {
                                if ((self.cur_input.state_field != token_list)
                                    || (self.cur_input.index_field != backed_up_char))
                                {
                                    {
                                        self.find_sa_element(
                                            inter_char_val,
                                            ((4095i32).wrapping_mul(char_class_limit))
                                                .wrapping_add(self.space_class),
                                            false,
                                        );
                                        if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                            && (self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh()
                                                != (268435455i32).wrapping_neg()))
                                        {
                                            {
                                                if (self.cur_cmd != letter) {
                                                    self.cur_cmd = other_char;
                                                }
                                                self.cur_tok = ((self.cur_cmd)
                                                    .wrapping_mul(max_char_val))
                                                .wrapping_add(self.cur_chr);
                                                self.back_input();
                                                self.cur_input.index_field = backed_up_char;
                                                self.begin_token_list(
                                                    self.mem[crate::ix::U(
                                                        ((self.cur_ptr).wrapping_add(1i32))
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh(),
                                                    inter_char_text,
                                                );
                                                {
                                                    __goto_1 = 1;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                self.find_sa_element(
                                    inter_char_val,
                                    ((self.prev_class).wrapping_mul(char_class_limit))
                                        .wrapping_add(self.space_class),
                                    false,
                                );
                                if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                    && (self.mem[crate::ix::U(
                                        ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                    )]
                                    .hh()
                                    .rh()
                                        != (268435455i32).wrapping_neg()))
                                {
                                    {
                                        if (self.cur_cmd != letter) {
                                            self.cur_cmd = other_char;
                                        }
                                        self.cur_tok = ((self.cur_cmd).wrapping_mul(max_char_val))
                                            .wrapping_add(self.cur_chr);
                                        self.back_input();
                                        self.cur_input.index_field = backed_up_char;
                                        self.begin_token_list(
                                            self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh(),
                                            inter_char_text,
                                        );
                                        self.prev_class = 4095i32;
                                        {
                                            __goto_1 = 1;
                                            continue 'l_dispatch_1;
                                        }
                                    }
                                }
                            }
                        }
                        self.prev_class = self.space_class;
                    }
                }
                self.main_f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                    .hh()
                    .rh();
                self.bchar = self.font_bchar[crate::ix::U((self.main_f) as usize)];
                self.false_bchar = self.font_false_bchar[crate::ix::U((self.main_f) as usize)];
                if (self.cur_list.mode_field > 0i32) {
                    if (self.eqtb[crate::ix::U(((7892314i32) - 1) as usize)].int()
                        != self.cur_list.aux_field.hh().rh())
                    {
                        self.fix_language();
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail =
                                self.mem[crate::ix::U((self.lig_stack) as usize)].hh().rh();
                            {
                                let __ix1862 = self.lig_stack;
                                self.mem[crate::ix::U((__ix1862) as usize)]
                                    .set_hh_rh((268435455i32).wrapping_neg());
                            }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                {
                    let __ix1863 = self.lig_stack;
                    let __v1864 = self.main_f;
                    self.mem[crate::ix::U((__ix1863) as usize)].set_hh_b0(__v1864);
                }
                self.cur_l = self.cur_chr;
                {
                    let __ix1865 = self.lig_stack;
                    let __v1866 = self.cur_l;
                    self.mem[crate::ix::U((__ix1865) as usize)].set_hh_b1(__v1866);
                }
                self.cur_q = self.cur_list.tail_field;
                if self.cancel_boundary {
                    {
                        self.cancel_boundary = false;
                        self.main_k = non_address;
                    }
                } else {
                    self.main_k = self.bchar_label[crate::ix::U((self.main_f) as usize)];
                }
                if (self.main_k == non_address) {
                    {
                        __goto_1 = 7;
                        continue 'l_dispatch_1;
                    }
                }
                self.cur_r = self.cur_l;
                self.cur_l = non_char;
                {
                    __goto_1 = 11;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 4 {
                // L80
                if (self.cur_l < non_char) {
                    // §1089
                    {
                        if (self.mem[crate::ix::U((self.cur_q) as usize)].hh().rh()
                            > (268435455i32).wrapping_neg())
                        {
                            if (self.mem[crate::ix::U((self.cur_list.tail_field) as usize)]
                                .hh()
                                .b1()
                                == self.hyphen_char[crate::ix::U((self.main_f) as usize)])
                            {
                                self.ins_disc = true;
                            }
                        }
                        if self.ligature_present {
                            {
                                self.main_p = self.new_ligature(
                                    self.main_f,
                                    self.cur_l,
                                    self.mem[crate::ix::U((self.cur_q) as usize)].hh().rh(),
                                );
                                if self.lft_hit {
                                    {
                                        {
                                            let __ix1867 = self.main_p;
                                            self.mem[crate::ix::U((__ix1867) as usize)]
                                                .set_hh_b1(2i32);
                                        }
                                        self.lft_hit = false;
                                    }
                                }
                                if self.rt_hit {
                                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                        {
                                            {
                                                let __ix1868 = self.main_p;
                                                let __v1869 = (self.mem
                                                    [crate::ix::U((self.main_p) as usize)]
                                                .hh()
                                                .b1())
                                                .wrapping_add(1i32);
                                                self.mem[crate::ix::U((__ix1868) as usize)]
                                                    .set_hh_b1(__v1869);
                                            }
                                            self.rt_hit = false;
                                        }
                                    }
                                }
                                {
                                    let __ix1870 = self.cur_q;
                                    let __v1871 = self.main_p;
                                    self.mem[crate::ix::U((__ix1870) as usize)].set_hh_rh(__v1871);
                                }
                                self.cur_list.tail_field = self.main_p;
                                self.ligature_present = false;
                            }
                        }
                        if self.ins_disc {
                            {
                                self.ins_disc = false;
                                if (self.cur_list.mode_field > 0i32) {
                                    {
                                        {
                                            let __ix1872 = self.cur_list.tail_field;
                                            let __v1873 = self.new_disc();
                                            self.mem[crate::ix::U((__ix1872) as usize)]
                                                .set_hh_rh(__v1873);
                                        }
                                        self.cur_list.tail_field = self.mem
                                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                                        .hh()
                                        .rh();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 5 {
                // L90
                // §1088
                if (self.lig_stack == (268435455i32).wrapping_neg()) {
                    // §1090
                    {
                        __goto_1 = 2;
                        continue 'l_dispatch_1;
                    }
                }
                self.cur_q = self.cur_list.tail_field;
                self.cur_l = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
            }
            if __goto_1 <= 6 {
                // L91
                if (!(self.lig_stack >= self.hi_mem_min)) {
                    {
                        __goto_1 = 13;
                        continue 'l_dispatch_1;
                    }
                }
            }
            if __goto_1 <= 7 {
                // L92
                if ((self.cur_chr < self.font_bc[crate::ix::U((self.main_f) as usize)])
                    || (self.cur_chr > self.font_ec[crate::ix::U((self.main_f) as usize)]))
                {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            {
                                let __ix1874 = self.lig_stack;
                                let __v1875 = self.avail;
                                self.mem[crate::ix::U((__ix1874) as usize)].set_hh_rh(__v1875);
                            }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        {
                            __goto_1 = 1;
                            continue 'l_dispatch_1;
                        }
                    }
                }
                self.main_i = self.font_info[crate::ix::U(
                    ((self.char_base[crate::ix::U((self.main_f) as usize)])
                        .wrapping_add(self.cur_l)) as usize,
                )]
                .qqqq();
                if (!(self.main_i.b0() > min_quarterword)) {
                    {
                        self.char_warning(self.main_f, self.cur_chr);
                        {
                            {
                                let __ix1876 = self.lig_stack;
                                let __v1877 = self.avail;
                                self.mem[crate::ix::U((__ix1876) as usize)].set_hh_rh(__v1877);
                            }
                            self.avail = self.lig_stack;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        {
                            __goto_1 = 1;
                            continue 'l_dispatch_1;
                        }
                    }
                }
                {
                    let __ix1878 = self.cur_list.tail_field;
                    let __v1879 = self.lig_stack;
                    self.mem[crate::ix::U((__ix1878) as usize)].set_hh_rh(__v1879);
                }
                self.cur_list.tail_field = self.lig_stack;
            }
            if __goto_1 <= 8 {
                // L100
                // §1088
                self.get_next();
                // §1092
                if (self.cur_cmd == letter) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd == other_char) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd == char_given) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                self.x_token();
                if (self.cur_cmd == letter) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd == other_char) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd == char_given) {
                    {
                        __goto_1 = 9;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd == char_num) {
                    {
                        self.scan_char_num();
                        self.cur_chr = self.cur_val;
                        {
                            __goto_1 = 9;
                            continue 'l_dispatch_1;
                        }
                    }
                }
                if (self.cur_cmd == no_boundary) {
                    self.bchar = non_char;
                }
                self.cur_r = self.bchar;
                self.lig_stack = (268435455i32).wrapping_neg();
                {
                    __goto_1 = 10;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 9 {
                // L101
                self.main_s = (self.eqtb
                    [crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)]
                .hh()
                .rh()
                    % 65536i32);
                if (self.main_s == 1000i32) {
                    self.cur_list.aux_field.set_hh_lh(1000i32);
                } else {
                    if (self.main_s < 1000i32) {
                        {
                            if (self.main_s > 0i32) {
                                {
                                    let __v1880 = self.main_s;
                                    self.cur_list.aux_field.set_hh_lh(__v1880);
                                }
                            }
                        }
                    } else {
                        if (self.cur_list.aux_field.hh().lh() < 1000i32) {
                            self.cur_list.aux_field.set_hh_lh(1000i32);
                        } else {
                            {
                                let __v1881 = self.main_s;
                                self.cur_list.aux_field.set_hh_lh(__v1881);
                            }
                        }
                    }
                }
                self.cur_ptr = (268435455i32).wrapping_neg();
                self.space_class = (self.eqtb
                    [crate::ix::U((((sf_code_base).wrapping_add(self.cur_chr)) - 1) as usize)]
                .hh()
                .rh()
                    / 65536i32);
                if ((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int() > 0i32)
                    && (self.space_class != char_class_ignored))
                {
                    {
                        if (self.prev_class == 4095i32) {
                            {
                                if ((self.cur_input.state_field != token_list)
                                    || (self.cur_input.index_field != backed_up_char))
                                {
                                    {
                                        self.find_sa_element(
                                            inter_char_val,
                                            ((4095i32).wrapping_mul(char_class_limit))
                                                .wrapping_add(self.space_class),
                                            false,
                                        );
                                        if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                            && (self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh()
                                                != (268435455i32).wrapping_neg()))
                                        {
                                            {
                                                if (self.cur_cmd != letter) {
                                                    self.cur_cmd = other_char;
                                                }
                                                self.cur_tok = ((self.cur_cmd)
                                                    .wrapping_mul(max_char_val))
                                                .wrapping_add(self.cur_chr);
                                                self.back_input();
                                                self.cur_input.index_field = backed_up_char;
                                                self.begin_token_list(
                                                    self.mem[crate::ix::U(
                                                        ((self.cur_ptr).wrapping_add(1i32))
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh(),
                                                    inter_char_text,
                                                );
                                                {
                                                    __goto_1 = 1;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                self.find_sa_element(
                                    inter_char_val,
                                    ((self.prev_class).wrapping_mul(char_class_limit))
                                        .wrapping_add(self.space_class),
                                    false,
                                );
                                if ((self.cur_ptr != (268435455i32).wrapping_neg())
                                    && (self.mem[crate::ix::U(
                                        ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                    )]
                                    .hh()
                                    .rh()
                                        != (268435455i32).wrapping_neg()))
                                {
                                    {
                                        if (self.cur_cmd != letter) {
                                            self.cur_cmd = other_char;
                                        }
                                        self.cur_tok = ((self.cur_cmd).wrapping_mul(max_char_val))
                                            .wrapping_add(self.cur_chr);
                                        self.back_input();
                                        self.cur_input.index_field = backed_up_char;
                                        self.begin_token_list(
                                            self.mem[crate::ix::U(
                                                ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                            )]
                                            .hh()
                                            .rh(),
                                            inter_char_text,
                                        );
                                        self.prev_class = 4095i32;
                                        {
                                            __goto_1 = 1;
                                            continue 'l_dispatch_1;
                                        }
                                    }
                                }
                            }
                        }
                        self.prev_class = self.space_class;
                    }
                }
                {
                    self.lig_stack = self.avail;
                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                        self.lig_stack = self.get_avail();
                    } else {
                        {
                            self.avail =
                                self.mem[crate::ix::U((self.lig_stack) as usize)].hh().rh();
                            {
                                let __ix1882 = self.lig_stack;
                                self.mem[crate::ix::U((__ix1882) as usize)]
                                    .set_hh_rh((268435455i32).wrapping_neg());
                            }
                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                        }
                    }
                }
                {
                    let __ix1883 = self.lig_stack;
                    let __v1884 = self.main_f;
                    self.mem[crate::ix::U((__ix1883) as usize)].set_hh_b0(__v1884);
                }
                self.cur_r = self.cur_chr;
                {
                    let __ix1885 = self.lig_stack;
                    let __v1886 = self.cur_r;
                    self.mem[crate::ix::U((__ix1885) as usize)].set_hh_b1(__v1886);
                }
                if (self.cur_r == self.false_bchar) {
                    self.cur_r = non_char;
                }
            }
            if __goto_1 <= 10 {
                // L110
                // §1088
                if ((self.main_i.b2() % 4i32) != lig_tag) {
                    // §1093
                    {
                        __goto_1 = 4;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_r == non_char) {
                    {
                        __goto_1 = 4;
                        continue 'l_dispatch_1;
                    }
                }
                self.main_k = (self.lig_kern_base[crate::ix::U((self.main_f) as usize)])
                    .wrapping_add(self.main_i.b3());
                self.main_j = self.font_info[crate::ix::U((self.main_k) as usize)].qqqq();
                if (self.main_j.b0() <= stop_flag) {
                    {
                        __goto_1 = 12;
                        continue 'l_dispatch_1;
                    }
                }
                self.main_k = ((((self.lig_kern_base[crate::ix::U((self.main_f) as usize)])
                    .wrapping_add((256i32).wrapping_mul(self.main_j.b2())))
                .wrapping_add(self.main_j.b3()))
                .wrapping_add(32768i32))
                .wrapping_sub((256i32).wrapping_mul(128i32));
            }
            if __goto_1 <= 11 {
                // L111
                self.main_j = self.font_info[crate::ix::U((self.main_k) as usize)].qqqq();
            }
            if __goto_1 <= 12 {
                // L112
                if (self.main_j.b1() == self.cur_r) {
                    if (self.main_j.b0() <= stop_flag) {
                        // §1094
                        {
                            if (self.main_j.b2() >= kern_flag) {
                                {
                                    if (self.cur_l < non_char) {
                                        {
                                            if (self.mem[crate::ix::U((self.cur_q) as usize)]
                                                .hh()
                                                .rh()
                                                > (268435455i32).wrapping_neg())
                                            {
                                                if (self.mem[crate::ix::U(
                                                    (self.cur_list.tail_field) as usize,
                                                )]
                                                .hh()
                                                .b1()
                                                    == self.hyphen_char
                                                        [crate::ix::U((self.main_f) as usize)])
                                                {
                                                    self.ins_disc = true;
                                                }
                                            }
                                            if self.ligature_present {
                                                {
                                                    self.main_p = self.new_ligature(
                                                        self.main_f,
                                                        self.cur_l,
                                                        self.mem
                                                            [crate::ix::U((self.cur_q) as usize)]
                                                        .hh()
                                                        .rh(),
                                                    );
                                                    if self.lft_hit {
                                                        {
                                                            {
                                                                let __ix1887 = self.main_p;
                                                                self.mem[crate::ix::U(
                                                                    (__ix1887) as usize,
                                                                )]
                                                                .set_hh_b1(2i32);
                                                            }
                                                            self.lft_hit = false;
                                                        }
                                                    }
                                                    if self.rt_hit {
                                                        if (self.lig_stack
                                                            == (268435455i32).wrapping_neg())
                                                        {
                                                            {
                                                                {
                                                                    let __ix1888 = self.main_p;
                                                                    let __v1889 = (self.mem
                                                                        [crate::ix::U(
                                                                            (self.main_p) as usize,
                                                                        )]
                                                                    .hh()
                                                                    .b1())
                                                                    .wrapping_add(1i32);
                                                                    self.mem[crate::ix::U(
                                                                        (__ix1888) as usize,
                                                                    )]
                                                                    .set_hh_b1(__v1889);
                                                                }
                                                                self.rt_hit = false;
                                                            }
                                                        }
                                                    }
                                                    {
                                                        let __ix1890 = self.cur_q;
                                                        let __v1891 = self.main_p;
                                                        self.mem[crate::ix::U((__ix1890) as usize)]
                                                            .set_hh_rh(__v1891);
                                                    }
                                                    self.cur_list.tail_field = self.main_p;
                                                    self.ligature_present = false;
                                                }
                                            }
                                            if self.ins_disc {
                                                {
                                                    self.ins_disc = false;
                                                    if (self.cur_list.mode_field > 0i32) {
                                                        {
                                                            {
                                                                let __ix1892 =
                                                                    self.cur_list.tail_field;
                                                                let __v1893 = self.new_disc();
                                                                self.mem[crate::ix::U(
                                                                    (__ix1892) as usize,
                                                                )]
                                                                .set_hh_rh(__v1893);
                                                            }
                                                            self.cur_list.tail_field = self.mem
                                                                [crate::ix::U(
                                                                    (self.cur_list.tail_field)
                                                                        as usize,
                                                                )]
                                                            .hh()
                                                            .rh();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        {
                                            let __ix1894 = self.cur_list.tail_field;
                                            let __v1895 = self.new_kern(
                                                self.font_info[crate::ix::U(
                                                    (((self.kern_base
                                                        [crate::ix::U((self.main_f) as usize)])
                                                    .wrapping_add(
                                                        (256i32).wrapping_mul(self.main_j.b2()),
                                                    ))
                                                    .wrapping_add(self.main_j.b3()))
                                                        as usize,
                                                )]
                                                .int(),
                                            );
                                            self.mem[crate::ix::U((__ix1894) as usize)]
                                                .set_hh_rh(__v1895);
                                        }
                                        self.cur_list.tail_field = self.mem
                                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                                        .hh()
                                        .rh();
                                    }
                                    {
                                        __goto_1 = 5;
                                        continue 'l_dispatch_1;
                                    }
                                }
                            }
                            if (self.cur_l == non_char) {
                                self.lft_hit = true;
                            } else {
                                if (self.lig_stack == (268435455i32).wrapping_neg()) {
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
                                    self.cur_l = self.main_j.b3();
                                    self.main_i = self.font_info[crate::ix::U(
                                        ((self.char_base[crate::ix::U((self.main_f) as usize)])
                                            .wrapping_add(self.cur_l))
                                            as usize,
                                    )]
                                    .qqqq();
                                    self.ligature_present = true;
                                }
                                2 | 6 => {
                                    self.cur_r = self.main_j.b3();
                                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                        {
                                            self.lig_stack = self.new_lig_item(self.cur_r);
                                            self.bchar = non_char;
                                        }
                                    } else {
                                        if (self.lig_stack >= self.hi_mem_min) {
                                            {
                                                self.main_p = self.lig_stack;
                                                self.lig_stack = self.new_lig_item(self.cur_r);
                                                {
                                                    let __ix1896 =
                                                        (self.lig_stack).wrapping_add(1i32);
                                                    let __v1897 = self.main_p;
                                                    self.mem[crate::ix::U((__ix1896) as usize)]
                                                        .set_hh_rh(__v1897);
                                                }
                                            }
                                        } else {
                                            {
                                                let __ix1898 = self.lig_stack;
                                                let __v1899 = self.cur_r;
                                                self.mem[crate::ix::U((__ix1898) as usize)]
                                                    .set_hh_b1(__v1899);
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    self.cur_r = self.main_j.b3();
                                    self.main_p = self.lig_stack;
                                    self.lig_stack = self.new_lig_item(self.cur_r);
                                    {
                                        let __ix1900 = self.lig_stack;
                                        let __v1901 = self.main_p;
                                        self.mem[crate::ix::U((__ix1900) as usize)]
                                            .set_hh_rh(__v1901);
                                    }
                                }
                                7 | 11 => {
                                    if (self.cur_l < non_char) {
                                        {
                                            if (self.mem[crate::ix::U((self.cur_q) as usize)]
                                                .hh()
                                                .rh()
                                                > (268435455i32).wrapping_neg())
                                            {
                                                if (self.mem[crate::ix::U(
                                                    (self.cur_list.tail_field) as usize,
                                                )]
                                                .hh()
                                                .b1()
                                                    == self.hyphen_char
                                                        [crate::ix::U((self.main_f) as usize)])
                                                {
                                                    self.ins_disc = true;
                                                }
                                            }
                                            if self.ligature_present {
                                                {
                                                    self.main_p = self.new_ligature(
                                                        self.main_f,
                                                        self.cur_l,
                                                        self.mem
                                                            [crate::ix::U((self.cur_q) as usize)]
                                                        .hh()
                                                        .rh(),
                                                    );
                                                    if self.lft_hit {
                                                        {
                                                            {
                                                                let __ix1902 = self.main_p;
                                                                self.mem[crate::ix::U(
                                                                    (__ix1902) as usize,
                                                                )]
                                                                .set_hh_b1(2i32);
                                                            }
                                                            self.lft_hit = false;
                                                        }
                                                    }
                                                    if false {
                                                        if (self.lig_stack
                                                            == (268435455i32).wrapping_neg())
                                                        {
                                                            {
                                                                {
                                                                    let __ix1903 = self.main_p;
                                                                    let __v1904 = (self.mem
                                                                        [crate::ix::U(
                                                                            (self.main_p) as usize,
                                                                        )]
                                                                    .hh()
                                                                    .b1())
                                                                    .wrapping_add(1i32);
                                                                    self.mem[crate::ix::U(
                                                                        (__ix1903) as usize,
                                                                    )]
                                                                    .set_hh_b1(__v1904);
                                                                }
                                                                self.rt_hit = false;
                                                            }
                                                        }
                                                    }
                                                    {
                                                        let __ix1905 = self.cur_q;
                                                        let __v1906 = self.main_p;
                                                        self.mem[crate::ix::U((__ix1905) as usize)]
                                                            .set_hh_rh(__v1906);
                                                    }
                                                    self.cur_list.tail_field = self.main_p;
                                                    self.ligature_present = false;
                                                }
                                            }
                                            if self.ins_disc {
                                                {
                                                    self.ins_disc = false;
                                                    if (self.cur_list.mode_field > 0i32) {
                                                        {
                                                            {
                                                                let __ix1907 =
                                                                    self.cur_list.tail_field;
                                                                let __v1908 = self.new_disc();
                                                                self.mem[crate::ix::U(
                                                                    (__ix1907) as usize,
                                                                )]
                                                                .set_hh_rh(__v1908);
                                                            }
                                                            self.cur_list.tail_field = self.mem
                                                                [crate::ix::U(
                                                                    (self.cur_list.tail_field)
                                                                        as usize,
                                                                )]
                                                            .hh()
                                                            .rh();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    self.cur_q = self.cur_list.tail_field;
                                    self.cur_l = self.main_j.b3();
                                    self.main_i = self.font_info[crate::ix::U(
                                        ((self.char_base[crate::ix::U((self.main_f) as usize)])
                                            .wrapping_add(self.cur_l))
                                            as usize,
                                    )]
                                    .qqqq();
                                    self.ligature_present = true;
                                }
                                _ => {
                                    self.cur_l = self.main_j.b3();
                                    self.ligature_present = true;
                                    if (self.lig_stack == (268435455i32).wrapping_neg()) {
                                        {
                                            __goto_1 = 4;
                                            continue 'l_dispatch_1;
                                        }
                                    } else {
                                        {
                                            __goto_1 = 6;
                                            continue 'l_dispatch_1;
                                        }
                                    }
                                }
                            }
                            if (self.main_j.b2() > 4i32) {
                                if (self.main_j.b2() != 7i32) {
                                    {
                                        __goto_1 = 4;
                                        continue 'l_dispatch_1;
                                    }
                                }
                            }
                            if (self.cur_l < non_char) {
                                {
                                    __goto_1 = 10;
                                    continue 'l_dispatch_1;
                                }
                            }
                            self.main_k = self.bchar_label[crate::ix::U((self.main_f) as usize)];
                            {
                                __goto_1 = 11;
                                continue 'l_dispatch_1;
                            }
                        }
                    }
                }
                // §1093
                if (self.main_j.b0() == 0i32) {
                    self.main_k = (self.main_k).wrapping_add(1i32);
                } else {
                    {
                        if (self.main_j.b0() >= stop_flag) {
                            {
                                __goto_1 = 4;
                                continue 'l_dispatch_1;
                            }
                        }
                        self.main_k =
                            ((self.main_k).wrapping_add(self.main_j.b0())).wrapping_add(1i32);
                    }
                }
                {
                    __goto_1 = 11;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 13 {
                // L95
                // §1088
                self.main_p = self.mem
                    [crate::ix::U(((self.lig_stack).wrapping_add(1i32)) as usize)]
                .hh()
                .rh();
                // §1091
                if (self.main_p > (268435455i32).wrapping_neg()) {
                    {
                        {
                            let __ix1909 = self.cur_list.tail_field;
                            let __v1910 = self.main_p;
                            self.mem[crate::ix::U((__ix1909) as usize)].set_hh_rh(__v1910);
                        }
                        self.cur_list.tail_field = self.mem
                            [crate::ix::U((self.cur_list.tail_field) as usize)]
                        .hh()
                        .rh();
                    }
                }
                self.temp_ptr = self.lig_stack;
                self.lig_stack = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                self.free_node(self.temp_ptr, small_node_size);
                self.main_i = self.font_info[crate::ix::U(
                    ((self.char_base[crate::ix::U((self.main_f) as usize)])
                        .wrapping_add(self.cur_l)) as usize,
                )]
                .qqqq();
                self.ligature_present = true;
                if (self.lig_stack == (268435455i32).wrapping_neg()) {
                    if (self.main_p > (268435455i32).wrapping_neg()) {
                        {
                            __goto_1 = 8;
                            continue 'l_dispatch_1;
                        }
                    } else {
                        self.cur_r = self.bchar;
                    }
                } else {
                    self.cur_r = self.mem[crate::ix::U((self.lig_stack) as usize)].hh().b1();
                }
                {
                    __goto_1 = 10;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 14 {
                // L120
                // §1084
                if (((self.eqtb[crate::ix::U(((7892343i32) - 1) as usize)].int() > 0i32)
                    && (self.space_class != char_class_ignored))
                    && (self.prev_class != 4095i32))
                {
                    {
                        self.prev_class = 4095i32;
                        self.find_sa_element(
                            inter_char_val,
                            ((self.space_class).wrapping_mul(char_class_limit))
                                .wrapping_add(4095i32),
                            false,
                        );
                        if ((self.cur_ptr != (268435455i32).wrapping_neg())
                            && (self.mem
                                [crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)]
                            .hh()
                            .rh()
                                != (268435455i32).wrapping_neg()))
                        {
                            {
                                if (self.cur_cs == 0i32) {
                                    {
                                        if (self.cur_cmd == char_num) {
                                            self.cur_cmd = other_char;
                                        }
                                        self.cur_tok = ((self.cur_cmd).wrapping_mul(max_char_val))
                                            .wrapping_add(self.cur_chr);
                                    }
                                } else {
                                    self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                                }
                                self.back_input();
                                self.begin_token_list(
                                    self.mem[crate::ix::U(
                                        ((self.cur_ptr).wrapping_add(1i32)) as usize,
                                    )]
                                    .hh()
                                    .rh(),
                                    inter_char_text,
                                );
                                {
                                    __goto_1 = 1;
                                    continue 'l_dispatch_1;
                                }
                            }
                        }
                    }
                }
                // §1095
                if (self.eqtb[crate::ix::U(((1205776i32) - 1) as usize)]
                    .hh()
                    .rh()
                    == zero_glue)
                {
                    {
                        // §1096
                        {
                            self.main_p = self.font_glue[crate::ix::U(
                                (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                                    .hh()
                                    .rh()) as usize,
                            )];
                            if (self.main_p == (268435455i32).wrapping_neg()) {
                                {
                                    self.main_p = self.new_spec(zero_glue);
                                    self.main_k = (self.param_base[crate::ix::U(
                                        (self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]
                                            .hh()
                                            .rh()) as usize,
                                    )])
                                    .wrapping_add(2i32);
                                    {
                                        let __ix1911 = (self.main_p).wrapping_add(1i32);
                                        let __v1912 = self.font_info
                                            [crate::ix::U((self.main_k) as usize)]
                                        .int();
                                        self.mem[crate::ix::U((__ix1911) as usize)]
                                            .set_int(__v1912);
                                    }
                                    {
                                        let __ix1913 = (self.main_p).wrapping_add(2i32);
                                        let __v1914 = self.font_info[crate::ix::U(
                                            ((self.main_k).wrapping_add(1i32)) as usize,
                                        )]
                                        .int();
                                        self.mem[crate::ix::U((__ix1913) as usize)]
                                            .set_int(__v1914);
                                    }
                                    {
                                        let __ix1915 = (self.main_p).wrapping_add(3i32);
                                        let __v1916 = self.font_info[crate::ix::U(
                                            ((self.main_k).wrapping_add(2i32)) as usize,
                                        )]
                                        .int();
                                        self.mem[crate::ix::U((__ix1915) as usize)]
                                            .set_int(__v1916);
                                    }
                                    {
                                        let __ix1917 = self.eqtb
                                            [crate::ix::U(((cur_font_loc) - 1) as usize)]
                                        .hh()
                                        .rh();
                                        let __v1918 = self.main_p;
                                        self.font_glue[crate::ix::U((__ix1917) as usize)] = __v1918;
                                    }
                                }
                            }
                        }
                        // §1095
                        self.temp_ptr = self.new_glue(self.main_p);
                    }
                } else {
                    self.temp_ptr = self.new_param_glue(space_skip_code);
                }
                {
                    let __ix1919 = self.cur_list.tail_field;
                    let __v1920 = self.temp_ptr;
                    self.mem[crate::ix::U((__ix1919) as usize)].set_hh_rh(__v1920);
                }
                self.cur_list.tail_field = self.temp_ptr;
                {
                    __goto_1 = 1;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 15 { // exit
                 // §1084
            }
            break 'l_dispatch_1;
        }
    }

    /// The `error` routine calls on `give_err_help` if help is requested from
    /// the `err_help` parameter.
    // §1338
    pub fn give_err_help(&mut self) {
        self.token_show(
            self.eqtb[crate::ix::U(((err_help_loc) - 1) as usize)]
                .hh()
                .rh(),
        );
    }

    /// Here is the only place we use `pack_buffered_name`. This part of the program
    /// becomes active when a ``virgin'' \TeX\ is trying to get going, just after
    /// the preliminary initialization, or when the user is substituting another
    /// format file by typing `\.\&' after the initial `\.{**}' prompt.  The buffer
    /// contains the first line of input in `buffer[loc..(last-1)]`, where
    /// `loc<last` and `buffer[loc]<>" "`.
    /// @<Declare the function called `open_fmt_file`
    // §559
    pub fn open_fmt_file(&mut self) -> bool {
        let mut open_fmt_file: bool = false;
        let mut j: i32 = 0; // §559
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
                        self.pack_buffered_name(
                            0i32,
                            self.cur_input.loc_field,
                            (j).wrapping_sub(1i32),
                        );
                        if {
                            let mut __f0 = ::core::mem::take(&mut self.fmt_file);
                            let __r = self.w_open_in(&mut __f0);
                            self.fmt_file = __f0;
                            __r
                        } {
                            break 'l_found_f;
                        }
                        {
                            crate::system::wr_str(
                                &mut self.term_out,
                                "Sorry, I can't find the format `",
                            );
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
                if (!{
                    let mut __f0 = ::core::mem::take(&mut self.fmt_file);
                    let __r = self.w_open_in(&mut __f0);
                    self.fmt_file = __f0;
                    __r
                }) {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.term_out,
                                "I can't find the format file `",
                            );
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
    // §1357
    pub fn load_fmt_file(&mut self) -> bool {
        let mut load_fmt_file: bool = false;
        let mut j: i32 = 0; // §1357
        let mut k: i32 = 0; // §1357
        let mut p: halfword = 0; // §1357
        let mut q: halfword = 0; // §1357
        let mut x: i32 = 0; // §1357
        let mut w: four_quarters = four_quarters::default(); // §1357
        'l_exit_f: {
            'l_L6666_f: {
                // §1362
                x = self.fmt_file.buf.int();
                if (x != 173681847i32) {
                    break 'l_L6666_f;
                }
                // §1465
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
                        // §1624
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 66953i32;
                    }
                } else {
                    // §1465
                    {
                        // §1623
                        self.max_reg_num = 255i32;
                        self.max_reg_help_line = 66952i32;
                    }
                }
                // §1362
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
                // §1364
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
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
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
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
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
                    k = too_big_char;
                    while k <= __for_end_4 {
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                x = self.fmt_file.buf.int();
                            }
                            if ((x < 0i32) || (x > self.pool_ptr)) {
                                break 'l_L6666_f;
                            } else {
                                self.str_start
                                    [crate::ix::U(((k).wrapping_sub(65536i32)) as usize)] = x;
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
                // §1366
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
                        let __for_end_5 = inter_char_val;
                        k = int_val;
                        while k <= __for_end_5 {
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < (268435455i32).wrapping_neg()) || (x > self.lo_mem_max)) {
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
                                {
                                    let __v1921 = self.fmt_file.buf;
                                    self.mem[crate::ix::U((k) as usize)] = __v1921;
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    p = (q).wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().lh());
                    if ((p > self.lo_mem_max)
                        || ((q
                            >= self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                                .hh()
                                .rh())
                            && (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                                .hh()
                                .rh()
                                != self.rover)))
                    {
                        break 'l_L6666_f;
                    }
                    q = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                        .hh()
                        .rh();
                    if (q == self.rover) {
                        break;
                    }
                }
                {
                    let __for_end_4 = self.lo_mem_max;
                    k = p;
                    while k <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            {
                                let __v1922 = self.fmt_file.buf;
                                self.mem[crate::ix::U((k) as usize)] = __v1922;
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                if (mem_min < (2i32).wrapping_neg()) {
                    {
                        p = self.mem[crate::ix::U(((self.rover).wrapping_add(1i32)) as usize)]
                            .hh()
                            .lh();
                        q = (mem_min).wrapping_add(1i32);
                        {
                            let __ix1923 = mem_min;
                            self.mem[crate::ix::U((__ix1923) as usize)]
                                .set_hh_rh((268435455i32).wrapping_neg());
                        }
                        {
                            let __ix1924 = mem_min;
                            self.mem[crate::ix::U((__ix1924) as usize)]
                                .set_hh_lh((268435455i32).wrapping_neg());
                        }
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(q);
                        {
                            let __ix1925 = (self.rover).wrapping_add(1i32);
                            self.mem[crate::ix::U((__ix1925) as usize)].set_hh_lh(q);
                        }
                        {
                            let __v1926 = self.rover;
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                                .set_hh_rh(__v1926);
                        }
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(empty_flag);
                        self.mem[crate::ix::U((q) as usize)]
                            .set_hh_lh(((0i32).wrapping_neg()).wrapping_sub(q));
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
                    if ((x < (268435455i32).wrapping_neg()) || (x > mem_top)) {
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
                            {
                                let __v1927 = self.fmt_file.buf;
                                self.mem[crate::ix::U((k) as usize)] = __v1927;
                            }
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
                // §1371
                k = active_base;
                loop {
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 1i32) || ((k).wrapping_add(x) > 9006999i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1928 = self.fmt_file.buf;
                                    self.eqtb[crate::ix::U(((j) - 1) as usize)] = __v1928;
                                }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    {
                        crate::system::get_word(&mut self.fmt_file);
                        x = self.fmt_file.buf.int();
                    }
                    if ((x < 0i32) || ((k).wrapping_add(x) > 9006999i32)) {
                        break 'l_L6666_f;
                    }
                    {
                        let __for_end_5 = ((k).wrapping_add(x)).wrapping_sub(1i32);
                        j = k;
                        while j <= __for_end_5 {
                            {
                                let __v1929 = self.eqtb
                                    [crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)];
                                self.eqtb[crate::ix::U(((j) - 1) as usize)] = __v1929;
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                    k = (k).wrapping_add(x);
                    if (k > eqtb_size) {
                        break;
                    }
                }
                {
                    let __for_end_4 = eqtb_top;
                    j = 9006999i32;
                    while j <= __for_end_4 {
                        {
                            let __v1930 = self.eqtb
                                [crate::ix::U(((undefined_control_sequence) - 1) as usize)];
                            self.eqtb[crate::ix::U(((j) - 1) as usize)] = __v1930;
                        }
                        j = j.wrapping_add(1);
                    }
                }
                if (self.hash_high > 0i32) {
                    {
                        let __for_end_5 = (eqtb_size).wrapping_add(self.hash_high);
                        j = 9006999i32;
                        while j <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1931 = self.fmt_file.buf;
                                    self.eqtb[crate::ix::U(((j) - 1) as usize)] = __v1931;
                                }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                }
                // §1368
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
                // §1373
                {
                    let __for_end_4 = prim_size;
                    p = 0i32;
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            {
                                let __v1932 = self.fmt_file.buf.hh();
                                self.prim[crate::ix::U((p) as usize)] = __v1932;
                            }
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
                p = 1179649i32;
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
                        {
                            let __v1933 = self.fmt_file.buf.hh();
                            self.hash[crate::ix::U(((p) - 1179650) as usize)] = __v1933;
                        }
                    }
                    if (p == self.hash_used) {
                        break;
                    }
                }
                {
                    let __for_end_4 = 1205762i32;
                    p = (self.hash_used).wrapping_add(1i32);
                    while p <= __for_end_4 {
                        {
                            crate::system::get_word(&mut self.fmt_file);
                            {
                                let __v1934 = self.fmt_file.buf.hh();
                                self.hash[crate::ix::U(((p) - 1179650) as usize)] = __v1934;
                            }
                        }
                        p = p.wrapping_add(1);
                    }
                }
                if (self.hash_high > 0i32) {
                    {
                        let __for_end_5 = (eqtb_size).wrapping_add(self.hash_high);
                        p = 9006999i32;
                        while p <= __for_end_5 {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1935 = self.fmt_file.buf.hh();
                                    self.hash[crate::ix::U(((p) - 1179650) as usize)] = __v1935;
                                }
                            }
                            p = p.wrapping_add(1);
                        }
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.cs_count = self.fmt_file.buf.int();
                }
                // §1375
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
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
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
                            {
                                let __v1936 = self.fmt_file.buf;
                                self.font_info[crate::ix::U((k) as usize)] = __v1936;
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
                    if (x < font_base) {
                        break 'l_L6666_f;
                    }
                    if (x > font_max) {
                        {
                            {
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
                                crate::system::wr_str(&mut self.term_out, "font max");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            break 'l_L6666_f;
                        }
                    } else {
                        self.font_ptr = x;
                    }
                }
                {
                    let __for_end_4 = self.font_ptr;
                    k = null_font;
                    while k <= __for_end_4 {
                        // §1377
                        {
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1937 = self.fmt_file.buf.qqqq();
                                    self.font_check[crate::ix::U((k) as usize)] = __v1937;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1938 = self.fmt_file.buf.int();
                                    self.font_size[crate::ix::U((k) as usize)] = __v1938;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1939 = self.fmt_file.buf.int();
                                    self.font_dsize[crate::ix::U((k) as usize)] = __v1939;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < (268435455i32).wrapping_neg()) || (x > max_halfword)) {
                                    break 'l_L6666_f;
                                } else {
                                    self.font_params[crate::ix::U((k) as usize)] = x;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1940 = self.fmt_file.buf.int();
                                    self.hyphen_char[crate::ix::U((k) as usize)] = __v1940;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1941 = self.fmt_file.buf.int();
                                    self.skew_char[crate::ix::U((k) as usize)] = __v1941;
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
                                {
                                    let __v1942 = self.fmt_file.buf.int();
                                    self.char_base[crate::ix::U((k) as usize)] = __v1942;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1943 = self.fmt_file.buf.int();
                                    self.width_base[crate::ix::U((k) as usize)] = __v1943;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1944 = self.fmt_file.buf.int();
                                    self.height_base[crate::ix::U((k) as usize)] = __v1944;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1945 = self.fmt_file.buf.int();
                                    self.depth_base[crate::ix::U((k) as usize)] = __v1945;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1946 = self.fmt_file.buf.int();
                                    self.italic_base[crate::ix::U((k) as usize)] = __v1946;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1947 = self.fmt_file.buf.int();
                                    self.lig_kern_base[crate::ix::U((k) as usize)] = __v1947;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1948 = self.fmt_file.buf.int();
                                    self.kern_base[crate::ix::U((k) as usize)] = __v1948;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1949 = self.fmt_file.buf.int();
                                    self.exten_base[crate::ix::U((k) as usize)] = __v1949;
                                }
                            }
                            {
                                crate::system::get_word(&mut self.fmt_file);
                                {
                                    let __v1950 = self.fmt_file.buf.int();
                                    self.param_base[crate::ix::U((k) as usize)] = __v1950;
                                }
                            }
                            {
                                {
                                    crate::system::get_word(&mut self.fmt_file);
                                    x = self.fmt_file.buf.int();
                                }
                                if ((x < (268435455i32).wrapping_neg()) || (x > self.lo_mem_max)) {
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
                // §1379
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
                                if ((x < (268435455i32).wrapping_neg()) || (x > max_halfword)) {
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
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
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
                            {
                                let __v1951 = self.fmt_file.buf.hh();
                                self.trie[crate::ix::U((k) as usize)] = __v1951;
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    crate::system::get_word(&mut self.fmt_file);
                    self.max_hyph_char = self.fmt_file.buf.int();
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
                                crate::system::wr_str(
                                    &mut self.term_out,
                                    "---! Must increase the ",
                                );
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
                    let __for_end_4 = biggest_lang;
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
                // §1381
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
                // §1357
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
    // §1387
    pub fn close_files_and_terminate(&mut self) {
        let mut k: i32 = 0; // §1387
                            // §1441
        self.terminate_font_manager();
        {
            let __for_end_2 = 15i32;
            k = 0i32;
            while k <= __for_end_2 {
                if self.write_open[crate::ix::U((k) as usize)] {
                    {
                        let mut __f0 =
                            ::core::mem::take(&mut self.write_file[crate::ix::U((k) as usize)]);
                        let __r = self.a_close(&mut __f0);
                        self.write_file[crate::ix::U((k) as usize)] = __f0;
                        __r
                    };
                }
                k = k.wrapping_add(1);
            }
        }
        // §1387
        self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].set_int((1i32).wrapping_neg());
        if (self.eqtb[crate::ix::U(((7892295i32) - 1) as usize)].int() > 0i32) {
            // §1388
            if self.log_opened {
                {
                    {
                        let __w0 = b' ';
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(
                            &mut self.log_file,
                            "Here is how much of TeX's memory",
                        );
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
                        let __w1 = ((((self.lo_mem_max).wrapping_sub(mem_min))
                            .wrapping_add(self.mem_end))
                        .wrapping_sub(self.hi_mem_min))
                        .wrapping_add(2i32);
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
                        crate::system::wr_str(
                            &mut self.log_file,
                            " multiletter control sequences out of ",
                        );
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
        // §680
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
            self.print_nl(66232i32);
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
                    self.last_bop =
                        ((self.dvi_offset).wrapping_add(self.dvi_ptr)).wrapping_sub(5i32);
                    self.dvi_four(25400000i32);
                    self.dvi_four(473628672i32);
                    self.prepare_mag();
                    self.dvi_four(self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                    self.dvi_four(self.max_v);
                    self.dvi_four(self.max_h);
                    {
                        {
                            let __ix1952 = self.dvi_ptr;
                            let __v1953 = (self.max_push / 256i32);
                            self.dvi_buf[crate::ix::U((__ix1952) as usize)] = __v1953;
                        }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        {
                            let __ix1954 = self.dvi_ptr;
                            let __v1955 = (self.max_push % 256i32);
                            self.dvi_buf[crate::ix::U((__ix1954) as usize)] = __v1955;
                        }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        {
                            let __ix1956 = self.dvi_ptr;
                            let __v1957 = ((self.total_pages / 256i32) % 256i32);
                            self.dvi_buf[crate::ix::U((__ix1956) as usize)] = __v1957;
                        }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        {
                            let __ix1958 = self.dvi_ptr;
                            let __v1959 = (self.total_pages % 256i32);
                            self.dvi_buf[crate::ix::U((__ix1958) as usize)] = __v1959;
                        }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    // §681
                    while (self.font_ptr > font_base) {
                        {
                            if self.font_used[crate::ix::U((self.font_ptr) as usize)] {
                                self.dvi_font_def(self.font_ptr);
                            }
                            self.font_ptr = (self.font_ptr).wrapping_sub(1i32);
                        }
                    }
                    // §680
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
                    // §635
                    if (self.dvi_limit == self.half_buf) {
                        self.write_dvi(self.half_buf, (dvi_buf_size).wrapping_sub(1i32));
                    }
                    if (self.dvi_ptr > (2147483647i32).wrapping_sub(self.dvi_offset)) {
                        {
                            self.cur_s = (2i32).wrapping_neg();
                            self.fatal_error(66219i32);
                        }
                    }
                    if (self.dvi_ptr > 0i32) {
                        self.write_dvi(0i32, (self.dvi_ptr).wrapping_sub(1i32));
                    }
                    // §680
                    k = {
                        let mut __f0 = ::core::mem::take(&mut self.dvi_file);
                        let __r = self.dvi_close(&mut __f0);
                        self.dvi_file = __f0;
                        __r
                    };
                    if (k == 0i32) {
                        {
                            self.print_nl(66233i32);
                            self.print(self.output_file_name);
                            self.print(65566i32);
                            self.print_int(self.total_pages);
                            if (self.total_pages != 1i32) {
                                self.print(66234i32);
                            } else {
                                self.print(66235i32);
                            }
                            if self.no_pdf_output {
                                {
                                    self.print(66236i32);
                                    self.print_int((self.dvi_offset).wrapping_add(self.dvi_ptr));
                                    self.print(66237i32);
                                }
                            } else {
                                self.print(66238i32);
                            }
                        }
                    } else {
                        {
                            self.print_nl(66239i32);
                            self.print_int(k);
                            self.print(65566i32);
                            if self.no_pdf_output {
                                self.print_strerror(k);
                            } else {
                                self.print(66240i32);
                            }
                            self.print(66241i32);
                            self.print_nl(66242i32);
                            self.print(self.output_file_name);
                            self.print(66243i32);
                            self.history = output_failure;
                        }
                    }
                }
            }
        }
        // §1387
        if self.log_opened {
            {
                {
                    crate::system::wr_ln(&mut self.log_file);
                }
                {
                    let mut __f0 = ::core::mem::take(&mut self.log_file);
                    let __r = self.a_close(&mut __f0);
                    self.log_file = __f0;
                    __r
                };
                self.selector = (self.selector).wrapping_sub(2i32);
                if (self.selector == term_only) {
                    {
                        self.print_nl(66726i32);
                        self.print(self.log_name);
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
    // §1389
    pub fn final_cleanup(&mut self) {
        let mut c: small_number = 0; // §1389
        'l_exit_f: {
            c = self.cur_chr;
            if (c != 1i32) {
                self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].set_int((1i32).wrapping_neg());
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
                    self.print(66727i32);
                    self.open_parens = (self.open_parens).wrapping_sub(1i32);
                }
            }
            if (self.cur_level > level_one) {
                {
                    self.print_nl(40i32);
                    self.print_esc(66728i32);
                    self.print(66729i32);
                    self.print_int((self.cur_level).wrapping_sub(1i32));
                    self.print_char(41i32);
                    if (self.eTeX_mode == 1i32) {
                        self.show_save_groups();
                    }
                }
            }
            while (self.cond_ptr != (268435455i32).wrapping_neg()) {
                {
                    self.print_nl(40i32);
                    self.print_esc(66728i32);
                    self.print(66730i32);
                    self.print_cmd_chr(if_test, self.cur_if);
                    if (self.if_line != 0i32) {
                        {
                            self.print(66731i32);
                            self.print_int(self.if_line);
                        }
                    }
                    self.print(66732i32);
                    self.if_line =
                        self.mem[crate::ix::U(((self.cond_ptr).wrapping_add(1i32)) as usize)].int();
                    self.cur_if = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().b1();
                    self.temp_ptr = self.cond_ptr;
                    self.cond_ptr = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
                    self.free_node(self.temp_ptr, if_node_size);
                }
            }
            if (self.history != spotless) {
                if ((self.history == warning_issued) || (self.interaction < error_stop_mode)) {
                    if (self.selector == term_and_log) {
                        {
                            self.selector = term_only;
                            self.print_nl(66733i32);
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
                            if (self.cur_mark[crate::ix::U((c) as usize)]
                                != (268435455i32).wrapping_neg())
                            {
                                self.delete_token_ref(self.cur_mark[crate::ix::U((c) as usize)]);
                            }
                            c = c.wrapping_add(1);
                        }
                    }
                    if (self.sa_root[crate::ix::U((mark_val) as usize)]
                        != (268435455i32).wrapping_neg())
                    {
                        if self.do_marks(
                            destroy_marks,
                            0i32,
                            self.sa_root[crate::ix::U((mark_val) as usize)],
                        ) {
                            self.sa_root[crate::ix::U((mark_val) as usize)] =
                                (268435455i32).wrapping_neg();
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
                    self.print_nl(66734i32);
                    break 'l_exit_f;
                }
            }
        }
    }

    /// @<Last-minute...
    // §1390
    pub fn init_prim(&mut self) {
        self.no_new_control_sequence = false;
        self.first = 0i32;
        // §252
        self.primitive(65667i32, assign_glue, 1205764i32);
        self.primitive(65668i32, assign_glue, 1205765i32);
        self.primitive(65669i32, assign_glue, 1205766i32);
        self.primitive(65670i32, assign_glue, 1205767i32);
        self.primitive(65671i32, assign_glue, 1205768i32);
        self.primitive(65672i32, assign_glue, 1205769i32);
        self.primitive(65673i32, assign_glue, 1205770i32);
        self.primitive(65674i32, assign_glue, 1205771i32);
        self.primitive(65675i32, assign_glue, 1205772i32);
        self.primitive(65676i32, assign_glue, 1205773i32);
        self.primitive(65677i32, assign_glue, 1205774i32);
        self.primitive(65678i32, assign_glue, 1205775i32);
        self.primitive(65679i32, assign_glue, 1205776i32);
        self.primitive(65680i32, assign_glue, 1205777i32);
        self.primitive(65681i32, assign_glue, 1205778i32);
        self.primitive(65682i32, assign_glue, 1205779i32);
        self.primitive(65683i32, assign_mu_glue, 1205780i32);
        self.primitive(65684i32, assign_mu_glue, 1205781i32);
        self.primitive(65685i32, assign_mu_glue, 1205782i32);
        // §256
        self.primitive(65690i32, assign_toks, output_routine_loc);
        self.primitive(65691i32, assign_toks, every_par_loc);
        self.primitive(65692i32, assign_toks, every_math_loc);
        self.primitive(65693i32, assign_toks, every_display_loc);
        self.primitive(65694i32, assign_toks, every_hbox_loc);
        self.primitive(65695i32, assign_toks, every_vbox_loc);
        self.primitive(65696i32, assign_toks, every_job_loc);
        self.primitive(65697i32, assign_toks, every_cr_loc);
        self.primitive(65698i32, assign_toks, err_help_loc);
        // §264
        self.primitive(65712i32, assign_int, 7892264i32);
        self.primitive(65713i32, assign_int, 7892265i32);
        self.primitive(65714i32, assign_int, 7892266i32);
        self.primitive(65715i32, assign_int, 7892267i32);
        self.primitive(65716i32, assign_int, 7892268i32);
        self.primitive(65717i32, assign_int, 7892269i32);
        self.primitive(65718i32, assign_int, 7892270i32);
        self.primitive(65719i32, assign_int, 7892271i32);
        self.primitive(65720i32, assign_int, 7892272i32);
        self.primitive(65721i32, assign_int, 7892273i32);
        self.primitive(65722i32, assign_int, 7892274i32);
        self.primitive(65723i32, assign_int, 7892275i32);
        self.primitive(65724i32, assign_int, 7892276i32);
        self.primitive(65725i32, assign_int, 7892277i32);
        self.primitive(65726i32, assign_int, 7892278i32);
        self.primitive(65727i32, assign_int, 7892279i32);
        self.primitive(65728i32, assign_int, 7892280i32);
        self.primitive(65729i32, assign_int, 7892281i32);
        self.primitive(65730i32, assign_int, 7892282i32);
        self.primitive(65731i32, assign_int, 7892283i32);
        self.primitive(65732i32, assign_int, 7892284i32);
        self.primitive(65733i32, assign_int, 7892285i32);
        self.primitive(65734i32, assign_int, 7892286i32);
        self.primitive(65735i32, assign_int, 7892287i32);
        self.primitive(65736i32, assign_int, 7892288i32);
        self.primitive(65737i32, assign_int, 7892289i32);
        self.primitive(65738i32, assign_int, 7892290i32);
        self.primitive(65739i32, assign_int, 7892291i32);
        self.primitive(65740i32, assign_int, 7892292i32);
        self.primitive(65741i32, assign_int, 7892293i32);
        self.primitive(65742i32, assign_int, 7892294i32);
        self.primitive(65743i32, assign_int, 7892295i32);
        self.primitive(65744i32, assign_int, 7892296i32);
        self.primitive(65745i32, assign_int, 7892297i32);
        self.primitive(65746i32, assign_int, 7892298i32);
        self.primitive(65747i32, assign_int, 7892299i32);
        self.primitive(65748i32, assign_int, 7892300i32);
        self.primitive(65749i32, assign_int, 7892301i32);
        self.primitive(65750i32, assign_int, 7892302i32);
        self.primitive(65751i32, assign_int, 7892303i32);
        self.primitive(65752i32, assign_int, 7892304i32);
        self.primitive(65753i32, assign_int, 7892305i32);
        self.primitive(65754i32, assign_int, 7892306i32);
        self.primitive(65755i32, assign_int, 7892307i32);
        self.primitive(65756i32, assign_int, 7892308i32);
        self.primitive(65757i32, assign_int, 7892309i32);
        self.primitive(65758i32, assign_int, 7892310i32);
        self.primitive(65759i32, assign_int, 7892311i32);
        self.primitive(65760i32, assign_int, 7892312i32);
        self.primitive(65761i32, assign_int, 7892313i32);
        self.primitive(65762i32, assign_int, 7892314i32);
        self.primitive(65763i32, assign_int, 7892315i32);
        self.primitive(65764i32, assign_int, 7892316i32);
        self.primitive(65765i32, assign_int, 7892317i32);
        self.primitive(65766i32, assign_int, 7892318i32);
        if self.mltex_p {
            {
                self.mltex_enabled_p = true;
                if false {
                    self.primitive(65767i32, assign_int, 7892319i32);
                }
                self.primitive(65768i32, assign_int, 7892320i32);
                self.primitive(65769i32, assign_int, 7892321i32);
            }
        }
        self.primitive(65770i32, assign_int, 7892322i32);
        self.primitive(65776i32, partoken_name, 0i32);
        self.primitive(65771i32, assign_int, 7892323i32);
        self.primitive(65772i32, assign_int, 7892324i32);
        self.primitive(65773i32, assign_int, 7892337i32);
        self.primitive(65774i32, assign_int, 7892338i32);
        // §274
        self.primitive(65779i32, assign_dimen, 9006720i32);
        self.primitive(65780i32, assign_dimen, 9006721i32);
        self.primitive(65781i32, assign_dimen, 9006722i32);
        self.primitive(65782i32, assign_dimen, 9006723i32);
        self.primitive(65783i32, assign_dimen, 9006724i32);
        self.primitive(65784i32, assign_dimen, 9006725i32);
        self.primitive(65785i32, assign_dimen, 9006726i32);
        self.primitive(65786i32, assign_dimen, 9006727i32);
        self.primitive(65787i32, assign_dimen, 9006728i32);
        self.primitive(65788i32, assign_dimen, 9006729i32);
        self.primitive(65789i32, assign_dimen, 9006730i32);
        self.primitive(65790i32, assign_dimen, 9006731i32);
        self.primitive(65791i32, assign_dimen, 9006732i32);
        self.primitive(65792i32, assign_dimen, 9006733i32);
        self.primitive(65793i32, assign_dimen, 9006734i32);
        self.primitive(65794i32, assign_dimen, 9006735i32);
        self.primitive(65795i32, assign_dimen, 9006736i32);
        self.primitive(65796i32, assign_dimen, 9006737i32);
        self.primitive(65797i32, assign_dimen, 9006738i32);
        self.primitive(65798i32, assign_dimen, 9006739i32);
        self.primitive(65799i32, assign_dimen, 9006740i32);
        self.primitive(65800i32, assign_dimen, 9006741i32);
        self.primitive(65801i32, assign_dimen, 9006742i32);
        // §295
        self.primitive(32i32, ex_space, 0i32);
        self.primitive(47i32, ital_corr, 0i32);
        self.primitive(65813i32, accent, 0i32);
        self.primitive(65814i32, advance, 0i32);
        self.primitive(65815i32, after_assignment, 0i32);
        self.primitive(65816i32, after_group, 0i32);
        self.primitive(65817i32, begin_group, 0i32);
        self.primitive(65818i32, char_num, 0i32);
        self.primitive(65809i32, cs_name, 0i32);
        self.primitive(65819i32, delim_num, 0i32);
        self.primitive(65820i32, delim_num, 1i32);
        self.primitive(65821i32, delim_num, 1i32);
        self.primitive(65822i32, divide, 0i32);
        self.primitive(65810i32, end_cs_name, 0i32);
        self.primitive(65823i32, end_group, 0i32);
        self.hash[crate::ix::U(((frozen_end_group) - 1179650) as usize)].set_rh(65823i32);
        {
            let __v1960 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_end_group) - 1) as usize)] = __v1960;
        }
        self.primitive(65824i32, expand_after, 0i32);
        self.primitive(65825i32, def_font, 0i32);
        self.primitive(65826i32, assign_font_dimen, 0i32);
        self.primitive(65827i32, halign, 0i32);
        self.primitive(65828i32, hrule, 0i32);
        self.primitive(65829i32, ignore_spaces, 0i32);
        self.primitive(65618i32, insert, 0i32);
        self.primitive(65641i32, mark, 0i32);
        self.primitive(65830i32, math_accent, 0i32);
        self.primitive(65831i32, math_accent, 1i32);
        self.primitive(65832i32, math_accent, 1i32);
        self.primitive(65833i32, math_char_num, 0i32);
        self.primitive(65834i32, math_char_num, 1i32);
        self.primitive(65835i32, math_char_num, 1i32);
        self.primitive(65836i32, math_char_num, 2i32);
        self.primitive(65837i32, math_char_num, 2i32);
        self.primitive(65838i32, math_choice, 0i32);
        self.primitive(65839i32, multiply, 0i32);
        self.primitive(65840i32, no_align, 0i32);
        self.primitive(65841i32, no_boundary, 0i32);
        self.primitive(65842i32, no_expand, 0i32);
        self.primitive(65806i32, no_expand, 1i32);
        self.primitive(65623i32, non_script, 0i32);
        self.primitive(65843i32, omit, 0i32);
        self.primitive(65844i32, set_shape, par_shape_loc);
        self.primitive(65845i32, break_penalty, 0i32);
        self.primitive(65846i32, set_prev_graf, 0i32);
        self.primitive(65847i32, radical, 0i32);
        self.primitive(65848i32, radical, 1i32);
        self.primitive(65849i32, radical, 1i32);
        self.primitive(65850i32, read_to_cs, 0i32);
        self.primitive(65851i32, relax, too_big_usv);
        self.hash[crate::ix::U(((frozen_relax) - 1179650) as usize)].set_rh(65851i32);
        {
            let __v1961 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_relax) - 1) as usize)] = __v1961;
        }
        self.primitive(65852i32, set_box, 0i32);
        self.primitive(65853i32, the, 0i32);
        self.primitive(65699i32, toks_register, mem_bot);
        self.primitive(65642i32, vadjust, 0i32);
        self.primitive(65854i32, valign, 0i32);
        self.primitive(65855i32, vcenter, 0i32);
        self.primitive(65856i32, vrule, 0i32);
        // §364
        self.primitive(65919i32, par_end, too_big_usv);
        self.par_loc = self.cur_val;
        self.par_token = (cs_token_flag).wrapping_add(self.par_loc);
        // §410
        self.primitive(65953i32, input, 0i32);
        self.primitive(65954i32, input, 1i32);
        // §418
        self.primitive(65955i32, top_bot_mark, top_mark_code);
        self.primitive(65956i32, top_bot_mark, first_mark_code);
        self.primitive(65957i32, top_bot_mark, bot_mark_code);
        self.primitive(65958i32, top_bot_mark, split_first_mark_code);
        self.primitive(65959i32, top_bot_mark, split_bot_mark_code);
        // §445
        self.primitive(65777i32, register, 0i32);
        self.primitive(65803i32, register, 1i32);
        self.primitive(65687i32, register, 2i32);
        self.primitive(65688i32, register, 3i32);
        // §450
        self.primitive(66003i32, set_aux, hmode);
        self.primitive(66004i32, set_aux, vmode);
        self.primitive(66005i32, set_page_int, 0i32);
        self.primitive(66006i32, set_page_int, 1i32);
        self.primitive(66007i32, set_box_dimen, width_offset);
        self.primitive(66008i32, set_box_dimen, height_offset);
        self.primitive(66009i32, set_box_dimen, depth_offset);
        self.primitive(66010i32, last_item, int_val);
        self.primitive(66011i32, last_item, dimen_val);
        self.primitive(66012i32, last_item, glue_val);
        self.primitive(66013i32, last_item, input_line_no_code);
        self.primitive(66014i32, last_item, badness_code);
        self.primitive(66015i32, last_item, pdf_last_x_pos_code);
        self.primitive(66016i32, last_item, pdf_last_y_pos_code);
        self.primitive(66017i32, last_item, elapsed_time_code);
        self.primitive(66018i32, last_item, pdf_shell_escape_code);
        self.primitive(66019i32, last_item, random_seed_code);
        // §503
        self.primitive(66087i32, convert, number_code);
        self.primitive(66088i32, convert, roman_numeral_code);
        self.primitive(66089i32, convert, string_code);
        self.primitive(66090i32, convert, meaning_code);
        self.primitive(66091i32, convert, font_name_code);
        self.primitive(66092i32, convert, expanded_code);
        self.primitive(66093i32, convert, left_margin_kern_code);
        self.primitive(66094i32, convert, right_margin_kern_code);
        self.primitive(66095i32, convert, pdf_creation_date_code);
        self.primitive(66096i32, convert, pdf_file_mod_date_code);
        self.primitive(66097i32, convert, pdf_file_size_code);
        self.primitive(66098i32, convert, pdf_mdfive_sum_code);
        self.primitive(66099i32, convert, pdf_file_dump_code);
        self.primitive(66100i32, convert, pdf_strcmp_code);
        self.primitive(66101i32, convert, uniform_deviate_code);
        self.primitive(66102i32, convert, normal_deviate_code);
        self.primitive(66103i32, convert, job_name_code);
        self.primitive(66104i32, convert, XeTeX_Uchar_code);
        self.primitive(66105i32, convert, XeTeX_Ucharcat_code);
        // §522
        self.primitive(66137i32, if_test, if_char_code);
        self.primitive(66138i32, if_test, if_cat_code);
        self.primitive(66139i32, if_test, if_int_code);
        self.primitive(66140i32, if_test, if_dim_code);
        self.primitive(66141i32, if_test, if_odd_code);
        self.primitive(66142i32, if_test, if_vmode_code);
        self.primitive(66143i32, if_test, if_hmode_code);
        self.primitive(66144i32, if_test, if_mmode_code);
        self.primitive(66145i32, if_test, if_inner_code);
        self.primitive(66146i32, if_test, if_void_code);
        self.primitive(66147i32, if_test, if_hbox_code);
        self.primitive(66148i32, if_test, if_vbox_code);
        self.primitive(66149i32, if_test, ifx_code);
        self.primitive(66150i32, if_test, if_eof_code);
        self.primitive(66151i32, if_test, if_true_code);
        self.primitive(66152i32, if_test, if_false_code);
        self.primitive(66153i32, if_test, if_case_code);
        self.primitive(66154i32, if_test, if_primitive_code);
        // §526
        self.primitive(66156i32, fi_or_else, fi_code);
        self.hash[crate::ix::U(((frozen_fi) - 1179650) as usize)].set_rh(66156i32);
        {
            let __v1962 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_fi) - 1) as usize)] = __v1962;
        }
        self.primitive(66157i32, fi_or_else, or_code);
        self.primitive(66158i32, fi_or_else, else_code);
        // §588
        self.primitive(66187i32, set_font, null_font);
        self.hash[crate::ix::U(((frozen_null_font) - 1179650) as usize)].set_rh(66187i32);
        {
            let __v1963 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_null_font) - 1) as usize)] = __v1963;
        }
        // §828
        self.primitive(66319i32, tab_mark, span_code);
        self.primitive(66320i32, car_ret, cr_code);
        self.hash[crate::ix::U(((frozen_cr) - 1179650) as usize)].set_rh(66320i32);
        {
            let __v1964 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_cr) - 1) as usize)] = __v1964;
        }
        self.primitive(66321i32, car_ret, cr_cr_code);
        self.hash[crate::ix::U(((frozen_end_template) - 1179650) as usize)].set_rh(66322i32);
        self.hash[crate::ix::U(((frozen_endv) - 1179650) as usize)].set_rh(66322i32);
        self.eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_b0(endv);
        self.eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_rh(null_list);
        self.eqtb[crate::ix::U(((frozen_endv) - 1) as usize)].set_hh_b1(level_one);
        {
            let __v1965 = self.eqtb[crate::ix::U(((frozen_endv) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_end_template) - 1) as usize)] = __v1965;
        }
        self.eqtb[crate::ix::U(((frozen_end_template) - 1) as usize)].set_hh_b0(end_template);
        // §1037
        self.primitive(66399i32, set_page_dimen, 0i32);
        self.primitive(66400i32, set_page_dimen, 1i32);
        self.primitive(66401i32, set_page_dimen, 2i32);
        self.primitive(66402i32, set_page_dimen, 3i32);
        self.primitive(66403i32, set_page_dimen, 4i32);
        self.primitive(66404i32, set_page_dimen, 5i32);
        self.primitive(66405i32, set_page_dimen, 6i32);
        self.primitive(66406i32, set_page_dimen, 7i32);
        // §1106
        self.primitive(65631i32, stop, 0i32);
        self.primitive(66453i32, stop, 1i32);
        // §1112
        self.primitive(66454i32, hskip, skip_code);
        self.primitive(66455i32, hskip, fil_code);
        self.primitive(66456i32, hskip, fill_code);
        self.primitive(66457i32, hskip, ss_code);
        self.primitive(66458i32, hskip, fil_neg_code);
        self.primitive(66459i32, vskip, skip_code);
        self.primitive(66460i32, vskip, fil_code);
        self.primitive(66461i32, vskip, fill_code);
        self.primitive(66462i32, vskip, ss_code);
        self.primitive(66463i32, vskip, fil_neg_code);
        self.primitive(65624i32, mskip, mskip_code);
        self.primitive(65603i32, kern, explicit);
        self.primitive(65630i32, mkern, mu_glue);
        // §1125
        self.primitive(66481i32, hmove, 1i32);
        self.primitive(66482i32, hmove, 0i32);
        self.primitive(66483i32, vmove, 1i32);
        self.primitive(66484i32, vmove, 0i32);
        self.primitive(65701i32, make_box, box_code);
        self.primitive(66485i32, make_box, copy_code);
        self.primitive(66486i32, make_box, last_box_code);
        self.primitive(66394i32, make_box, vsplit_code);
        self.primitive(66487i32, make_box, vtop_code);
        self.primitive(66396i32, make_box, 5i32);
        self.primitive(66488i32, make_box, 109i32);
        self.primitive(66489i32, leader_ship, 99i32);
        self.primitive(66490i32, leader_ship, a_leaders);
        self.primitive(66491i32, leader_ship, c_leaders);
        self.primitive(66492i32, leader_ship, x_leaders);
        // §1142
        self.primitive(66508i32, start_par, 1i32);
        self.primitive(66509i32, start_par, 0i32);
        // §1161
        self.primitive(66519i32, remove_item, penalty_node);
        self.primitive(66520i32, remove_item, kern_node);
        self.primitive(66521i32, remove_item, glue_node);
        self.primitive(66522i32, un_hbox, box_code);
        self.primitive(66523i32, un_hbox, copy_code);
        self.primitive(66524i32, un_vbox, box_code);
        self.primitive(66525i32, un_vbox, copy_code);
        // §1168
        self.primitive(45i32, discretionary, 1i32);
        self.primitive(65639i32, discretionary, 0i32);
        // §1195
        self.primitive(66556i32, eq_no, 0i32);
        self.primitive(66557i32, eq_no, 1i32);
        // §1210
        self.primitive(66269i32, math_comp, ord_noad);
        self.primitive(66270i32, math_comp, op_noad);
        self.primitive(66271i32, math_comp, bin_noad);
        self.primitive(66272i32, math_comp, rel_noad);
        self.primitive(66273i32, math_comp, open_noad);
        self.primitive(66274i32, math_comp, close_noad);
        self.primitive(66275i32, math_comp, punct_noad);
        self.primitive(66276i32, math_comp, inner_noad);
        self.primitive(66278i32, math_comp, under_noad);
        self.primitive(66277i32, math_comp, over_noad);
        self.primitive(66558i32, limit_switch, normal);
        self.primitive(66282i32, limit_switch, limits);
        self.primitive(66283i32, limit_switch, no_limits);
        // §1223
        self.primitive(66264i32, math_style, display_style);
        self.primitive(66265i32, math_style, text_style);
        self.primitive(66266i32, math_style, script_style);
        self.primitive(66267i32, math_style, script_script_style);
        // §1232
        self.primitive(66578i32, above, above_code);
        self.primitive(66579i32, above, over_code);
        self.primitive(66580i32, above, atop_code);
        self.primitive(66581i32, above, 3i32);
        self.primitive(66582i32, above, 4i32);
        self.primitive(66583i32, above, 5i32);
        // §1242
        self.primitive(66279i32, left_right, left_noad);
        self.primitive(66280i32, left_right, right_noad);
        self.hash[crate::ix::U(((frozen_right) - 1179650) as usize)].set_rh(66280i32);
        {
            let __v1966 = self.eqtb[crate::ix::U(((self.cur_val) - 1) as usize)];
            self.eqtb[crate::ix::U(((frozen_right) - 1) as usize)] = __v1966;
        }
        // §1262
        self.primitive(66603i32, prefix, 1i32);
        self.primitive(66604i32, prefix, 2i32);
        self.primitive(66605i32, prefix, 4i32);
        self.primitive(66606i32, def, 0i32);
        self.primitive(66607i32, def, 1i32);
        self.primitive(66608i32, def, 2i32);
        self.primitive(66609i32, def, 3i32);
        // §1273
        self.primitive(66626i32, let_, normal);
        self.primitive(66627i32, let_, 1i32);
        // §1276
        self.primitive(66628i32, shorthand_def, char_def_code);
        self.primitive(66629i32, shorthand_def, math_char_def_code);
        self.primitive(66630i32, shorthand_def, XeTeX_math_char_num_def_code);
        self.primitive(66631i32, shorthand_def, XeTeX_math_char_num_def_code);
        self.primitive(66632i32, shorthand_def, XeTeX_math_char_def_code);
        self.primitive(66633i32, shorthand_def, XeTeX_math_char_def_code);
        self.primitive(66634i32, shorthand_def, count_def_code);
        self.primitive(66635i32, shorthand_def, dimen_def_code);
        self.primitive(66636i32, shorthand_def, skip_def_code);
        self.primitive(66637i32, shorthand_def, mu_skip_def_code);
        self.primitive(66638i32, shorthand_def, toks_def_code);
        // §1284
        self.primitive(65707i32, def_code, cat_code_base);
        self.primitive(65711i32, def_code, math_code_base);
        self.primitive(66641i32, XeTeX_def_code, math_code_base);
        self.primitive(66642i32, XeTeX_def_code, math_code_base);
        self.primitive(66643i32, XeTeX_def_code, 5664041i32);
        self.primitive(66644i32, XeTeX_def_code, 5664041i32);
        self.primitive(65708i32, def_code, lc_code_base);
        self.primitive(65709i32, def_code, uc_code_base);
        self.primitive(65710i32, def_code, sf_code_base);
        self.primitive(66645i32, XeTeX_def_code, sf_code_base);
        self.primitive(65778i32, def_code, del_code_base);
        self.primitive(66646i32, XeTeX_def_code, del_code_base);
        self.primitive(66647i32, XeTeX_def_code, del_code_base);
        self.primitive(66648i32, XeTeX_def_code, 7892609i32);
        self.primitive(66649i32, XeTeX_def_code, 7892609i32);
        self.primitive(65704i32, def_family, math_font_base);
        self.primitive(65705i32, def_family, 1207080i32);
        self.primitive(65706i32, def_family, 1207336i32);
        // §1304
        self.primitive(66370i32, hyph_data, 0i32);
        self.primitive(66382i32, hyph_data, 1i32);
        // §1308
        self.primitive(66664i32, assign_font_int, 0i32);
        self.primitive(66665i32, assign_font_int, 1i32);
        self.primitive(66666i32, assign_font_int, lp_code_base);
        self.primitive(66667i32, assign_font_int, rp_code_base);
        // §1316
        self.primitive(65554i32, set_interaction, batch_mode);
        self.primitive(65555i32, set_interaction, nonstop_mode);
        self.primitive(65556i32, set_interaction, scroll_mode);
        self.primitive(66676i32, set_interaction, error_stop_mode);
        // §1326
        self.primitive(66677i32, in_stream, 1i32);
        self.primitive(66678i32, in_stream, 0i32);
        // §1331
        self.primitive(66679i32, message, 0i32);
        self.primitive(66680i32, message, 1i32);
        // §1340
        self.primitive(66686i32, case_shift, lc_code_base);
        self.primitive(66687i32, case_shift, uc_code_base);
        // §1345
        self.primitive(66688i32, xray, show_code);
        self.primitive(66689i32, xray, show_box_code);
        self.primitive(66690i32, xray, show_the_code);
        self.primitive(66691i32, xray, show_lists_code);
        // §1398
        self.primitive(66736i32, extension, open_node);
        self.primitive(65915i32, extension, write_node);
        self.write_loc = self.cur_val;
        self.primitive(66737i32, extension, close_node);
        self.primitive(66738i32, extension, special_node);
        self.primitive(66739i32, extension, immediate_code);
        self.primitive(66740i32, extension, set_language_code);
        self.primitive(66741i32, extension, reset_timer_code);
        self.primitive(66742i32, extension, set_random_seed_code);
        // §1703
        self.primitive(66962i32, assign_int, 7892351i32);
        // §1390
        self.no_new_control_sequence = true;
    }
}
