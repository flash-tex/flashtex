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
    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn pdf_set_text_pos(&mut self, mut v: scaled, mut v_out: scaled, mut f: internal_font_number) {
        let mut pdf_new_Tm_a: i32 = 0; // §692
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
        pdf_new_Tm_a = self.get_font_auto_expand_ratio(f);
        if ((pdf_new_Tm_a != 0i32) || ((pdf_new_Tm_a == 0i32) && (self.pdf_cur_Tm_a != 0i32))) {
            {
                self.pdf_print_real((1000i32).wrapping_add(pdf_new_Tm_a), 3i32);
                self.pdf_print(1035i32);
                self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
                self.pdf_h = (self.pdf_origin_h).wrapping_add(self.scaled_out);
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
                self.pdf_print_bp((self.pdf_origin_v).wrapping_sub(self.cur_v));
                self.pdf_v = (self.pdf_origin_v).wrapping_sub(self.scaled_out);
                self.pdf_print(1036i32);
                self.pdf_cur_Tm_a = pdf_new_Tm_a;
                self.pdfassert((self.pdf_cur_Tm_a > (1000i32).wrapping_neg()));
            }
        } else {
            {
                self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_tj_start_h));
                self.pdf_h = (self.pdf_tj_start_h).wrapping_add(self.scaled_out);
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
                self.pdf_print_real(v, self.fixed_decimal_digits);
                self.pdf_v = (self.pdf_v).wrapping_sub(v_out);
                self.pdf_print(1037i32);
            }
        }
        self.pdf_tj_start_h = self.pdf_h;
        self.pdf_delta_h = 0i32;
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn pdf_use_font(&mut self, mut f: internal_font_number, mut fontnum: i32) {
        {
            if (self.divide_scaled(self.font_size[(f) as usize], self.one_hundred_bp, 6i32) != 0i32) {
            }
        }
        { let __v547 = self.scaled_out; self.pdf_font_size[(f) as usize] = __v547; }
        { let __v548 = true; self.font_used[(f) as usize] = __v548; }
        self.pdfassert(((fontnum > 0i32) || ((fontnum < 0i32) && (self.pdf_font_num[((fontnum).wrapping_neg()) as usize] > 0i32))));
        self.pdf_font_num[(f) as usize] = fontnum;
        if (self.eqtb[((29082i32) - 1) as usize].int() > 0i32) {
            {
                self.pdf_warning(0i32, 1038i32, true, true);
                self.eqtb[((29082i32) - 1) as usize].set_int(0i32);
            }
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_init_font(&mut self, mut f: internal_font_number) {
        let mut k: internal_font_number = 0; // §693
        let mut b: internal_font_number = 0; // §693
        let mut i: i32 = 0; // §693
        self.pdfassert((!self.font_used[(f) as usize]));
        if (self.pdf_font_auto_expand[(f) as usize] && (self.pdf_font_blink[(f) as usize] != 0i32)) {
            {
                b = self.pdf_font_blink[(f) as usize];
                if (!self.isscalable(b)) {
                    self.pdf_error(1039i32, 1040i32);
                }
                if (!self.font_used[(b) as usize]) {
                    self.pdf_init_font(b);
                }
                { let __v549 = self.pdf_font_map[(b) as usize]; self.pdf_font_map[(f) as usize] = __v549; }
            }
        }
        if self.isscalable(f) {
            {
                i = self.head_tab[((3i32) - 1) as usize];
                while (i != 0i32) {
                    {
                        k = self.obj_tab[(i) as usize].int0;
                        if ((self.isscalable(k) && (self.pdf_font_map[(k) as usize] == self.pdf_font_map[(f) as usize])) && (self.str_eq_str(self.font_name[(k) as usize], self.font_name[(f) as usize]) || ((self.pdf_font_auto_expand[(f) as usize] && (self.pdf_font_blink[(f) as usize] != 0i32)) && self.str_eq_str(self.font_name[(k) as usize], self.font_name[(self.pdf_font_blink[(f) as usize]) as usize])))) {
                            {
                                self.pdfassert((self.pdf_font_num[(k) as usize] != 0i32));
                                if (self.pdf_font_num[(k) as usize] < 0i32) {
                                    self.pdf_use_font(f, self.pdf_font_num[(k) as usize]);
                                } else {
                                    self.pdf_use_font(f, (k).wrapping_neg());
                                }
                                return;
                            }
                        }
                        i = self.obj_tab[(i) as usize].int1;
                    }
                }
            }
        }
        self.pdf_create_obj(3i32, f);
        { let __v550 = (self.hasspacechar(f) && (self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(32i32)) as usize].qqqq().b0())) as usize].int() > self.one_bp)); self.pdf_font_has_space_char[(f) as usize] = __v550; }
        self.pdf_use_font(f, self.obj_ptr);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_init_font_cur_val(&mut self) {
        self.pdf_init_font(self.cur_val);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_set_font(&mut self, mut f: internal_font_number) {
        let mut p: halfword = 0; // §693
        let mut k: internal_font_number = 0; // §693
        'l_found_f: {
            self.pdf_f = f;
            if (!self.font_used[(f) as usize]) {
                self.pdf_init_font(f);
            }
            {
                if (self.pdf_font_num[(f) as usize] < 0i32) {
                    self.ff = (self.pdf_font_num[(f) as usize]).wrapping_neg();
                } else {
                    self.ff = f;
                }
            }
            k = self.ff;
            p = self.pdf_font_list;
            while (p != 0i32) {
                {
                    {
                        if (self.pdf_font_num[(self.mem[(p) as usize].hh().lh()) as usize] < 0i32) {
                            self.ff = (self.pdf_font_num[(self.mem[(p) as usize].hh().lh()) as usize]).wrapping_neg();
                        } else {
                            self.ff = self.mem[(p) as usize].hh().lh();
                        }
                    }
                    if (self.ff == k) {
                        break 'l_found_f;
                    }
                    p = self.mem[(p) as usize].hh().rh();
                }
            }
            {
                self.pdf_append_list_arg = f;
                self.pdf_font_list = self.append_ptr(self.pdf_font_list, self.pdf_append_list_arg);
            }
        }
        if ((k == self.pdf_last_f) && (self.font_size[(f) as usize] == self.pdf_last_fs)) {
            return;
        }
        self.pdf_print(1041i32);
        self.pdf_print_int(((k) as i64));
        if (self.pdf_resname_prefix != 0i32) {
            self.pdf_print(self.pdf_resname_prefix);
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
                self.pdf_buf_set(self.pdf_ptr, 32i32);
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
        { let __a551_0 = self.divide_scaled(self.font_size[(f) as usize], self.one_hundred_bp, 6i32); let __a551_1 = 4i32; self.pdf_print_real(__a551_0, __a551_1) };
        self.pdf_print(1042i32);
        self.pdf_last_f = k;
        self.pdf_last_fs = self.font_size[(f) as usize];
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_begin_text(&mut self) {
        self.pdf_set_origin(0i32, self.cur_page_height);
        {
            self.pdf_print(1043i32);
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
        self.pdf_doing_text = true;
        self.pdf_f = 0i32;
        self.pdf_last_f = 0i32;
        self.pdf_last_fs = 0i32;
        self.pdf_doing_string = false;
        self.pdf_cur_Tm_a = 0i32;
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_read_dummy_font(&mut self) {
        if (self.pdf_dummy_font == 0i32) {
            {
                self.pdf_dummy_font = self.read_font_info(513i32, self.pdf_space_font_name, 348i32, (1000i32).wrapping_neg());
                self.pdfmaplinesp();
                self.pdf_mark_char(self.pdf_dummy_font, 32i32);
            }
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_insert_interword_space(&mut self) {
        self.pdf_read_dummy_font();
        self.pdf_set_font(self.pdf_dummy_font);
        self.pdf_print(1044i32);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_begin_string(&mut self, mut f: internal_font_number) {
        let mut s_out: scaled = 0; // §693
        let mut v: scaled = 0; // §693
        let mut v_out: scaled = 0; // §693
        let mut s: i32 = 0; // §693
        let mut must_end_string: bool = false; // §693
        let mut must_insert_space: bool = false; // §693
        let mut must_set_text_pos: bool = false; // §693
        let mut move_looks_like_interword_space: bool = false; // §693
        must_end_string = false;
        must_insert_space = false;
        must_set_text_pos = false;
        if (!self.pdf_doing_text) {
            {
                self.pdf_begin_text();
                must_set_text_pos = true;
            }
        }
        if (((!self.gen_faked_interword_space) && (self.pdf_f != f)) || (self.gen_faked_interword_space && (!(self.font_used[(f) as usize] && self.font_used[(self.pdf_f) as usize])))) {
            {
                self.pdf_end_string();
                self.pdf_set_font(f);
            }
        }
        if (self.pdf_cur_Tm_a == 0i32) {
            {
                s = self.divide_scaled((self.cur_h).wrapping_sub((self.pdf_tj_start_h).wrapping_add(self.pdf_delta_h)), self.pdf_font_size[(f) as usize], 3i32);
                s_out = self.scaled_out;
            }
        } else {
            {
                s = { let __a552_0 = self.round_xn_over_d((self.cur_h).wrapping_sub((self.pdf_tj_start_h).wrapping_add(self.pdf_delta_h)), 1000i32, (1000i32).wrapping_add(self.pdf_cur_Tm_a)); let __a552_1 = self.pdf_font_size[(f) as usize]; let __a552_2 = 3i32; self.divide_scaled(__a552_0, __a552_1, __a552_2) };
                if ((s).wrapping_abs() < 32768i32) {
                    {
                        s_out = { let __a553_0 = self.round_xn_over_d(self.pdf_font_size[(f) as usize], (s).wrapping_abs(), 1000i32); let __a553_1 = (1000i32).wrapping_add(self.pdf_cur_Tm_a); let __a553_2 = 1000i32; self.round_xn_over_d(__a553_0, __a553_1, __a553_2) };
                        if (s < 0i32) {
                            s_out = (s_out).wrapping_neg();
                        }
                    }
                }
            }
        }
        if (((self.cur_v).wrapping_sub(self.pdf_v)).wrapping_abs() >= self.min_bp_val) {
            {
                v = self.divide_scaled((self.pdf_v).wrapping_sub(self.cur_v), self.one_hundred_bp, (self.fixed_decimal_digits).wrapping_add(2i32));
                v_out = self.scaled_out;
            }
        } else {
            {
                v = 0i32;
                v_out = 0i32;
            }
        }
        if (!must_set_text_pos) {
            {
                must_set_text_pos = ((((v != 0i32) || ((s).wrapping_abs() >= 32768i32)) || (self.get_font_auto_expand_ratio(f) != self.pdf_cur_Tm_a)) || (self.get_font_auto_expand_ratio(f) != self.get_font_auto_expand_ratio(self.pdf_f)));
            }
        }
        if must_set_text_pos {
            {
                must_end_string = true;
            }
        }
        move_looks_like_interword_space = (((self.font_info[((2i32).wrapping_add(self.param_base[(f) as usize])) as usize].int() > self.one_bp) && (s_out > ((self.font_info[((2i32).wrapping_add(self.param_base[(f) as usize])) as usize].int()).wrapping_sub(self.font_info[((4i32).wrapping_add(self.param_base[(f) as usize])) as usize].int())).wrapping_sub((self.one_bp / 10i32)))) && (v == 0i32));
        if (self.gen_faked_interword_space && move_looks_like_interword_space) {
            {
                must_insert_space = true;
            }
        }
        if must_insert_space {
            {
                if (self.pdf_font_has_space_char[(f) as usize] && self.pdf_doing_string) {
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
                                self.pdf_buf_set(self.pdf_ptr, 32i32);
                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                            }
                        }
                        self.adv_char_width(f, 32i32);
                        s = (s).wrapping_sub(self.adv_char_width_s);
                        s_out = (s_out).wrapping_sub(self.adv_char_width_s_out);
                        self.pdf_mark_char(f, 32i32);
                    }
                } else {
                    must_end_string = true;
                }
            }
        }
        if must_end_string {
            {
                self.pdf_end_string();
                if (must_insert_space && (!self.pdf_font_has_space_char[(f) as usize])) {
                    {
                        self.pdf_insert_interword_space();
                    }
                }
                self.pdf_set_font(f);
                self.pdf_set_text_pos(v, v_out, f);
                s = 0i32;
            }
        }
        if (self.gen_faked_interword_space && (self.pdf_f != f)) {
            {
                self.pdf_end_string();
                self.pdf_set_font(f);
            }
        }
        if (!self.pdf_doing_string) {
            {
                self.pdf_print(1045i32);
                if (s == 0i32) {
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
                            self.pdf_buf_set(self.pdf_ptr, 40i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        if (s != 0i32) {
            {
                if self.pdf_doing_string {
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
                            self.pdf_buf_set(self.pdf_ptr, 41i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
                self.pdf_print_int((((s).wrapping_neg()) as i64));
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
                        self.pdf_buf_set(self.pdf_ptr, 40i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_delta_h = (self.pdf_delta_h).wrapping_add(s_out);
            }
        }
        self.pdf_doing_string = true;
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_insert_fake_space(&mut self) {
        let mut s: i32 = 0; // §693
        s = ((self.gen_faked_interword_space) as i32);
        self.gen_faked_interword_space = ((0i32) != 0);
        self.pdf_read_dummy_font();
        self.pdf_begin_string(self.pdf_dummy_font);
        self.pdf_print(32i32);
        self.adv_char_width(self.pdf_dummy_font, 32i32);
        self.pdf_end_string_nl();
        self.gen_faked_interword_space = ((s) != 0);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_end_text(&mut self) {
        if self.pdf_doing_text {
            {
                self.pdf_end_string_nl();
                {
                    self.pdf_print(1046i32);
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
                self.pdf_doing_text = false;
            }
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_set_rule(&mut self, mut x: scaled, mut y: scaled, mut w: scaled, mut h: scaled) {
        self.pdf_end_text();
        {
            self.pdf_print(113i32);
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
        if (h <= self.one_bp) {
            {
                self.pdf_set_origin_temp(x, (((((y) as f64) - ((((h).wrapping_add(1i32)) as f64) / ((2i32) as f64)))) as i32));
                self.pdf_print(1047i32);
                self.pdf_print_bp(h);
                self.pdf_print(1048i32);
                self.pdf_print_bp(w);
                {
                    self.pdf_print(1049i32);
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
            if (w <= self.one_bp) {
                {
                    self.pdf_set_origin_temp((((((x) as f64) + ((((w).wrapping_add(1i32)) as f64) / ((2i32) as f64)))) as i32), y);
                    self.pdf_print(1047i32);
                    self.pdf_print_bp(w);
                    self.pdf_print(1050i32);
                    self.pdf_print_bp(h);
                    {
                        self.pdf_print(1051i32);
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
                    self.pdf_set_origin_temp(x, y);
                    self.pdf_print(1052i32);
                    self.pdf_print_bp(w);
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
                    self.pdf_print_bp(h);
                    {
                        self.pdf_print(1053i32);
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
        {
            self.pdf_print(81i32);
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

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn pdf_rectangle(&mut self, mut left: scaled, mut top: scaled, mut right: scaled, mut bottom: scaled) {
        self.prepare_mag();
        self.pdf_print(1054i32);
        self.pdf_print_mag_bp((left).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(bottom));
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
        self.pdf_print_mag_bp((right).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(top));
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
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn slow_print_substr(&mut self, mut s: i32, mut max_len: i32) {
        let mut j: pool_pointer = 0; // §693
        if ((s >= self.str_ptr) || (s < 256i32)) {
            self.print(s);
        } else {
            {
                j = self.str_start[(s) as usize];
                while ((j < self.str_start[((s).wrapping_add(1i32)) as usize]) && (j <= (self.str_start[(s) as usize]).wrapping_add(max_len))) {
                    {
                        self.print(self.str_pool[(j) as usize]);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
            self.print(279i32);
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §693
    pub fn literal(&mut self, mut s: str_number, mut literal_mode: i32, mut warn: bool) {
        let mut j: pool_pointer = 0; // §693
        j = self.str_start[(s) as usize];
        if (literal_mode == 3i32) {
            {
                if (!(self.str_in_str(s, 1055i32, 0i32) || self.str_in_str(s, 1056i32, 0i32))) {
                    {
                        if (warn && (!((self.str_in_str(s, 1057i32, 0i32) || self.str_in_str(s, 1058i32, 0i32)) || ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]) == 0i32)))) {
                            {
                                self.print_nl(1059i32);
                                self.print_nl(1060i32);
                                self.slow_print_substr(s, 64i32);
                                self.print_ln();
                            }
                        }
                        return;
                    }
                }
                j = (j).wrapping_add((self.str_start[(1056i32) as usize]).wrapping_sub(self.str_start[(1055i32) as usize]));
                if self.str_in_str(s, 1061i32, (self.str_start[(1056i32) as usize]).wrapping_sub(self.str_start[(1055i32) as usize])) {
                    {
                        j = (j).wrapping_add((self.str_start[(1062i32) as usize]).wrapping_sub(self.str_start[(1061i32) as usize]));
                        literal_mode = 2i32;
                    }
                } else {
                    if self.str_in_str(s, 1062i32, (self.str_start[(1056i32) as usize]).wrapping_sub(self.str_start[(1055i32) as usize])) {
                        {
                            j = (j).wrapping_add((self.str_start[(1063i32) as usize]).wrapping_sub(self.str_start[(1062i32) as usize]));
                            literal_mode = 1i32;
                        }
                    } else {
                        literal_mode = 0i32;
                    }
                }
            }
        }
        match literal_mode {
            0 => {
                {
                    self.pdf_end_text();
                    self.pdf_set_origin(self.cur_h, self.cur_v);
                }
            }
            1 => {
                self.pdf_end_text();
            }
            2 => {
                self.pdf_end_string_nl();
            }
            _ => {
                self.confusion(1063i32);
            }
        }
        while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
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
                        self.pdf_buf_set(self.pdf_ptr, self.str_pool[(j) as usize]);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                j = (j).wrapping_add(1i32);
            }
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
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_print_fw_int(&mut self, mut n: longinteger, mut w: i32) {
        let mut k: i32 = 0; // §702
        k = 0i32;
        loop {
            self.dig[(k) as usize] = (((n % ((10i32) as i64))) as i32);
            n = (n / ((10i32) as i64));
            k = (k).wrapping_add(1i32);
            if (k == w) { break; }
        }
        {
            if (self.pdf_os_mode && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                self.pdf_os_get_os_buf(k);
            } else {
                if ((!self.pdf_os_mode) && (k > self.pdf_buf_size)) {
                    self.overflow(1004i32, pdf_op_buf_size);
                } else {
                    if ((!self.pdf_os_mode) && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_flush();
                    }
                }
            }
        }
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                {
                    self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add(self.dig[(k) as usize]));
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_out_bytes(&mut self, mut n: longinteger, mut w: i32) {
        let mut k: i32 = 0; // §702
        let mut byte: Vec<i32> = vec![0; 8]; // §702
        k = 0i32;
        loop {
            byte[(k) as usize] = (((n % ((256i32) as i64))) as i32);
            n = (n / ((256i32) as i64));
            k = (k).wrapping_add(1i32);
            if (k == w) { break; }
        }
        {
            if (self.pdf_os_mode && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                self.pdf_os_get_os_buf(k);
            } else {
                if ((!self.pdf_os_mode) && (k > self.pdf_buf_size)) {
                    self.overflow(1004i32, pdf_op_buf_size);
                } else {
                    if ((!self.pdf_os_mode) && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_flush();
                    }
                }
            }
        }
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                {
                    self.pdf_buf_set(self.pdf_ptr, byte[(k) as usize]);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_int_entry(&mut self, mut s: str_number, mut v: i32) {
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
                self.pdf_buf_set(self.pdf_ptr, 47i32);
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
        self.pdf_print(s);
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
        self.pdf_print_int(((v) as i64));
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_int_entry_ln(&mut self, mut s: str_number, mut v: i32) {
        self.pdf_int_entry(s, v);
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

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_indirect(&mut self, mut s: str_number, mut o: i32) {
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
                self.pdf_buf_set(self.pdf_ptr, 47i32);
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
        self.pdf_print(s);
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
        self.pdf_print_int(((o) as i64));
        self.pdf_print(1073i32);
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_indirect_ln(&mut self, mut s: str_number, mut o: i32) {
        self.pdf_indirect(s, o);
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

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_print_str(&mut self, mut s: str_number) {
        let mut i: pool_pointer = 0; // §702
        let mut j: pool_pointer = 0; // §702
        let mut is_hex_string: bool = false; // §702
        'l_done_f: {
            i = self.str_start[(s) as usize];
            j = ((i).wrapping_add((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]))).wrapping_sub(1i32);
            if (i > j) {
                {
                    self.pdf_print(1074i32);
                    return;
                }
            }
            if ((self.str_pool[(i) as usize] == ((b'(') as i32)) && (self.str_pool[(j) as usize] == ((b')') as i32))) {
                {
                    self.pdf_print(s);
                    return;
                }
            }
            is_hex_string = false;
            if (((self.str_pool[(i) as usize] != ((b'<') as i32)) || (self.str_pool[(j) as usize] != ((b'>') as i32))) || ((((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) % 2) != 0)) {
                break 'l_done_f;
            }
            i = (i).wrapping_add(1i32);
            j = (j).wrapping_sub(1i32);
            while (i < j) {
                {
                    if (((((self.str_pool[(i) as usize] >= ((b'0') as i32)) && (self.str_pool[(i) as usize] <= ((b'9') as i32))) || ((self.str_pool[(i) as usize] >= ((b'A') as i32)) && (self.str_pool[(i) as usize] <= ((b'F') as i32)))) || ((self.str_pool[(i) as usize] >= ((b'a') as i32)) && (self.str_pool[(i) as usize] <= ((b'f') as i32)))) && ((((self.str_pool[((i).wrapping_add(1i32)) as usize] >= ((b'0') as i32)) && (self.str_pool[((i).wrapping_add(1i32)) as usize] <= ((b'9') as i32))) || ((self.str_pool[((i).wrapping_add(1i32)) as usize] >= ((b'A') as i32)) && (self.str_pool[((i).wrapping_add(1i32)) as usize] <= ((b'F') as i32)))) || ((self.str_pool[((i).wrapping_add(1i32)) as usize] >= ((b'a') as i32)) && (self.str_pool[((i).wrapping_add(1i32)) as usize] <= ((b'f') as i32))))) {
                        i = (i).wrapping_add(2i32);
                    } else {
                        break 'l_done_f;
                    }
                }
            }
            is_hex_string = true;
        }
        if is_hex_string {
            self.pdf_print(s);
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
                        self.pdf_buf_set(self.pdf_ptr, 40i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print(s);
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
                        self.pdf_buf_set(self.pdf_ptr, 41i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_print_str_ln(&mut self, mut s: str_number) {
        self.pdf_print_str(s);
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

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_str_entry(&mut self, mut s: str_number, mut v: str_number) {
        if (v == 0i32) {
            return;
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
                self.pdf_buf_set(self.pdf_ptr, 47i32);
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
        self.pdf_print(s);
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
        self.pdf_print_str(v);
    }

    /// Subroutines to print out various PDF objects:
    // §702
    pub fn pdf_str_entry_ln(&mut self, mut s: str_number, mut v: str_number) {
        if (v == 0i32) {
            return;
        }
        self.pdf_str_entry(s, v);
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

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_tag_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        let mut fixedi: i32 = 0; // §705
        if (((self.font_bc[(f) as usize] <= c) && (c <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b0() > 0i32)) {
            {
                fixedi = (self.fix_int(i, (7i32).wrapping_neg(), 0i32)).wrapping_abs();
                if (fixedi >= 4i32) {
                    {
                        if (((self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(0i32) % 4i32) == 3i32) {
                            { let __ix554 = (self.char_base[(f) as usize]).wrapping_add(c); let __v555 = (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(3i32); self.font_info[(__ix554) as usize].set_qqqq_b2(__v555); }
                        }
                        fixedi = (fixedi).wrapping_sub(4i32);
                    }
                }
                if (fixedi >= 2i32) {
                    {
                        if (((self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(0i32) % 4i32) == 2i32) {
                            { let __ix556 = (self.char_base[(f) as usize]).wrapping_add(c); let __v557 = (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(2i32); self.font_info[(__ix556) as usize].set_qqqq_b2(__v557); }
                        }
                        fixedi = (fixedi).wrapping_sub(2i32);
                    }
                }
                if (fixedi >= 1i32) {
                    {
                        if (((self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(0i32) % 4i32) == 1i32) {
                            { let __ix558 = (self.char_base[(f) as usize]).wrapping_add(c); let __v559 = (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(1i32); self.font_info[(__ix558) as usize].set_qqqq_b2(__v559); }
                        }
                    }
                }
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_no_ligatures(&mut self, mut f: internal_font_number) {
        let mut c: i32 = 0; // §705
        {
            let __for_end_2 = self.font_ec[(f) as usize];
            c = self.font_bc[(f) as usize];
            while c <= __for_end_2 {
                if (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b0() > 0i32) {
                    if (((self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(0i32) % 4i32) == 1i32) {
                        { let __ix560 = (self.char_base[(f) as usize]).wrapping_add(c); let __v561 = (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b2()).wrapping_sub(1i32); self.font_info[(__ix560) as usize].set_qqqq_b2(__v561); }
                    }
                }
                c = c.wrapping_add(1);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn init_font_base(&mut self, mut v: i32) -> i32 {
        let mut init_font_base: i32 = 0;
        let mut i: i32 = 0; // §705
        let mut j: i32 = 0; // §705
        i = self.pdf_get_mem(256i32);
        {
            let __for_end_2 = 255i32;
            j = 0i32;
            while j <= __for_end_2 {
                self.pdf_mem[((i).wrapping_add(j)) as usize] = v;
                j = j.wrapping_add(1);
            }
        }
        init_font_base = i;
        init_font_base
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_lp_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_lp_base[(f) as usize] == 0i32) {
            { let __v562 = self.init_font_base(0i32); self.pdf_font_lp_base[(f) as usize] = __v562; }
        }
        { let __ix563 = (self.pdf_font_lp_base[(f) as usize]).wrapping_add(c); let __v564 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix563) as usize] = __v564; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_rp_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_rp_base[(f) as usize] == 0i32) {
            { let __v565 = self.init_font_base(0i32); self.pdf_font_rp_base[(f) as usize] = __v565; }
        }
        { let __ix566 = (self.pdf_font_rp_base[(f) as usize]).wrapping_add(c); let __v567 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix566) as usize] = __v567; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_ef_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_ef_base[(f) as usize] == 0i32) {
            { let __v568 = self.init_font_base(1000i32); self.pdf_font_ef_base[(f) as usize] = __v568; }
        }
        { let __ix569 = (self.pdf_font_ef_base[(f) as usize]).wrapping_add(c); let __v570 = self.fix_int(i, 0i32, 1000i32); self.pdf_mem[(__ix569) as usize] = __v570; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_kn_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_bs_base[(f) as usize] == 0i32) {
            { let __v571 = self.init_font_base(0i32); self.pdf_font_kn_bs_base[(f) as usize] = __v571; }
        }
        { let __ix572 = (self.pdf_font_kn_bs_base[(f) as usize]).wrapping_add(c); let __v573 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix572) as usize] = __v573; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_st_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_st_bs_base[(f) as usize] == 0i32) {
            { let __v574 = self.init_font_base(0i32); self.pdf_font_st_bs_base[(f) as usize] = __v574; }
        }
        { let __ix575 = (self.pdf_font_st_bs_base[(f) as usize]).wrapping_add(c); let __v576 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix575) as usize] = __v576; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_sh_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_sh_bs_base[(f) as usize] == 0i32) {
            { let __v577 = self.init_font_base(0i32); self.pdf_font_sh_bs_base[(f) as usize] = __v577; }
        }
        { let __ix578 = (self.pdf_font_sh_bs_base[(f) as usize]).wrapping_add(c); let __v579 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix578) as usize] = __v579; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_kn_bc_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_bc_base[(f) as usize] == 0i32) {
            { let __v580 = self.init_font_base(0i32); self.pdf_font_kn_bc_base[(f) as usize] = __v580; }
        }
        { let __ix581 = (self.pdf_font_kn_bc_base[(f) as usize]).wrapping_add(c); let __v582 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix581) as usize] = __v582; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_kn_ac_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_ac_base[(f) as usize] == 0i32) {
            { let __v583 = self.init_font_base(0i32); self.pdf_font_kn_ac_base[(f) as usize] = __v583; }
        }
        { let __ix584 = (self.pdf_font_kn_ac_base[(f) as usize]).wrapping_add(c); let __v585 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[(__ix584) as usize] = __v585; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn adjust_interword_glue(&mut self, mut p: halfword, mut g: halfword) {
        let mut kn: scaled = 0; // §705
        let mut st: scaled = 0; // §705
        let mut sh: scaled = 0; // §705
        let mut q: halfword = 0; // §705
        let mut r: halfword = 0; // §705
        let mut c: halfword = 0; // §705
        let mut f: internal_font_number = 0; // §705
        if (!((!(g >= self.hi_mem_min)) && (self.mem[(g) as usize].hh().b0() == 10i32))) {
            {
                self.pdf_warning(1076i32, 1077i32, true, true);
                return;
            }
        }
        c = 256i32;
        if (p >= self.hi_mem_min) {
            {
                c = self.mem[(p) as usize].hh().b1();
                f = self.mem[(p) as usize].hh().b0();
            }
        } else {
            if (self.mem[(p) as usize].hh().b0() == 6i32) {
                {
                    c = self.mem[((p).wrapping_add(1i32)) as usize].hh().b1();
                    f = self.mem[((p).wrapping_add(1i32)) as usize].hh().b0();
                }
            } else {
                if (((self.mem[(p) as usize].hh().b0() == 11i32) && (self.mem[(p) as usize].hh().b1() == 3i32)) && (self.save_tail != 0i32)) {
                    {
                        r = self.save_tail;
                        while ((self.mem[(r) as usize].hh().rh() != 0i32) && (self.mem[(r) as usize].hh().rh() != p)) {
                            r = self.mem[(r) as usize].hh().rh();
                        }
                        if (self.mem[(r) as usize].hh().rh() == p) {
                            if (r >= self.hi_mem_min) {
                                {
                                    c = self.mem[(r) as usize].hh().b1();
                                    f = self.mem[(r) as usize].hh().b0();
                                }
                            } else {
                                if (self.mem[(r) as usize].hh().b0() == 6i32) {
                                    {
                                        c = self.mem[((r).wrapping_add(1i32)) as usize].hh().b1();
                                        f = self.mem[((r).wrapping_add(1i32)) as usize].hh().b0();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if (c == 256i32) {
            return;
        }
        kn = self.get_kn_bs_code(f, c);
        st = self.get_st_bs_code(f, c);
        sh = self.get_sh_bs_code(f, c);
        if (((kn != 0i32) || (st != 0i32)) || (sh != 0i32)) {
            {
                q = self.new_spec(self.mem[((g).wrapping_add(1i32)) as usize].hh().lh());
                self.delete_glue_ref(self.mem[((g).wrapping_add(1i32)) as usize].hh().lh());
                { let __v586 = (self.mem[((q).wrapping_add(1i32)) as usize].int()).wrapping_add(self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), kn, 1000i32)); self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v586); }
                { let __v587 = (self.mem[((q).wrapping_add(2i32)) as usize].int()).wrapping_add(self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), st, 1000i32)); self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v587); }
                { let __v588 = (self.mem[((q).wrapping_add(3i32)) as usize].int()).wrapping_add(self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), sh, 1000i32)); self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v588); }
                self.mem[((g).wrapping_add(1i32)) as usize].set_hh_lh(q);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn get_auto_kern(&mut self, mut f: internal_font_number, mut l: halfword, mut r: halfword) -> halfword {
        let mut get_auto_kern: halfword = 0;
        let mut tmp_w: scaled = 0; // §705
        let mut k: i32 = 0; // §705
        let mut p: halfword = 0; // §705
        self.pdfassert(((l >= 0i32) && (r >= 0i32)));
        get_auto_kern = 0i32;
        if ((self.eqtb[((29103i32) - 1) as usize].int() <= 0i32) && (self.eqtb[((29102i32) - 1) as usize].int() <= 0i32)) {
            return get_auto_kern;
        }
        tmp_w = 0i32;
        if ((self.eqtb[((29103i32) - 1) as usize].int() > 0i32) && (l < 256i32)) {
            {
                k = self.get_kn_ac_code(f, l);
                if (k != 0i32) {
                    tmp_w = self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), k, 1000i32);
                }
            }
        }
        if ((self.eqtb[((29102i32) - 1) as usize].int() > 0i32) && (r < 256i32)) {
            {
                k = self.get_kn_bc_code(f, r);
                if (k != 0i32) {
                    tmp_w = (tmp_w).wrapping_add(self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), k, 1000i32));
                }
            }
        }
        if (tmp_w != 0i32) {
            {
                p = self.new_kern(tmp_w);
                self.mem[(p) as usize].set_hh_b1(3i32);
                get_auto_kern = p;
            }
        }
        get_auto_kern
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn expand_font_name(&mut self, mut f: internal_font_number, mut e: i32) -> str_number {
        let mut expand_font_name: str_number = 0;
        let mut old_setting: i32 = 0; // §705
        old_setting = self.selector;
        self.selector = 21i32;
        self.print(self.font_name[(f) as usize]);
        if (e > 0i32) {
            self.print(43i32);
        }
        self.print_int(((e) as i64));
        self.selector = old_setting;
        expand_font_name = self.make_string();
        expand_font_name
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn auto_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut auto_expand_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §705
        let mut nw: i32 = 0; // §705
        let mut nk: i32 = 0; // §705
        let mut ni: i32 = 0; // §705
        let mut i: i32 = 0; // §705
        k = (self.font_ptr).wrapping_add(1i32);
        self.font_ptr = (self.font_ptr).wrapping_add(1i32);
        if (self.font_ptr >= font_max) {
            self.overflow(1078i32, font_max);
        }
        { let __v589 = self.expand_font_name(f, e); self.font_name[(k) as usize] = __v589; }
        { let __v590 = self.font_area[(f) as usize]; self.font_area[(k) as usize] = __v590; }
        { let __v591 = self.hash[(((17626i32).wrapping_add(f)) - 514) as usize].rh(); self.hash[(((17626i32).wrapping_add(k)) - 514) as usize].set_rh(__v591); }
        { let __v592 = self.hyphen_char[(f) as usize]; self.hyphen_char[(k) as usize] = __v592; }
        { let __v593 = self.skew_char[(f) as usize]; self.skew_char[(k) as usize] = __v593; }
        { let __v594 = self.font_bchar[(f) as usize]; self.font_bchar[(k) as usize] = __v594; }
        { let __v595 = self.font_false_bchar[(f) as usize]; self.font_false_bchar[(k) as usize] = __v595; }
        { let __v596 = self.font_bc[(f) as usize]; self.font_bc[(k) as usize] = __v596; }
        { let __v597 = self.font_ec[(f) as usize]; self.font_ec[(k) as usize] = __v597; }
        { let __v598 = self.font_size[(f) as usize]; self.font_size[(k) as usize] = __v598; }
        { let __v599 = self.font_dsize[(f) as usize]; self.font_dsize[(k) as usize] = __v599; }
        { let __v600 = self.font_params[(f) as usize]; self.font_params[(k) as usize] = __v600; }
        { let __v601 = self.font_glue[(f) as usize]; self.font_glue[(k) as usize] = __v601; }
        { let __v602 = self.bchar_label[(f) as usize]; self.bchar_label[(k) as usize] = __v602; }
        { let __v603 = self.char_base[(f) as usize]; self.char_base[(k) as usize] = __v603; }
        { let __v604 = self.height_base[(f) as usize]; self.height_base[(k) as usize] = __v604; }
        { let __v605 = self.depth_base[(f) as usize]; self.depth_base[(k) as usize] = __v605; }
        { let __v606 = self.lig_kern_base[(f) as usize]; self.lig_kern_base[(k) as usize] = __v606; }
        { let __v607 = self.exten_base[(f) as usize]; self.exten_base[(k) as usize] = __v607; }
        { let __v608 = self.param_base[(f) as usize]; self.param_base[(k) as usize] = __v608; }
        nw = (self.height_base[(f) as usize]).wrapping_sub(self.width_base[(f) as usize]);
        ni = (self.lig_kern_base[(f) as usize]).wrapping_sub(self.italic_base[(f) as usize]);
        nk = (self.exten_base[(f) as usize]).wrapping_sub((self.kern_base[(f) as usize]).wrapping_add((256i32).wrapping_mul(128i32)));
        if ((((self.fmem_ptr).wrapping_add(nw)).wrapping_add(ni)).wrapping_add(nk) >= font_mem_size) {
            self.overflow(1079i32, font_mem_size);
        }
        { let __v609 = self.fmem_ptr; self.width_base[(k) as usize] = __v609; }
        { let __v610 = (self.width_base[(k) as usize]).wrapping_add(nw); self.italic_base[(k) as usize] = __v610; }
        { let __v611 = ((self.italic_base[(k) as usize]).wrapping_add(ni)).wrapping_sub((256i32).wrapping_mul(128i32)); self.kern_base[(k) as usize] = __v611; }
        self.fmem_ptr = (((self.fmem_ptr).wrapping_add(nw)).wrapping_add(ni)).wrapping_add(nk);
        {
            let __for_end_2 = (nw).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix612 = (self.width_base[(k) as usize]).wrapping_add(i); let __v613 = self.round_xn_over_d(self.font_info[((self.width_base[(f) as usize]).wrapping_add(i)) as usize].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[(__ix612) as usize].set_int(__v613); }
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (ni).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix614 = (self.italic_base[(k) as usize]).wrapping_add(i); let __v615 = self.round_xn_over_d(self.font_info[((self.italic_base[(f) as usize]).wrapping_add(i)) as usize].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[(__ix614) as usize].set_int(__v615); }
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (nk).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix616 = ((self.kern_base[(k) as usize]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(i); let __v617 = self.round_xn_over_d(self.font_info[(((self.kern_base[(f) as usize]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(i)) as usize].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[(__ix616) as usize].set_int(__v617); }
                i = i.wrapping_add(1);
            }
        }
        auto_expand_font = k;
        auto_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn copy_expand_params(&mut self, mut k: internal_font_number, mut f: internal_font_number, mut e: i32) {
        if (self.pdf_font_rp_base[(f) as usize] == 0i32) {
            { let __v618 = self.init_font_base(0i32); self.pdf_font_rp_base[(f) as usize] = __v618; }
        }
        if (self.pdf_font_lp_base[(f) as usize] == 0i32) {
            { let __v619 = self.init_font_base(0i32); self.pdf_font_lp_base[(f) as usize] = __v619; }
        }
        if (self.pdf_font_ef_base[(f) as usize] == 0i32) {
            { let __v620 = self.init_font_base(1000i32); self.pdf_font_ef_base[(f) as usize] = __v620; }
        }
        self.pdf_font_expand_ratio[(k) as usize] = e;
        { let __v621 = self.pdf_font_step[(f) as usize]; self.pdf_font_step[(k) as usize] = __v621; }
        { let __v622 = self.pdf_font_auto_expand[(f) as usize]; self.pdf_font_auto_expand[(k) as usize] = __v622; }
        self.pdf_font_blink[(k) as usize] = f;
        { let __v623 = self.pdf_font_lp_base[(f) as usize]; self.pdf_font_lp_base[(k) as usize] = __v623; }
        { let __v624 = self.pdf_font_rp_base[(f) as usize]; self.pdf_font_rp_base[(k) as usize] = __v624; }
        { let __v625 = self.pdf_font_ef_base[(f) as usize]; self.pdf_font_ef_base[(k) as usize] = __v625; }
        if (self.pdf_font_kn_bs_base[(f) as usize] == 0i32) {
            { let __v626 = self.init_font_base(0i32); self.pdf_font_kn_bs_base[(f) as usize] = __v626; }
        }
        if (self.pdf_font_st_bs_base[(f) as usize] == 0i32) {
            { let __v627 = self.init_font_base(0i32); self.pdf_font_st_bs_base[(f) as usize] = __v627; }
        }
        if (self.pdf_font_sh_bs_base[(f) as usize] == 0i32) {
            { let __v628 = self.init_font_base(0i32); self.pdf_font_sh_bs_base[(f) as usize] = __v628; }
        }
        if (self.pdf_font_kn_bc_base[(f) as usize] == 0i32) {
            { let __v629 = self.init_font_base(0i32); self.pdf_font_kn_bc_base[(f) as usize] = __v629; }
        }
        if (self.pdf_font_kn_ac_base[(f) as usize] == 0i32) {
            { let __v630 = self.init_font_base(0i32); self.pdf_font_kn_ac_base[(f) as usize] = __v630; }
        }
        { let __v631 = self.pdf_font_kn_bs_base[(f) as usize]; self.pdf_font_kn_bs_base[(k) as usize] = __v631; }
        { let __v632 = self.pdf_font_st_bs_base[(f) as usize]; self.pdf_font_st_bs_base[(k) as usize] = __v632; }
        { let __v633 = self.pdf_font_sh_bs_base[(f) as usize]; self.pdf_font_sh_bs_base[(k) as usize] = __v633; }
        { let __v634 = self.pdf_font_kn_bc_base[(f) as usize]; self.pdf_font_kn_bc_base[(k) as usize] = __v634; }
        { let __v635 = self.pdf_font_kn_ac_base[(f) as usize]; self.pdf_font_kn_ac_base[(k) as usize] = __v635; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn tfm_lookup(&mut self, mut s: str_number, mut fs: scaled) -> internal_font_number {
        let mut tfm_lookup: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §705
        if (fs != 0i32) {
            {
                {
                    let __for_end_4 = self.font_ptr;
                    k = 1i32;
                    while k <= __for_end_4 {
                        if (((self.font_area[(k) as usize] != 1075i32) && self.str_eq_str(self.font_name[(k) as usize], s)) && (self.font_size[(k) as usize] == fs)) {
                            {
                                self.flush_str(s);
                                tfm_lookup = k;
                                return tfm_lookup;
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
            }
        } else {
            {
                {
                    let __for_end_4 = self.font_ptr;
                    k = 1i32;
                    while k <= __for_end_4 {
                        if ((self.font_area[(k) as usize] != 1075i32) && self.str_eq_str(self.font_name[(k) as usize], s)) {
                            {
                                self.flush_str(s);
                                tfm_lookup = k;
                                return tfm_lookup;
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
            }
        }
        tfm_lookup = 0i32;
        tfm_lookup
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn load_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut load_expand_font: internal_font_number = 0;
        let mut s: str_number = 0; // §705
        let mut k: internal_font_number = 0; // §705
        s = self.expand_font_name(f, e);
        k = self.tfm_lookup(s, self.font_size[(f) as usize]);
        if (k == 0i32) {
            {
                if self.pdf_font_auto_expand[(f) as usize] {
                    k = self.auto_expand_font(f, e);
                } else {
                    k = self.read_font_info(513i32, s, 348i32, self.font_size[(f) as usize]);
                }
            }
        }
        if (k != 0i32) {
            self.copy_expand_params(k, f, e);
        }
        load_expand_font = k;
        load_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn fix_expand_value(&mut self, mut f: internal_font_number, mut e: i32) -> i32 {
        let mut fix_expand_value: i32 = 0;
        let mut step: i32 = 0; // §705
        let mut max_expand: i32 = 0; // §705
        let mut neg: bool = false; // §705
        fix_expand_value = 0i32;
        if (e == 0i32) {
            return fix_expand_value;
        }
        if (e < 0i32) {
            {
                e = (e).wrapping_neg();
                neg = true;
                max_expand = (self.pdf_font_expand_ratio[(self.pdf_font_shrink[(f) as usize]) as usize]).wrapping_neg();
            }
        } else {
            {
                neg = false;
                max_expand = self.pdf_font_expand_ratio[(self.pdf_font_stretch[(f) as usize]) as usize];
            }
        }
        if (e > max_expand) {
            e = max_expand;
        } else {
            {
                step = self.pdf_font_step[(f) as usize];
                if ((e % step) > 0i32) {
                    e = (step).wrapping_mul(self.round_xn_over_d(e, 1i32, step));
                }
            }
        }
        if neg {
            e = (e).wrapping_neg();
        }
        fix_expand_value = e;
        fix_expand_value
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn get_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut get_expand_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §705
        k = self.pdf_font_elink[(f) as usize];
        while (k != 0i32) {
            {
                if (self.pdf_font_expand_ratio[(k) as usize] == e) {
                    {
                        get_expand_font = k;
                        return get_expand_font;
                    }
                }
                k = self.pdf_font_elink[(k) as usize];
            }
        }
        k = self.load_expand_font(f, e);
        { let __v636 = self.pdf_font_elink[(f) as usize]; self.pdf_font_elink[(k) as usize] = __v636; }
        self.pdf_font_elink[(f) as usize] = k;
        get_expand_font = k;
        get_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut expand_font: internal_font_number = 0;
        expand_font = f;
        if (e == 0i32) {
            return expand_font;
        }
        e = self.fix_expand_value(f, e);
        if (e == 0i32) {
            return expand_font;
        }
        if (self.pdf_font_elink[(f) as usize] == 0i32) {
            self.pdf_error(1039i32, 1080i32);
        }
        expand_font = self.get_expand_font(f, e);
        expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn set_expand_params(&mut self, mut f: internal_font_number, mut auto_expand: bool, mut stretch_limit: i32, mut shrink_limit: i32, mut font_step: i32, mut expand_ratio: i32) {
        self.pdf_font_step[(f) as usize] = font_step;
        self.pdf_font_auto_expand[(f) as usize] = auto_expand;
        if (stretch_limit > 0i32) {
            { let __v637 = self.get_expand_font(f, stretch_limit); self.pdf_font_stretch[(f) as usize] = __v637; }
        }
        if (shrink_limit > 0i32) {
            { let __v638 = self.get_expand_font(f, (shrink_limit).wrapping_neg()); self.pdf_font_shrink[(f) as usize] = __v638; }
        }
        if (expand_ratio != 0i32) {
            self.pdf_font_expand_ratio[(f) as usize] = expand_ratio;
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn vf_expand_local_fonts(&mut self, mut f: internal_font_number) {
        let mut lf: internal_font_number = 0; // §705
        let mut k: i32 = 0; // §705
        self.pdfassert((self.pdf_font_type[(f) as usize] == 1i32));
        {
            let __for_end_2 = (self.vf_local_font_num[(f) as usize]).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    lf = self.vf_i_fnts[((self.vf_default_font[(f) as usize]).wrapping_add(k)) as usize];
                    self.set_expand_params(lf, self.pdf_font_auto_expand[(f) as usize], self.pdf_font_expand_ratio[(self.pdf_font_stretch[(f) as usize]) as usize], (self.pdf_font_expand_ratio[(self.pdf_font_shrink[(f) as usize]) as usize]).wrapping_neg(), self.pdf_font_step[(f) as usize], self.pdf_font_expand_ratio[(f) as usize]);
                    if (self.pdf_font_type[(lf) as usize] == 1i32) {
                        self.vf_expand_local_fonts(lf);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §705
    pub fn read_expand_font(&mut self) {
        let mut shrink_limit: i32 = 0; // §705
        let mut stretch_limit: i32 = 0; // §705
        let mut font_step: i32 = 0; // §705
        let mut f: internal_font_number = 0; // §705
        let mut auto_expand: bool = false; // §705
        self.scan_font_ident();
        f = self.cur_val;
        if (f == 0i32) {
            self.pdf_error(1039i32, 873i32);
        }
        if (self.pdf_font_blink[(f) as usize] != 0i32) {
            self.pdf_error(1039i32, 1081i32);
        }
        self.scan_optional_equals();
        self.scan_int();
        stretch_limit = self.fix_int(self.cur_val, 0i32, 1000i32);
        self.scan_int();
        shrink_limit = self.fix_int(self.cur_val, 0i32, 500i32);
        self.scan_int();
        font_step = self.fix_int(self.cur_val, 0i32, 100i32);
        if (font_step == 0i32) {
            self.pdf_error(1039i32, 1082i32);
        }
        stretch_limit = (stretch_limit).wrapping_sub((stretch_limit % font_step));
        if (stretch_limit < 0i32) {
            stretch_limit = 0i32;
        }
        shrink_limit = (shrink_limit).wrapping_sub((shrink_limit % font_step));
        if (shrink_limit < 0i32) {
            shrink_limit = 0i32;
        }
        if ((stretch_limit == 0i32) && (shrink_limit == 0i32)) {
            self.pdf_error(1039i32, 1083i32);
        }
        auto_expand = false;
        if self.scan_keyword(1084i32) {
            {
                auto_expand = true;
                // §469
                {
                    self.get_x_token();
                    if (self.cur_cmd != 10i32) {
                        self.back_input();
                    }
                }
            }
        }
        // §705
        if (self.pdf_font_expand_ratio[(f) as usize] != 0i32) {
            self.pdf_error(1039i32, 1085i32);
        }
        if (self.pdf_font_step[(f) as usize] != 0i32) {
            {
                if (self.pdf_font_step[(f) as usize] != font_step) {
                    self.pdf_error(1039i32, 1086i32);
                }
                if (((self.pdf_font_stretch[(f) as usize] == 0i32) && (stretch_limit != 0i32)) || ((self.pdf_font_stretch[(f) as usize] != 0i32) && (self.pdf_font_expand_ratio[(self.pdf_font_stretch[(f) as usize]) as usize] != stretch_limit))) {
                    self.pdf_error(1039i32, 1087i32);
                }
                if (((self.pdf_font_shrink[(f) as usize] == 0i32) && (shrink_limit != 0i32)) || ((self.pdf_font_shrink[(f) as usize] != 0i32) && ((self.pdf_font_expand_ratio[(self.pdf_font_shrink[(f) as usize]) as usize]).wrapping_neg() != shrink_limit))) {
                    self.pdf_error(1039i32, 1088i32);
                }
                if (self.pdf_font_auto_expand[(f) as usize] != auto_expand) {
                    self.pdf_error(1039i32, 1089i32);
                }
            }
        } else {
            {
                if ((self.pdf_font_type[(f) as usize] != 0i32) && (self.pdf_font_type[(f) as usize] != 1i32)) {
                    self.pdf_warning(1039i32, 1090i32, true, true);
                }
                self.set_expand_params(f, auto_expand, stretch_limit, shrink_limit, font_step, 0i32);
                if (self.pdf_font_type[(f) as usize] == 1i32) {
                    self.vf_expand_local_fonts(f);
                }
            }
        }
    }

    /// We implement robust letter spacing using virtual font.
    // §706
    pub fn letter_space_font(&mut self, mut u: halfword, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut letter_space_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §706
        let mut w: scaled = 0; // §706
        let mut r: scaled = 0; // §706
        let mut s: str_number = 0; // §706
        let mut i: i32 = 0; // §706
        let mut nw: i32 = 0; // §706
        let mut old_setting: i32 = 0; // §706
        let mut vf_z: i32 = 0; // §706
        let mut vf_alpha: i32 = 0; // §706
        let mut vf_beta: i32 = 0; // §706
        k = self.read_font_info(u, self.font_name[(f) as usize], 348i32, self.font_size[(f) as usize]);
        if self.scan_keyword(1091i32) {
            self.set_no_ligatures(k);
        }
        nw = (self.height_base[(k) as usize]).wrapping_sub(self.width_base[(k) as usize]);
        if ((self.font_info[((6i32).wrapping_add(self.param_base[(k) as usize])) as usize].int() == 0i32) && (self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int() > 0i32)) {
            { let __ix639 = (6i32).wrapping_add(self.param_base[(k) as usize]); let __v640 = self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(); self.font_info[(__ix639) as usize].set_int(__v640); }
        }
        if (self.font_info[((6i32).wrapping_add(self.param_base[(k) as usize])) as usize].int() == 0i32) {
            self.pdf_warning(1092i32, 1093i32, true, true);
        }
        {
            let __for_end_2 = (nw).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix641 = (self.width_base[(k) as usize]).wrapping_add(i); let __v642 = (self.font_info[((self.width_base[(k) as usize]).wrapping_add(i)) as usize].int()).wrapping_add(self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(k) as usize])) as usize].int(), e, 1000i32)); self.font_info[(__ix641) as usize].set_int(__v642); }
                i = i.wrapping_add(1);
            }
        }
        {
            if (((self.pool_ptr).wrapping_add((self.str_start[((self.font_name[(k) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.font_name[(k) as usize]) as usize]))).wrapping_add(7i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        old_setting = self.selector;
        self.selector = 21i32;
        self.print(self.font_name[(k) as usize]);
        if (e > 0i32) {
            self.print(43i32);
        }
        self.print_int(((e) as i64));
        self.print(1094i32);
        self.selector = old_setting;
        { let __v643 = self.make_string(); self.font_name[(k) as usize] = __v643; }
        self.allocvffnts();
        self.vf_e_fnts[(self.vf_nf) as usize] = 0i32;
        self.vf_i_fnts[(self.vf_nf) as usize] = f;
        self.vf_nf = (self.vf_nf).wrapping_add(1i32);
        self.vf_local_font_num[(k) as usize] = 1i32;
        { let __v644 = (self.vf_nf).wrapping_sub(1i32); self.vf_default_font[(k) as usize] = __v644; }
        self.pdf_font_type[(k) as usize] = 1i32;
        vf_z = self.font_size[(f) as usize];
        {
            vf_alpha = 16i32;
            while (vf_z >= 8388608i32) {
                {
                    vf_z = (vf_z / 2i32);
                    vf_alpha = (vf_alpha).wrapping_add(vf_alpha);
                }
            }
            vf_beta = (256i32 / vf_alpha);
            vf_alpha = (vf_alpha).wrapping_mul(vf_z);
        }
        w = self.round_xn_over_d(self.font_info[((6i32).wrapping_add(self.param_base[(f) as usize])) as usize].int(), e, 2000i32);
        if (w >= 0i32) {
            self.tmp_w.set_qqqq_b0(0i32);
        } else {
            {
                self.tmp_w.set_qqqq_b0(255i32);
                w = (vf_alpha).wrapping_add(w);
            }
        }
        r = (w).wrapping_mul(vf_beta);
        self.tmp_w.set_qqqq_b1((r / vf_z));
        r = (r % vf_z);
        if (r == 0i32) {
            self.tmp_w.set_qqqq_b2(0i32);
        } else {
            {
                r = (r).wrapping_mul(256i32);
                self.tmp_w.set_qqqq_b2((r / vf_z));
                r = (r % vf_z);
            }
        }
        if (r == 0i32) {
            self.tmp_w.set_qqqq_b3(0i32);
        } else {
            {
                r = (r).wrapping_mul(256i32);
                self.tmp_w.set_qqqq_b3((r / vf_z));
            }
        }
        { let __v645 = self.new_vf_packet(k); self.vf_packet_base[(k) as usize] = __v645; }
        {
            let __for_end_2 = self.font_ec[(k) as usize];
            self.c = self.font_bc[(k) as usize];
            while self.c <= __for_end_2 {
                {
                    {
                        if ((self.pool_ptr).wrapping_add(12i32) > pool_size) {
                            self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                        }
                    }
                    {
                        self.str_pool[(self.pool_ptr) as usize] = 146i32;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix646 = self.pool_ptr; let __v647 = self.tmp_w.qqqq().b0(); self.str_pool[(__ix646) as usize] = __v647; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix648 = self.pool_ptr; let __v649 = self.tmp_w.qqqq().b1(); self.str_pool[(__ix648) as usize] = __v649; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix650 = self.pool_ptr; let __v651 = self.tmp_w.qqqq().b2(); self.str_pool[(__ix650) as usize] = __v651; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix652 = self.pool_ptr; let __v653 = self.tmp_w.qqqq().b3(); self.str_pool[(__ix652) as usize] = __v653; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    if (self.c < 128i32) {
                        {
                            { let __ix654 = self.pool_ptr; let __v655 = self.c; self.str_pool[(__ix654) as usize] = __v655; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            {
                                self.str_pool[(self.pool_ptr) as usize] = 128i32;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                            {
                                { let __ix656 = self.pool_ptr; let __v657 = self.c; self.str_pool[(__ix656) as usize] = __v657; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    {
                        self.str_pool[(self.pool_ptr) as usize] = 146i32;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix658 = self.pool_ptr; let __v659 = self.tmp_w.qqqq().b0(); self.str_pool[(__ix658) as usize] = __v659; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix660 = self.pool_ptr; let __v661 = self.tmp_w.qqqq().b1(); self.str_pool[(__ix660) as usize] = __v661; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix662 = self.pool_ptr; let __v663 = self.tmp_w.qqqq().b2(); self.str_pool[(__ix662) as usize] = __v663; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix664 = self.pool_ptr; let __v665 = self.tmp_w.qqqq().b3(); self.str_pool[(__ix664) as usize] = __v665; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    s = self.make_string();
                    self.storepacket(k, self.c, s);
                    self.flush_str(s);
                }
                self.c = self.c.wrapping_add(1);
            }
        }
        letter_space_font = k;
        letter_space_font
    }

    /// We implement robust letter spacing using virtual font.
    // §706
    pub fn new_letterspaced_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §706
        let mut t: str_number = 0; // §706
        let mut old_setting: i32 = 0; // §706
        let mut f: internal_font_number = 0; // §706
        let mut k: internal_font_number = 0; // §706
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
        self.scan_font_ident();
        k = self.cur_val;
        self.scan_int();
        f = { let __a666_0 = u; let __a666_1 = k; let __a666_2 = self.fix_int(self.cur_val, (1000i32).wrapping_neg(), 1000i32); self.letter_space_font(__a666_0, __a666_1, __a666_2) };
        self.eqtb[((u) - 1) as usize].set_hh_rh(f);
        { let __v667 = self.eqtb[((u) - 1) as usize]; self.eqtb[(((17626i32).wrapping_add(f)) - 1) as usize] = __v667; }
        self.hash[(((17626i32).wrapping_add(f)) - 514) as usize].set_rh(t);
    }

    /// We implement robust letter spacing using virtual font.
    // §706
    pub fn is_letterspaced_font(&mut self, mut f: internal_font_number) -> bool {
        let mut is_letterspaced_font: bool = false;
        let mut i: pool_pointer = 0; // §706
        let mut j: pool_pointer = 0; // §706
        'l_done_f: {
            is_letterspaced_font = false;
            if (self.pdf_font_type[(f) as usize] != 1i32) {
                return is_letterspaced_font;
            }
            i = (self.str_start[((self.font_name[(f) as usize]).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            j = self.str_start[(self.font_name[(f) as usize]) as usize];
            if ((self.str_pool[((i).wrapping_sub(1i32)) as usize] != ((b'l') as i32)) || (self.str_pool[(i) as usize] != ((b's') as i32))) {
                return is_letterspaced_font;
            }
            i = (i).wrapping_sub(2i32);
            while (i >= j) {
                {
                    if ((self.str_pool[(i) as usize] < ((b'0') as i32)) || (self.str_pool[(i) as usize] > ((b'9') as i32))) {
                        break 'l_done_f;
                    }
                    i = (i).wrapping_sub(1i32);
                }
            }
        }
        if (i < j) {
            return is_letterspaced_font;
        }
        if ((self.str_pool[(i) as usize] != ((b'+') as i32)) && (self.str_pool[(i) as usize] != ((b'-') as i32))) {
            return is_letterspaced_font;
        }
        is_letterspaced_font = true;
        is_letterspaced_font
    }

    /// We implement robust letter spacing using virtual font.
    // §706
    pub fn copy_font_info(&mut self, mut f: internal_font_number) -> internal_font_number {
        let mut copy_font_info: internal_font_number = 0;
        let mut lf: halfword = 0; // §706
        let mut bc: halfword = 0; // §706
        let mut ec: halfword = 0; // §706
        let mut i: halfword = 0; // §706
        let mut k: internal_font_number = 0; // §706
        if ((self.pdf_font_expand_ratio[(f) as usize] != 0i32) || (self.pdf_font_step[(f) as usize] != 0i32)) {
            self.pdf_error(1096i32, 1097i32);
        }
        if self.is_letterspaced_font(f) {
            self.pdf_error(1096i32, 1098i32);
        }
        k = (self.font_ptr).wrapping_add(1i32);
        self.font_ptr = (self.font_ptr).wrapping_add(1i32);
        if (self.font_ptr >= font_max) {
            self.overflow(1078i32, font_max);
        }
        { let __v668 = self.font_name[(f) as usize]; self.font_name[(k) as usize] = __v668; }
        self.font_area[(k) as usize] = 1075i32;
        { let __v669 = self.hyphen_char[(f) as usize]; self.hyphen_char[(k) as usize] = __v669; }
        { let __v670 = self.skew_char[(f) as usize]; self.skew_char[(k) as usize] = __v670; }
        { let __v671 = self.font_bchar[(f) as usize]; self.font_bchar[(k) as usize] = __v671; }
        { let __v672 = self.font_false_bchar[(f) as usize]; self.font_false_bchar[(k) as usize] = __v672; }
        { let __v673 = self.font_bc[(f) as usize]; self.font_bc[(k) as usize] = __v673; }
        { let __v674 = self.font_ec[(f) as usize]; self.font_ec[(k) as usize] = __v674; }
        { let __v675 = self.font_size[(f) as usize]; self.font_size[(k) as usize] = __v675; }
        { let __v676 = self.font_dsize[(f) as usize]; self.font_dsize[(k) as usize] = __v676; }
        { let __v677 = self.font_params[(f) as usize]; self.font_params[(k) as usize] = __v677; }
        { let __v678 = self.font_glue[(f) as usize]; self.font_glue[(k) as usize] = __v678; }
        { let __v679 = self.bchar_label[(f) as usize]; self.bchar_label[(k) as usize] = __v679; }
        bc = self.font_bc[(f) as usize];
        ec = self.font_ec[(f) as usize];
        { let __v680 = (self.fmem_ptr).wrapping_sub(bc); self.char_base[(k) as usize] = __v680; }
        { let __v681 = ((self.char_base[(k) as usize]).wrapping_add(ec)).wrapping_add(1i32); self.width_base[(k) as usize] = __v681; }
        { let __v682 = (self.width_base[(k) as usize]).wrapping_add((self.height_base[(f) as usize]).wrapping_sub(self.width_base[(f) as usize])); self.height_base[(k) as usize] = __v682; }
        { let __v683 = (self.height_base[(k) as usize]).wrapping_add((self.depth_base[(f) as usize]).wrapping_sub(self.height_base[(f) as usize])); self.depth_base[(k) as usize] = __v683; }
        { let __v684 = (self.depth_base[(k) as usize]).wrapping_add((self.italic_base[(f) as usize]).wrapping_sub(self.depth_base[(f) as usize])); self.italic_base[(k) as usize] = __v684; }
        { let __v685 = (self.italic_base[(k) as usize]).wrapping_add((self.lig_kern_base[(f) as usize]).wrapping_sub(self.italic_base[(f) as usize])); self.lig_kern_base[(k) as usize] = __v685; }
        { let __v686 = (self.lig_kern_base[(k) as usize]).wrapping_add((self.kern_base[(f) as usize]).wrapping_sub(self.lig_kern_base[(f) as usize])); self.kern_base[(k) as usize] = __v686; }
        { let __v687 = (self.kern_base[(k) as usize]).wrapping_add((self.exten_base[(f) as usize]).wrapping_sub(self.kern_base[(f) as usize])); self.exten_base[(k) as usize] = __v687; }
        { let __v688 = (self.exten_base[(k) as usize]).wrapping_add((self.param_base[(f) as usize]).wrapping_sub(self.exten_base[(f) as usize])); self.param_base[(k) as usize] = __v688; }
        lf = (((self.param_base[(f) as usize]).wrapping_sub(self.char_base[(f) as usize])).wrapping_add(self.font_params[(f) as usize])).wrapping_add(1i32);
        if ((self.fmem_ptr).wrapping_add(lf) >= font_mem_size) {
            self.overflow(1079i32, font_mem_size);
        }
        {
            let __for_end_2 = (lf).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix689 = ((self.char_base[(k) as usize]).wrapping_add(bc)).wrapping_add(i); let __v690 = self.font_info[(((self.char_base[(f) as usize]).wrapping_add(bc)).wrapping_add(i)) as usize]; self.font_info[(__ix689) as usize] = __v690; }
                i = i.wrapping_add(1);
            }
        }
        self.fmem_ptr = (self.fmem_ptr).wrapping_add(lf);
        copy_font_info = k;
        copy_font_info
    }

    /// We implement robust letter spacing using virtual font.
    // §706
    pub fn make_font_copy(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §706
        let mut t: str_number = 0; // §706
        let mut old_setting: i32 = 0; // §706
        let mut f: internal_font_number = 0; // §706
        let mut k: internal_font_number = 0; // §706
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
        self.scan_font_ident();
        k = self.cur_val;
        f = self.copy_font_info(k);
        self.eqtb[((u) - 1) as usize].set_hh_rh(f);
        { let __v691 = self.eqtb[((u) - 1) as usize]; self.eqtb[(((17626i32).wrapping_add(f)) - 1) as usize] = __v691; }
        self.hash[(((17626i32).wrapping_add(f)) - 514) as usize].set_rh(t);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_error(&mut self, mut filename: str_number, mut msg: str_number) {
        let mut old_setting: i32 = 0; // §712
        let mut s: str_number = 0; // §712
        {
            if (((self.pool_ptr).wrapping_add((self.str_start[((filename).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(filename) as usize]))).wrapping_add(3i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        old_setting = self.selector;
        self.selector = 21i32;
        self.print(filename);
        self.print(1099i32);
        s = self.make_string();
        self.selector = old_setting;
        self.pdf_error(s, msg);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_byte(&mut self) -> eight_bits {
        let mut vf_byte: eight_bits = 0;
        let mut i: i32 = 0; // §712
        i = { let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.getc(&mut __f0); self.vf_file = __f0; __r };
        if (i < 0i32) {
            self.pdf_error(1100i32, 1101i32);
        }
        vf_byte = i;
        vf_byte
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_read_signed(&mut self, mut k: i32) -> i32 {
        let mut vf_read_signed: i32 = 0;
        let mut i: i32 = 0; // §712
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.vf_byte();
        if (i >= 128i32) {
            i = (i).wrapping_sub(256i32);
        }
        k = (k).wrapping_sub(1i32);
        while (k > 0i32) {
            {
                i = ((i).wrapping_mul(256i32)).wrapping_add(self.vf_byte());
                k = (k).wrapping_sub(1i32);
            }
        }
        vf_read_signed = i;
        vf_read_signed
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_read_unsigned(&mut self, mut k: i32) -> i32 {
        let mut vf_read_unsigned: i32 = 0;
        let mut i: i32 = 0; // §712
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.vf_byte();
        if ((k == 4i32) && (i >= 128i32)) {
            self.vf_error(self.font_name[(self.f) as usize], 1031i32);
        }
        k = (k).wrapping_sub(1i32);
        while (k > 0i32) {
            {
                i = ((i).wrapping_mul(256i32)).wrapping_add(self.vf_byte());
                k = (k).wrapping_sub(1i32);
            }
        }
        vf_read_unsigned = i;
        vf_read_unsigned
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_local_font_warning(&mut self, mut f: internal_font_number, mut k: internal_font_number, mut s: str_number) {
        self.print_nl(s);
        self.print(1102i32);
        self.print(self.font_name[(k) as usize]);
        self.print(1103i32);
        self.print(self.font_name[(f) as usize]);
        self.print(1104i32);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn vf_def_font(&mut self, mut f: internal_font_number) -> internal_font_number {
        let mut vf_def_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §712
        let mut s: str_number = 0; // §712
        let mut ds: scaled = 0; // §712
        let mut fs: scaled = 0; // §712
        let mut cs: four_quarters = four_quarters::default(); // §712
        { let __v692 = self.vf_byte(); cs.set_b0(__v692); }
        { let __v693 = self.vf_byte(); cs.set_b1(__v693); }
        { let __v694 = self.vf_byte(); cs.set_b2(__v694); }
        { let __v695 = self.vf_byte(); cs.set_b3(__v695); }
        fs = { let __a696_0 = self.vf_read_signed(4i32); let __a696_1 = self.font_size[(f) as usize]; self.store_scaled_f(__a696_0, __a696_1) };
        ds = (self.vf_read_signed(4i32) / 16i32);
        { let __v697 = self.vf_byte(); self.tmp_w.set_qqqq_b0(__v697); }
        { let __v698 = self.vf_byte(); self.tmp_w.set_qqqq_b1(__v698); }
        while (self.tmp_w.qqqq().b0() > 0i32) {
            {
                { let __v699 = (self.tmp_w.qqqq().b0()).wrapping_sub(1i32); self.tmp_w.set_qqqq_b0(__v699); }
                {
                    if (self.vf_byte() != 0i32) {
                    }
                }
            }
        }
        {
            if ((self.pool_ptr).wrapping_add(self.tmp_w.qqqq().b1()) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        while (self.tmp_w.qqqq().b1() > 0i32) {
            {
                { let __v700 = (self.tmp_w.qqqq().b1()).wrapping_sub(1i32); self.tmp_w.set_qqqq_b1(__v700); }
                {
                    { let __ix701 = self.pool_ptr; let __v702 = self.vf_byte(); self.str_pool[(__ix701) as usize] = __v702; }
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
            }
        }
        s = self.make_string();
        k = self.tfm_lookup(s, fs);
        if (k == 0i32) {
            k = self.read_font_info(513i32, s, 348i32, fs);
        }
        if (k != 0i32) {
            {
                if ((((((cs.b0() != 0i32) || (cs.b1() != 0i32)) || (cs.b2() != 0i32)) || (cs.b3() != 0i32)) && ((((self.font_check[(k) as usize].b0() != 0i32) || (self.font_check[(k) as usize].b1() != 0i32)) || (self.font_check[(k) as usize].b2() != 0i32)) || (self.font_check[(k) as usize].b3() != 0i32))) && ((((cs.b0() != self.font_check[(k) as usize].b0()) || (cs.b1() != self.font_check[(k) as usize].b1())) || (cs.b2() != self.font_check[(k) as usize].b2())) || (cs.b3() != self.font_check[(k) as usize].b3()))) {
                    self.vf_local_font_warning(f, k, 1105i32);
                }
                if (ds != self.font_dsize[(k) as usize]) {
                    self.vf_local_font_warning(f, k, 1106i32);
                }
                if (self.pdf_font_step[(f) as usize] != 0i32) {
                    self.set_expand_params(k, self.pdf_font_auto_expand[(f) as usize], self.pdf_font_expand_ratio[(self.pdf_font_stretch[(f) as usize]) as usize], (self.pdf_font_expand_ratio[(self.pdf_font_shrink[(f) as usize]) as usize]).wrapping_neg(), self.pdf_font_step[(f) as usize], self.pdf_font_expand_ratio[(f) as usize]);
                }
            }
        }
        vf_def_font = k;
        vf_def_font
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §712
    pub fn do_vf(&mut self, mut f: internal_font_number) {
        let mut cmd: i32 = 0; // §712
        let mut k: i32 = 0; // §712
        let mut n: i32 = 0; // §712
        let mut cc: i32 = 0; // §712
        let mut cmd_length: i32 = 0; // §712
        let mut packet_length: i32 = 0; // §712
        let mut tfm_width: scaled = 0; // §712
        let mut s: str_number = 0; // §712
        let mut stack_level: vf_stack_index = 0; // §712
        let mut save_vf_nf: internal_font_number = 0; // §712
        self.pdf_font_type[(f) as usize] = 2i32;
        if self.auto_expand_vf(f) {
            return;
        }
        stack_level = 0i32;
        // §713
        self.pack_file_name(self.font_name[(f) as usize], 348i32, 1099i32);
        if (!{ let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.vf_b_open_in(&mut __f0); self.vf_file = __f0; __r }) {
            return;
        }
        // §714
        if (self.vf_byte() != 247i32) {
            self.vf_error(self.font_name[(f) as usize], 1108i32);
        }
        if (self.vf_byte() != 202i32) {
            self.vf_error(self.font_name[(f) as usize], 1109i32);
        }
        cmd_length = self.vf_byte();
        {
            let __for_end_2 = cmd_length;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    if (self.vf_byte() != 0i32) {
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        { let __v703 = self.vf_byte(); self.tmp_w.set_qqqq_b0(__v703); }
        { let __v704 = self.vf_byte(); self.tmp_w.set_qqqq_b1(__v704); }
        { let __v705 = self.vf_byte(); self.tmp_w.set_qqqq_b2(__v705); }
        { let __v706 = self.vf_byte(); self.tmp_w.set_qqqq_b3(__v706); }
        if ((((((self.tmp_w.qqqq().b0() != 0i32) || (self.tmp_w.qqqq().b1() != 0i32)) || (self.tmp_w.qqqq().b2() != 0i32)) || (self.tmp_w.qqqq().b3() != 0i32)) && ((((self.font_check[(f) as usize].b0() != 0i32) || (self.font_check[(f) as usize].b1() != 0i32)) || (self.font_check[(f) as usize].b2() != 0i32)) || (self.font_check[(f) as usize].b3() != 0i32))) && ((((self.tmp_w.qqqq().b0() != self.font_check[(f) as usize].b0()) || (self.tmp_w.qqqq().b1() != self.font_check[(f) as usize].b1())) || (self.tmp_w.qqqq().b2() != self.font_check[(f) as usize].b2())) || (self.tmp_w.qqqq().b3() != self.font_check[(f) as usize].b3()))) {
            {
                self.print_nl(1110i32);
                self.print(self.font_name[(f) as usize]);
                self.print(1111i32);
            }
        }
        if ((self.vf_read_signed(4i32) / 16i32) != self.font_dsize[(f) as usize]) {
            {
                self.print_nl(1112i32);
                self.print(self.font_name[(f) as usize]);
                self.print(1111i32);
            }
        }
        crate::system::break_out(&mut self.term_out);
        // §715
        cmd = self.vf_byte();
        save_vf_nf = self.vf_nf;
        while ((cmd >= 243i32) && (cmd <= 246i32)) {
            {
                self.allocvffnts();
                { let __ix707 = self.vf_nf; let __v708 = self.vf_read_unsigned((cmd).wrapping_sub(242i32)); self.vf_e_fnts[(__ix707) as usize] = __v708; }
                { let __ix709 = self.vf_nf; let __v710 = self.vf_def_font(f); self.vf_i_fnts[(__ix709) as usize] = __v710; }
                self.vf_nf = (self.vf_nf).wrapping_add(1i32);
                cmd = self.vf_byte();
            }
        }
        self.vf_default_font[(f) as usize] = save_vf_nf;
        { let __v711 = (self.vf_nf).wrapping_sub(save_vf_nf); self.vf_local_font_num[(f) as usize] = __v711; }
        // §716
        { let __v712 = self.new_vf_packet(f); self.vf_packet_base[(f) as usize] = __v712; }
        // §712
        while (cmd <= 242i32) {
            {
                // §717
                if (cmd == 242i32) {
                    {
                        packet_length = self.vf_read_unsigned(4i32);
                        cc = self.vf_read_unsigned(4i32);
                        if (!(((self.font_bc[(f) as usize] <= cc) && (cc <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(cc)) as usize].qqqq().b0() > 0i32))) {
                            self.vf_error(self.font_name[(f) as usize], 1113i32);
                        }
                        tfm_width = { let __a713_0 = self.vf_read_signed(4i32); let __a713_1 = self.font_size[(f) as usize]; self.store_scaled_f(__a713_0, __a713_1) };
                    }
                } else {
                    {
                        packet_length = cmd;
                        cc = self.vf_byte();
                        if (!(((self.font_bc[(f) as usize] <= cc) && (cc <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(cc)) as usize].qqqq().b0() > 0i32))) {
                            self.vf_error(self.font_name[(f) as usize], 1113i32);
                        }
                        tfm_width = { let __a714_0 = self.vf_read_unsigned(3i32); let __a714_1 = self.font_size[(f) as usize]; self.store_scaled_f(__a714_0, __a714_1) };
                    }
                }
                if (packet_length < 0i32) {
                    self.vf_error(self.font_name[(f) as usize], 1114i32);
                }
                if (packet_length > 10000i32) {
                    self.vf_error(self.font_name[(f) as usize], 1115i32);
                }
                if (tfm_width != self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(cc)) as usize].qqqq().b0())) as usize].int()) {
                    {
                        self.print_nl(1116i32);
                        self.print(self.font_name[(f) as usize]);
                        self.print(1111i32);
                    }
                }
                {
                    if ((self.pool_ptr).wrapping_add(packet_length) > pool_size) {
                        self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                    }
                }
                while (packet_length > 0i32) {
                    {
                        cmd = self.vf_byte();
                        packet_length = (packet_length).wrapping_sub(1i32);
                        // §719
                        if ((cmd >= 0i32) && (cmd <= 127i32)) {
                            cmd_length = 0i32;
                        } else {
                            if (((171i32 <= cmd) && (cmd <= 234i32)) || ((235i32 <= cmd) && (cmd <= 238i32))) {
                                {
                                    if (cmd >= 235i32) {
                                        {
                                            k = self.vf_read_unsigned((cmd).wrapping_sub(234i32));
                                            packet_length = (packet_length).wrapping_sub((cmd).wrapping_sub(234i32));
                                        }
                                    } else {
                                        k = (cmd).wrapping_sub(171i32);
                                    }
                                    if (k >= 256i32) {
                                        self.vf_error(self.font_name[(f) as usize], 1119i32);
                                    }
                                    n = 0i32;
                                    while ((n < self.vf_local_font_num[(f) as usize]) && (self.vf_e_fnts[((self.vf_default_font[(f) as usize]).wrapping_add(n)) as usize] != k)) {
                                        n = (n).wrapping_add(1i32);
                                    }
                                    if (n == self.vf_local_font_num[(f) as usize]) {
                                        self.vf_error(self.font_name[(f) as usize], 1120i32);
                                    }
                                    if (k <= 63i32) {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = (171i32).wrapping_add(k);
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            {
                                                self.str_pool[(self.pool_ptr) as usize] = 235i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            {
                                                self.str_pool[(self.pool_ptr) as usize] = k;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    cmd_length = 0i32;
                                    cmd = 138i32;
                                }
                            } else {
                                match cmd {
                                    132 | 137 => {
                                        cmd_length = 8i32;
                                    }
                                    128 | 129 | 130 | 131 => {
                                        cmd_length = (cmd).wrapping_sub(127i32);
                                    }
                                    133 | 134 | 135 | 136 => {
                                        cmd_length = (cmd).wrapping_sub(132i32);
                                    }
                                    143 | 144 | 145 | 146 => {
                                        cmd_length = (cmd).wrapping_sub(142i32);
                                    }
                                    148 | 149 | 150 | 151 => {
                                        cmd_length = (cmd).wrapping_sub(147i32);
                                    }
                                    153 | 154 | 155 | 156 => {
                                        cmd_length = (cmd).wrapping_sub(152i32);
                                    }
                                    157 | 158 | 159 | 160 => {
                                        cmd_length = (cmd).wrapping_sub(156i32);
                                    }
                                    162 | 163 | 164 | 165 => {
                                        cmd_length = (cmd).wrapping_sub(161i32);
                                    }
                                    167 | 168 | 169 | 170 => {
                                        cmd_length = (cmd).wrapping_sub(166i32);
                                    }
                                    239 | 240 | 241 | 242 => {
                                        {
                                            cmd_length = self.vf_read_unsigned((cmd).wrapping_sub(238i32));
                                            packet_length = (packet_length).wrapping_sub((cmd).wrapping_sub(238i32));
                                            if (cmd_length > 10000i32) {
                                                self.vf_error(self.font_name[(f) as usize], 1115i32);
                                            }
                                            if (cmd_length < 0i32) {
                                                self.vf_error(self.font_name[(f) as usize], 1121i32);
                                            }
                                            {
                                                self.str_pool[(self.pool_ptr) as usize] = 239i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            {
                                                self.str_pool[(self.pool_ptr) as usize] = cmd_length;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            cmd = 138i32;
                                        }
                                    }
                                    147 | 152 | 161 | 166 | 138 => {
                                        cmd_length = 0i32;
                                    }
                                    141 | 142 => {
                                        {
                                            cmd_length = 0i32;
                                            if (cmd == 141i32) {
                                                if (stack_level == vf_stack_size) {
                                                    self.overflow(1122i32, vf_stack_size);
                                                } else {
                                                    stack_level = (stack_level).wrapping_add(1i32);
                                                }
                                            } else {
                                                if (stack_level == 0i32) {
                                                    self.vf_error(self.font_name[(f) as usize], 1123i32);
                                                } else {
                                                    stack_level = (stack_level).wrapping_sub(1i32);
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        self.vf_error(self.font_name[(f) as usize], 1124i32);
                                    }
                                }
                            }
                        }
                        // §717
                        if (cmd != 138i32) {
                            {
                                self.str_pool[(self.pool_ptr) as usize] = cmd;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                        packet_length = (packet_length).wrapping_sub(cmd_length);
                        while (cmd_length > 0i32) {
                            {
                                cmd_length = (cmd_length).wrapping_sub(1i32);
                                {
                                    { let __ix715 = self.pool_ptr; let __v716 = self.vf_byte(); self.str_pool[(__ix715) as usize] = __v716; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
                if (stack_level != 0i32) {
                    self.vf_error(self.font_name[(f) as usize], 1117i32);
                }
                if (packet_length != 0i32) {
                    self.vf_error(self.font_name[(f) as usize], 1118i32);
                }
                // §718
                s = self.make_string();
                self.storepacket(f, cc, s);
                self.flush_str(s);
                // §717
                cmd = self.vf_byte();
            }
        }
        // §712
        if (cmd != 248i32) {
            self.vf_error(self.font_name[(f) as usize], 1107i32);
        }
        { let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.b_close(&mut __f0); self.vf_file = __f0; __r };
        self.pdf_font_type[(f) as usize] = 1i32;
    }

    // §720
    pub fn pdf_check_vf_cur_val(&mut self) {
        let mut f: internal_font_number = 0; // §720
        f = self.cur_val;
        self.do_vf(f);
        if (self.pdf_font_type[(f) as usize] == 1i32) {
            self.pdf_error(594i32, 1125i32);
        }
    }

    // §720
    pub fn auto_expand_vf(&mut self, mut f: internal_font_number) -> bool {
        let mut auto_expand_vf: bool = false;
        let mut bf: internal_font_number = 0; // §720
        let mut lf: internal_font_number = 0; // §720
        let mut e: i32 = 0; // §720
        let mut k: i32 = 0; // §720
        auto_expand_vf = false;
        if ((!self.pdf_font_auto_expand[(f) as usize]) || (self.pdf_font_blink[(f) as usize] == 0i32)) {
            return auto_expand_vf;
        }
        bf = self.pdf_font_blink[(f) as usize];
        if (self.pdf_font_type[(bf) as usize] == 0i32) {
            self.do_vf(bf);
        }
        if (self.pdf_font_type[(bf) as usize] != 1i32) {
            return auto_expand_vf;
        }
        e = self.pdf_font_expand_ratio[(f) as usize];
        {
            let __for_end_2 = (self.vf_local_font_num[(bf) as usize]).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    lf = (self.vf_default_font[(bf) as usize]).wrapping_add(k);
                    self.allocvffnts();
                    { let __ix717 = self.vf_nf; let __v718 = self.vf_e_fnts[(lf) as usize]; self.vf_e_fnts[(__ix717) as usize] = __v718; }
                    { let __ix719 = self.vf_nf; let __v720 = self.auto_expand_font(self.vf_i_fnts[(lf) as usize], e); self.vf_i_fnts[(__ix719) as usize] = __v720; }
                    self.copy_expand_params(self.vf_i_fnts[(self.vf_nf) as usize], self.vf_i_fnts[(lf) as usize], e);
                    self.vf_nf = (self.vf_nf).wrapping_add(1i32);
                }
                k = k.wrapping_add(1);
            }
        }
        { let __v721 = self.vf_packet_base[(bf) as usize]; self.vf_packet_base[(f) as usize] = __v721; }
        { let __v722 = self.vf_local_font_num[(bf) as usize]; self.vf_local_font_num[(f) as usize] = __v722; }
        { let __v723 = (self.vf_nf).wrapping_sub(self.vf_local_font_num[(f) as usize]); self.vf_default_font[(f) as usize] = __v723; }
        self.pdf_font_type[(f) as usize] = 1i32;
        auto_expand_vf = true;
        auto_expand_vf
    }

    /// Some functions for processing character packets.
    // §725
    pub fn packet_read_signed(&mut self, mut k: i32) -> i32 {
        let mut packet_read_signed: i32 = 0;
        let mut i: i32 = 0; // §725
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.packet_byte();
        if (i >= 128i32) {
            i = (i).wrapping_sub(256i32);
        }
        k = (k).wrapping_sub(1i32);
        while (k > 0i32) {
            {
                i = ((i).wrapping_mul(256i32)).wrapping_add(self.packet_byte());
                k = (k).wrapping_sub(1i32);
            }
        }
        packet_read_signed = i;
        packet_read_signed
    }

    /// Some functions for processing character packets.
    // §725
    pub fn packet_read_unsigned(&mut self, mut k: i32) -> i32 {
        let mut packet_read_unsigned: i32 = 0;
        let mut i: i32 = 0; // §725
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.packet_byte();
        if ((k == 4i32) && (i >= 128i32)) {
            self.vf_error(self.font_name[(self.f) as usize], 1031i32);
        }
        k = (k).wrapping_sub(1i32);
        while (k > 0i32) {
            {
                i = ((i).wrapping_mul(256i32)).wrapping_add(self.packet_byte());
                k = (k).wrapping_sub(1i32);
            }
        }
        packet_read_unsigned = i;
        packet_read_unsigned
    }

    /// Some functions for processing character packets.
    // §725
    pub fn packet_scaled(&mut self, mut k: i32, mut fs: scaled) -> scaled {
        let mut packet_scaled: scaled = 0;
        packet_scaled = { let __a724_0 = self.packet_read_signed(k); let __a724_1 = fs; self.store_scaled_f(__a724_0, __a724_1) };
        packet_scaled
    }

    /// Some functions for processing character packets.
    // §725
    pub fn do_vf_packet(&mut self, mut vf_f: internal_font_number, mut c: eight_bits) {
        let mut f: internal_font_number = 0; // §725
        let mut k: internal_font_number = 0; // §725
        let mut n: internal_font_number = 0; // §725
        let mut save_cur_h: scaled = 0; // §725
        let mut save_cur_v: scaled = 0; // §725
        let mut cmd: i32 = 0; // §725
        let mut char_move: bool = false; // §725
        let mut w: scaled = 0; // §725
        let mut x: scaled = 0; // §725
        let mut y: scaled = 0; // §725
        let mut z: scaled = 0; // §725
        let mut s: str_number = 0; // §725
        self.vf_cur_s = (self.vf_cur_s).wrapping_add(1i32);
        if (self.vf_cur_s > vf_max_recursion) {
            self.overflow(1126i32, vf_max_recursion);
        }
        save_cur_v = self.cur_v;
        save_cur_h = self.cur_h;
        self.push_packet_state();
        self.start_packet(vf_f, c);
        f = self.vf_i_fnts[(self.vf_default_font[(vf_f) as usize]) as usize];
        w = 0i32;
        x = 0i32;
        y = 0i32;
        z = 0i32;
        while (self.vf_packet_length > 0i32) {
            {
                'l_continue_f: {
                    'l_L70_f: {
                        cmd = self.packet_byte();
                        // §726
                        if ((cmd >= 0i32) && (cmd <= 127i32)) {
                            {
                                if (!(((self.font_bc[(f) as usize] <= cmd) && (cmd <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(cmd)) as usize].qqqq().b0() > 0i32))) {
                                    {
                                        self.char_warning(f, cmd);
                                        break 'l_continue_f;
                                    }
                                }
                                c = cmd;
                                char_move = true;
                                break 'l_L70_f;
                            }
                        } else {
                            if (((171i32 <= cmd) && (cmd <= 234i32)) || (cmd == 235i32)) {
                                {
                                    if (cmd == 235i32) {
                                        k = self.packet_byte();
                                    } else {
                                        k = (cmd).wrapping_sub(171i32);
                                    }
                                    n = 0i32;
                                    while ((n < self.vf_local_font_num[(vf_f) as usize]) && (self.vf_e_fnts[((self.vf_default_font[(vf_f) as usize]).wrapping_add(n)) as usize] != k)) {
                                        n = (n).wrapping_add(1i32);
                                    }
                                    if (n == self.vf_local_font_num[(vf_f) as usize]) {
                                        self.pdf_error(1100i32, 1127i32);
                                    } else {
                                        f = self.vf_i_fnts[((self.vf_default_font[(vf_f) as usize]).wrapping_add(n)) as usize];
                                    }
                                }
                            } else {
                                match cmd {
                                    141 => {
                                        {
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_h = self.cur_h;
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_v = self.cur_v;
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_w = w;
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_x = x;
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_y = y;
                                            self.vf_stack[(self.vf_stack_ptr) as usize].stack_z = z;
                                            self.vf_stack_ptr = (self.vf_stack_ptr).wrapping_add(1i32);
                                        }
                                    }
                                    142 => {
                                        {
                                            self.vf_stack_ptr = (self.vf_stack_ptr).wrapping_sub(1i32);
                                            self.cur_h = self.vf_stack[(self.vf_stack_ptr) as usize].stack_h;
                                            self.cur_v = self.vf_stack[(self.vf_stack_ptr) as usize].stack_v;
                                            w = self.vf_stack[(self.vf_stack_ptr) as usize].stack_w;
                                            x = self.vf_stack[(self.vf_stack_ptr) as usize].stack_x;
                                            y = self.vf_stack[(self.vf_stack_ptr) as usize].stack_y;
                                            z = self.vf_stack[(self.vf_stack_ptr) as usize].stack_z;
                                        }
                                    }
                                    128 | 129 | 130 | 131 | 133 | 134 | 135 | 136 => {
                                        {
                                            if ((128i32 <= cmd) && (cmd <= 131i32)) {
                                                {
                                                    { let __v725 = self.packet_read_unsigned((cmd).wrapping_sub(127i32)); self.tmp_w.set_int(__v725); }
                                                    char_move = true;
                                                }
                                            } else {
                                                {
                                                    { let __v726 = self.packet_read_unsigned((cmd).wrapping_sub(132i32)); self.tmp_w.set_int(__v726); }
                                                    char_move = false;
                                                }
                                            }
                                            if (!(((self.font_bc[(f) as usize] <= self.tmp_w.int()) && (self.tmp_w.int() <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(self.tmp_w.int())) as usize].qqqq().b0() > 0i32))) {
                                                {
                                                    self.char_warning(f, self.tmp_w.int());
                                                    break 'l_continue_f;
                                                }
                                            }
                                            c = self.tmp_w.int();
                                            break 'l_L70_f;
                                        }
                                    }
                                    132 | 137 => {
                                        {
                                            self.rule_ht = self.packet_scaled(4i32, self.font_size[(vf_f) as usize]);
                                            self.rule_wd = self.packet_scaled(4i32, self.font_size[(vf_f) as usize]);
                                            if ((self.rule_wd > 0i32) && (self.rule_ht > 0i32)) {
                                                {
                                                    self.pdf_set_rule(self.cur_h, self.cur_v, self.rule_wd, self.rule_ht);
                                                    if (cmd == 132i32) {
                                                        self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    143 | 144 | 145 | 146 => {
                                        self.cur_h = (self.cur_h).wrapping_add(self.packet_scaled((cmd).wrapping_sub(142i32), self.font_size[(vf_f) as usize]));
                                    }
                                    147 | 148 | 149 | 150 | 151 => {
                                        {
                                            if (cmd > 147i32) {
                                                w = self.packet_scaled((cmd).wrapping_sub(147i32), self.font_size[(vf_f) as usize]);
                                            }
                                            self.cur_h = (self.cur_h).wrapping_add(w);
                                        }
                                    }
                                    152 | 153 | 154 | 155 | 156 => {
                                        {
                                            if (cmd > 152i32) {
                                                x = self.packet_scaled((cmd).wrapping_sub(152i32), self.font_size[(vf_f) as usize]);
                                            }
                                            self.cur_h = (self.cur_h).wrapping_add(x);
                                        }
                                    }
                                    157 | 158 | 159 | 160 => {
                                        self.cur_v = (self.cur_v).wrapping_add(self.packet_scaled((cmd).wrapping_sub(156i32), self.font_size[(vf_f) as usize]));
                                    }
                                    161 | 162 | 163 | 164 | 165 => {
                                        {
                                            if (cmd > 161i32) {
                                                y = self.packet_scaled((cmd).wrapping_sub(161i32), self.font_size[(vf_f) as usize]);
                                            }
                                            self.cur_v = (self.cur_v).wrapping_add(y);
                                        }
                                    }
                                    166 | 167 | 168 | 169 | 170 => {
                                        {
                                            if (cmd > 166i32) {
                                                z = self.packet_scaled((cmd).wrapping_sub(166i32), self.font_size[(vf_f) as usize]);
                                            }
                                            self.cur_v = (self.cur_v).wrapping_add(z);
                                        }
                                    }
                                    239 | 240 | 241 | 242 => {
                                        {
                                            { let __v727 = self.packet_read_unsigned((cmd).wrapping_sub(238i32)); self.tmp_w.set_int(__v727); }
                                            {
                                                if ((self.pool_ptr).wrapping_add(self.tmp_w.int()) > pool_size) {
                                                    self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                                }
                                            }
                                            while (self.tmp_w.int() > 0i32) {
                                                {
                                                    { let __v728 = (self.tmp_w.int()).wrapping_sub(1i32); self.tmp_w.set_int(__v728); }
                                                    {
                                                        { let __ix729 = self.pool_ptr; let __v730 = self.packet_byte(); self.str_pool[(__ix729) as usize] = __v730; }
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            s = self.make_string();
                                            self.literal(s, 3i32, false);
                                            self.flush_str(s);
                                        }
                                    }
                                    _ => {
                                        self.pdf_error(1100i32, 1128i32);
                                    }
                                }
                            }
                        }
                        break 'l_continue_f;
                    }
                    if (((self.font_bc[(f) as usize] <= c) && (c <= self.font_ec[(f) as usize])) && (self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b0() > 0i32)) {
                        {
                            if (self.pdf_font_type[(f) as usize] == 0i32) {
                                self.do_vf(f);
                            }
                            if (self.pdf_font_type[(f) as usize] == 1i32) {
                                self.do_vf_packet(f, c);
                            } else {
                                {
                                    self.pdf_begin_string(f);
                                    self.pdf_print_char(f, c);
                                    self.adv_char_width(f, c);
                                }
                            }
                        }
                    } else {
                        self.char_warning(f, c);
                    }
                    if char_move {
                        self.cur_h = (self.cur_h).wrapping_add(self.font_info[((self.width_base[(f) as usize]).wrapping_add(self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq().b0())) as usize].int());
                    }
                }
                // §725
            }
        }
        self.pop_packet_state();
        self.cur_v = save_cur_v;
        self.cur_h = save_cur_h;
        self.vf_cur_s = (self.vf_cur_s).wrapping_sub(1i32);
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_literal(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §727
        let mut s: str_number = 0; // §727
        let mut h: halfword = 0; // §727
        let mut q: halfword = 0; // §727
        let mut r: halfword = 0; // §727
        let mut old_mode: i32 = 0; // §727
        old_setting = self.selector;
        if (self.mem[(p) as usize].hh().b1() == 8i32) {
            {
                // §1618
                q = self.get_avail();
                self.mem[(q) as usize].set_hh_lh(637i32);
                r = self.get_avail();
                self.mem[(q) as usize].set_hh_rh(r);
                self.mem[(r) as usize].set_hh_lh(19617i32);
                self.begin_token_list(q, 4i32);
                self.begin_token_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(), 20i32);
                q = self.get_avail();
                self.mem[(q) as usize].set_hh_lh(379i32);
                self.begin_token_list(q, 4i32);
                old_mode = self.cur_list.mode_field;
                self.cur_list.mode_field = 0i32;
                self.cur_cs = self.write_loc;
                q = self.scan_toks(false, true);
                self.cur_list.mode_field = old_mode;
                self.get_token();
                if (self.cur_tok != 19617i32) {
                    // §1619
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(701i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 1912i32;
                            self.help_line[(0i32) as usize] = 1425i32;
                        }
                        self.error();
                        loop {
                            self.get_token();
                            if (self.cur_tok == 19617i32) { break; }
                        }
                    }
                }
                // §1618
                self.end_token_list();
                // §727
                h = self.def_ref;
            }
        } else {
            h = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
        }
        self.selector = 21i32;
        self.show_token_list(self.mem[(h) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        s = self.make_string();
        self.literal(s, self.mem[((p).wrapping_add(1i32)) as usize].hh().lh(), false);
        self.flush_str(s);
        if (self.mem[(p) as usize].hh().b1() == 8i32) {
            self.flush_list(self.def_ref);
        }
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_colorstack(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §727
        let mut s: str_number = 0; // §727
        let mut cmd: i32 = 0; // §727
        let mut stack_no: i32 = 0; // §727
        let mut literal_mode: i32 = 0; // §727
        cmd = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
        stack_no = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
        if (stack_no >= self.colorstackused()) {
            {
                self.print_nl(348i32);
                self.print(1129i32);
                self.print_int(((stack_no) as i64));
                self.print(1130i32);
                self.print_nl(348i32);
                return;
            }
        }
        match cmd {
            0 | 1 => {
                {
                    old_setting = self.selector;
                    self.selector = 21i32;
                    self.show_token_list(self.mem[(self.mem[((p).wrapping_add(2i32)) as usize].hh().rh()) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    if (cmd == 0i32) {
                        literal_mode = self.colorstackset(stack_no, s);
                    } else {
                        literal_mode = self.colorstackpush(stack_no, s);
                    }
                    if ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]) > 0i32) {
                        self.literal(s, literal_mode, false);
                    }
                    self.flush_str(s);
                    return;
                }
            }
            2 => {
                literal_mode = self.colorstackpop(stack_no);
            }
            3 => {
                literal_mode = self.colorstackcurrent(stack_no);
            }
            _ => {
                self.confusion(1131i32);
            }
        }
        if ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) > 0i32) {
            {
                s = self.make_string();
                self.literal(s, literal_mode, false);
                self.flush_str(s);
            }
        }
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_colorstack_startpage(&mut self) {
        let mut i: i32 = 0; // §727
        let mut max: i32 = 0; // §727
        let mut start_status: i32 = 0; // §727
        let mut literal_mode: i32 = 0; // §727
        let mut s: str_number = 0; // §727
        i = 0i32;
        max = self.colorstackused();
        while (i < max) {
            {
                start_status = self.colorstackskippagestart(i);
                if (start_status == 0i32) {
                    {
                        literal_mode = self.colorstackcurrent(i);
                        if ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) > 0i32) {
                            {
                                s = self.make_string();
                                self.literal(s, literal_mode, false);
                                self.flush_str(s);
                            }
                        }
                    }
                }
                i = (i).wrapping_add(1i32);
            }
        }
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_setmatrix(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §727
        let mut s: str_number = 0; // §727
        old_setting = self.selector;
        self.selector = 21i32;
        self.show_token_list(self.mem[(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        {
            if ((self.pool_ptr).wrapping_add(7i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.str_pool[(self.pool_ptr) as usize] = 0i32;
        if (self.pdfsetmatrix(self.str_start[(self.str_ptr) as usize], self.cur_h, (self.cur_page_height).wrapping_sub(self.cur_v)) == 1i32) {
            {
                {
                    if ((self.pool_ptr).wrapping_add(7i32) > pool_size) {
                        self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                    }
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 48i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 48i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 99i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = 109i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                s = self.make_string();
                self.literal(s, 0i32, false);
            }
        } else {
            {
                self.pdf_error(1132i32, 1133i32);
            }
        }
        self.flush_str(s);
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_save(&mut self, mut p: halfword) {
        self.checkpdfsave(self.cur_h, self.cur_v);
        self.literal(113i32, 0i32, false);
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_out_restore(&mut self, mut p: halfword) {
        self.checkpdfrestore(self.cur_h, self.cur_v);
        self.literal(81i32, 0i32, false);
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_special(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §727
        let mut s: str_number = 0; // §727
        let mut h: halfword = 0; // §727
        let mut q: halfword = 0; // §727
        let mut r: halfword = 0; // §727
        let mut old_mode: i32 = 0; // §727
        old_setting = self.selector;
        self.selector = self.selector;
        if (self.mem[(p) as usize].hh().b1() == 4i32) {
            {
                // §1618
                q = self.get_avail();
                self.mem[(q) as usize].set_hh_lh(637i32);
                r = self.get_avail();
                self.mem[(q) as usize].set_hh_rh(r);
                self.mem[(r) as usize].set_hh_lh(19617i32);
                self.begin_token_list(q, 4i32);
                self.begin_token_list(self.mem[((p).wrapping_add(1i32)) as usize].hh().rh(), 20i32);
                q = self.get_avail();
                self.mem[(q) as usize].set_hh_lh(379i32);
                self.begin_token_list(q, 4i32);
                old_mode = self.cur_list.mode_field;
                self.cur_list.mode_field = 0i32;
                self.cur_cs = self.write_loc;
                q = self.scan_toks(false, true);
                self.cur_list.mode_field = old_mode;
                self.get_token();
                if (self.cur_tok != 19617i32) {
                    // §1619
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(701i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 1912i32;
                            self.help_line[(0i32) as usize] = 1425i32;
                        }
                        self.error();
                        loop {
                            self.get_token();
                            if (self.cur_tok == 19617i32) { break; }
                        }
                    }
                }
                // §1618
                self.end_token_list();
                // §727
                h = self.def_ref;
            }
        } else {
            h = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
        }
        self.selector = 21i32;
        self.show_token_list(self.mem[(h) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        s = self.make_string();
        self.literal(s, 3i32, true);
        self.flush_str(s);
        if (self.mem[(p) as usize].hh().b1() == 4i32) {
            self.flush_list(self.def_ref);
        }
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §727
    pub fn pdf_print_toks(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §727
        s = self.tokens_to_string(p);
        if ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]) > 0i32) {
            self.pdf_print(s);
        }
        self.flush_str(s);
    }

}
