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
    // §693
    pub fn pdf_set_origin_temp(&mut self, mut h: scaled, mut v: scaled) {
        if ((((h).wrapping_sub(self.pdf_origin_h)).wrapping_abs() >= self.min_bp_val) || (((v).wrapping_sub(self.pdf_origin_v)).wrapping_abs() >= self.min_bp_val)) {
            {
                self.pdf_print(1046i32);
                self.pdf_print_bp((h).wrapping_sub(self.pdf_origin_h));
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
                self.pdf_print_bp((self.pdf_origin_v).wrapping_sub(v));
                {
                    self.pdf_print(1047i32);
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

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §693
    pub fn pdf_end_string(&mut self) {
        if self.pdf_doing_string {
            {
                self.pdf_print(1048i32);
                self.pdf_doing_string = false;
            }
        }
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §693
    pub fn pdf_end_string_nl(&mut self) {
        if self.pdf_doing_string {
            {
                {
                    self.pdf_print(1048i32);
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
                self.pdf_doing_string = false;
            }
        }
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §693
    pub fn get_font_auto_expand_ratio(&mut self, mut f: internal_font_number) -> i32 {
        let mut get_font_auto_expand_ratio: i32 = 0;
        if self.pdf_font_auto_expand[crate::ix::U((f) as usize)] {
            get_font_auto_expand_ratio = self.pdf_font_expand_ratio[crate::ix::U((f) as usize)];
        } else {
            get_font_auto_expand_ratio = 0i32;
        }
        get_font_auto_expand_ratio
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §693
    pub fn pdf_set_text_pos(&mut self, mut v: scaled, mut v_out: scaled, mut f: internal_font_number) {
        let mut pdf_new_Tm_a: i32 = 0; // §693
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
        pdf_new_Tm_a = self.get_font_auto_expand_ratio(f);
        if ((pdf_new_Tm_a != 0i32) || ((pdf_new_Tm_a == 0i32) && (self.pdf_cur_Tm_a != 0i32))) {
            {
                self.pdf_print_real((1000i32).wrapping_add(pdf_new_Tm_a), 3i32);
                self.pdf_print(1049i32);
                self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
                self.pdf_h = (self.pdf_origin_h).wrapping_add(self.scaled_out);
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
                self.pdf_print_bp((self.pdf_origin_v).wrapping_sub(self.cur_v));
                self.pdf_v = (self.pdf_origin_v).wrapping_sub(self.scaled_out);
                self.pdf_print(1050i32);
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
                self.pdf_print_real(v, self.fixed_decimal_digits);
                self.pdf_v = (self.pdf_v).wrapping_sub(v_out);
                self.pdf_print(1051i32);
            }
        }
        self.pdf_tj_start_h = self.pdf_h;
        self.pdf_delta_h = 0i32;
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §693
    pub fn pdf_use_font(&mut self, mut f: internal_font_number, mut fontnum: i32) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        {
            if (self.divide_scaled(self.font_size[crate::ix::U((f) as usize)], self.one_hundred_bp, 6i32) != 0i32) {
            }
        }
        { let __v621 = self.scaled_out; self.pdf_font_size[crate::ix::U((f) as usize)] = __v621; }
        { let __v622 = true; self.font_used[crate::ix::U((f) as usize)] = __v622; }
        self.pdfassert(((fontnum > 0i32) || ((fontnum < 0i32) && (self.pdf_font_num[crate::ix::U(((fontnum).wrapping_neg()) as usize)] > 0i32))));
        self.pdf_font_num[crate::ix::U((f) as usize)] = fontnum;
        if (__av_eqtb[crate::ix::U(((29345i32) - 1) as usize)].int() > 0i32) {
            {
                self.pdf_warning(0i32, 1052i32, true, true);
                __av_eqtb[crate::ix::U(((29345i32) - 1) as usize)].set_int(0i32);
            }
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_init_font(&mut self, mut f: internal_font_number) {
        let mut k: internal_font_number = 0; // §694
        let mut b: internal_font_number = 0; // §694
        let mut i: i32 = 0; // §694
        self.pdfassert((!self.font_used[crate::ix::U((f) as usize)]));
        if (self.pdf_font_auto_expand[crate::ix::U((f) as usize)] && (self.pdf_font_blink[crate::ix::U((f) as usize)] != null_font)) {
            {
                b = self.pdf_font_blink[crate::ix::U((f) as usize)];
                if (!self.isscalable(b)) {
                    self.pdf_error(1053i32, 1054i32);
                }
                if (!self.font_used[crate::ix::U((b) as usize)]) {
                    self.pdf_init_font(b);
                }
                { let __v623 = self.pdf_font_map[crate::ix::U((b) as usize)]; self.pdf_font_map[crate::ix::U((f) as usize)] = __v623; }
            }
        }
        if self.isscalable(f) {
            {
                i = self.head_tab[crate::ix::U(((obj_type_font) - 1) as usize)];
                while (i != 0i32) {
                    {
                        k = self.obj_tab[crate::ix::U((i) as usize)].int0;
                        if ((self.isscalable(k) && (self.pdf_font_map[crate::ix::U((k) as usize)] == self.pdf_font_map[crate::ix::U((f) as usize)])) && (self.str_eq_str(self.font_name[crate::ix::U((k) as usize)], self.font_name[crate::ix::U((f) as usize)]) || ((self.pdf_font_auto_expand[crate::ix::U((f) as usize)] && (self.pdf_font_blink[crate::ix::U((f) as usize)] != null_font)) && self.str_eq_str(self.font_name[crate::ix::U((k) as usize)], self.font_name[crate::ix::U((self.pdf_font_blink[crate::ix::U((f) as usize)]) as usize)])))) {
                            {
                                self.pdfassert((self.pdf_font_num[crate::ix::U((k) as usize)] != 0i32));
                                if (self.pdf_font_num[crate::ix::U((k) as usize)] < 0i32) {
                                    self.pdf_use_font(f, self.pdf_font_num[crate::ix::U((k) as usize)]);
                                } else {
                                    self.pdf_use_font(f, (k).wrapping_neg());
                                }
                                return;
                            }
                        }
                        i = self.obj_tab[crate::ix::U((i) as usize)].int1;
                    }
                }
            }
        }
        self.pdf_create_obj(obj_type_font, f);
        { let __v624 = (self.hasspacechar(f) && ({ let __s626 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s625 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, 32i32))) as usize; self.font_info[crate::ix::U(__s625)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s626)] }.int() > self.one_bp)); self.pdf_font_has_space_char[crate::ix::U((f) as usize)] = __v624; }
        self.pdf_use_font(f, self.obj_ptr);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_init_font_cur_val(&mut self) {
        self.pdf_init_font(self.cur_val);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_set_font(&mut self, mut f: internal_font_number) {
        let mut p: halfword = 0; // §694
        let mut k: internal_font_number = 0; // §694
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_found_f: {
            self.pdf_f = f;
            if (!self.font_used[crate::ix::U((f) as usize)]) {
                self.pdf_init_font(f);
            }
            {
                if (self.pdf_font_num[crate::ix::U((f) as usize)] < 0i32) {
                    self.ff = (self.pdf_font_num[crate::ix::U((f) as usize)]).wrapping_neg();
                } else {
                    self.ff = f;
                }
            }
            k = self.ff;
            p = self.pdf_font_list;
            while (p != null) {
                {
                    {
                        if (self.pdf_font_num[crate::ix::U((__av_mem[crate::ix::U((p) as usize)].hh().lh()) as usize)] < 0i32) {
                            self.ff = (self.pdf_font_num[crate::ix::U((__av_mem[crate::ix::U((p) as usize)].hh().lh()) as usize)]).wrapping_neg();
                        } else {
                            self.ff = __av_mem[crate::ix::U((p) as usize)].hh().lh();
                        }
                    }
                    if (self.ff == k) {
                        break 'l_found_f;
                    }
                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
            {
                self.pdf_append_list_arg = f;
                self.pdf_font_list = self.append_ptr(self.pdf_font_list, self.pdf_append_list_arg);
            }
        }
        if ((k == self.pdf_last_f) && (self.font_size[crate::ix::U((f) as usize)] == self.pdf_last_fs)) {
            return;
        }
        self.pdf_print(1055i32);
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
        { let __a627_0 = self.divide_scaled(self.font_size[crate::ix::U((f) as usize)], self.one_hundred_bp, 6i32); let __a627_1 = 4i32; self.pdf_print_real(__a627_0, __a627_1) };
        self.pdf_print(1056i32);
        self.pdf_last_f = k;
        self.pdf_last_fs = self.font_size[crate::ix::U((f) as usize)];
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_begin_text(&mut self) {
        self.pdf_set_origin(0i32, self.cur_page_height);
        {
            self.pdf_print(1057i32);
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
        self.pdf_doing_text = true;
        self.pdf_f = null_font;
        self.pdf_last_f = null_font;
        self.pdf_last_fs = 0i32;
        self.pdf_doing_string = false;
        self.pdf_cur_Tm_a = 0i32;
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_read_dummy_font(&mut self) {
        if (self.pdf_dummy_font == null_font) {
            {
                self.pdf_dummy_font = self.read_font_info(null_cs, self.pdf_space_font_name, 345i32, (1000i32).wrapping_neg());
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
    // §694
    pub fn pdf_insert_interword_space(&mut self) {
        self.pdf_read_dummy_font();
        self.pdf_set_font(self.pdf_dummy_font);
        self.pdf_print(1058i32);
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_begin_string(&mut self, mut f: internal_font_number) {
        let mut s_out: scaled = 0; // §694
        let mut v: scaled = 0; // §694
        let mut v_out: scaled = 0; // §694
        let mut s: i32 = 0; // §694
        let mut must_end_string: bool = false; // §694
        let mut must_insert_space: bool = false; // §694
        let mut must_set_text_pos: bool = false; // §694
        let mut move_looks_like_interword_space: bool = false; // §694
        must_end_string = false;
        must_insert_space = false;
        must_set_text_pos = false;
        if (!self.pdf_doing_text) {
            {
                self.pdf_begin_text();
                must_set_text_pos = true;
            }
        }
        if (((!self.gen_faked_interword_space) && (self.pdf_f != f)) || (self.gen_faked_interword_space && (!(self.font_used[crate::ix::U((f) as usize)] && self.font_used[crate::ix::U((self.pdf_f) as usize)])))) {
            {
                self.pdf_end_string();
                self.pdf_set_font(f);
            }
        }
        if (self.pdf_cur_Tm_a == 0i32) {
            {
                s = self.divide_scaled((self.cur_h).wrapping_sub((self.pdf_tj_start_h).wrapping_add(self.pdf_delta_h)), self.pdf_font_size[crate::ix::U((f) as usize)], 3i32);
                s_out = self.scaled_out;
            }
        } else {
            {
                s = { let __a628_0 = self.round_xn_over_d((self.cur_h).wrapping_sub((self.pdf_tj_start_h).wrapping_add(self.pdf_delta_h)), 1000i32, (1000i32).wrapping_add(self.pdf_cur_Tm_a)); let __a628_1 = self.pdf_font_size[crate::ix::U((f) as usize)]; let __a628_2 = 3i32; self.divide_scaled(__a628_0, __a628_1, __a628_2) };
                if ((s).wrapping_abs() < 32768i32) {
                    {
                        s_out = { let __a629_0 = self.round_xn_over_d(self.pdf_font_size[crate::ix::U((f) as usize)], (s).wrapping_abs(), 1000i32); let __a629_1 = (1000i32).wrapping_add(self.pdf_cur_Tm_a); let __a629_2 = 1000i32; self.round_xn_over_d(__a629_0, __a629_1, __a629_2) };
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
        move_looks_like_interword_space = (((self.font_info[crate::ix::U(((space_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int() > self.one_bp) && (s_out > ((self.font_info[crate::ix::U(((space_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()).wrapping_sub(self.font_info[crate::ix::U(((space_shrink_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int())).wrapping_sub((self.one_bp / 10i32)))) && (v == 0i32));
        if (self.gen_faked_interword_space && move_looks_like_interword_space) {
            {
                must_insert_space = true;
            }
        }
        if must_insert_space {
            {
                if (self.pdf_font_has_space_char[crate::ix::U((f) as usize)] && self.pdf_doing_string) {
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
                if (must_insert_space && (!self.pdf_font_has_space_char[crate::ix::U((f) as usize)])) {
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
                self.pdf_print(1059i32);
                if (s == 0i32) {
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
                                    self.overflow(1018i32, pdf_op_buf_size);
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
                                self.overflow(1018i32, pdf_op_buf_size);
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
    // §694
    pub fn pdf_insert_fake_space(&mut self) {
        let mut s: i32 = 0; // §694
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
    // §694
    pub fn pdf_end_text(&mut self) {
        if self.pdf_doing_text {
            {
                self.pdf_end_string_nl();
                {
                    self.pdf_print(1060i32);
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
                self.pdf_doing_text = false;
            }
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
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
        if (h <= self.one_bp) {
            {
                self.pdf_set_origin_temp(x, (((((y) as f64) - ((((h).wrapping_add(1i32)) as f64) / ((2i32) as f64)))) as i32));
                self.pdf_print(1061i32);
                self.pdf_print_bp(h);
                self.pdf_print(1062i32);
                self.pdf_print_bp(w);
                {
                    self.pdf_print(1063i32);
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
            if (w <= self.one_bp) {
                {
                    self.pdf_set_origin_temp((((((x) as f64) + ((((w).wrapping_add(1i32)) as f64) / ((2i32) as f64)))) as i32), y);
                    self.pdf_print(1061i32);
                    self.pdf_print_bp(w);
                    self.pdf_print(1064i32);
                    self.pdf_print_bp(h);
                    {
                        self.pdf_print(1065i32);
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
                    self.pdf_set_origin_temp(x, y);
                    self.pdf_print(1066i32);
                    self.pdf_print_bp(w);
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
                    self.pdf_print_bp(h);
                    {
                        self.pdf_print(1067i32);
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
        {
            self.pdf_print(81i32);
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

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn pdf_rectangle(&mut self, mut left: scaled, mut top: scaled, mut right: scaled, mut bottom: scaled) {
        self.prepare_mag();
        self.pdf_print(1068i32);
        self.pdf_print_mag_bp((left).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(bottom));
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
        self.pdf_print_mag_bp((right).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(top));
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
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn slow_print_substr(&mut self, mut s: i32, mut max_len: i32) {
        let mut j: pool_pointer = 0; // §694
        if ((s >= self.str_ptr) || (s < 256i32)) {
            self.print(s);
        } else {
            {
                j = self.str_start[crate::ix::U((s) as usize)];
                while ((j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) && (j <= (self.str_start[crate::ix::U((s) as usize)]).wrapping_add(max_len))) {
                    {
                        self.print(self.str_pool[crate::ix::U((j) as usize)]);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
            self.print(277i32);
        }
    }

    /// To set PDF font we need to find out fonts with the same name, because \TeX\
    /// can load the same font several times for various sizes. For such fonts we
    /// define only one font resource. The array `pdf_font_num` holds the object
    /// number of font resource. A negative value of an entry of `pdf_font_num`
    /// indicates that the corresponding font shares the font resource with the font.
    // §694
    pub fn literal(&mut self, mut s: str_number, mut literal_mode: i32, mut warn: bool) {
        let mut j: pool_pointer = 0; // §694
        j = self.str_start[crate::ix::U((s) as usize)];
        if (literal_mode == scan_special) {
            {
                if (!(self.str_in_str(s, 1069i32, 0i32) || self.str_in_str(s, 1070i32, 0i32))) {
                    {
                        if (warn && (!((self.str_in_str(s, 1071i32, 0i32) || self.str_in_str(s, 1072i32, 0i32)) || ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) == 0i32)))) {
                            {
                                self.print_nl(1073i32);
                                self.print_nl(1074i32);
                                self.slow_print_substr(s, 64i32);
                                self.print_ln();
                            }
                        }
                        return;
                    }
                }
                j = (j).wrapping_add((self.str_start[crate::ix::U((1070i32) as usize)]).wrapping_sub(self.str_start[crate::ix::U((1069i32) as usize)]));
                if self.str_in_str(s, 1075i32, (self.str_start[crate::ix::U((1070i32) as usize)]).wrapping_sub(self.str_start[crate::ix::U((1069i32) as usize)])) {
                    {
                        j = (j).wrapping_add((self.str_start[crate::ix::U((1076i32) as usize)]).wrapping_sub(self.str_start[crate::ix::U((1075i32) as usize)]));
                        literal_mode = direct_always;
                    }
                } else {
                    if self.str_in_str(s, 1076i32, (self.str_start[crate::ix::U((1070i32) as usize)]).wrapping_sub(self.str_start[crate::ix::U((1069i32) as usize)])) {
                        {
                            j = (j).wrapping_add((self.str_start[crate::ix::U((1077i32) as usize)]).wrapping_sub(self.str_start[crate::ix::U((1076i32) as usize)]));
                            literal_mode = direct_page;
                        }
                    } else {
                        literal_mode = set_origin;
                    }
                }
            }
        }
        match literal_mode {
            set_origin => {
                {
                    self.pdf_end_text();
                    self.pdf_set_origin(self.cur_h, self.cur_v);
                }
            }
            direct_page => {
                self.pdf_end_text();
            }
            direct_always => {
                self.pdf_end_string_nl();
            }
            _ => {
                self.confusion(1077i32);
            }
        }
        while (j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
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
                        self.pdf_buf_set(self.pdf_ptr, self.str_pool[crate::ix::U((j) as usize)]);
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

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_print_fw_int(&mut self, mut n: longinteger, mut w: i32) {
        let mut k: i32 = 0; // §703
        k = 0i32;
        loop {
            self.dig[crate::ix::U((k) as usize)] = (((n % ((10i32) as i64))) as i32);
            n = (n / ((10i32) as i64));
            k = (k).wrapping_add(1i32);
            if (k == w) { break; }
        }
        {
            if (self.pdf_os_mode && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                self.pdf_os_get_os_buf(k);
            } else {
                if ((!self.pdf_os_mode) && (k > self.pdf_buf_size)) {
                    self.overflow(1018i32, pdf_op_buf_size);
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
                    self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add(self.dig[crate::ix::U((k) as usize)]));
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_out_bytes(&mut self, mut n: longinteger, mut w: i32) {
        let mut k: i32 = 0; // §703
        let mut byte: Vec<i32> = vec![0; 8]; // §703
        k = 0i32;
        loop {
            byte[crate::ix::U((k) as usize)] = (((n % ((256i32) as i64))) as i32);
            n = (n / ((256i32) as i64));
            k = (k).wrapping_add(1i32);
            if (k == w) { break; }
        }
        {
            if (self.pdf_os_mode && ((k).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                self.pdf_os_get_os_buf(k);
            } else {
                if ((!self.pdf_os_mode) && (k > self.pdf_buf_size)) {
                    self.overflow(1018i32, pdf_op_buf_size);
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
                    self.pdf_buf_set(self.pdf_ptr, byte[crate::ix::U((k) as usize)]);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_int_entry(&mut self, mut s: str_number, mut v: i32) {
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
        self.pdf_print_int(((v) as i64));
    }

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_int_entry_ln(&mut self, mut s: str_number, mut v: i32) {
        self.pdf_int_entry(s, v);
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

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_indirect(&mut self, mut s: str_number, mut o: i32) {
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
        self.pdf_print_int(((o) as i64));
        self.pdf_print(1087i32);
    }

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_indirect_ln(&mut self, mut s: str_number, mut o: i32) {
        self.pdf_indirect(s, o);
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

    /// Subroutines to print out various PDF objects:
    // §703
    pub fn pdf_print_str(&mut self, mut s: str_number) {
        let mut i: pool_pointer = 0; // §703
        let mut j: pool_pointer = 0; // §703
        let mut is_hex_string: bool = false; // §703
        'l_done_f: {
            i = self.str_start[crate::ix::U((s) as usize)];
            j = ((i).wrapping_add((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]))).wrapping_sub(1i32);
            if (i > j) {
                {
                    self.pdf_print(1088i32);
                    return;
                }
            }
            if ((self.str_pool[crate::ix::U((i) as usize)] == ((b'(') as i32)) && (self.str_pool[crate::ix::U((j) as usize)] == ((b')') as i32))) {
                {
                    self.pdf_print(s);
                    return;
                }
            }
            is_hex_string = false;
            if (((self.str_pool[crate::ix::U((i) as usize)] != ((b'<') as i32)) || (self.str_pool[crate::ix::U((j) as usize)] != ((b'>') as i32))) || ((((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)])) % 2) != 0)) {
                break 'l_done_f;
            }
            i = (i).wrapping_add(1i32);
            j = (j).wrapping_sub(1i32);
            while (i < j) {
                {
                    if (((((self.str_pool[crate::ix::U((i) as usize)] >= ((b'0') as i32)) && (self.str_pool[crate::ix::U((i) as usize)] <= ((b'9') as i32))) || ((self.str_pool[crate::ix::U((i) as usize)] >= ((b'A') as i32)) && (self.str_pool[crate::ix::U((i) as usize)] <= ((b'F') as i32)))) || ((self.str_pool[crate::ix::U((i) as usize)] >= ((b'a') as i32)) && (self.str_pool[crate::ix::U((i) as usize)] <= ((b'f') as i32)))) && ((((self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] >= ((b'0') as i32)) && (self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] <= ((b'9') as i32))) || ((self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] >= ((b'A') as i32)) && (self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] <= ((b'F') as i32)))) || ((self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] >= ((b'a') as i32)) && (self.str_pool[crate::ix::U(((i).wrapping_add(1i32)) as usize)] <= ((b'f') as i32))))) {
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
                                self.overflow(1018i32, pdf_op_buf_size);
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
                                self.overflow(1018i32, pdf_op_buf_size);
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
    // §703
    pub fn pdf_print_str_ln(&mut self, mut s: str_number) {
        self.pdf_print_str(s);
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

    /// Subroutines to print out various PDF objects:
    // §703
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
                        self.overflow(1018i32, pdf_op_buf_size);
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
        self.pdf_print_str(v);
    }

    /// Subroutines to print out various PDF objects:
    // §703
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

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_tag_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        let mut fixedi: i32 = 0; // §706
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
            {
                fixedi = (self.fix_int(i, (7i32).wrapping_neg(), 0i32)).wrapping_abs();
                if (fixedi >= 4i32) {
                    {
                        if (({ let __s630 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s630)] }.qqqq().b2() % 4i32) == ext_tag) {
                            { let __ix631 = (self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c)); let __v632 = ({ let __s633 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s633)] }.qqqq().b2()).wrapping_sub(3i32); self.font_info[crate::ix::U((__ix631) as usize)].set_qqqq_b2(__v632); }
                        }
                        fixedi = (fixedi).wrapping_sub(4i32);
                    }
                }
                if (fixedi >= 2i32) {
                    {
                        if (({ let __s634 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s634)] }.qqqq().b2() % 4i32) == list_tag) {
                            { let __ix635 = (self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c)); let __v636 = ({ let __s637 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s637)] }.qqqq().b2()).wrapping_sub(2i32); self.font_info[crate::ix::U((__ix635) as usize)].set_qqqq_b2(__v636); }
                        }
                        fixedi = (fixedi).wrapping_sub(2i32);
                    }
                }
                if (fixedi >= 1i32) {
                    {
                        if (({ let __s638 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s638)] }.qqqq().b2() % 4i32) == lig_tag) {
                            { let __ix639 = (self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c)); let __v640 = ({ let __s641 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s641)] }.qqqq().b2()).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix639) as usize)].set_qqqq_b2(__v640); }
                        }
                    }
                }
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_no_ligatures(&mut self, mut f: internal_font_number) {
        let mut c: i32 = 0; // §706
        {
            let __for_end_2 = self.font_ec[crate::ix::U((f) as usize)];
            c = self.font_bc[crate::ix::U((f) as usize)];
            while c <= __for_end_2 {
                if (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword) {
                    if ((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b2() % 4i32) == lig_tag) {
                        { let __ix642 = (self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v643 = (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b2()).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix642) as usize)].set_qqqq_b2(__v643); }
                    }
                }
                c = c.wrapping_add(1);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn init_font_base(&mut self, mut v: i32) -> i32 {
        let mut init_font_base: i32 = 0;
        let mut i: i32 = 0; // §706
        let mut j: i32 = 0; // §706
        i = self.pdf_get_mem(256i32);
        {
            let __for_end_2 = 255i32;
            j = 0i32;
            while j <= __for_end_2 {
                self.pdf_mem[crate::ix::U(((i).wrapping_add(j)) as usize)] = v;
                j = j.wrapping_add(1);
            }
        }
        init_font_base = i;
        init_font_base
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_lp_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_lp_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v644 = self.init_font_base(0i32); self.pdf_font_lp_base[crate::ix::U((f) as usize)] = __v644; }
        }
        { let __ix645 = (self.pdf_font_lp_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v646 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix645) as usize)] = __v646; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_rp_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_rp_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v647 = self.init_font_base(0i32); self.pdf_font_rp_base[crate::ix::U((f) as usize)] = __v647; }
        }
        { let __ix648 = (self.pdf_font_rp_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v649 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix648) as usize)] = __v649; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_ef_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_ef_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v650 = self.init_font_base(1000i32); self.pdf_font_ef_base[crate::ix::U((f) as usize)] = __v650; }
        }
        { let __ix651 = (self.pdf_font_ef_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v652 = self.fix_int(i, 0i32, 1000i32); self.pdf_mem[crate::ix::U((__ix651) as usize)] = __v652; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_kn_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v653 = self.init_font_base(0i32); self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)] = __v653; }
        }
        { let __ix654 = (self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v655 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix654) as usize)] = __v655; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_st_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_st_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v656 = self.init_font_base(0i32); self.pdf_font_st_bs_base[crate::ix::U((f) as usize)] = __v656; }
        }
        { let __ix657 = (self.pdf_font_st_bs_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v658 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix657) as usize)] = __v658; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_sh_bs_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v659 = self.init_font_base(0i32); self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)] = __v659; }
        }
        { let __ix660 = (self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v661 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix660) as usize)] = __v661; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_kn_bc_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v662 = self.init_font_base(0i32); self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)] = __v662; }
        }
        { let __ix663 = (self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v664 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix663) as usize)] = __v664; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_kn_ac_code(&mut self, mut f: internal_font_number, mut c: eight_bits, mut i: i32) {
        if (self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v665 = self.init_font_base(0i32); self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)] = __v665; }
        }
        { let __ix666 = (self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)]).wrapping_add(c); let __v667 = self.fix_int(i, (1000i32).wrapping_neg(), 1000i32); self.pdf_mem[crate::ix::U((__ix666) as usize)] = __v667; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn adjust_interword_glue(&mut self, mut p: halfword, mut g: halfword) {
        let mut kn: scaled = 0; // §706
        let mut st: scaled = 0; // §706
        let mut sh: scaled = 0; // §706
        let mut q: halfword = 0; // §706
        let mut r: halfword = 0; // §706
        let mut c: halfword = 0; // §706
        let mut f: internal_font_number = 0; // §706
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!((!(g >= self.hi_mem_min)) && (__av_mem[crate::ix::U((g) as usize)].hh().b0() == glue_node))) {
            {
                self.pdf_warning(1090i32, 1091i32, true, true);
                return;
            }
        }
        c = non_char;
        if (p >= self.hi_mem_min) {
            {
                c = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                if (c > 32767i32) {
                    c = (c).wrapping_sub(65536i32);
                }
                f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
            }
        } else {
            if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == ligature_node) {
                {
                    c = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b1();
                    f = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b0();
                }
            } else {
                if (((__av_mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && (__av_mem[crate::ix::U((p) as usize)].hh().b1() == auto_kern)) && (self.save_tail != null)) {
                    {
                        r = self.save_tail;
                        while ((__av_mem[crate::ix::U((r) as usize)].hh().rh() != null) && (__av_mem[crate::ix::U((r) as usize)].hh().rh() != p)) {
                            r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                        }
                        if (__av_mem[crate::ix::U((r) as usize)].hh().rh() == p) {
                            if (r >= self.hi_mem_min) {
                                {
                                    c = __av_mem[crate::ix::U((r) as usize)].hh().b1();
                                    if (c > 32767i32) {
                                        c = (c).wrapping_sub(65536i32);
                                    }
                                    f = __av_mem[crate::ix::U((r) as usize)].hh().b0();
                                }
                            } else {
                                if (__av_mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node) {
                                    {
                                        c = __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().b1();
                                        f = __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].hh().b0();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if (c == non_char) {
            return;
        }
        kn = self.get_kn_bs_code(f, c);
        st = self.get_st_bs_code(f, c);
        sh = self.get_sh_bs_code(f, c);
        if (((kn != 0i32) || (st != 0i32)) || (sh != 0i32)) {
            {
                q = self.new_spec(__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].hh().lh());
                self.delete_glue_ref(__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].hh().lh());
                { let __v668 = (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), kn, 1000i32)); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v668); }
                { let __v669 = (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), st, 1000i32)); __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v669); }
                { let __v670 = (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), sh, 1000i32)); __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v670); }
                __av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].set_hh_lh(q);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn get_auto_kern(&mut self, mut f: internal_font_number, mut l: halfword, mut r: halfword) -> halfword {
        let mut get_auto_kern: halfword = 0;
        let mut tmp_w: scaled = 0; // §706
        let mut k: i32 = 0; // §706
        let mut p: halfword = 0; // §706
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.pdfassert(((l >= 0i32) && (r >= 0i32)));
        get_auto_kern = null;
        if ((__av_eqtb[crate::ix::U(((29366i32) - 1) as usize)].int() <= 0i32) && (__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() <= 0i32)) {
            return get_auto_kern;
        }
        tmp_w = 0i32;
        if ((__av_eqtb[crate::ix::U(((29366i32) - 1) as usize)].int() > 0i32) && (l < non_char)) {
            {
                k = self.get_kn_ac_code(f, l);
                if (k != 0i32) {
                    tmp_w = self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), k, 1000i32);
                }
            }
        }
        if ((__av_eqtb[crate::ix::U(((29365i32) - 1) as usize)].int() > 0i32) && (r < non_char)) {
            {
                k = self.get_kn_bc_code(f, r);
                if (k != 0i32) {
                    tmp_w = (tmp_w).wrapping_add(self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), k, 1000i32));
                }
            }
        }
        if (tmp_w != 0i32) {
            {
                p = self.new_kern(tmp_w);
                __av_mem[crate::ix::U((p) as usize)].set_hh_b1(auto_kern);
                get_auto_kern = p;
            }
        }
        get_auto_kern
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn expand_font_name(&mut self, mut f: internal_font_number, mut e: i32) -> str_number {
        let mut expand_font_name: str_number = 0;
        let mut old_setting: i32 = 0; // §706
        old_setting = self.selector;
        self.selector = new_string;
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        if (e > 0i32) {
            self.print(43i32);
        }
        self.print_int(((e) as i64));
        self.selector = old_setting;
        expand_font_name = self.make_string();
        expand_font_name
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn auto_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut auto_expand_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §706
        let mut nw: i32 = 0; // §706
        let mut nk: i32 = 0; // §706
        let mut ni: i32 = 0; // §706
        let mut i: i32 = 0; // §706
        k = (self.font_ptr).wrapping_add(1i32);
        self.font_ptr = (self.font_ptr).wrapping_add(1i32);
        if (self.font_ptr >= font_max) {
            self.overflow(1092i32, font_max);
        }
        { let __v671 = self.expand_font_name(f, e); self.font_name[crate::ix::U((k) as usize)] = __v671; }
        { let __v672 = self.font_area[crate::ix::U((f) as usize)]; self.font_area[crate::ix::U((k) as usize)] = __v672; }
        { let __v673 = self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].rh(); self.hash[crate::ix::U((((font_id_base).wrapping_add(k)) - 514) as usize)].set_rh(__v673); }
        { let __v674 = self.hyphen_char[crate::ix::U((f) as usize)]; self.hyphen_char[crate::ix::U((k) as usize)] = __v674; }
        { let __v675 = self.skew_char[crate::ix::U((f) as usize)]; self.skew_char[crate::ix::U((k) as usize)] = __v675; }
        { let __v676 = self.font_bchar[crate::ix::U((f) as usize)]; self.font_bchar[crate::ix::U((k) as usize)] = __v676; }
        { let __v677 = self.font_false_bchar[crate::ix::U((f) as usize)]; self.font_false_bchar[crate::ix::U((k) as usize)] = __v677; }
        { let __v678 = self.font_bc[crate::ix::U((f) as usize)]; self.font_bc[crate::ix::U((k) as usize)] = __v678; }
        { let __v679 = self.font_ec[crate::ix::U((f) as usize)]; self.font_ec[crate::ix::U((k) as usize)] = __v679; }
        { let __v680 = self.font_size[crate::ix::U((f) as usize)]; self.font_size[crate::ix::U((k) as usize)] = __v680; }
        { let __v681 = self.font_dsize[crate::ix::U((f) as usize)]; self.font_dsize[crate::ix::U((k) as usize)] = __v681; }
        { let __v682 = self.font_params[crate::ix::U((f) as usize)]; self.font_params[crate::ix::U((k) as usize)] = __v682; }
        { let __v683 = self.font_glue[crate::ix::U((f) as usize)]; self.font_glue[crate::ix::U((k) as usize)] = __v683; }
        { let __v684 = self.bchar_label[crate::ix::U((f) as usize)]; self.bchar_label[crate::ix::U((k) as usize)] = __v684; }
        { let __v685 = self.char_base[crate::ix::U((f) as usize)]; self.char_base[crate::ix::U((k) as usize)] = __v685; }
        { let __v686 = self.height_base[crate::ix::U((f) as usize)]; self.height_base[crate::ix::U((k) as usize)] = __v686; }
        { let __v687 = self.depth_base[crate::ix::U((f) as usize)]; self.depth_base[crate::ix::U((k) as usize)] = __v687; }
        { let __v688 = self.lig_kern_base[crate::ix::U((f) as usize)]; self.lig_kern_base[crate::ix::U((k) as usize)] = __v688; }
        { let __v689 = self.exten_base[crate::ix::U((f) as usize)]; self.exten_base[crate::ix::U((k) as usize)] = __v689; }
        { let __v690 = self.param_base[crate::ix::U((f) as usize)]; self.param_base[crate::ix::U((k) as usize)] = __v690; }
        nw = (self.height_base[crate::ix::U((f) as usize)]).wrapping_sub(self.width_base[crate::ix::U((f) as usize)]);
        ni = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_sub(self.italic_base[crate::ix::U((f) as usize)]);
        nk = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_sub((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32)));
        if ((((self.fmem_ptr).wrapping_add(nw)).wrapping_add(ni)).wrapping_add(nk) >= font_mem_size) {
            self.overflow(1093i32, font_mem_size);
        }
        { let __v691 = self.fmem_ptr; self.width_base[crate::ix::U((k) as usize)] = __v691; }
        { let __v692 = (self.width_base[crate::ix::U((k) as usize)]).wrapping_add(nw); self.italic_base[crate::ix::U((k) as usize)] = __v692; }
        { let __v693 = ((self.italic_base[crate::ix::U((k) as usize)]).wrapping_add(ni)).wrapping_sub((256i32).wrapping_mul(128i32)); self.kern_base[crate::ix::U((k) as usize)] = __v693; }
        self.fmem_ptr = (((self.fmem_ptr).wrapping_add(nw)).wrapping_add(ni)).wrapping_add(nk);
        {
            let __for_end_2 = (nw).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix694 = (self.width_base[crate::ix::U((k) as usize)]).wrapping_add(i); let __v695 = self.round_xn_over_d(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i)) as usize)].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[crate::ix::U((__ix694) as usize)].set_int(__v695); }
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (ni).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix696 = (self.italic_base[crate::ix::U((k) as usize)]).wrapping_add(i); let __v697 = self.round_xn_over_d(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(i)) as usize)].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[crate::ix::U((__ix696) as usize)].set_int(__v697); }
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (nk).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix698 = ((self.kern_base[crate::ix::U((k) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(i); let __v699 = self.round_xn_over_d(self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(i)) as usize)].int(), (1000i32).wrapping_add(e), 1000i32); self.font_info[crate::ix::U((__ix698) as usize)].set_int(__v699); }
                i = i.wrapping_add(1);
            }
        }
        auto_expand_font = k;
        auto_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn copy_expand_params(&mut self, mut k: internal_font_number, mut f: internal_font_number, mut e: i32) {
        if (self.pdf_font_rp_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v700 = self.init_font_base(0i32); self.pdf_font_rp_base[crate::ix::U((f) as usize)] = __v700; }
        }
        if (self.pdf_font_lp_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v701 = self.init_font_base(0i32); self.pdf_font_lp_base[crate::ix::U((f) as usize)] = __v701; }
        }
        if (self.pdf_font_ef_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v702 = self.init_font_base(1000i32); self.pdf_font_ef_base[crate::ix::U((f) as usize)] = __v702; }
        }
        self.pdf_font_expand_ratio[crate::ix::U((k) as usize)] = e;
        { let __v703 = self.pdf_font_step[crate::ix::U((f) as usize)]; self.pdf_font_step[crate::ix::U((k) as usize)] = __v703; }
        { let __v704 = self.pdf_font_auto_expand[crate::ix::U((f) as usize)]; self.pdf_font_auto_expand[crate::ix::U((k) as usize)] = __v704; }
        self.pdf_font_blink[crate::ix::U((k) as usize)] = f;
        { let __v705 = self.pdf_font_lp_base[crate::ix::U((f) as usize)]; self.pdf_font_lp_base[crate::ix::U((k) as usize)] = __v705; }
        { let __v706 = self.pdf_font_rp_base[crate::ix::U((f) as usize)]; self.pdf_font_rp_base[crate::ix::U((k) as usize)] = __v706; }
        { let __v707 = self.pdf_font_ef_base[crate::ix::U((f) as usize)]; self.pdf_font_ef_base[crate::ix::U((k) as usize)] = __v707; }
        if (self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v708 = self.init_font_base(0i32); self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)] = __v708; }
        }
        if (self.pdf_font_st_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v709 = self.init_font_base(0i32); self.pdf_font_st_bs_base[crate::ix::U((f) as usize)] = __v709; }
        }
        if (self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v710 = self.init_font_base(0i32); self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)] = __v710; }
        }
        if (self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v711 = self.init_font_base(0i32); self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)] = __v711; }
        }
        if (self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)] == 0i32) {
            { let __v712 = self.init_font_base(0i32); self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)] = __v712; }
        }
        { let __v713 = self.pdf_font_kn_bs_base[crate::ix::U((f) as usize)]; self.pdf_font_kn_bs_base[crate::ix::U((k) as usize)] = __v713; }
        { let __v714 = self.pdf_font_st_bs_base[crate::ix::U((f) as usize)]; self.pdf_font_st_bs_base[crate::ix::U((k) as usize)] = __v714; }
        { let __v715 = self.pdf_font_sh_bs_base[crate::ix::U((f) as usize)]; self.pdf_font_sh_bs_base[crate::ix::U((k) as usize)] = __v715; }
        { let __v716 = self.pdf_font_kn_bc_base[crate::ix::U((f) as usize)]; self.pdf_font_kn_bc_base[crate::ix::U((k) as usize)] = __v716; }
        { let __v717 = self.pdf_font_kn_ac_base[crate::ix::U((f) as usize)]; self.pdf_font_kn_ac_base[crate::ix::U((k) as usize)] = __v717; }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn tfm_lookup(&mut self, mut s: str_number, mut fs: scaled) -> internal_font_number {
        let mut tfm_lookup: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §706
        if (fs != 0i32) {
            {
                {
                    let __for_end_4 = self.font_ptr;
                    k = 1i32;
                    while k <= __for_end_4 {
                        if (((self.font_area[crate::ix::U((k) as usize)] != non_existent_path) && self.str_eq_str(self.font_name[crate::ix::U((k) as usize)], s)) && (self.font_size[crate::ix::U((k) as usize)] == fs)) {
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
                        if ((self.font_area[crate::ix::U((k) as usize)] != non_existent_path) && self.str_eq_str(self.font_name[crate::ix::U((k) as usize)], s)) {
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
        tfm_lookup = null_font;
        tfm_lookup
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn load_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut load_expand_font: internal_font_number = 0;
        let mut s: str_number = 0; // §706
        let mut k: internal_font_number = 0; // §706
        s = self.expand_font_name(f, e);
        k = self.tfm_lookup(s, self.font_size[crate::ix::U((f) as usize)]);
        if (k == null_font) {
            {
                if self.pdf_font_auto_expand[crate::ix::U((f) as usize)] {
                    k = self.auto_expand_font(f, e);
                } else {
                    k = self.read_font_info(null_cs, s, 345i32, self.font_size[crate::ix::U((f) as usize)]);
                }
            }
        }
        if (k != null_font) {
            self.copy_expand_params(k, f, e);
        }
        load_expand_font = k;
        load_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn fix_expand_value(&mut self, mut f: internal_font_number, mut e: i32) -> i32 {
        let mut fix_expand_value: i32 = 0;
        let mut step: i32 = 0; // §706
        let mut max_expand: i32 = 0; // §706
        let mut neg: bool = false; // §706
        fix_expand_value = 0i32;
        if (e == 0i32) {
            return fix_expand_value;
        }
        if (e < 0i32) {
            {
                e = (e).wrapping_neg();
                neg = true;
                max_expand = (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg();
            }
        } else {
            {
                neg = false;
                max_expand = self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)];
            }
        }
        if (e > max_expand) {
            e = max_expand;
        } else {
            {
                step = self.pdf_font_step[crate::ix::U((f) as usize)];
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
    // §706
    pub fn get_expand_font(&mut self, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut get_expand_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §706
        k = self.pdf_font_elink[crate::ix::U((f) as usize)];
        while (k != null_font) {
            {
                if (self.pdf_font_expand_ratio[crate::ix::U((k) as usize)] == e) {
                    {
                        get_expand_font = k;
                        return get_expand_font;
                    }
                }
                k = self.pdf_font_elink[crate::ix::U((k) as usize)];
            }
        }
        k = self.load_expand_font(f, e);
        { let __v718 = self.pdf_font_elink[crate::ix::U((f) as usize)]; self.pdf_font_elink[crate::ix::U((k) as usize)] = __v718; }
        self.pdf_font_elink[crate::ix::U((f) as usize)] = k;
        get_expand_font = k;
        get_expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
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
        if (self.pdf_font_elink[crate::ix::U((f) as usize)] == null_font) {
            self.pdf_error(1053i32, 1094i32);
        }
        expand_font = self.get_expand_font(f, e);
        expand_font
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn set_expand_params(&mut self, mut f: internal_font_number, mut auto_expand: bool, mut stretch_limit: i32, mut shrink_limit: i32, mut font_step: i32, mut expand_ratio: i32) {
        self.pdf_font_step[crate::ix::U((f) as usize)] = font_step;
        self.pdf_font_auto_expand[crate::ix::U((f) as usize)] = auto_expand;
        if (stretch_limit > 0i32) {
            { let __v719 = self.get_expand_font(f, stretch_limit); self.pdf_font_stretch[crate::ix::U((f) as usize)] = __v719; }
        }
        if (shrink_limit > 0i32) {
            { let __v720 = self.get_expand_font(f, (shrink_limit).wrapping_neg()); self.pdf_font_shrink[crate::ix::U((f) as usize)] = __v720; }
        }
        if (expand_ratio != 0i32) {
            self.pdf_font_expand_ratio[crate::ix::U((f) as usize)] = expand_ratio;
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn vf_expand_local_fonts(&mut self, mut f: internal_font_number) {
        let mut lf: internal_font_number = 0; // §706
        let mut k: i32 = 0; // §706
        self.pdfassert((self.pdf_font_type[crate::ix::U((f) as usize)] == virtual_font_type));
        {
            let __for_end_2 = (self.vf_local_font_num[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    lf = self.vf_i_fnts[crate::ix::U(((self.vf_default_font[crate::ix::U((f) as usize)]).wrapping_add(k)) as usize)];
                    self.set_expand_params(lf, self.pdf_font_auto_expand[crate::ix::U((f) as usize)], self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)], (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg(), self.pdf_font_step[crate::ix::U((f) as usize)], self.pdf_font_expand_ratio[crate::ix::U((f) as usize)]);
                    if (self.pdf_font_type[crate::ix::U((lf) as usize)] == virtual_font_type) {
                        self.vf_expand_local_fonts(lf);
                    }
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Here come some subroutines to deal with expanded fonts for HZ-algorithm.
    // §706
    pub fn read_expand_font(&mut self) {
        let mut shrink_limit: i32 = 0; // §706
        let mut stretch_limit: i32 = 0; // §706
        let mut font_step: i32 = 0; // §706
        let mut f: internal_font_number = 0; // §706
        let mut auto_expand: bool = false; // §706
        self.scan_font_ident();
        f = self.cur_val;
        if (f == null_font) {
            self.pdf_error(1053i32, 884i32);
        }
        if (self.pdf_font_blink[crate::ix::U((f) as usize)] != null_font) {
            self.pdf_error(1053i32, 1095i32);
        }
        self.scan_optional_equals();
        self.scan_int();
        stretch_limit = self.fix_int(self.cur_val, 0i32, 1000i32);
        self.scan_int();
        shrink_limit = self.fix_int(self.cur_val, 0i32, 500i32);
        self.scan_int();
        font_step = self.fix_int(self.cur_val, 0i32, 100i32);
        if (font_step == 0i32) {
            self.pdf_error(1053i32, 1096i32);
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
            self.pdf_error(1053i32, 1097i32);
        }
        auto_expand = false;
        if self.scan_keyword(1098i32) {
            {
                auto_expand = true;
                // §470
                {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) {
                        self.back_input();
                    }
                }
            }
        }
        // §706
        if (self.pdf_font_expand_ratio[crate::ix::U((f) as usize)] != 0i32) {
            self.pdf_error(1053i32, 1099i32);
        }
        if (self.pdf_font_step[crate::ix::U((f) as usize)] != 0i32) {
            {
                if (self.pdf_font_step[crate::ix::U((f) as usize)] != font_step) {
                    self.pdf_error(1053i32, 1100i32);
                }
                if (((self.pdf_font_stretch[crate::ix::U((f) as usize)] == null_font) && (stretch_limit != 0i32)) || ((self.pdf_font_stretch[crate::ix::U((f) as usize)] != null_font) && (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)] != stretch_limit))) {
                    self.pdf_error(1053i32, 1101i32);
                }
                if (((self.pdf_font_shrink[crate::ix::U((f) as usize)] == null_font) && (shrink_limit != 0i32)) || ((self.pdf_font_shrink[crate::ix::U((f) as usize)] != null_font) && ((self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg() != shrink_limit))) {
                    self.pdf_error(1053i32, 1102i32);
                }
                if (self.pdf_font_auto_expand[crate::ix::U((f) as usize)] != auto_expand) {
                    self.pdf_error(1053i32, 1103i32);
                }
            }
        } else {
            {
                if ((self.pdf_font_type[crate::ix::U((f) as usize)] != new_font_type) && (self.pdf_font_type[crate::ix::U((f) as usize)] != virtual_font_type)) {
                    self.pdf_warning(1053i32, 1104i32, true, true);
                }
                self.set_expand_params(f, auto_expand, stretch_limit, shrink_limit, font_step, 0i32);
                if (self.pdf_font_type[crate::ix::U((f) as usize)] == virtual_font_type) {
                    self.vf_expand_local_fonts(f);
                }
            }
        }
    }

    /// We implement robust letter spacing using virtual font.
    // §707
    pub fn letter_space_font(&mut self, mut u: halfword, mut f: internal_font_number, mut e: i32) -> internal_font_number {
        let mut letter_space_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §707
        let mut w: scaled = 0; // §707
        let mut r: scaled = 0; // §707
        let mut s: str_number = 0; // §707
        let mut i: i32 = 0; // §707
        let mut nw: i32 = 0; // §707
        let mut old_setting: i32 = 0; // §707
        let mut vf_z: i32 = 0; // §707
        let mut vf_alpha: i32 = 0; // §707
        let mut vf_beta: i32 = 0; // §707
        k = self.read_font_info(u, self.font_name[crate::ix::U((f) as usize)], 345i32, self.font_size[crate::ix::U((f) as usize)]);
        if self.scan_keyword(1105i32) {
            self.set_no_ligatures(k);
        }
        nw = (self.height_base[crate::ix::U((k) as usize)]).wrapping_sub(self.width_base[crate::ix::U((k) as usize)]);
        if ((self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((k) as usize)])) as usize)].int() == 0i32) && (self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int() > 0i32)) {
            { let __ix721 = (quad_code).wrapping_add(self.param_base[crate::ix::U((k) as usize)]); let __v722 = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(); self.font_info[crate::ix::U((__ix721) as usize)].set_int(__v722); }
        }
        if (self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((k) as usize)])) as usize)].int() == 0i32) {
            self.pdf_warning(1106i32, 1107i32, true, true);
        }
        {
            let __for_end_2 = (nw).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix723 = (self.width_base[crate::ix::U((k) as usize)]).wrapping_add(i); let __v724 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((k) as usize)]).wrapping_add(i)) as usize)].int()).wrapping_add(self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((k) as usize)])) as usize)].int(), e, 1000i32)); self.font_info[crate::ix::U((__ix723) as usize)].set_int(__v724); }
                i = i.wrapping_add(1);
            }
        }
        {
            if (((self.pool_ptr).wrapping_add((self.str_start[crate::ix::U(((self.font_name[crate::ix::U((k) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.font_name[crate::ix::U((k) as usize)]) as usize)]))).wrapping_add(7i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        old_setting = self.selector;
        self.selector = new_string;
        self.print(self.font_name[crate::ix::U((k) as usize)]);
        if (e > 0i32) {
            self.print(43i32);
        }
        self.print_int(((e) as i64));
        self.print(1108i32);
        self.selector = old_setting;
        { let __v725 = self.make_string(); self.font_name[crate::ix::U((k) as usize)] = __v725; }
        self.allocvffnts();
        self.vf_e_fnts[crate::ix::U((self.vf_nf) as usize)] = 0i32;
        self.vf_i_fnts[crate::ix::U((self.vf_nf) as usize)] = f;
        self.vf_nf = (self.vf_nf).wrapping_add(1i32);
        self.vf_local_font_num[crate::ix::U((k) as usize)] = 1i32;
        { let __v726 = (self.vf_nf).wrapping_sub(1i32); self.vf_default_font[crate::ix::U((k) as usize)] = __v726; }
        self.pdf_font_type[crate::ix::U((k) as usize)] = virtual_font_type;
        vf_z = self.font_size[crate::ix::U((f) as usize)];
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
        w = self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), e, 2000i32);
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
        { let __v727 = self.new_vf_packet(k); self.vf_packet_base[crate::ix::U((k) as usize)] = __v727; }
        {
            let __for_end_2 = self.font_ec[crate::ix::U((k) as usize)];
            self.c = self.font_bc[crate::ix::U((k) as usize)];
            while self.c <= __for_end_2 {
                {
                    {
                        if ((self.pool_ptr).wrapping_add(12i32) > pool_size) {
                            self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                        }
                    }
                    {
                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 146i32;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix728 = self.pool_ptr; let __v729 = self.tmp_w.qqqq().b0(); self.str_pool[crate::ix::U((__ix728) as usize)] = __v729; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix730 = self.pool_ptr; let __v731 = self.tmp_w.qqqq().b1(); self.str_pool[crate::ix::U((__ix730) as usize)] = __v731; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix732 = self.pool_ptr; let __v733 = self.tmp_w.qqqq().b2(); self.str_pool[crate::ix::U((__ix732) as usize)] = __v733; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix734 = self.pool_ptr; let __v735 = self.tmp_w.qqqq().b3(); self.str_pool[crate::ix::U((__ix734) as usize)] = __v735; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    if (self.c < set1) {
                        {
                            { let __ix736 = self.pool_ptr; let __v737 = self.c; self.str_pool[crate::ix::U((__ix736) as usize)] = __v737; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            {
                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = set1;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                            {
                                { let __ix738 = self.pool_ptr; let __v739 = self.c; self.str_pool[crate::ix::U((__ix738) as usize)] = __v739; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    {
                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 146i32;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix740 = self.pool_ptr; let __v741 = self.tmp_w.qqqq().b0(); self.str_pool[crate::ix::U((__ix740) as usize)] = __v741; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix742 = self.pool_ptr; let __v743 = self.tmp_w.qqqq().b1(); self.str_pool[crate::ix::U((__ix742) as usize)] = __v743; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix744 = self.pool_ptr; let __v745 = self.tmp_w.qqqq().b2(); self.str_pool[crate::ix::U((__ix744) as usize)] = __v745; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    {
                        { let __ix746 = self.pool_ptr; let __v747 = self.tmp_w.qqqq().b3(); self.str_pool[crate::ix::U((__ix746) as usize)] = __v747; }
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
    // §707
    pub fn new_letterspaced_font(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §707
        let mut t: str_number = 0; // §707
        let mut old_setting: i32 = 0; // §707
        let mut f: internal_font_number = 0; // §707
        let mut k: internal_font_number = 0; // §707
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
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
        self.scan_font_ident();
        k = self.cur_val;
        self.scan_int();
        f = { let __a748_0 = u; let __a748_1 = k; let __a748_2 = self.fix_int(self.cur_val, (1000i32).wrapping_neg(), 1000i32); self.letter_space_font(__a748_0, __a748_1, __a748_2) };
        __av_eqtb[crate::ix::U(((u) - 1) as usize)].set_hh_rh(f);
        { let __v749 = __av_eqtb[crate::ix::U(((u) - 1) as usize)]; __av_eqtb[crate::ix::U((((font_id_base).wrapping_add(f)) - 1) as usize)] = __v749; }
        self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].set_rh(t);
    }

    /// We implement robust letter spacing using virtual font.
    // §707
    pub fn is_letterspaced_font(&mut self, mut f: internal_font_number) -> bool {
        let mut is_letterspaced_font: bool = false;
        let mut i: pool_pointer = 0; // §707
        let mut j: pool_pointer = 0; // §707
        'l_done_f: {
            is_letterspaced_font = false;
            if (self.pdf_font_type[crate::ix::U((f) as usize)] != virtual_font_type) {
                return is_letterspaced_font;
            }
            i = (self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U((self.font_name[crate::ix::U((f) as usize)]) as usize)];
            if ((self.str_pool[crate::ix::U(((i).wrapping_sub(1i32)) as usize)] != ((b'l') as i32)) || (self.str_pool[crate::ix::U((i) as usize)] != ((b's') as i32))) {
                return is_letterspaced_font;
            }
            i = (i).wrapping_sub(2i32);
            while (i >= j) {
                {
                    if ((self.str_pool[crate::ix::U((i) as usize)] < ((b'0') as i32)) || (self.str_pool[crate::ix::U((i) as usize)] > ((b'9') as i32))) {
                        break 'l_done_f;
                    }
                    i = (i).wrapping_sub(1i32);
                }
            }
        }
        if (i < j) {
            return is_letterspaced_font;
        }
        if ((self.str_pool[crate::ix::U((i) as usize)] != ((b'+') as i32)) && (self.str_pool[crate::ix::U((i) as usize)] != ((b'-') as i32))) {
            return is_letterspaced_font;
        }
        is_letterspaced_font = true;
        is_letterspaced_font
    }

    /// We implement robust letter spacing using virtual font.
    // §707
    pub fn copy_font_info(&mut self, mut f: internal_font_number) -> internal_font_number {
        let mut copy_font_info: internal_font_number = 0;
        let mut lf: halfword = 0; // §707
        let mut bc: halfword = 0; // §707
        let mut ec: halfword = 0; // §707
        let mut i: halfword = 0; // §707
        let mut k: internal_font_number = 0; // §707
        if ((self.pdf_font_expand_ratio[crate::ix::U((f) as usize)] != 0i32) || (self.pdf_font_step[crate::ix::U((f) as usize)] != 0i32)) {
            self.pdf_error(1110i32, 1111i32);
        }
        if self.is_letterspaced_font(f) {
            self.pdf_error(1110i32, 1112i32);
        }
        k = (self.font_ptr).wrapping_add(1i32);
        self.font_ptr = (self.font_ptr).wrapping_add(1i32);
        if (self.font_ptr >= font_max) {
            self.overflow(1092i32, font_max);
        }
        { let __v750 = self.font_name[crate::ix::U((f) as usize)]; self.font_name[crate::ix::U((k) as usize)] = __v750; }
        self.font_area[crate::ix::U((k) as usize)] = non_existent_path;
        { let __v751 = self.hyphen_char[crate::ix::U((f) as usize)]; self.hyphen_char[crate::ix::U((k) as usize)] = __v751; }
        { let __v752 = self.skew_char[crate::ix::U((f) as usize)]; self.skew_char[crate::ix::U((k) as usize)] = __v752; }
        { let __v753 = self.font_bchar[crate::ix::U((f) as usize)]; self.font_bchar[crate::ix::U((k) as usize)] = __v753; }
        { let __v754 = self.font_false_bchar[crate::ix::U((f) as usize)]; self.font_false_bchar[crate::ix::U((k) as usize)] = __v754; }
        { let __v755 = self.font_bc[crate::ix::U((f) as usize)]; self.font_bc[crate::ix::U((k) as usize)] = __v755; }
        { let __v756 = self.font_ec[crate::ix::U((f) as usize)]; self.font_ec[crate::ix::U((k) as usize)] = __v756; }
        { let __v757 = self.font_size[crate::ix::U((f) as usize)]; self.font_size[crate::ix::U((k) as usize)] = __v757; }
        { let __v758 = self.font_dsize[crate::ix::U((f) as usize)]; self.font_dsize[crate::ix::U((k) as usize)] = __v758; }
        { let __v759 = self.font_params[crate::ix::U((f) as usize)]; self.font_params[crate::ix::U((k) as usize)] = __v759; }
        { let __v760 = self.font_glue[crate::ix::U((f) as usize)]; self.font_glue[crate::ix::U((k) as usize)] = __v760; }
        { let __v761 = self.bchar_label[crate::ix::U((f) as usize)]; self.bchar_label[crate::ix::U((k) as usize)] = __v761; }
        bc = self.font_bc[crate::ix::U((f) as usize)];
        ec = self.font_ec[crate::ix::U((f) as usize)];
        { let __v762 = (self.fmem_ptr).wrapping_sub(bc); self.char_base[crate::ix::U((k) as usize)] = __v762; }
        { let __v763 = ((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(ec)).wrapping_add(1i32); self.width_base[crate::ix::U((k) as usize)] = __v763; }
        { let __v764 = (self.width_base[crate::ix::U((k) as usize)]).wrapping_add((self.height_base[crate::ix::U((f) as usize)]).wrapping_sub(self.width_base[crate::ix::U((f) as usize)])); self.height_base[crate::ix::U((k) as usize)] = __v764; }
        { let __v765 = (self.height_base[crate::ix::U((k) as usize)]).wrapping_add((self.depth_base[crate::ix::U((f) as usize)]).wrapping_sub(self.height_base[crate::ix::U((f) as usize)])); self.depth_base[crate::ix::U((k) as usize)] = __v765; }
        { let __v766 = (self.depth_base[crate::ix::U((k) as usize)]).wrapping_add((self.italic_base[crate::ix::U((f) as usize)]).wrapping_sub(self.depth_base[crate::ix::U((f) as usize)])); self.italic_base[crate::ix::U((k) as usize)] = __v766; }
        { let __v767 = (self.italic_base[crate::ix::U((k) as usize)]).wrapping_add((self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_sub(self.italic_base[crate::ix::U((f) as usize)])); self.lig_kern_base[crate::ix::U((k) as usize)] = __v767; }
        { let __v768 = (self.lig_kern_base[crate::ix::U((k) as usize)]).wrapping_add((self.kern_base[crate::ix::U((f) as usize)]).wrapping_sub(self.lig_kern_base[crate::ix::U((f) as usize)])); self.kern_base[crate::ix::U((k) as usize)] = __v768; }
        { let __v769 = (self.kern_base[crate::ix::U((k) as usize)]).wrapping_add((self.exten_base[crate::ix::U((f) as usize)]).wrapping_sub(self.kern_base[crate::ix::U((f) as usize)])); self.exten_base[crate::ix::U((k) as usize)] = __v769; }
        { let __v770 = (self.exten_base[crate::ix::U((k) as usize)]).wrapping_add((self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(self.exten_base[crate::ix::U((f) as usize)])); self.param_base[crate::ix::U((k) as usize)] = __v770; }
        lf = (((self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(self.char_base[crate::ix::U((f) as usize)])).wrapping_add(self.font_params[crate::ix::U((f) as usize)])).wrapping_add(1i32);
        if ((self.fmem_ptr).wrapping_add(lf) >= font_mem_size) {
            self.overflow(1093i32, font_mem_size);
        }
        {
            let __for_end_2 = (lf).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __ix771 = ((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(bc)).wrapping_add(i); let __v772 = self.font_info[crate::ix::U((((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(bc)).wrapping_add(i)) as usize)]; self.font_info[crate::ix::U((__ix771) as usize)] = __v772; }
                i = i.wrapping_add(1);
            }
        }
        self.fmem_ptr = (self.fmem_ptr).wrapping_add(lf);
        copy_font_info = k;
        copy_font_info
    }

    /// We implement robust letter spacing using virtual font.
    // §707
    pub fn make_font_copy(&mut self, mut a: small_number) {
        let mut u: halfword = 0; // §707
        let mut t: str_number = 0; // §707
        let mut old_setting: i32 = 0; // §707
        let mut f: internal_font_number = 0; // §707
        let mut k: internal_font_number = 0; // §707
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
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
        self.scan_font_ident();
        k = self.cur_val;
        f = self.copy_font_info(k);
        __av_eqtb[crate::ix::U(((u) - 1) as usize)].set_hh_rh(f);
        { let __v773 = __av_eqtb[crate::ix::U(((u) - 1) as usize)]; __av_eqtb[crate::ix::U((((font_id_base).wrapping_add(f)) - 1) as usize)] = __v773; }
        self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].set_rh(t);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §713
    pub fn vf_error(&mut self, mut filename: str_number, mut msg: str_number) {
        let mut old_setting: i32 = 0; // §713
        let mut s: str_number = 0; // §713
        {
            if (((self.pool_ptr).wrapping_add((self.str_start[crate::ix::U(((filename).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((filename) as usize)]))).wrapping_add(3i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        old_setting = self.selector;
        self.selector = new_string;
        self.print(filename);
        self.print(1113i32);
        s = self.make_string();
        self.selector = old_setting;
        self.pdf_error(s, msg);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §713
    pub fn vf_byte(&mut self) -> eight_bits {
        let mut vf_byte: eight_bits = 0;
        let mut i: i32 = 0; // §713
        i = { let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.getc(&mut __f0); self.vf_file = __f0; __r };
        if (i < 0i32) {
            self.pdf_error(1114i32, 1115i32);
        }
        vf_byte = i;
        vf_byte
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §713
    pub fn vf_read_signed(&mut self, mut k: i32) -> i32 {
        let mut vf_read_signed: i32 = 0;
        let mut i: i32 = 0; // §713
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
    // §713
    pub fn vf_read_unsigned(&mut self, mut k: i32) -> i32 {
        let mut vf_read_unsigned: i32 = 0;
        let mut i: i32 = 0; // §713
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.vf_byte();
        if ((k == 4i32) && (i >= 128i32)) {
            self.vf_error(self.font_name[crate::ix::U((self.f) as usize)], 1045i32);
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
    // §713
    pub fn vf_local_font_warning(&mut self, mut f: internal_font_number, mut k: internal_font_number, mut s: str_number) {
        self.print_nl(s);
        self.print(1116i32);
        self.print(self.font_name[crate::ix::U((k) as usize)]);
        self.print(1117i32);
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        self.print(1118i32);
    }

    /// The `do_vf` procedure attempts to read the \.{VF} file for a font, and sets
    /// `pdf_font_type` to `real_font_type` if the \.{VF} file could not be found
    /// or loaded, otherwise sets `pdf_font_type` to `virtual_font_type`. To
    /// process font definitions in virtual font we call `vf_def_font`.
    // §713
    pub fn vf_def_font(&mut self, mut f: internal_font_number) -> internal_font_number {
        let mut vf_def_font: internal_font_number = 0;
        let mut k: internal_font_number = 0; // §713
        let mut s: str_number = 0; // §713
        let mut ds: scaled = 0; // §713
        let mut fs: scaled = 0; // §713
        let mut cs: four_quarters = four_quarters::default(); // §713
        { let __v774 = self.vf_byte(); cs.set_b0(__v774); }
        { let __v775 = self.vf_byte(); cs.set_b1(__v775); }
        { let __v776 = self.vf_byte(); cs.set_b2(__v776); }
        { let __v777 = self.vf_byte(); cs.set_b3(__v777); }
        fs = { let __a778_0 = self.vf_read_signed(4i32); let __a778_1 = self.font_size[crate::ix::U((f) as usize)]; self.store_scaled_f(__a778_0, __a778_1) };
        ds = (self.vf_read_signed(4i32) / 16i32);
        { let __v779 = self.vf_byte(); self.tmp_w.set_qqqq_b0(__v779); }
        { let __v780 = self.vf_byte(); self.tmp_w.set_qqqq_b1(__v780); }
        while (self.tmp_w.qqqq().b0() > 0i32) {
            {
                { let __v781 = (self.tmp_w.qqqq().b0()).wrapping_sub(1i32); self.tmp_w.set_qqqq_b0(__v781); }
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
                { let __v782 = (self.tmp_w.qqqq().b1()).wrapping_sub(1i32); self.tmp_w.set_qqqq_b1(__v782); }
                {
                    { let __ix783 = self.pool_ptr; let __v784 = self.vf_byte(); self.str_pool[crate::ix::U((__ix783) as usize)] = __v784; }
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
            }
        }
        s = self.make_string();
        k = self.tfm_lookup(s, fs);
        if (k == null_font) {
            k = self.read_font_info(null_cs, s, 345i32, fs);
        }
        if (k != null_font) {
            {
                if ((((((cs.b0() != 0i32) || (cs.b1() != 0i32)) || (cs.b2() != 0i32)) || (cs.b3() != 0i32)) && ((((self.font_check[crate::ix::U((k) as usize)].b0() != 0i32) || (self.font_check[crate::ix::U((k) as usize)].b1() != 0i32)) || (self.font_check[crate::ix::U((k) as usize)].b2() != 0i32)) || (self.font_check[crate::ix::U((k) as usize)].b3() != 0i32))) && ((((cs.b0() != self.font_check[crate::ix::U((k) as usize)].b0()) || (cs.b1() != self.font_check[crate::ix::U((k) as usize)].b1())) || (cs.b2() != self.font_check[crate::ix::U((k) as usize)].b2())) || (cs.b3() != self.font_check[crate::ix::U((k) as usize)].b3()))) {
                    self.vf_local_font_warning(f, k, 1119i32);
                }
                if (ds != self.font_dsize[crate::ix::U((k) as usize)]) {
                    self.vf_local_font_warning(f, k, 1120i32);
                }
                if (self.pdf_font_step[crate::ix::U((f) as usize)] != 0i32) {
                    self.set_expand_params(k, self.pdf_font_auto_expand[crate::ix::U((f) as usize)], self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)], (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg(), self.pdf_font_step[crate::ix::U((f) as usize)], self.pdf_font_expand_ratio[crate::ix::U((f) as usize)]);
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
    // §713
    pub fn do_vf(&mut self, mut f: internal_font_number) {
        let mut cmd: i32 = 0; // §713
        let mut k: i32 = 0; // §713
        let mut n: i32 = 0; // §713
        let mut cc: i32 = 0; // §713
        let mut cmd_length: i32 = 0; // §713
        let mut packet_length: i32 = 0; // §713
        let mut tfm_width: scaled = 0; // §713
        let mut s: str_number = 0; // §713
        let mut stack_level: vf_stack_index = 0; // §713
        let mut save_vf_nf: internal_font_number = 0; // §713
        self.pdf_font_type[crate::ix::U((f) as usize)] = real_font_type;
        if self.auto_expand_vf(f) {
            return;
        }
        stack_level = 0i32;
        // §714
        self.pack_file_name(self.font_name[crate::ix::U((f) as usize)], 345i32, 1113i32);
        if (!{ let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.vf_b_open_in(&mut __f0); self.vf_file = __f0; __r }) {
            return;
        }
        // §715
        if (self.vf_byte() != pre) {
            self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1122i32);
        }
        if (self.vf_byte() != vf_id) {
            self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1123i32);
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
        { let __v785 = self.vf_byte(); self.tmp_w.set_qqqq_b0(__v785); }
        { let __v786 = self.vf_byte(); self.tmp_w.set_qqqq_b1(__v786); }
        { let __v787 = self.vf_byte(); self.tmp_w.set_qqqq_b2(__v787); }
        { let __v788 = self.vf_byte(); self.tmp_w.set_qqqq_b3(__v788); }
        if ((((((self.tmp_w.qqqq().b0() != 0i32) || (self.tmp_w.qqqq().b1() != 0i32)) || (self.tmp_w.qqqq().b2() != 0i32)) || (self.tmp_w.qqqq().b3() != 0i32)) && ((((self.font_check[crate::ix::U((f) as usize)].b0() != 0i32) || (self.font_check[crate::ix::U((f) as usize)].b1() != 0i32)) || (self.font_check[crate::ix::U((f) as usize)].b2() != 0i32)) || (self.font_check[crate::ix::U((f) as usize)].b3() != 0i32))) && ((((self.tmp_w.qqqq().b0() != self.font_check[crate::ix::U((f) as usize)].b0()) || (self.tmp_w.qqqq().b1() != self.font_check[crate::ix::U((f) as usize)].b1())) || (self.tmp_w.qqqq().b2() != self.font_check[crate::ix::U((f) as usize)].b2())) || (self.tmp_w.qqqq().b3() != self.font_check[crate::ix::U((f) as usize)].b3()))) {
            {
                self.print_nl(1124i32);
                self.print(self.font_name[crate::ix::U((f) as usize)]);
                self.print(1125i32);
            }
        }
        if ((self.vf_read_signed(4i32) / 16i32) != self.font_dsize[crate::ix::U((f) as usize)]) {
            {
                self.print_nl(1126i32);
                self.print(self.font_name[crate::ix::U((f) as usize)]);
                self.print(1125i32);
            }
        }
        crate::system::break_out(&mut self.term_out);
        // §716
        cmd = self.vf_byte();
        save_vf_nf = self.vf_nf;
        while ((cmd >= fnt_def1) && (cmd <= 246i32)) {
            {
                self.allocvffnts();
                { let __ix789 = self.vf_nf; let __v790 = self.vf_read_unsigned((cmd).wrapping_sub(242i32)); self.vf_e_fnts[crate::ix::U((__ix789) as usize)] = __v790; }
                { let __ix791 = self.vf_nf; let __v792 = self.vf_def_font(f); self.vf_i_fnts[crate::ix::U((__ix791) as usize)] = __v792; }
                self.vf_nf = (self.vf_nf).wrapping_add(1i32);
                cmd = self.vf_byte();
            }
        }
        self.vf_default_font[crate::ix::U((f) as usize)] = save_vf_nf;
        { let __v793 = (self.vf_nf).wrapping_sub(save_vf_nf); self.vf_local_font_num[crate::ix::U((f) as usize)] = __v793; }
        // §717
        { let __v794 = self.new_vf_packet(f); self.vf_packet_base[crate::ix::U((f) as usize)] = __v794; }
        // §713
        while (cmd <= long_char) {
            {
                // §718
                if (cmd == long_char) {
                    {
                        packet_length = self.vf_read_unsigned(4i32);
                        cc = self.vf_read_unsigned(4i32);
                        if (!(((self.font_bc[crate::ix::U((f) as usize)] <= cc) && (cc <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(cc)) as usize)].qqqq().b0() > min_quarterword))) {
                            self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1127i32);
                        }
                        tfm_width = { let __a795_0 = self.vf_read_signed(4i32); let __a795_1 = self.font_size[crate::ix::U((f) as usize)]; self.store_scaled_f(__a795_0, __a795_1) };
                    }
                } else {
                    {
                        packet_length = cmd;
                        cc = self.vf_byte();
                        if (!(((self.font_bc[crate::ix::U((f) as usize)] <= cc) && (cc <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(cc)) as usize)].qqqq().b0() > min_quarterword))) {
                            self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1127i32);
                        }
                        tfm_width = { let __a796_0 = self.vf_read_unsigned(3i32); let __a796_1 = self.font_size[crate::ix::U((f) as usize)]; self.store_scaled_f(__a796_0, __a796_1) };
                    }
                }
                if (packet_length < 0i32) {
                    self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1128i32);
                }
                if (packet_length > vf_max_packet_length) {
                    self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1129i32);
                }
                if (tfm_width != { let __s798 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s797 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, cc))) as usize; self.font_info[crate::ix::U(__s797)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s798)] }.int()) {
                    {
                        self.print_nl(1130i32);
                        self.print(self.font_name[crate::ix::U((f) as usize)]);
                        self.print(1125i32);
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
                        // §720
                        if ((cmd >= set_char_0) && (cmd <= 127i32)) {
                            cmd_length = 0i32;
                        } else {
                            if (((fnt_num_0 <= cmd) && (cmd <= 234i32)) || ((fnt1 <= cmd) && (cmd <= 238i32))) {
                                {
                                    if (cmd >= fnt1) {
                                        {
                                            k = self.vf_read_unsigned((cmd).wrapping_sub(234i32));
                                            packet_length = (packet_length).wrapping_sub((cmd).wrapping_sub(234i32));
                                        }
                                    } else {
                                        k = (cmd).wrapping_sub(171i32);
                                    }
                                    if (k >= 256i32) {
                                        self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1133i32);
                                    }
                                    n = 0i32;
                                    while ((n < self.vf_local_font_num[crate::ix::U((f) as usize)]) && (self.vf_e_fnts[crate::ix::U(((self.vf_default_font[crate::ix::U((f) as usize)]).wrapping_add(n)) as usize)] != k)) {
                                        n = (n).wrapping_add(1i32);
                                    }
                                    if (n == self.vf_local_font_num[crate::ix::U((f) as usize)]) {
                                        self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1134i32);
                                    }
                                    if (k <= 63i32) {
                                        {
                                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (fnt_num_0).wrapping_add(k);
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = fnt1;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = k;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                    cmd_length = 0i32;
                                    cmd = nop;
                                }
                            } else {
                                match cmd {
                                    set_rule | put_rule => {
                                        cmd_length = 8i32;
                                    }
                                    set1 | 129 | 130 | 131 => {
                                        cmd_length = (cmd).wrapping_sub(127i32);
                                    }
                                    put1 | 134 | 135 | 136 => {
                                        cmd_length = (cmd).wrapping_sub(132i32);
                                    }
                                    right1 | 144 | 145 | 146 => {
                                        cmd_length = (cmd).wrapping_sub(142i32);
                                    }
                                    w1 | 149 | 150 | 151 => {
                                        cmd_length = (cmd).wrapping_sub(147i32);
                                    }
                                    x1 | 154 | 155 | 156 => {
                                        cmd_length = (cmd).wrapping_sub(152i32);
                                    }
                                    down1 | 158 | 159 | 160 => {
                                        cmd_length = (cmd).wrapping_sub(156i32);
                                    }
                                    y1 | 163 | 164 | 165 => {
                                        cmd_length = (cmd).wrapping_sub(161i32);
                                    }
                                    z1 | 168 | 169 | 170 => {
                                        cmd_length = (cmd).wrapping_sub(166i32);
                                    }
                                    xxx1 | 240 | 241 | 242 => {
                                        {
                                            cmd_length = self.vf_read_unsigned((cmd).wrapping_sub(238i32));
                                            packet_length = (packet_length).wrapping_sub((cmd).wrapping_sub(238i32));
                                            if (cmd_length > vf_max_packet_length) {
                                                self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1129i32);
                                            }
                                            if (cmd_length < 0i32) {
                                                self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1135i32);
                                            }
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = xxx1;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = cmd_length;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            cmd = nop;
                                        }
                                    }
                                    w0 | x0 | y0 | z0 | nop => {
                                        cmd_length = 0i32;
                                    }
                                    push | pop => {
                                        {
                                            cmd_length = 0i32;
                                            if (cmd == push) {
                                                if (stack_level == vf_stack_size) {
                                                    self.overflow(1136i32, vf_stack_size);
                                                } else {
                                                    stack_level = (stack_level).wrapping_add(1i32);
                                                }
                                            } else {
                                                if (stack_level == 0i32) {
                                                    self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1137i32);
                                                } else {
                                                    stack_level = (stack_level).wrapping_sub(1i32);
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1138i32);
                                    }
                                }
                            }
                        }
                        // §718
                        if (cmd != nop) {
                            {
                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = cmd;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                        packet_length = (packet_length).wrapping_sub(cmd_length);
                        while (cmd_length > 0i32) {
                            {
                                cmd_length = (cmd_length).wrapping_sub(1i32);
                                {
                                    { let __ix799 = self.pool_ptr; let __v800 = self.vf_byte(); self.str_pool[crate::ix::U((__ix799) as usize)] = __v800; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
                if (stack_level != 0i32) {
                    self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1131i32);
                }
                if (packet_length != 0i32) {
                    self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1132i32);
                }
                // §719
                s = self.make_string();
                self.storepacket(f, cc, s);
                self.flush_str(s);
                // §718
                cmd = self.vf_byte();
            }
        }
        // §713
        if (cmd != post) {
            self.vf_error(self.font_name[crate::ix::U((f) as usize)], 1121i32);
        }
        { let mut __f0 = ::core::mem::take(&mut self.vf_file); let __r = self.b_close(&mut __f0); self.vf_file = __f0; __r };
        self.pdf_font_type[crate::ix::U((f) as usize)] = virtual_font_type;
    }

    // §721
    pub fn pdf_check_vf_cur_val(&mut self) {
        let mut f: internal_font_number = 0; // §721
        f = self.cur_val;
        self.do_vf(f);
        if (self.pdf_font_type[crate::ix::U((f) as usize)] == virtual_font_type) {
            self.pdf_error(604i32, 1139i32);
        }
    }

    // §721
    pub fn auto_expand_vf(&mut self, mut f: internal_font_number) -> bool {
        let mut auto_expand_vf: bool = false;
        let mut bf: internal_font_number = 0; // §721
        let mut lf: internal_font_number = 0; // §721
        let mut e: i32 = 0; // §721
        let mut k: i32 = 0; // §721
        auto_expand_vf = false;
        if ((!self.pdf_font_auto_expand[crate::ix::U((f) as usize)]) || (self.pdf_font_blink[crate::ix::U((f) as usize)] == null_font)) {
            return auto_expand_vf;
        }
        bf = self.pdf_font_blink[crate::ix::U((f) as usize)];
        if (self.pdf_font_type[crate::ix::U((bf) as usize)] == new_font_type) {
            self.do_vf(bf);
        }
        if (self.pdf_font_type[crate::ix::U((bf) as usize)] != virtual_font_type) {
            return auto_expand_vf;
        }
        e = self.pdf_font_expand_ratio[crate::ix::U((f) as usize)];
        {
            let __for_end_2 = (self.vf_local_font_num[crate::ix::U((bf) as usize)]).wrapping_sub(1i32);
            k = 0i32;
            while k <= __for_end_2 {
                {
                    lf = (self.vf_default_font[crate::ix::U((bf) as usize)]).wrapping_add(k);
                    self.allocvffnts();
                    { let __ix801 = self.vf_nf; let __v802 = self.vf_e_fnts[crate::ix::U((lf) as usize)]; self.vf_e_fnts[crate::ix::U((__ix801) as usize)] = __v802; }
                    { let __ix803 = self.vf_nf; let __v804 = self.auto_expand_font(self.vf_i_fnts[crate::ix::U((lf) as usize)], e); self.vf_i_fnts[crate::ix::U((__ix803) as usize)] = __v804; }
                    self.copy_expand_params(self.vf_i_fnts[crate::ix::U((self.vf_nf) as usize)], self.vf_i_fnts[crate::ix::U((lf) as usize)], e);
                    self.vf_nf = (self.vf_nf).wrapping_add(1i32);
                }
                k = k.wrapping_add(1);
            }
        }
        { let __v805 = self.vf_packet_base[crate::ix::U((bf) as usize)]; self.vf_packet_base[crate::ix::U((f) as usize)] = __v805; }
        { let __v806 = self.vf_local_font_num[crate::ix::U((bf) as usize)]; self.vf_local_font_num[crate::ix::U((f) as usize)] = __v806; }
        { let __v807 = (self.vf_nf).wrapping_sub(self.vf_local_font_num[crate::ix::U((f) as usize)]); self.vf_default_font[crate::ix::U((f) as usize)] = __v807; }
        self.pdf_font_type[crate::ix::U((f) as usize)] = virtual_font_type;
        auto_expand_vf = true;
        auto_expand_vf
    }

    /// Some functions for processing character packets.
    // §726
    pub fn packet_read_signed(&mut self, mut k: i32) -> i32 {
        let mut packet_read_signed: i32 = 0;
        let mut i: i32 = 0; // §726
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
    // §726
    pub fn packet_read_unsigned(&mut self, mut k: i32) -> i32 {
        let mut packet_read_unsigned: i32 = 0;
        let mut i: i32 = 0; // §726
        self.pdfassert(((k > 0i32) && (k <= 4i32)));
        i = self.packet_byte();
        if ((k == 4i32) && (i >= 128i32)) {
            self.vf_error(self.font_name[crate::ix::U((self.f) as usize)], 1045i32);
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
    // §726
    pub fn packet_scaled(&mut self, mut k: i32, mut fs: scaled) -> scaled {
        let mut packet_scaled: scaled = 0;
        packet_scaled = { let __a808_0 = self.packet_read_signed(k); let __a808_1 = fs; self.store_scaled_f(__a808_0, __a808_1) };
        packet_scaled
    }

    /// Some functions for processing character packets.
    // §726
    pub fn do_vf_packet(&mut self, mut vf_f: internal_font_number, mut c: eight_bits) {
        let mut f: internal_font_number = 0; // §726
        let mut k: internal_font_number = 0; // §726
        let mut n: internal_font_number = 0; // §726
        let mut save_cur_h: scaled = 0; // §726
        let mut save_cur_v: scaled = 0; // §726
        let mut cmd: i32 = 0; // §726
        let mut char_move: bool = false; // §726
        let mut w: scaled = 0; // §726
        let mut x: scaled = 0; // §726
        let mut y: scaled = 0; // §726
        let mut z: scaled = 0; // §726
        let mut s: str_number = 0; // §726
        self.vf_cur_s = (self.vf_cur_s).wrapping_add(1i32);
        if (self.vf_cur_s > vf_max_recursion) {
            self.overflow(1140i32, vf_max_recursion);
        }
        save_cur_v = self.cur_v;
        save_cur_h = self.cur_h;
        self.push_packet_state();
        self.start_packet(vf_f, c);
        f = self.vf_i_fnts[crate::ix::U((self.vf_default_font[crate::ix::U((vf_f) as usize)]) as usize)];
        w = 0i32;
        x = 0i32;
        y = 0i32;
        z = 0i32;
        while (self.vf_packet_length > 0i32) {
            {
                'l_continue_f: {
                    'l_L70_f: {
                        cmd = self.packet_byte();
                        // §727
                        if ((cmd >= set_char_0) && (cmd <= 127i32)) {
                            {
                                if (!(((self.font_bc[crate::ix::U((f) as usize)] <= cmd) && (cmd <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(cmd)) as usize)].qqqq().b0() > min_quarterword))) {
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
                            if (((fnt_num_0 <= cmd) && (cmd <= 234i32)) || (cmd == fnt1)) {
                                {
                                    if (cmd == fnt1) {
                                        k = self.packet_byte();
                                    } else {
                                        k = (cmd).wrapping_sub(171i32);
                                    }
                                    n = 0i32;
                                    while ((n < self.vf_local_font_num[crate::ix::U((vf_f) as usize)]) && (self.vf_e_fnts[crate::ix::U(((self.vf_default_font[crate::ix::U((vf_f) as usize)]).wrapping_add(n)) as usize)] != k)) {
                                        n = (n).wrapping_add(1i32);
                                    }
                                    if (n == self.vf_local_font_num[crate::ix::U((vf_f) as usize)]) {
                                        self.pdf_error(1114i32, 1141i32);
                                    } else {
                                        f = self.vf_i_fnts[crate::ix::U(((self.vf_default_font[crate::ix::U((vf_f) as usize)]).wrapping_add(n)) as usize)];
                                    }
                                }
                            } else {
                                match cmd {
                                    push => {
                                        {
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_h = self.cur_h;
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_v = self.cur_v;
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_w = w;
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_x = x;
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_y = y;
                                            self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_z = z;
                                            self.vf_stack_ptr = (self.vf_stack_ptr).wrapping_add(1i32);
                                        }
                                    }
                                    pop => {
                                        {
                                            self.vf_stack_ptr = (self.vf_stack_ptr).wrapping_sub(1i32);
                                            self.cur_h = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_h;
                                            self.cur_v = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_v;
                                            w = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_w;
                                            x = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_x;
                                            y = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_y;
                                            z = self.vf_stack[crate::ix::U((self.vf_stack_ptr) as usize)].stack_z;
                                        }
                                    }
                                    set1 | 129 | 130 | 131 | put1 | 134 | 135 | 136 => {
                                        {
                                            if ((set1 <= cmd) && (cmd <= 131i32)) {
                                                {
                                                    { let __v809 = self.packet_read_unsigned((cmd).wrapping_sub(127i32)); self.tmp_w.set_int(__v809); }
                                                    char_move = true;
                                                }
                                            } else {
                                                {
                                                    { let __v810 = self.packet_read_unsigned((cmd).wrapping_sub(132i32)); self.tmp_w.set_int(__v810); }
                                                    char_move = false;
                                                }
                                            }
                                            if (!(((self.font_bc[crate::ix::U((f) as usize)] <= self.tmp_w.int()) && (self.tmp_w.int() <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.tmp_w.int())) as usize)].qqqq().b0() > min_quarterword))) {
                                                {
                                                    self.char_warning(f, self.tmp_w.int());
                                                    break 'l_continue_f;
                                                }
                                            }
                                            c = self.tmp_w.int();
                                            break 'l_L70_f;
                                        }
                                    }
                                    set_rule | put_rule => {
                                        {
                                            self.rule_ht = self.packet_scaled(4i32, self.font_size[crate::ix::U((vf_f) as usize)]);
                                            self.rule_wd = self.packet_scaled(4i32, self.font_size[crate::ix::U((vf_f) as usize)]);
                                            if ((self.rule_wd > 0i32) && (self.rule_ht > 0i32)) {
                                                {
                                                    self.pdf_set_rule(self.cur_h, self.cur_v, self.rule_wd, self.rule_ht);
                                                    if (cmd == set_rule) {
                                                        self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    right1 | 144 | 145 | 146 => {
                                        self.cur_h = (self.cur_h).wrapping_add(self.packet_scaled((cmd).wrapping_sub(142i32), self.font_size[crate::ix::U((vf_f) as usize)]));
                                    }
                                    w0 | w1 | 149 | 150 | 151 => {
                                        {
                                            if (cmd > w0) {
                                                w = self.packet_scaled((cmd).wrapping_sub(147i32), self.font_size[crate::ix::U((vf_f) as usize)]);
                                            }
                                            self.cur_h = (self.cur_h).wrapping_add(w);
                                        }
                                    }
                                    x0 | x1 | 154 | 155 | 156 => {
                                        {
                                            if (cmd > x0) {
                                                x = self.packet_scaled((cmd).wrapping_sub(152i32), self.font_size[crate::ix::U((vf_f) as usize)]);
                                            }
                                            self.cur_h = (self.cur_h).wrapping_add(x);
                                        }
                                    }
                                    down1 | 158 | 159 | 160 => {
                                        self.cur_v = (self.cur_v).wrapping_add(self.packet_scaled((cmd).wrapping_sub(156i32), self.font_size[crate::ix::U((vf_f) as usize)]));
                                    }
                                    y0 | y1 | 163 | 164 | 165 => {
                                        {
                                            if (cmd > y0) {
                                                y = self.packet_scaled((cmd).wrapping_sub(161i32), self.font_size[crate::ix::U((vf_f) as usize)]);
                                            }
                                            self.cur_v = (self.cur_v).wrapping_add(y);
                                        }
                                    }
                                    z0 | z1 | 168 | 169 | 170 => {
                                        {
                                            if (cmd > z0) {
                                                z = self.packet_scaled((cmd).wrapping_sub(166i32), self.font_size[crate::ix::U((vf_f) as usize)]);
                                            }
                                            self.cur_v = (self.cur_v).wrapping_add(z);
                                        }
                                    }
                                    xxx1 | 240 | 241 | 242 => {
                                        {
                                            { let __v811 = self.packet_read_unsigned((cmd).wrapping_sub(238i32)); self.tmp_w.set_int(__v811); }
                                            {
                                                if ((self.pool_ptr).wrapping_add(self.tmp_w.int()) > pool_size) {
                                                    self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                                }
                                            }
                                            while (self.tmp_w.int() > 0i32) {
                                                {
                                                    { let __v812 = (self.tmp_w.int()).wrapping_sub(1i32); self.tmp_w.set_int(__v812); }
                                                    {
                                                        { let __ix813 = self.pool_ptr; let __v814 = self.packet_byte(); self.str_pool[crate::ix::U((__ix813) as usize)] = __v814; }
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            s = self.make_string();
                                            self.literal(s, scan_special, false);
                                            self.flush_str(s);
                                        }
                                    }
                                    _ => {
                                        self.pdf_error(1114i32, 1142i32);
                                    }
                                }
                            }
                        }
                        break 'l_continue_f;
                    }
                    if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
                        {
                            if (self.pdf_font_type[crate::ix::U((f) as usize)] == new_font_type) {
                                self.do_vf(f);
                            }
                            if (self.pdf_font_type[crate::ix::U((f) as usize)] == virtual_font_type) {
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
                        self.cur_h = (self.cur_h).wrapping_add({ let __s816 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s815 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s815)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s816)] }.int());
                    }
                }
                // §726
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
    // §728
    pub fn pdf_out_literal(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §728
        let mut s: str_number = 0; // §728
        let mut h: halfword = 0; // §728
        let mut q: halfword = 0; // §728
        let mut r: halfword = 0; // §728
        let mut old_mode: i32 = 0; // §728
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        old_setting = self.selector;
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_lateliteral_node) {
            {
                // §1619
                q = self.get_avail();
                __av_mem[crate::ix::U((q) as usize)].set_hh_lh(637i32);
                r = self.get_avail();
                __av_mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                __av_mem[crate::ix::U((r) as usize)].set_hh_lh(end_write_token);
                self.begin_token_list(q, inserted);
                self.begin_token_list(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), write_text);
                q = self.get_avail();
                __av_mem[crate::ix::U((q) as usize)].set_hh_lh(379i32);
                self.begin_token_list(q, inserted);
                old_mode = self.cur_list.mode_field;
                self.cur_list.mode_field = 0i32;
                self.cur_cs = self.write_loc;
                q = self.scan_toks(false, true);
                self.cur_list.mode_field = old_mode;
                self.get_token();
                if (self.cur_tok != end_write_token) {
                    // §1620
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
                            self.print(712i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1942i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1439i32;
                        }
                        self.error();
                        loop {
                            self.get_token();
                            if (self.cur_tok == end_write_token) { break; }
                        }
                    }
                }
                // §1619
                self.end_token_list();
                // §728
                h = self.def_ref;
            }
        } else {
            h = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
        }
        self.selector = new_string;
        self.show_token_list(__av_mem[crate::ix::U((h) as usize)].hh().rh(), null, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        s = self.make_string();
        self.literal(s, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), false);
        self.flush_str(s);
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_lateliteral_node) {
            self.flush_list(self.def_ref);
        }
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §728
    pub fn pdf_out_colorstack(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §728
        let mut s: str_number = 0; // §728
        let mut cmd: i32 = 0; // §728
        let mut stack_no: i32 = 0; // §728
        let mut literal_mode: i32 = 0; // §728
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        cmd = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
        stack_no = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
        if (stack_no >= self.colorstackused()) {
            {
                self.print_nl(345i32);
                self.print(1143i32);
                self.print_int(((stack_no) as i64));
                self.print(1144i32);
                self.print_nl(345i32);
                return;
            }
        }
        match cmd {
            colorstack_set | colorstack_push => {
                {
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.show_token_list(__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh()) as usize)].hh().rh(), null, (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    if (cmd == colorstack_set) {
                        literal_mode = self.colorstackset(stack_no, s);
                    } else {
                        literal_mode = self.colorstackpush(stack_no, s);
                    }
                    if ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) > 0i32) {
                        self.literal(s, literal_mode, false);
                    }
                    self.flush_str(s);
                    return;
                }
            }
            colorstack_pop => {
                literal_mode = self.colorstackpop(stack_no);
            }
            colorstack_current => {
                literal_mode = self.colorstackcurrent(stack_no);
            }
            _ => {
                self.confusion(1145i32);
            }
        }
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) > 0i32) {
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
    // §728
    pub fn pdf_out_colorstack_startpage(&mut self) {
        let mut i: i32 = 0; // §728
        let mut max: i32 = 0; // §728
        let mut start_status: i32 = 0; // §728
        let mut literal_mode: i32 = 0; // §728
        let mut s: str_number = 0; // §728
        i = 0i32;
        max = self.colorstackused();
        while (i < max) {
            {
                start_status = self.colorstackskippagestart(i);
                if (start_status == 0i32) {
                    {
                        literal_mode = self.colorstackcurrent(i);
                        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) > 0i32) {
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
    // §728
    pub fn pdf_out_setmatrix(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §728
        let mut s: str_number = 0; // §728
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        old_setting = self.selector;
        self.selector = new_string;
        self.show_token_list(__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().rh(), null, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        {
            if ((self.pool_ptr).wrapping_add(7i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 0i32;
        if (self.pdfsetmatrix(self.str_start[crate::ix::U((self.str_ptr) as usize)], self.cur_h, (self.cur_page_height).wrapping_sub(self.cur_v)) == 1i32) {
            {
                {
                    if ((self.pool_ptr).wrapping_add(7i32) > pool_size) {
                        self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                    }
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 48i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 48i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 99i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 109i32;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                s = self.make_string();
                self.literal(s, set_origin, false);
            }
        } else {
            {
                self.pdf_error(1146i32, 1147i32);
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
    // §728
    pub fn pdf_out_save(&mut self, mut p: halfword) {
        self.checkpdfsave(self.cur_h, self.cur_v);
        self.literal(113i32, set_origin, false);
    }

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §728
    pub fn pdf_out_restore(&mut self, mut p: halfword) {
        self.checkpdfrestore(self.cur_h, self.cur_v);
        self.literal(81i32, set_origin, false);
    }

}
