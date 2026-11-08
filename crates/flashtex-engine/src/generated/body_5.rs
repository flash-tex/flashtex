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

    /// \[32f] PDF shipping out.
    /// To ship out a \TeX\ box to PDF page description we need to implement
    /// `pdf_hlist_out`, `pdf_vlist_out` and `pdf_ship_out`, which are equivalent to
    /// the \TeX' original `hlist_out`, `vlist_out` and `ship_out` resp. But first we
    /// need to declare some procedures needed in `pdf_hlist_out` and `pdf_vlist_out`.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §728
    pub fn pdf_special(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §728
        let mut s: str_number = 0; // §728
        let mut h: halfword = 0; // §728
        let mut q: halfword = 0; // §728
        let mut r: halfword = 0; // §728
        let mut old_mode: i32 = 0; // §728
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        old_setting = self.selector;
        self.selector = self.selector;
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
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
        self.literal(s, scan_special, true);
        self.flush_str(s);
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
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
    pub fn pdf_print_toks(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §728
        s = self.tokens_to_string(p);
        if ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) > 0i32) {
            self.pdf_print(s);
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
    pub fn pdf_print_toks_ln(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §728
        s = self.tokens_to_string(p);
        if ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) > 0i32) {
            {
                {
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.flush_str(s);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §773
    pub fn pdf_write_obj(&mut self, mut n: i32) {
        let mut s: str_number = 0; // §773
        let mut f: crate::system::ByteFile = Default::default(); // §773
        s = self.tokens_to_string(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)]);
        {
            self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)]);
            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] = null;
        }
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
            {
                self.pdf_begin_dict(n, 0i32);
                if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] != null) {
                    {
                        self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]);
                        {
                            self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]);
                            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = null;
                        }
                    }
                }
                self.pdf_begin_stream();
            }
        } else {
            self.pdf_begin_obj(n, 1i32);
        }
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)] > 0i32) {
            {
                self.cur_name = s;
                self.cur_area = 345i32;
                self.cur_ext = 345i32;
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                if (!{ let mut __f0 = ::core::mem::take(&mut f); let __r = self.tex_b_openin(&mut __f0); f = __f0; __r }) {
                    {
                        self.print_nl(264i32);
                        self.print(s);
                        self.print(1178i32);
                        self.pdf_error(1179i32, 1180i32);
                    }
                }
                self.print(1083i32);
                self.print(s);
                if (!crate::system::eof(&f)) {
                    {
                        while (!crate::system::eof(&f)) {
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
                                    { let __a817_0 = self.pdf_ptr; let __a817_1 = { let mut __f0 = ::core::mem::take(&mut f); let __r = self.getc(&mut __f0); f = __f0; __r }; self.pdf_buf_set(__a817_0, __a817_1) };
                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                        if (((!((self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)]) != 0)) && (self.pdf_ptr > 0i32)) && (self.pdf_buf_get((self.pdf_ptr).wrapping_sub(1i32)) != 10i32)) {
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
                                    self.pdf_buf_set(self.pdf_ptr, 10i32);
                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
                self.print(1035i32);
                { let mut __f0 = ::core::mem::take(&mut f); let __r = self.b_close(&mut __f0); f = __f0; __r };
            }
        } else {
            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
                self.pdf_print(s);
            } else {
                {
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
            self.pdf_end_stream();
        } else {
            self.pdf_end_obj();
        }
        self.flush_str(s);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §773
    pub fn flush_whatsit_node(&mut self, mut p: halfword, mut s: small_number) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        if (__av_mem[crate::ix::U((p) as usize)].hh().rh() != null) {
            self.pdf_error(1181i32, 1182i32);
        }
        self.flush_node_list(p);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §779
    pub fn pdf_write_image(&mut self, mut n: i32) {
        self.pdf_begin_dict(n, 0i32);
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)] != null) {
            {
                self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)]);
                {
                    self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)]);
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)] = null;
                }
            }
        }
        if (self.fixed_pdf_draftmode == 0i32) {
            self.write_image(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(4i32)) as usize)]);
        }
        self.delete_image(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(4i32)) as usize)]);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §786
    pub fn pdf_print_rect_spec(&mut self, mut r: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.pdf_print_mag_bp((__av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(__av_mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()));
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
        self.pdf_print_mag_bp((__av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(__av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()));
    }

    /// When a destination is created we need to check whether another destination
    /// with the same identifier already exists and give a warning if needed.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1565
    pub fn warn_dest_dup(&mut self, mut id: i32, mut byname: small_number, mut s1: str_number, mut s2: str_number) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (__av_eqtb[crate::ix::U(((29370i32) - 1) as usize)].int() > 0i32) {
            return;
        }
        self.pdf_warning(s1, 1861i32, true, false);
        if (byname > 0i32) {
            {
                self.print(1845i32);
                self.print_mark(id);
            }
        } else {
            {
                self.print(1208i32);
                self.print_int(((id) as i64));
            }
        }
        self.print(1862i32);
        self.print(s2);
        self.print_ln();
        self.show_context();
    }

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1631
    pub fn write_action(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §1631
        let mut d: i32 = 0; // §1631
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == pdf_action_user) {
            {
                self.pdf_print_toks_ln(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                return;
            }
        }
        self.pdf_print(1254i32);
        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != null) {
            {
                self.pdf_print(1949i32);
                s = self.tokens_to_string(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                if ((self.str_pool[crate::ix::U((self.str_start[crate::ix::U((s) as usize)]) as usize)] == 40i32) && (self.str_pool[crate::ix::U((((self.str_start[crate::ix::U((s) as usize)]).wrapping_add((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]))).wrapping_sub(1i32)) as usize)] == 41i32)) {
                    self.pdf_print(s);
                } else {
                    {
                        self.pdf_print_str(s);
                    }
                }
                self.flush_str(s);
                self.pdf_print(32i32);
                if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() > 0i32) {
                    {
                        self.pdf_print(1950i32);
                        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == 1i32) {
                            self.pdf_print(1951i32);
                        } else {
                            self.pdf_print(1952i32);
                        }
                    }
                }
            }
        }
        match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
            pdf_action_page => {
                {
                    if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                        {
                            self.pdf_print(1953i32);
                            { let __a818_0 = ((self.get_obj(obj_type_page, __av_mem[crate::ix::U((p) as usize)].hh().rh(), false)) as i64); self.pdf_print_int(__a818_0) };
                            self.pdf_print(1087i32);
                        }
                    } else {
                        {
                            self.pdf_print(1954i32);
                            self.pdf_print_int((((__av_mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_sub(1i32)) as i64));
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
                            self.pdf_buf_set(self.pdf_ptr, 32i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                    { let __a819_0 = self.tokens_to_string(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()); self.pdf_print(__a819_0) };
                    self.flush_str(self.last_tokens_string);
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
                            self.pdf_buf_set(self.pdf_ptr, 93i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
            pdf_action_goto => {
                {
                    if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                        {
                            self.pdf_print(1955i32);
                            d = self.get_obj(obj_type_dest, __av_mem[crate::ix::U((p) as usize)].hh().rh(), (((__av_mem[crate::ix::U((p) as usize)].hh().b1() % 2i32)) != 0));
                        }
                    } else {
                        self.pdf_print(1956i32);
                    }
                    if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() % 2i32) == 1i32) {
                        {
                            { let __a820_0 = 68i32; let __a820_1 = self.tokens_to_string(__av_mem[crate::ix::U((p) as usize)].hh().rh()); self.pdf_str_entry(__a820_0, __a820_1) };
                            self.flush_str(self.last_tokens_string);
                        }
                    } else {
                        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                            self.pdf_indirect(68i32, d);
                        } else {
                            self.pdf_error(1875i32, 1850i32);
                        }
                    }
                }
            }
            pdf_action_thread => {
                {
                    self.pdf_print(1957i32);
                    if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                        d = self.get_obj(obj_type_thread, __av_mem[crate::ix::U((p) as usize)].hh().rh(), (((__av_mem[crate::ix::U((p) as usize)].hh().b1() % 2i32)) != 0));
                    }
                    if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() % 2i32) == 1i32) {
                        {
                            { let __a821_0 = 68i32; let __a821_1 = self.tokens_to_string(__av_mem[crate::ix::U((p) as usize)].hh().rh()); self.pdf_str_entry(__a821_0, __a821_1) };
                            self.flush_str(self.last_tokens_string);
                        }
                    } else {
                        if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                            self.pdf_indirect(68i32, d);
                        } else {
                            self.pdf_int_entry(68i32, __av_mem[crate::ix::U((p) as usize)].hh().rh());
                        }
                    }
                }
            }
            _ => {}
        }
        if (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh() != null) {
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
                if (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null) {
                    { let __a822_0 = 1958i32; let __a822_1 = self.get_obj(obj_type_struct_dest, __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh(), ((((__av_mem[crate::ix::U((p) as usize)].hh().b1() / 2i32) % 2i32)) != 0)); self.pdf_indirect(__a822_0, __a822_1) };
                } else {
                    {
                        self.pdf_print(1959i32);
                        { let __a823_0 = self.tokens_to_string(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh()); self.pdf_print(__a823_0) };
                        self.flush_str(self.last_tokens_string);
                    }
                }
            }
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

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1631
    pub fn set_rect_dimens(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled, mut w: scaled, mut h: scaled, mut d: scaled, mut margin: scaled) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        { let __v824 = self.cur_h; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v824); }
        if (w == (1073741824i32).wrapping_neg()) {
            { let __v825 = (x).wrapping_add(__av_mem[crate::ix::U(((parent_box).wrapping_add(1i32)) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v825); }
        } else {
            { let __v826 = (self.cur_h).wrapping_add(w); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v826); }
        }
        if (h == (1073741824i32).wrapping_neg()) {
            { let __v827 = (y).wrapping_sub(__av_mem[crate::ix::U(((parent_box).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v827); }
        } else {
            { let __v828 = (self.cur_v).wrapping_sub(h); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v828); }
        }
        if (d == (1073741824i32).wrapping_neg()) {
            { let __v829 = (y).wrapping_add(__av_mem[crate::ix::U(((parent_box).wrapping_add(2i32)) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v829); }
        } else {
            { let __v830 = (self.cur_v).wrapping_add(d); __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v830); }
        }
        if (self.is_shipping_page && self.matrixused()) {
            {
                self.matrixtransformrect(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), (self.cur_page_height).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int()), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), (self.cur_page_height).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()));
                { let __v831 = self.getllx(); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v831); }
                { let __v832 = (self.cur_page_height).wrapping_sub(self.getlly()); __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v832); }
                { let __v833 = self.geturx(); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v833); }
                { let __v834 = (self.cur_page_height).wrapping_sub(self.getury()); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v834); }
            }
        }
        { let __v835 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_sub(margin); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v835); }
        { let __v836 = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(margin); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v836); }
        { let __v837 = (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(margin); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v837); }
        { let __v838 = (__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int()).wrapping_add(margin); __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v838); }
    }

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1631
    pub fn do_annot(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!self.is_shipping_page) {
            self.pdf_error(1875i32, 1960i32);
        }
        if self.doing_leaders {
            return;
        }
        if (self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 > (((2i32).wrapping_neg()) as i64)) {
            { let __v839 = self.pdf_new_objnum(); __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(__v839); }
        }
        self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), 0i32);
        self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int();
            self.pdf_annot_list = self.append_ptr(self.pdf_annot_list, self.pdf_append_list_arg);
        }
        if (self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 == (((2i32).wrapping_neg()) as i64)) {
            self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 = (((1i32).wrapping_neg()) as i64);
        }
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1636
    pub fn push_link_level(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.pdf_link_stack_ptr >= pdf_max_link_level) {
            self.overflow(1961i32, pdf_max_link_level);
        }
        self.pdfassert(((__av_mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) && (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_start_link_node)));
        self.pdf_link_stack_ptr = (self.pdf_link_stack_ptr).wrapping_add(1i32);
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].nesting_level = self.cur_s;
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node = self.copy_node_list(p);
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].ref_link_node = p;
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1636
    pub fn pop_link_level(&mut self) {
        self.pdfassert((self.pdf_link_stack_ptr > 0i32));
        self.flush_node_list(self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node);
        self.pdf_link_stack_ptr = (self.pdf_link_stack_ptr).wrapping_sub(1i32);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1636
    pub fn do_link(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!self.is_shipping_page) {
            self.pdf_error(1875i32, 1962i32);
        }
        self.pdfassert((__av_mem[crate::ix::U((parent_box) as usize)].hh().b0() == hlist_node));
        if (self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 > (((2i32).wrapping_neg()) as i64)) {
            { let __v840 = self.pdf_new_objnum(); __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(__v840); }
        }
        self.push_link_level(p);
        self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int());
        self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int();
            self.pdf_link_list = self.append_ptr(self.pdf_link_list, self.pdf_append_list_arg);
        }
        if (self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 == (((2i32).wrapping_neg()) as i64)) {
            self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 = (((1i32).wrapping_neg()) as i64);
        }
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1636
    pub fn end_link(&mut self) {
        let mut p: halfword = 0; // §1636
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.pdf_link_stack_ptr < 1i32) {
            self.pdf_error(1875i32, 1963i32);
        }
        if (self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].nesting_level != self.cur_s) {
            self.pdf_warning(0i32, 1964i32, true, true);
        }
        if (__av_mem[crate::ix::U(((self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
            {
                p = self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].ref_link_node;
                if (self.is_shipping_page && self.matrixused()) {
                    {
                        self.matrixrecalculate((self.cur_h).wrapping_add(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()));
                        { let __v841 = (self.getllx()).wrapping_sub(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v841); }
                        { let __v842 = ((self.cur_page_height).wrapping_sub(self.getury())).wrapping_sub(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v842); }
                        { let __v843 = (self.geturx()).wrapping_add(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v843); }
                        { let __v844 = ((self.cur_page_height).wrapping_sub(self.getlly())).wrapping_add(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v844); }
                    }
                } else {
                    { let __v845 = (self.cur_h).wrapping_add(__av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int()); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v845); }
                }
            }
        }
        self.pop_link_level();
    }

    /// For ``running'' annotations we must append a new node when the end of
    /// annotation is in other box than its start. The new created node is identical to
    /// corresponding whatsit node representing the start of annotation,  but its
    /// `info` field is `max_halfword`. We set `info` field just before destroying the
    /// node, in order to use `flush_node_list` to do the job.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn append_link(&mut self, mut parent_box: halfword, mut x: scaled, mut y: scaled, mut i: small_number) {
        let mut p: halfword = 0; // §1637
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.pdfassert((__av_mem[crate::ix::U((parent_box) as usize)].hh().b0() == hlist_node));
        p = self.copy_node_list(self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].link_node);
        self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].ref_link_node = p;
        __av_mem[crate::ix::U((p) as usize)].set_hh_lh(max_halfword);
        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
        self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29928i32) - 1) as usize)].int());
        self.pdf_create_obj(obj_type_others, 0i32);
        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = self.obj_ptr;
            self.pdf_link_list = self.append_ptr(self.pdf_link_list, self.pdf_append_list_arg);
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn append_bead(&mut self, mut p: halfword) {
        let mut a: i32 = 0; // §1638
        let mut b: i32 = 0; // §1638
        let mut c: i32 = 0; // §1638
        let mut t: i32 = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!self.is_shipping_page) {
            self.pdf_error(1875i32, 1965i32);
        }
        t = self.get_obj(obj_type_thread, __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        b = self.pdf_new_objnum();
        self.obj_tab[crate::ix::U((b) as usize)].int4 = self.pdf_get_mem(pdfmem_bead_size);
        { let __ix846 = (self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(1i32); let __v847 = self.pdf_last_page; self.pdf_mem[crate::ix::U((__ix846) as usize)] = __v847; }
        self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((b) as usize)].int4) as usize)] = p;
        if (__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
            { let __ix848 = (self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(4i32); let __v849 = self.tokens_to_string(__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh()); self.pdf_mem[crate::ix::U((__ix848) as usize)] = __v849; }
        } else {
            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(4i32)) as usize)] = 0i32;
        }
        if (self.obj_tab[crate::ix::U((t) as usize)].int4 == 0i32) {
            {
                self.obj_tab[crate::ix::U((t) as usize)].int4 = b;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(2i32)) as usize)] = b;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(3i32)) as usize)] = b;
            }
        } else {
            {
                a = self.obj_tab[crate::ix::U((t) as usize)].int4;
                c = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(3i32)) as usize)];
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(3i32)) as usize)] = c;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(2i32)) as usize)] = a;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((a) as usize)].int4).wrapping_add(3i32)) as usize)] = b;
                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((c) as usize)].int4).wrapping_add(2i32)) as usize)] = b;
            }
        }
        {
            self.pdf_append_list_arg = b;
            self.pdf_bead_list = self.append_ptr(self.pdf_bead_list, self.pdf_append_list_arg);
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn do_thread(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if self.doing_leaders {
            return;
        }
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_start_thread_node) {
            {
                self.pdf_thread_wd = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                self.pdf_thread_ht = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                self.pdf_thread_dp = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                self.pdf_last_thread_id = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                self.pdf_last_thread_named_id = (__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32);
                if self.pdf_last_thread_named_id {
                    { let __ix850 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v851 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix850) as usize)].set_hh_lh(__v851); }
                }
                self.pdf_thread_level = self.cur_s;
            }
        }
        self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29930i32) - 1) as usize)].int());
        self.append_bead(p);
        self.last_thread = p;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn append_thread(&mut self, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        let mut p: halfword = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(pdf_thread_node_size);
        __av_mem[crate::ix::U((p) as usize)].set_hh_lh(max_halfword);
        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(null);
        { let __v852 = self.pdf_thread_wd; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v852); }
        { let __v853 = self.pdf_thread_ht; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v853); }
        { let __v854 = self.pdf_thread_dp; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v854); }
        __av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_hh_lh(null);
        { let __v855 = self.pdf_last_thread_id; __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_rh(__v855); }
        if self.pdf_last_thread_named_id {
            {
                { let __ix856 = __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v857 = (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); __av_mem[crate::ix::U((__ix856) as usize)].set_hh_lh(__v857); }
                __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b1(1i32);
            }
        } else {
            __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b1(0i32);
        }
        self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29930i32) - 1) as usize)].int());
        self.append_bead(p);
        self.last_thread = p;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn end_thread(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.pdf_thread_level != self.cur_s) {
            self.pdf_error(1875i32, 1966i32);
        }
        if ((self.pdf_thread_dp == (1073741824i32).wrapping_neg()) && (self.last_thread != null)) {
            { let __ix858 = (self.last_thread).wrapping_add(4i32); let __v859 = (self.cur_v).wrapping_add(__av_eqtb[crate::ix::U(((29930i32) - 1) as usize)].int()); __av_mem[crate::ix::U((__ix858) as usize)].set_int(__v859); }
        }
        if self.pdf_last_thread_named_id {
            self.delete_token_ref(self.pdf_last_thread_id);
        }
        self.last_thread = null;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn open_subentries(&mut self, mut p: halfword) -> i32 {
        let mut open_subentries: i32 = 0;
        let mut k: i32 = 0; // §1638
        let mut c: i32 = 0; // §1638
        let mut l: i32 = 0; // §1638
        let mut r: i32 = 0; // §1638
        k = 0i32;
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((p) as usize)].int4).wrapping_add(4i32)) as usize)] != 0i32) {
            {
                l = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((p) as usize)].int4).wrapping_add(4i32)) as usize)];
                loop {
                    k = (k).wrapping_add(1i32);
                    c = self.open_subentries(l);
                    if (self.obj_tab[crate::ix::U((l) as usize)].int0 > 0i32) {
                        k = (k).wrapping_add(c);
                    }
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((l) as usize)].int4).wrapping_add(1i32)) as usize)] = p;
                    r = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((l) as usize)].int4).wrapping_add(3i32)) as usize)];
                    if (r == 0i32) {
                        self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((p) as usize)].int4).wrapping_add(5i32)) as usize)] = l;
                    }
                    l = r;
                    if (l == 0i32) { break; }
                }
            }
        }
        if (self.obj_tab[crate::ix::U((p) as usize)].int0 > 0i32) {
            self.obj_tab[crate::ix::U((p) as usize)].int0 = k;
        } else {
            self.obj_tab[crate::ix::U((p) as usize)].int0 = (k).wrapping_neg();
        }
        open_subentries = k;
        open_subentries
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn do_dest(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        let mut k: i32 = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!self.is_shipping_page) {
            self.pdf_error(1875i32, 1967i32);
        }
        if self.doing_leaders {
            return;
        }
        if (__av_mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().rh() == null) {
            k = self.get_obj(obj_type_dest, __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        } else {
            k = self.get_obj(obj_type_struct_dest, __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        }
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != null) {
            {
                self.warn_dest_dup(__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 1875i32, 1876i32);
                return;
            }
        }
        self.obj_tab[crate::ix::U((k) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = k;
            self.pdf_dest_list = self.append_ptr(self.pdf_dest_list, self.pdf_append_list_arg);
        }
        match __av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() {
            pdf_dest_xyz => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29929i32) - 1) as usize)].int());
                } else {
                    {
                        { let __v860 = self.cur_h; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v860); }
                        { let __v861 = self.cur_v; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v861); }
                    }
                }
            }
            pdf_dest_fith | pdf_dest_fitbh => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29929i32) - 1) as usize)].int());
                } else {
                    { let __v862 = self.cur_v; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v862); }
                }
            }
            pdf_dest_fitv | pdf_dest_fitbv => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29929i32) - 1) as usize)].int());
                } else {
                    { let __v863 = self.cur_h; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v863); }
                }
            }
            pdf_dest_fit | pdf_dest_fitb => {
            }
            pdf_dest_fitr => {
                self.set_rect_dimens(p, parent_box, x, y, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), __av_eqtb[crate::ix::U(((29929i32) - 1) as usize)].int());
            }
            _ => {}
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn out_form(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
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
        if (self.pdf_lookup_list(self.pdf_xform_list, __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) == null) {
            {
                self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh();
                self.pdf_xform_list = self.append_ptr(self.pdf_xform_list, self.pdf_append_list_arg);
            }
        }
        self.cur_v = (self.cur_v).wrapping_add(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)]);
        self.pdf_print(1046i32);
        self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print(1162i32);
        self.pdf_print_int(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int0) as i64));
        if (self.pdf_resname_prefix != 0i32) {
            self.pdf_print(self.pdf_resname_prefix);
        }
        {
            self.pdf_print(1968i32);
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

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn out_image(&mut self, mut p: halfword) {
        let mut image: i32 = 0; // §1638
        let mut groupref: i32 = 0; // §1638
        let mut img_w: i32 = 0; // §1638
        let mut img_h: i32 = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        image = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(4i32)) as usize)];
        if ((self.image_rotate(image) == 90i32) || (self.image_rotate(image) == 270i32)) {
            {
                img_h = self.image_width(image);
                img_w = self.image_height(image);
            }
        } else {
            {
                img_w = self.image_width(image);
                img_h = self.image_height(image);
            }
        }
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
        if (self.pdf_lookup_list(self.pdf_ximage_list, __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) == null) {
            {
                self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh();
                self.pdf_ximage_list = self.append_ptr(self.pdf_ximage_list, self.pdf_append_list_arg);
            }
        }
        if (!self.is_pdf_image(image)) {
            {
                if self.is_png_image(image) {
                    {
                        groupref = self.get_image_group_ref(image);
                        if ((groupref > 0i32) && (self.pdf_page_group_val == 0i32)) {
                            self.pdf_page_group_val = groupref;
                        }
                    }
                }
                { let __a864_0 = self.ext_xn_over_d(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.ten_pow[crate::ix::U((6i32) as usize)], self.one_hundred_bp); let __a864_1 = 4i32; self.pdf_print_real(__a864_0, __a864_1) };
                self.pdf_print(1157i32);
                { let __a865_0 = self.ext_xn_over_d((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()), self.ten_pow[crate::ix::U((6i32) as usize)], self.one_hundred_bp); let __a865_1 = 4i32; self.pdf_print_real(__a865_0, __a865_1) };
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
                self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
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
            }
        } else {
            {
                groupref = self.get_image_group_ref(image);
                if ((groupref != 0i32) && (self.pdf_page_group_val == 0i32)) {
                    {
                        if (groupref == (1i32).wrapping_neg()) {
                            {
                                self.pdf_page_group_val = self.pdf_new_objnum();
                                self.set_image_group_ref(image, self.pdf_page_group_val);
                            }
                        } else {
                            self.pdf_page_group_val = groupref;
                        }
                    }
                }
                { let __a866_0 = self.ext_xn_over_d(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.ten_pow[crate::ix::U((6i32) as usize)], img_w); let __a866_1 = 6i32; self.pdf_print_real(__a866_0, __a866_1) };
                self.pdf_print(1157i32);
                { let __a867_0 = self.ext_xn_over_d((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()), self.ten_pow[crate::ix::U((6i32) as usize)], img_h); let __a867_1 = 6i32; self.pdf_print_real(__a867_0, __a867_1) };
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
                { let __a868_0 = ((self.cur_h).wrapping_sub(self.pdf_origin_h)).wrapping_sub({ let __a869_0 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); let __a869_1 = self.epdf_orig_x(image); let __a869_2 = img_w; self.ext_xn_over_d(__a869_0, __a869_1, __a869_2) }); self.pdf_print_bp(__a868_0) };
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
                { let __a870_0 = ((self.pdf_origin_v).wrapping_sub(self.cur_v)).wrapping_sub({ let __a871_0 = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); let __a871_1 = self.epdf_orig_y(image); let __a871_2 = img_h; self.ext_xn_over_d(__a871_0, __a871_1, __a871_2) }); self.pdf_print_bp(__a870_0) };
            }
        }
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
        self.pdf_print(1163i32);
        self.pdf_print_int(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int0) as i64));
        if (self.pdf_resname_prefix != 0i32) {
            self.pdf_print(self.pdf_resname_prefix);
        }
        {
            self.pdf_print(1968i32);
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

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn gap_amount(&mut self, mut p: halfword, mut cur_pos: scaled) -> scaled {
        let mut gap_amount: scaled = 0;
        let mut snap_unit: scaled = 0; // §1638
        let mut stretch_amount: scaled = 0; // §1638
        let mut shrink_amount: scaled = 0; // §1638
        let mut last_pos: scaled = 0; // §1638
        let mut next_pos: scaled = 0; // §1638
        let mut g: scaled = 0; // §1638
        let mut g2: scaled = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        snap_unit = __av_mem[crate::ix::U(((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int();
        if (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b0() > normal) {
            stretch_amount = max_dimen;
        } else {
            stretch_amount = __av_mem[crate::ix::U(((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(2i32)) as usize)].int();
        }
        if (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b1() > normal) {
            shrink_amount = max_dimen;
        } else {
            shrink_amount = __av_mem[crate::ix::U(((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(3i32)) as usize)].int();
        }
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_node) {
            last_pos = (self.pdf_snapy_refpos).wrapping_add((snap_unit).wrapping_mul(((cur_pos).wrapping_sub(self.pdf_snapy_refpos)).total_div(snap_unit)));
        } else {
            self.pdf_error(1969i32, 1970i32);
        }
        next_pos = (last_pos).wrapping_add(snap_unit);
        g = max_dimen;
        g2 = max_dimen;
        gap_amount = 0i32;
        if ((cur_pos).wrapping_sub(last_pos) < shrink_amount) {
            g = (cur_pos).wrapping_sub(last_pos);
        }
        if ((next_pos).wrapping_sub(cur_pos) < stretch_amount) {
            g2 = (next_pos).wrapping_sub(cur_pos);
        }
        if ((g == max_dimen) && (g2 == max_dimen)) {
            return gap_amount;
        }
        if (g2 <= g) {
            gap_amount = g2;
        } else {
            gap_amount = (g).wrapping_neg();
        }
        gap_amount
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn get_vpos(&mut self, mut p: halfword, mut q: halfword, mut b: halfword) -> halfword {
        let mut get_vpos: halfword = 0;
        let mut tmp_v: scaled = 0; // §1638
        let mut g_order: glue_ord = 0; // §1638
        let mut g_sign: i32 = 0; // §1638
        let mut glue_temp: f64 = 0.0; // §1638
        let mut cur_glue: f64 = 0.0; // §1638
        let mut cur_g: scaled = 0; // §1638
        let mut this_box: halfword = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        tmp_v = self.cur_v;
        this_box = b;
        cur_g = 0i32;
        cur_glue = 0.0f64;
        g_order = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        while ((p != q) && (p != null)) {
            {
                if (p >= self.hi_mem_min) {
                    self.confusion(1979i32);
                } else {
                    {
                        match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                            hlist_node | vlist_node | rule_node => {
                                tmp_v = ((tmp_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                            }
                            whatsit_node => {
                                if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                    tmp_v = ((tmp_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                }
                            }
                            glue_node => {
                                {
                                    // §1639
                                    {
                                        self.g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                        self.rule_ht = (__av_mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                        if (g_sign != normal) {
                                            {
                                                if (g_sign == stretching) {
                                                    {
                                                        if (__av_mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                            {
                                                                cur_glue = (cur_glue + ((__av_mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                                if (glue_temp > 1000000000.0f64) {
                                                                    glue_temp = 1000000000.0f64;
                                                                } else {
                                                                    if (glue_temp < (-1000000000.0f64)) {
                                                                        glue_temp = (-1000000000.0f64);
                                                                    }
                                                                }
                                                                cur_g = crate::system::pas_round(glue_temp);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    if (__av_mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                        {
                                                            cur_glue = (cur_glue - ((__av_mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                            glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                            if (glue_temp > 1000000000.0f64) {
                                                                glue_temp = 1000000000.0f64;
                                                            } else {
                                                                if (glue_temp < (-1000000000.0f64)) {
                                                                    glue_temp = (-1000000000.0f64);
                                                                }
                                                            }
                                                            cur_g = crate::system::pas_round(glue_temp);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.rule_ht = (self.rule_ht).wrapping_add(cur_g);
                                    }
                                    // §1638
                                    tmp_v = (tmp_v).wrapping_add(self.rule_ht);
                                }
                            }
                            kern_node => {
                                tmp_v = (tmp_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                            }
                            _ => {
                            }
                        }
                    }
                }
                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        get_vpos = tmp_v;
        get_vpos
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn do_snapy_comp(&mut self, mut p: halfword, mut b: halfword) {
        let mut q: halfword = 0; // §1638
        let mut tmp_v: scaled = 0; // §1638
        let mut g: scaled = 0; // §1638
        let mut g2: scaled = 0; // §1638
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (!(((!(p >= self.hi_mem_min)) && (__av_mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_snapy_comp_node))) {
            self.pdf_error(1969i32, 1980i32);
        }
        q = p;
        while (q != null) {
            {
                if (((!(q >= self.hi_mem_min)) && (__av_mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node)) && (__av_mem[crate::ix::U((q) as usize)].hh().b1() == pdf_snapy_node)) {
                    {
                        tmp_v = self.get_vpos(p, q, b);
                        g = self.gap_amount(q, tmp_v);
                        g2 = self.round_xn_over_d(g, __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), 1000i32);
                        self.cur_v = (self.cur_v).wrapping_add(g2);
                        __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int((g).wrapping_sub(g2));
                        if (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                            __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(1i32);
                        }
                        return;
                    }
                }
                q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
            }
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1638
    pub fn do_snapy(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.count_do_snapy = (self.count_do_snapy).wrapping_add(1i32);
        if (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() != 0i32) {
            self.cur_v = (self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
        } else {
            self.cur_v = (self.cur_v).wrapping_add(self.gap_amount(p, self.cur_v));
        }
    }

    /// The implementation of procedure `pdf_hlist_out` is similar to `hlist_out`.
    // §730
    pub fn pdf_hlist_out(&mut self) {
        let mut base_line: scaled = 0; // §730
        let mut left_edge: scaled = 0; // §730
        let mut save_h: scaled = 0; // §730
        let mut this_box: halfword = 0; // §730
        let mut g_order: glue_ord = 0; // §730
        let mut g_sign: i32 = 0; // §730
        let mut p: halfword = 0; // §730
        let mut leader_box: halfword = 0; // §730
        let mut leader_wd: scaled = 0; // §730
        let mut lx: scaled = 0; // §730
        let mut outer_doing_leaders: bool = false; // §730
        let mut edge: scaled = 0; // §730
        let mut prev_p: halfword = 0; // §730
        let mut glue_temp: f64 = 0.0; // §730
        let mut cur_glue: f64 = 0.0; // §730
        let mut cur_g: scaled = 0; // §730
        let mut i: small_number = 0; // §730
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        base_line = self.cur_v;
        prev_p = (this_box).wrapping_add(5i32);
        // §1715
        if (self.eTeX_mode == 1i32) {
            {
                // §1711
                {
                    self.temp_ptr = self.get_avail();
                    { let __ix872 = self.temp_ptr; __av_mem[crate::ix::U((__ix872) as usize)].set_hh_lh(before); }
                    { let __ix873 = self.temp_ptr; let __v874 = self.LR_ptr; __av_mem[crate::ix::U((__ix873) as usize)].set_hh_rh(__v874); }
                    self.LR_ptr = self.temp_ptr;
                }
                // §1715
                if (__av_mem[crate::ix::U((this_box) as usize)].hh().b1() == dlist) {
                    if (self.cur_dir == right_to_left) {
                        {
                            self.cur_dir = left_to_right;
                            self.cur_h = (self.cur_h).wrapping_sub(__av_mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        __av_mem[crate::ix::U((this_box) as usize)].set_hh_b1(0i32);
                    }
                }
                if ((self.cur_dir == right_to_left) && (__av_mem[crate::ix::U((this_box) as usize)].hh().b1() != reversed)) {
                    // §1722
                    {
                        save_h = self.cur_h;
                        self.temp_ptr = p;
                        p = self.new_kern(0i32);
                        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
                        __av_mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        self.cur_h = 0i32;
                        { let __v875 = { let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, null, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v875); }
                        { let __v876 = (self.cur_h).wrapping_neg(); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v876); }
                        self.cur_h = save_h;
                        __av_mem[crate::ix::U((this_box) as usize)].set_hh_b1(reversed);
                    }
                }
            }
        }
        // §730
        left_edge = self.cur_h;
        // §731
        {
            let __for_end_2 = self.pdf_link_stack_ptr;
            i = 1i32;
            while i <= __for_end_2 {
                {
                    self.pdfassert((__av_mem[crate::ix::U(((self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].link_node).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()));
                    if ((self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].nesting_level == self.cur_s) && self.gen_running_link) {
                        self.append_link(this_box, left_edge, base_line, i);
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        // §730
        while (p != null) {
            'l_reswitch_b: loop {
                // §732
                if (p >= self.hi_mem_min) {
                    {
                        loop {
                            'l_continue_f: {
                                self.dl_node(p);
                                self.f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                self.c = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                                if (((self.font_bc[crate::ix::U((self.f) as usize)] <= self.c) && (self.c <= self.font_ec[crate::ix::U((self.f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0() > min_quarterword)) {
                                    {
                                        {
                                            if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == new_font_type) {
                                                self.do_vf(self.f);
                                            }
                                            if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == virtual_font_type) {
                                                self.do_vf_packet(self.f, self.c);
                                            } else {
                                                {
                                                    self.pdf_begin_string(self.f);
                                                    self.pdf_print_char(self.f, self.c);
                                                    self.adv_char_width(self.f, self.c);
                                                }
                                            }
                                        }
                                        self.cur_h = (self.cur_h).wrapping_add({ let __s878 = ((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add({ let __s877 = ((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.effective_char(true, self.f, self.c))) as usize; self.font_info[crate::ix::U(__s877)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s878)] }.int());
                                        break 'l_continue_f;
                                    }
                                }
                                if self.mltex_enabled_p {
                                    // §1902
                                    {
                                        'l_found_f: {
                                            // §1904
                                            if (self.c >= __av_eqtb[crate::ix::U(((29332i32) - 1) as usize)].int()) {
                                                if (self.c <= __av_eqtb[crate::ix::U(((29333i32) - 1) as usize)].int()) {
                                                    if (__av_eqtb[crate::ix::U((((char_sub_code_base).wrapping_add(self.c)) - 1) as usize)].hh().rh() > 0i32) {
                                                        {
                                                            self.base_c = (__av_eqtb[crate::ix::U((((char_sub_code_base).wrapping_add(self.c)) - 1) as usize)].hh().rh() % 256i32);
                                                            self.accent_c = (__av_eqtb[crate::ix::U((((char_sub_code_base).wrapping_add(self.c)) - 1) as usize)].hh().rh() / 256i32);
                                                            if (self.font_ec[crate::ix::U((self.f) as usize)] >= self.base_c) {
                                                                if (self.font_bc[crate::ix::U((self.f) as usize)] <= self.base_c) {
                                                                    if (self.font_ec[crate::ix::U((self.f) as usize)] >= self.accent_c) {
                                                                        if (self.font_bc[crate::ix::U((self.f) as usize)] <= self.accent_c) {
                                                                            {
                                                                                self.ia_c = { let __s879 = ((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.effective_char(true, self.f, self.accent_c))) as usize; self.font_info[crate::ix::U(__s879)] }.qqqq();
                                                                                self.ib_c = { let __s880 = ((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.effective_char(true, self.f, self.base_c))) as usize; self.font_info[crate::ix::U(__s880)] }.qqqq();
                                                                                if (self.ib_c.b0() > min_quarterword) {
                                                                                    if (self.ia_c.b0() > min_quarterword) {
                                                                                        break 'l_found_f;
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            self.begin_diagnostic();
                                                            self.print_nl(2102i32);
                                                            self.print(self.c);
                                                            self.print(1642i32);
                                                            self.print(self.accent_c);
                                                            self.print(32i32);
                                                            self.print(self.base_c);
                                                            self.print(999i32);
                                                            self.slow_print(self.font_name[crate::ix::U((self.f) as usize)]);
                                                            self.print_char(33i32);
                                                            self.end_diagnostic(false);
                                                            break 'l_continue_f;
                                                        }
                                                    }
                                                }
                                            }
                                            self.begin_diagnostic();
                                            self.print_nl(998i32);
                                            self.print(2101i32);
                                            self.print(self.c);
                                            self.print(999i32);
                                            self.slow_print(self.font_name[crate::ix::U((self.f) as usize)]);
                                            self.print_char(33i32);
                                            self.end_diagnostic(false);
                                            break 'l_continue_f;
                                        }
                                        // §1902
                                        if (__av_eqtb[crate::ix::U(((29312i32) - 1) as usize)].int() > 99i32) {
                                            // §1905
                                            {
                                                self.begin_diagnostic();
                                                self.print_nl(2103i32);
                                                self.print(self.c);
                                                self.print(1642i32);
                                                self.print(self.accent_c);
                                                self.print(32i32);
                                                self.print(self.base_c);
                                                self.print(999i32);
                                                self.slow_print(self.font_name[crate::ix::U((self.f) as usize)]);
                                                self.print_char(46i32);
                                                self.end_diagnostic(false);
                                            }
                                        }
                                        // §1907
                                        self.base_x_height = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((self.f) as usize)])) as usize)].int();
                                        self.base_slant = (((self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((self.f) as usize)])) as usize)].int()) as f64) / 65536.0f64);
                                        self.accent_slant = self.base_slant;
                                        self.base_width = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.ib_c.b0())) as usize)].int();
                                        self.base_height = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((self.f) as usize)]).wrapping_add((self.ib_c.b1() / 16i32))) as usize)].int();
                                        self.accent_width = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.ia_c.b0())) as usize)].int();
                                        self.accent_height = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((self.f) as usize)]).wrapping_add((self.ia_c.b1() / 16i32))) as usize)].int();
                                        self.delta = crate::system::pas_round(((((((self.base_width).wrapping_sub(self.accent_width)) as f64) / 2.0f64) + (((self.base_height) as f64) * self.base_slant)) - (((self.base_x_height) as f64) * self.accent_slant)));
                                        self.cur_h = (self.cur_h).wrapping_add(self.delta);
                                        if ((self.base_height != self.base_x_height) && (self.accent_height > 0i32)) {
                                            {
                                                self.cur_v = (base_line).wrapping_add((self.base_x_height).wrapping_sub(self.base_height));
                                                {
                                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == new_font_type) {
                                                        self.do_vf(self.f);
                                                    }
                                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == virtual_font_type) {
                                                        self.do_vf_packet(self.f, self.accent_c);
                                                    } else {
                                                        {
                                                            self.pdf_begin_string(self.f);
                                                            self.pdf_print_char(self.f, self.accent_c);
                                                            self.adv_char_width(self.f, self.accent_c);
                                                        }
                                                    }
                                                }
                                                self.cur_v = base_line;
                                            }
                                        } else {
                                            {
                                                {
                                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == new_font_type) {
                                                        self.do_vf(self.f);
                                                    }
                                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == virtual_font_type) {
                                                        self.do_vf_packet(self.f, self.accent_c);
                                                    } else {
                                                        {
                                                            self.pdf_begin_string(self.f);
                                                            self.pdf_print_char(self.f, self.accent_c);
                                                            self.adv_char_width(self.f, self.accent_c);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        self.cur_h = (self.cur_h).wrapping_add(self.accent_width);
                                        self.cur_h = (self.cur_h).wrapping_add(((self.accent_width).wrapping_neg()).wrapping_sub(self.delta));
                                        {
                                            if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == new_font_type) {
                                                self.do_vf(self.f);
                                            }
                                            if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == virtual_font_type) {
                                                self.do_vf_packet(self.f, self.base_c);
                                            } else {
                                                {
                                                    self.pdf_begin_string(self.f);
                                                    self.pdf_print_char(self.f, self.base_c);
                                                    self.adv_char_width(self.f, self.base_c);
                                                }
                                            }
                                        }
                                        self.cur_h = (self.cur_h).wrapping_add(self.base_width);
                                    }
                                }
                            }
                            // §732
                            prev_p = __av_mem[crate::ix::U((prev_p) as usize)].hh().rh();
                            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                            if (!(p >= self.hi_mem_min)) { break; }
                        }
                    }
                } else {
                    // §733
                    {
                        'l_L15_f: {
                            'l_L13_f: {
                                'l_L14_f: {
                                    self.dl_node(p);
                                    match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §734
                                            if (__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null) {
                                                self.cur_h = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            } else {
                                                {
                                                    self.cur_v = (base_line).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    self.temp_ptr = p;
                                                    edge = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    if (self.cur_dir == right_to_left) {
                                                        self.cur_h = edge;
                                                    }
                                                    if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                                        self.pdf_vlist_out();
                                                    } else {
                                                        self.pdf_hlist_out();
                                                    }
                                                    self.cur_h = edge;
                                                    self.cur_v = base_line;
                                                }
                                            }
                                        }
                                        rule_node => {
                                            // §733
                                            {
                                                self.rule_ht = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1646
                                            match __av_mem[crate::ix::U((p) as usize)].hh().b1() {
                                                pdf_literal_node | pdf_lateliteral_node => {
                                                    self.pdf_out_literal(p);
                                                }
                                                pdf_colorstack_node => {
                                                    self.pdf_out_colorstack(p);
                                                }
                                                pdf_setmatrix_node => {
                                                    self.pdf_out_setmatrix(p);
                                                }
                                                pdf_save_node => {
                                                    self.pdf_out_save(p);
                                                }
                                                pdf_restore_node => {
                                                    self.pdf_out_restore(p);
                                                }
                                                pdf_refobj_node => {
                                                    {
                                                        self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                        self.pdf_obj_list = self.append_ptr(self.pdf_obj_list, self.pdf_append_list_arg);
                                                    }
                                                }
                                                pdf_refxform_node => {
                                                    // §1648
                                                    {
                                                        self.cur_v = base_line;
                                                        edge = self.cur_h;
                                                        self.out_form(p);
                                                        self.cur_h = (edge).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                        self.cur_v = base_line;
                                                    }
                                                }
                                                pdf_refximage_node => {
                                                    // §1647
                                                    {
                                                        self.cur_v = (base_line).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        edge = self.cur_h;
                                                        self.out_image(p);
                                                        self.cur_h = (edge).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                        self.cur_v = base_line;
                                                    }
                                                }
                                                pdf_annot_node => {
                                                    // §1646
                                                    self.do_annot(p, this_box, left_edge, base_line);
                                                }
                                                pdf_start_link_node => {
                                                    self.do_link(p, this_box, left_edge, base_line);
                                                }
                                                pdf_end_link_node => {
                                                    self.end_link();
                                                }
                                                pdf_dest_node => {
                                                    self.do_dest(p, this_box, left_edge, base_line);
                                                }
                                                pdf_thread_node => {
                                                    self.do_thread(p, this_box, left_edge, base_line);
                                                }
                                                pdf_start_thread_node => {
                                                    self.pdf_error(1875i32, 1991i32);
                                                }
                                                pdf_end_thread_node => {
                                                    self.pdf_error(1875i32, 1992i32);
                                                }
                                                pdf_save_pos_node => {
                                                    // §1642
                                                    {
                                                        self.pdf_last_x_pos = self.cur_h;
                                                        if self.is_shipping_page {
                                                            self.pdf_last_y_pos = (self.cur_page_height).wrapping_sub(self.cur_v);
                                                        } else {
                                                            self.pdf_last_y_pos = ((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth)).wrapping_sub(self.cur_v);
                                                        }
                                                    }
                                                }
                                                special_node | latespecial_node => {
                                                    // §1646
                                                    self.pdf_special(p);
                                                }
                                                pdf_snap_ref_point_node => {
                                                    // §1643
                                                    {
                                                        self.pdf_snapx_refpos = self.cur_h;
                                                        self.pdf_snapy_refpos = self.cur_v;
                                                    }
                                                }
                                                pdf_snapy_comp_node | pdf_snapy_node => {
                                                    // §1646
                                                }
                                                pdf_interword_space_on_node => {
                                                    self.gen_faked_interword_space = true;
                                                }
                                                pdf_interword_space_off_node => {
                                                    self.gen_faked_interword_space = false;
                                                }
                                                pdf_fake_space_node => {
                                                    self.pdf_insert_fake_space();
                                                }
                                                pdf_running_link_off_node => {
                                                    self.gen_running_link = false;
                                                }
                                                pdf_running_link_on_node => {
                                                    self.gen_running_link = true;
                                                }
                                                _ => {
                                                    self.out_what(p);
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §736
                                            {
                                                self.g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_wd = (__av_mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != normal) {
                                                    {
                                                        if (g_sign == stretching) {
                                                            {
                                                                if (__av_mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((__av_mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                        glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                                        if (glue_temp > 1000000000.0f64) {
                                                                            glue_temp = 1000000000.0f64;
                                                                        } else {
                                                                            if (glue_temp < (-1000000000.0f64)) {
                                                                                glue_temp = (-1000000000.0f64);
                                                                            }
                                                                        }
                                                                        cur_g = crate::system::pas_round(glue_temp);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            if (__av_mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((__av_mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                                    glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                                    if (glue_temp > 1000000000.0f64) {
                                                                        glue_temp = 1000000000.0f64;
                                                                    } else {
                                                                        if (glue_temp < (-1000000000.0f64)) {
                                                                            glue_temp = (-1000000000.0f64);
                                                                        }
                                                                    }
                                                                    cur_g = crate::system::pas_round(glue_temp);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                self.rule_wd = (self.rule_wd).wrapping_add(cur_g);
                                                if (self.eTeX_mode == 1i32) {
                                                    // §1700
                                                    if (((g_sign == stretching) && (__av_mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == shrinking) && (__av_mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (__av_mem[crate::ix::U((self.g) as usize)].hh().rh() == null) {
                                                                    self.free_node(self.g, glue_spec_size);
                                                                } else {
                                                                    { let __ix881 = self.g; let __v882 = (__av_mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); __av_mem[crate::ix::U((__ix881) as usize)].set_hh_rh(__v882); }
                                                                }
                                                            }
                                                            if (__av_mem[crate::ix::U((p) as usize)].hh().b1() < a_leaders) {
                                                                {
                                                                    __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    { let __v883 = self.rule_wd; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v883); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(glue_spec_size);
                                                                    { let __ix884 = self.g; __av_mem[crate::ix::U((__ix884) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix885 = self.g; __av_mem[crate::ix::U((__ix885) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix886 = (self.g).wrapping_add(1i32); let __v887 = self.rule_wd; __av_mem[crate::ix::U((__ix886) as usize)].set_int(__v887); }
                                                                    { let __ix888 = (self.g).wrapping_add(2i32); __av_mem[crate::ix::U((__ix888) as usize)].set_int(0i32); }
                                                                    { let __ix889 = (self.g).wrapping_add(3i32); __av_mem[crate::ix::U((__ix889) as usize)].set_int(0i32); }
                                                                    { let __v890 = self.g; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v890); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // §736
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    // §737
                                                    {
                                                        leader_box = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (__av_mem[crate::ix::U((leader_box) as usize)].hh().b0() == rule_node) {
                                                            {
                                                                self.rule_ht = __av_mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int();
                                                                self.rule_dp = __av_mem[crate::ix::U(((leader_box).wrapping_add(2i32)) as usize)].int();
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_wd = __av_mem[crate::ix::U(((leader_box).wrapping_add(1i32)) as usize)].int();
                                                        if ((leader_wd > 0i32) && (self.rule_wd > 0i32)) {
                                                            {
                                                                self.rule_wd = (self.rule_wd).wrapping_add(10i32);
                                                                if (self.cur_dir == right_to_left) {
                                                                    self.cur_h = (self.cur_h).wrapping_sub(10i32);
                                                                }
                                                                edge = (self.cur_h).wrapping_add(self.rule_wd);
                                                                lx = 0i32;
                                                                // §656
                                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == a_leaders) {
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.cur_h = (left_edge).wrapping_add((leader_wd).wrapping_mul(((self.cur_h).wrapping_sub(left_edge)).total_div(leader_wd)));
                                                                        if (self.cur_h < save_h) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_wd).total_div(leader_wd);
                                                                        self.lr = (self.rule_wd).total_mod(leader_wd);
                                                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                                                            self.cur_h = (self.cur_h).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr).total_div((self.lq).wrapping_add(1i32));
                                                                                self.cur_h = (self.cur_h).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §737
                                                                while ((self.cur_h).wrapping_add(leader_wd) <= edge) {
                                                                    // §738
                                                                    {
                                                                        self.cur_v = (base_line).wrapping_add(__av_mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = leader_box;
                                                                        if (self.cur_dir == right_to_left) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (__av_mem[crate::ix::U((leader_box) as usize)].hh().b0() == vlist_node) {
                                                                            self.pdf_vlist_out();
                                                                        } else {
                                                                            self.pdf_hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.cur_v = base_line;
                                                                        self.cur_h = ((save_h).wrapping_add(leader_wd)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §737
                                                                if (self.cur_dir == right_to_left) {
                                                                    self.cur_h = edge;
                                                                } else {
                                                                    self.cur_h = (edge).wrapping_sub(10i32);
                                                                }
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §736
                                                break 'l_L13_f;
                                            }
                                        }
                                        margin_kern_node | kern_node => {
                                            // §733
                                            self.cur_h = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        math_node => {
                                            // §1717
                                            {
                                                if (self.eTeX_mode == 1i32) {
                                                    // §1718
                                                    {
                                                        if (((__av_mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                            if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = self.LR_ptr;
                                                                    self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix891 = self.temp_ptr; let __v892 = self.avail; __av_mem[crate::ix::U((__ix891) as usize)].set_hh_rh(__v892); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (__av_mem[crate::ix::U((p) as usize)].hh().b1() > L_code) {
                                                                        self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    self.temp_ptr = self.get_avail();
                                                                    { let __ix893 = self.temp_ptr; let __v894 = ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); __av_mem[crate::ix::U((__ix893) as usize)].set_hh_lh(__v894); }
                                                                    { let __ix895 = self.temp_ptr; let __v896 = self.LR_ptr; __av_mem[crate::ix::U((__ix895) as usize)].set_hh_rh(__v896); }
                                                                    self.LR_ptr = self.temp_ptr;
                                                                }
                                                                if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir) {
                                                                    // §1723
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                                        self.rule_wd = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                                        self.free_node(p, medium_node_size);
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        p = self.new_edge(self.cur_dir, self.rule_wd);
                                                                        __av_mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                                                                        self.cur_h = ((self.cur_h).wrapping_sub(left_edge)).wrapping_add(self.rule_wd);
                                                                        { let __v897 = { let __a1 = self.new_edge((1i32).wrapping_sub(self.cur_dir), 0i32); let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, __a1, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v897); }
                                                                        { let __v898 = self.cur_h; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v898); }
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        self.cur_h = save_h;
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1718
                                                        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                    }
                                                }
                                                // §1717
                                                self.cur_h = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        ligature_node => {
                                            // §827
                                            {
                                                { let __v899 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((lig_trick) as usize)] = __v899; }
                                                { let __v900 = __av_mem[crate::ix::U((p) as usize)].hh().rh(); __av_mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v900); }
                                                p = lig_trick;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        edge_node => {
                                            // §1721
                                            {
                                                self.cur_h = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                left_edge = (self.cur_h).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                self.cur_dir = __av_mem[crate::ix::U((p) as usize)].hh().b1();
                                            }
                                        }
                                        _ => {
                                            // §733
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_ht == (1073741824i32).wrapping_neg()) {
                                    // §735
                                    self.rule_ht = __av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int();
                                }
                                if (self.rule_dp == (1073741824i32).wrapping_neg()) {
                                    self.rule_dp = __av_mem[crate::ix::U(((this_box).wrapping_add(2i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        self.cur_v = (base_line).wrapping_add(self.rule_dp);
                                        self.pdf_set_rule(self.cur_h, self.cur_v, self.rule_wd, self.rule_ht);
                                        self.cur_v = base_line;
                                    }
                                }
                            }
                            // §733
                            self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                        }
                        prev_p = p;
                        p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
                break 'l_reswitch_b;
            }
        }
        // §1716
        if (self.eTeX_mode == 1i32) {
            {
                // §1719
                {
                    while (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != before) {
                        {
                            if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() > L_code) {
                                self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                            }
                            {
                                self.temp_ptr = self.LR_ptr;
                                self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                {
                                    { let __ix901 = self.temp_ptr; let __v902 = self.avail; __av_mem[crate::ix::U((__ix901) as usize)].set_hh_rh(__v902); }
                                    self.avail = self.temp_ptr;
                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                }
                            }
                        }
                    }
                    {
                        self.temp_ptr = self.LR_ptr;
                        self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                        {
                            { let __ix903 = self.temp_ptr; let __v904 = self.avail; __av_mem[crate::ix::U((__ix903) as usize)].set_hh_rh(__v904); }
                            self.avail = self.temp_ptr;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                    }
                }
                // §1716
                if (__av_mem[crate::ix::U((this_box) as usize)].hh().b1() == dlist) {
                    self.cur_dir = right_to_left;
                }
            }
        }
        // §730
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `pdf_vlist_out` routine is similar to `pdf_hlist_out`, but a bit simpler.
    // §739
    pub fn pdf_vlist_out(&mut self) {
        let mut left_edge: scaled = 0; // §739
        let mut top_edge: scaled = 0; // §739
        let mut save_v: scaled = 0; // §739
        let mut this_box: halfword = 0; // §739
        let mut g_order: glue_ord = 0; // §739
        let mut g_sign: i32 = 0; // §739
        let mut p: halfword = 0; // §739
        let mut leader_box: halfword = 0; // §739
        let mut leader_ht: scaled = 0; // §739
        let mut lx: scaled = 0; // §739
        let mut outer_doing_leaders: bool = false; // §739
        let mut edge: scaled = 0; // §739
        let mut glue_temp: f64 = 0.0; // §739
        let mut cur_glue: f64 = 0.0; // §739
        let mut cur_g: scaled = 0; // §739
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = __av_mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        left_edge = self.cur_h;
        self.cur_v = (self.cur_v).wrapping_sub(__av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int());
        top_edge = self.cur_v;
        // §740
        if (((self.last_thread != null) && (self.pdf_thread_dp == (1073741824i32).wrapping_neg())) && (self.pdf_thread_level == self.cur_s)) {
            self.append_thread(this_box, left_edge, (top_edge).wrapping_add(__av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
        }
        // §739
        while (p != null) {
            // §741
            {
                'l_L15_f: {
                    if (p >= self.hi_mem_min) {
                        self.confusion(1148i32);
                    } else {
                        // §742
                        {
                            'l_L13_f: {
                                'l_L14_f: {
                                    self.dl_node(p);
                                    match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §743
                                            if (__av_mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null) {
                                                self.cur_v = ((self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            } else {
                                                {
                                                    self.cur_v = (self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    save_v = self.cur_v;
                                                    if (self.cur_dir == right_to_left) {
                                                        self.cur_h = (left_edge).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    } else {
                                                        self.cur_h = (left_edge).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    }
                                                    self.temp_ptr = p;
                                                    if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                                        self.pdf_vlist_out();
                                                    } else {
                                                        self.pdf_hlist_out();
                                                    }
                                                    self.cur_v = (save_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    self.cur_h = left_edge;
                                                }
                                            }
                                        }
                                        rule_node => {
                                            // §742
                                            {
                                                self.rule_ht = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1640
                                            match __av_mem[crate::ix::U((p) as usize)].hh().b1() {
                                                pdf_literal_node | pdf_lateliteral_node => {
                                                    self.pdf_out_literal(p);
                                                }
                                                pdf_colorstack_node => {
                                                    self.pdf_out_colorstack(p);
                                                }
                                                pdf_setmatrix_node => {
                                                    self.pdf_out_setmatrix(p);
                                                }
                                                pdf_save_node => {
                                                    self.pdf_out_save(p);
                                                }
                                                pdf_restore_node => {
                                                    self.pdf_out_restore(p);
                                                }
                                                pdf_refobj_node => {
                                                    {
                                                        self.pdf_append_list_arg = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                        self.pdf_obj_list = self.append_ptr(self.pdf_obj_list, self.pdf_append_list_arg);
                                                    }
                                                }
                                                pdf_refxform_node => {
                                                    // §1645
                                                    {
                                                        self.cur_v = (self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                        save_v = self.cur_v;
                                                        self.cur_h = left_edge;
                                                        self.out_form(p);
                                                        self.cur_v = (save_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        self.cur_h = left_edge;
                                                    }
                                                }
                                                pdf_refximage_node => {
                                                    // §1644
                                                    {
                                                        self.cur_v = ((self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        save_v = self.cur_v;
                                                        self.cur_h = left_edge;
                                                        self.out_image(p);
                                                        self.cur_v = save_v;
                                                        self.cur_h = left_edge;
                                                    }
                                                }
                                                pdf_annot_node => {
                                                    // §1640
                                                    self.do_annot(p, this_box, left_edge, (top_edge).wrapping_add(__av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                pdf_start_link_node => {
                                                    self.pdf_error(1875i32, 1989i32);
                                                }
                                                pdf_end_link_node => {
                                                    self.pdf_error(1875i32, 1990i32);
                                                }
                                                pdf_dest_node => {
                                                    self.do_dest(p, this_box, left_edge, (top_edge).wrapping_add(__av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                pdf_thread_node | pdf_start_thread_node => {
                                                    self.do_thread(p, this_box, left_edge, (top_edge).wrapping_add(__av_mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                pdf_end_thread_node => {
                                                    self.end_thread();
                                                }
                                                pdf_save_pos_node => {
                                                    // §1642
                                                    {
                                                        self.pdf_last_x_pos = self.cur_h;
                                                        if self.is_shipping_page {
                                                            self.pdf_last_y_pos = (self.cur_page_height).wrapping_sub(self.cur_v);
                                                        } else {
                                                            self.pdf_last_y_pos = ((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth)).wrapping_sub(self.cur_v);
                                                        }
                                                    }
                                                }
                                                special_node | latespecial_node => {
                                                    // §1640
                                                    self.pdf_special(p);
                                                }
                                                pdf_snap_ref_point_node => {
                                                    // §1643
                                                    {
                                                        self.pdf_snapx_refpos = self.cur_h;
                                                        self.pdf_snapy_refpos = self.cur_v;
                                                    }
                                                }
                                                pdf_snapy_comp_node => {
                                                    // §1640
                                                    self.do_snapy_comp(p, this_box);
                                                }
                                                pdf_snapy_node => {
                                                    self.do_snapy(p);
                                                }
                                                pdf_interword_space_on_node => {
                                                    self.gen_faked_interword_space = true;
                                                }
                                                pdf_interword_space_off_node => {
                                                    self.gen_faked_interword_space = false;
                                                }
                                                pdf_fake_space_node => {
                                                    self.pdf_insert_fake_space();
                                                }
                                                pdf_running_link_off_node => {
                                                    self.gen_running_link = false;
                                                }
                                                pdf_running_link_on_node => {
                                                    self.gen_running_link = true;
                                                }
                                                _ => {
                                                    self.out_what(p);
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §745
                                            {
                                                self.g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_ht = (__av_mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != normal) {
                                                    {
                                                        if (g_sign == stretching) {
                                                            {
                                                                if (__av_mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((__av_mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                        glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                                        if (glue_temp > 1000000000.0f64) {
                                                                            glue_temp = 1000000000.0f64;
                                                                        } else {
                                                                            if (glue_temp < (-1000000000.0f64)) {
                                                                                glue_temp = (-1000000000.0f64);
                                                                            }
                                                                        }
                                                                        cur_g = crate::system::pas_round(glue_temp);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            if (__av_mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((__av_mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                                    glue_temp = (__av_mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
                                                                    if (glue_temp > 1000000000.0f64) {
                                                                        glue_temp = 1000000000.0f64;
                                                                    } else {
                                                                        if (glue_temp < (-1000000000.0f64)) {
                                                                            glue_temp = (-1000000000.0f64);
                                                                        }
                                                                    }
                                                                    cur_g = crate::system::pas_round(glue_temp);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                self.rule_ht = (self.rule_ht).wrapping_add(cur_g);
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    // §746
                                                    {
                                                        leader_box = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (__av_mem[crate::ix::U((leader_box) as usize)].hh().b0() == rule_node) {
                                                            {
                                                                self.rule_wd = __av_mem[crate::ix::U(((leader_box).wrapping_add(1i32)) as usize)].int();
                                                                self.rule_dp = 0i32;
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_ht = (__av_mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((leader_box).wrapping_add(2i32)) as usize)].int());
                                                        if ((leader_ht > 0i32) && (self.rule_ht > 0i32)) {
                                                            {
                                                                self.rule_ht = (self.rule_ht).wrapping_add(10i32);
                                                                edge = (self.cur_v).wrapping_add(self.rule_ht);
                                                                lx = 0i32;
                                                                // §665
                                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == a_leaders) {
                                                                    {
                                                                        save_v = self.cur_v;
                                                                        self.cur_v = (top_edge).wrapping_add((leader_ht).wrapping_mul(((self.cur_v).wrapping_sub(top_edge)).total_div(leader_ht)));
                                                                        if (self.cur_v < save_v) {
                                                                            self.cur_v = (self.cur_v).wrapping_add(leader_ht);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_ht).total_div(leader_ht);
                                                                        self.lr = (self.rule_ht).total_mod(leader_ht);
                                                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                                                            self.cur_v = (self.cur_v).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr).total_div((self.lq).wrapping_add(1i32));
                                                                                self.cur_v = (self.cur_v).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §746
                                                                while ((self.cur_v).wrapping_add(leader_ht) <= edge) {
                                                                    // §747
                                                                    {
                                                                        if (self.cur_dir == right_to_left) {
                                                                            self.cur_h = (left_edge).wrapping_sub(__av_mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        } else {
                                                                            self.cur_h = (left_edge).wrapping_add(__av_mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        }
                                                                        self.cur_v = (self.cur_v).wrapping_add(__av_mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int());
                                                                        save_v = self.cur_v;
                                                                        self.temp_ptr = leader_box;
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (__av_mem[crate::ix::U((leader_box) as usize)].hh().b0() == vlist_node) {
                                                                            self.pdf_vlist_out();
                                                                        } else {
                                                                            self.pdf_hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.cur_h = left_edge;
                                                                        self.cur_v = (((save_v).wrapping_sub(__av_mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int())).wrapping_add(leader_ht)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §746
                                                                self.cur_v = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §745
                                                break 'l_L13_f;
                                            }
                                        }
                                        kern_node => {
                                            // §742
                                            self.cur_v = (self.cur_v).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        _ => {
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_wd == (1073741824i32).wrapping_neg()) {
                                    // §744
                                    self.rule_wd = __av_mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_dir == right_to_left) {
                                            self.cur_h = (self.cur_h).wrapping_sub(self.rule_wd);
                                        }
                                        self.pdf_set_rule(self.cur_h, self.cur_v, self.rule_wd, self.rule_ht);
                                        self.cur_h = left_edge;
                                    }
                                }
                                break 'l_L15_f;
                            }
                            // §742
                            self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                        }
                    }
                }
                // §741
                p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        // §739
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// `fix_pdfoutput` freezes `pdfoutput` when something has been written to
    /// the output.
    // §748
    pub fn fix_pdfoutput(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (!self.fixed_pdfoutput_set) {
            {
                self.fixed_pdfoutput = __av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int();
                self.fixed_pdfoutput_set = true;
            }
        } else {
            if (self.fixed_pdfoutput != __av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int()) {
                self.pdf_error(1028i32, 1149i32);
            }
        }
        if self.fixed_pdfoutput_set {
            self.fix_pdf_draftmode();
        }
    }

    /// `fix_pdf_draftmode` freezes `pdfdraftmode` when something has been written to
    /// the output and also switches some things off when draftmode is on.
    // §749
    pub fn fix_pdf_draftmode(&mut self) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        if (!self.fixed_pdf_draftmode_set) {
            {
                self.fixed_pdf_draftmode = __av_eqtb[crate::ix::U(((29368i32) - 1) as usize)].int();
                self.fixed_pdf_draftmode_set = true;
            }
        } else {
            if (self.fixed_pdf_draftmode != __av_eqtb[crate::ix::U(((29368i32) - 1) as usize)].int()) {
                self.pdf_error(1028i32, 1150i32);
            }
        }
        if (self.fixed_pdf_draftmode_set && (self.fixed_pdf_draftmode > 0i32)) {
            {
                self.fixed_pdf_draftmode_set = true;
                __av_eqtb[crate::ix::U(((29343i32) - 1) as usize)].set_int(0i32);
                self.fixed_pdf_objcompresslevel = 0i32;
            }
        }
    }

    /// `substr_of_str` is used in `pdf_ship_out` and `pdf_print_info`.
    // §750
    pub fn substr_of_str(&mut self, mut s: str_number, mut t: str_number) -> bool {
        let mut substr_of_str: bool = false;
        let mut j: pool_pointer = 0; // §750
        let mut k: pool_pointer = 0; // §750
        let mut kk: pool_pointer = 0; // §750
        k = self.str_start[crate::ix::U((t) as usize)];
        while (k < (self.str_start[crate::ix::U(((t).wrapping_add(1i32)) as usize)]).wrapping_sub((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]))) {
            {
                'l_continue_f: {
                    j = self.str_start[crate::ix::U((s) as usize)];
                    kk = k;
                    while (j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
                        {
                            if (self.str_pool[crate::ix::U((j) as usize)] != self.str_pool[crate::ix::U((kk) as usize)]) {
                                break 'l_continue_f;
                            }
                            j = (j).wrapping_add(1i32);
                            kk = (kk).wrapping_add(1i32);
                        }
                    }
                    substr_of_str = true;
                    return substr_of_str;
                }
                k = (k).wrapping_add(1i32);
            }
        }
        substr_of_str = false;
        substr_of_str
    }

    /// `pdf_ship_out` is used instead of `ship_out` to shipout a box to PDF
    /// output. If `shipping_page` is not set then the output will be a Form object,
    /// otherwise it will be a Page object.
    // §751
    pub fn pdf_ship_out(&mut self, mut p: halfword, mut shipping_page: bool) {
        let mut i: i32 = 0; // §751
        let mut j: i32 = 0; // §751
        let mut k: i32 = 0; // §751
        let mut s: pool_pointer = 0; // §751
        let mut mediabox_given: bool = false; // §751
        let mut save_font_list: halfword = 0; // §751
        let mut save_obj_list: halfword = 0; // §751
        let mut save_ximage_list: halfword = 0; // §751
        let mut save_xform_list: halfword = 0; // §751
        let mut save_image_procset: i32 = 0; // §751
        let mut save_text_procset: i32 = 0; // §751
        let mut pdf_last_resources: i32 = 0; // §751
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_done_f: {
            if (__av_eqtb[crate::ix::U(((29311i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_nl(345i32);
                    self.print_ln();
                    self.print(1003i32);
                }
            }
            if (!self.init_pdf_output) {
                {
                    // §793
                    self.check_pdfversion();
                    self.prepare_mag();
                    self.fixed_decimal_digits = self.fix_int(__av_eqtb[crate::ix::U(((29344i32) - 1) as usize)].int(), 0i32, 4i32);
                    self.min_bp_val = self.divide_scaled(self.one_hundred_bp, self.ten_pow[crate::ix::U(((self.fixed_decimal_digits).wrapping_add(2i32)) as usize)], 0i32);
                    if (__av_eqtb[crate::ix::U(((29347i32) - 1) as usize)].int() == 0i32) {
                        { let __v905 = self.pk_dpi; __av_eqtb[crate::ix::U(((29347i32) - 1) as usize)].set_int(__v905); }
                    }
                    self.fixed_pk_resolution = self.fix_int(__av_eqtb[crate::ix::U(((29347i32) - 1) as usize)].int(), 72i32, 8000i32);
                    self.pk_scale_factor = self.divide_scaled(72i32, self.fixed_pk_resolution, (5i32).wrapping_add(self.fixed_decimal_digits));
                    if (__av_eqtb[crate::ix::U(((pdf_pk_mode_loc) - 1) as usize)].hh().rh() != null) {
                        {
                            { let __a906_0 = self.fixed_pk_resolution; let __a906_1 = self.tokens_to_string(__av_eqtb[crate::ix::U(((pdf_pk_mode_loc) - 1) as usize)].hh().rh()); self.pk_init(__a906_0, __a906_1) };
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                            }
                        }
                    } else {
                        self.pk_init(self.fixed_pk_resolution, 0i32);
                    }
                    self.set_job_id(__av_eqtb[crate::ix::U(((29300i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29299i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29298i32) - 1) as usize)].int(), __av_eqtb[crate::ix::U(((29297i32) - 1) as usize)].int());
                    if ((__av_eqtb[crate::ix::U(((29348i32) - 1) as usize)].int() > 0i32) && (self.pdf_resname_prefix == 0i32)) {
                        self.pdf_resname_prefix = self.get_resname_prefix();
                    }
                    // §751
                    self.init_pdf_output = true;
                }
            }
            self.is_shipping_page = shipping_page;
            if shipping_page {
                {
                    if (self.term_offset > (self.max_print_line).wrapping_sub(9i32)) {
                        self.print_ln();
                    } else {
                        if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                            self.print_char(32i32);
                        }
                    }
                    self.print_char(91i32);
                    j = 9i32;
                    while ((__av_eqtb[crate::ix::U((((count_base).wrapping_add(j)) - 1) as usize)].int() == 0i32) && (j > 0i32)) {
                        j = (j).wrapping_sub(1i32);
                    }
                    {
                        let __for_end_5 = j;
                        k = 0i32;
                        while k <= __for_end_5 {
                            {
                                self.print_int(((__av_eqtb[crate::ix::U((((count_base).wrapping_add(k)) - 1) as usize)].int()) as i64));
                                if (k < j) {
                                    self.print_char(46i32);
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    crate::system::break_out(&mut self.term_out);
                }
            }
            if (__av_eqtb[crate::ix::U(((29311i32) - 1) as usize)].int() > 0i32) {
                {
                    if shipping_page {
                        self.print_char(93i32);
                    }
                    self.begin_diagnostic();
                    self.show_box(p);
                    self.end_diagnostic(true);
                }
            }
            // §670
            if ((((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > max_dimen) || (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > max_dimen)) || (((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(__av_eqtb[crate::ix::U(((29922i32) - 1) as usize)].int()) > max_dimen)) || ((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(__av_eqtb[crate::ix::U(((29921i32) - 1) as usize)].int()) > max_dimen)) {
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
                        self.print(1007i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 1008i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 1009i32;
                    }
                    self.error();
                    if (__av_eqtb[crate::ix::U(((29311i32) - 1) as usize)].int() <= 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(1010i32);
                            self.show_box(p);
                            self.end_diagnostic(true);
                        }
                    }
                    break 'l_done_f;
                }
            }
            if (((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(__av_eqtb[crate::ix::U(((29922i32) - 1) as usize)].int()) > self.max_v) {
                self.max_v = ((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(__av_eqtb[crate::ix::U(((29922i32) - 1) as usize)].int());
            }
            if ((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(__av_eqtb[crate::ix::U(((29921i32) - 1) as usize)].int()) > self.max_h) {
                self.max_h = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(__av_eqtb[crate::ix::U(((29921i32) - 1) as usize)].int());
            }
            // §753
            self.fix_pdfoutput();
            self.temp_ptr = p;
            self.prepare_mag();
            pdf_last_resources = self.pdf_new_objnum();
            self.pdf_page_group_val = 0i32;
            // §754
            self.pdf_font_list = null;
            self.pdf_obj_list = null;
            self.pdf_xform_list = null;
            self.pdf_ximage_list = null;
            self.pdf_text_procset = false;
            self.pdf_image_procset = 0i32;
            // §753
            if (!shipping_page) {
                {
                    self.pdf_xform_width = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                    self.pdf_xform_height = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                    self.pdf_xform_depth = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                    self.pdf_begin_dict(self.pdf_cur_form, 0i32);
                    self.pdf_last_stream = self.pdf_cur_form;
                    self.cur_v = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                    self.cur_h = 0i32;
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = (self.pdf_xform_height).wrapping_add(self.pdf_xform_depth);
                }
            } else {
                {
                    // §756
                    self.cur_h_offset = (__av_eqtb[crate::ix::U(((29924i32) - 1) as usize)].int()).wrapping_add(__av_eqtb[crate::ix::U(((29921i32) - 1) as usize)].int());
                    self.cur_v_offset = (__av_eqtb[crate::ix::U(((29925i32) - 1) as usize)].int()).wrapping_add(__av_eqtb[crate::ix::U(((29922i32) - 1) as usize)].int());
                    if (__av_eqtb[crate::ix::U(((29926i32) - 1) as usize)].int() != 0i32) {
                        self.cur_page_width = __av_eqtb[crate::ix::U(((29926i32) - 1) as usize)].int();
                    } else {
                        self.cur_page_width = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add((2i32).wrapping_mul(self.cur_h_offset));
                    }
                    if (__av_eqtb[crate::ix::U(((29927i32) - 1) as usize)].int() != 0i32) {
                        self.cur_page_height = __av_eqtb[crate::ix::U(((29927i32) - 1) as usize)].int();
                    } else {
                        self.cur_page_height = ((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add((2i32).wrapping_mul(self.cur_v_offset));
                    }
                    // §753
                    self.pdf_last_page = self.get_obj(obj_type_page, (self.total_pages).wrapping_add(1i32), ((0i32) != 0));
                    self.obj_tab[crate::ix::U((self.pdf_last_page) as usize)].int4 = 1i32;
                    self.pdf_new_dict(obj_type_others, 0i32, 0i32);
                    self.pdf_last_stream = self.obj_ptr;
                    self.cur_h = self.cur_h_offset;
                    self.cur_v = (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.cur_v_offset);
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = self.cur_page_height;
                    // §755
                    self.pdf_annot_list = null;
                    self.pdf_link_list = null;
                    self.pdf_dest_list = null;
                    self.pdf_bead_list = null;
                    self.last_thread = null;
                }
            }
            // §753
            if (!shipping_page) {
                {
                    // §757
                    {
                        self.pdf_print(1151i32);
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
                        self.pdf_print(1152i32);
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
                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)] != null) {
                        {
                            self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)]);
                            {
                                self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)]);
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)] = null;
                            }
                        }
                    }
                    self.pdf_print(1153i32);
                    self.pdf_print(1066i32);
                    self.pdf_print_bp(self.pdf_xform_width);
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
                    self.pdf_print_bp((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth));
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
                    {
                        self.pdf_print(1154i32);
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
                        self.pdf_print(1155i32);
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
                    self.pdf_indirect_ln(1156i32, pdf_last_resources);
                }
            }
            // §758
            self.pdf_begin_stream();
            if shipping_page {
                {
                    // §759
                    self.prepare_mag();
                    if (__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int() != 1000i32) {
                        {
                            self.pdf_print_real(__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int(), 3i32);
                            self.pdf_print(1157i32);
                            self.pdf_print_real(__av_eqtb[crate::ix::U(((29294i32) - 1) as usize)].int(), 3i32);
                            {
                                self.pdf_print(1158i32);
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
            }
            // §758
            self.pdfshipoutbegin(shipping_page);
            if shipping_page {
                self.pdf_out_colorstack_startpage();
            }
            // §752
            if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                self.pdf_vlist_out();
            } else {
                self.pdf_hlist_out();
            }
            if shipping_page {
                self.total_pages = (self.total_pages).wrapping_add(1i32);
            }
            self.cur_s = (1i32).wrapping_neg();
            // §761
            self.pdf_end_text();
            self.pdfshipoutend(shipping_page);
            self.pdf_end_stream();
            // §760
            if shipping_page {
                {
                    // §770
                    self.pdf_begin_dict(self.pdf_last_page, 1i32);
                    {
                        self.pdf_print(1170i32);
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
                    self.pdf_indirect_ln(1171i32, self.pdf_last_stream);
                    self.pdf_indirect_ln(1156i32, pdf_last_resources);
                    mediabox_given = false;
                    if (__av_eqtb[crate::ix::U(((pdf_page_attr_loc) - 1) as usize)].hh().rh() != null) {
                        {
                            s = self.tokens_to_string(__av_eqtb[crate::ix::U(((pdf_page_attr_loc) - 1) as usize)].hh().rh());
                            mediabox_given = self.substr_of_str(1172i32, s);
                            self.flush_str(s);
                        }
                    }
                    if (!mediabox_given) {
                        {
                            self.pdf_print(1173i32);
                            self.pdf_print_mag_bp(self.cur_page_width);
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
                            self.pdf_print_mag_bp(self.cur_page_height);
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
                    }
                    if (__av_eqtb[crate::ix::U(((pdf_page_attr_loc) - 1) as usize)].hh().rh() != null) {
                        self.pdf_print_toks_ln(__av_eqtb[crate::ix::U(((pdf_page_attr_loc) - 1) as usize)].hh().rh());
                    }
                    // §771
                    if ((self.total_pages % pages_tree_kids_max) == 1i32) {
                        {
                            self.pdf_create_obj(obj_type_pages, pages_tree_kids_max);
                            self.pdf_last_pages = self.obj_ptr;
                        }
                    }
                    self.pdf_indirect_ln(1175i32, self.pdf_last_pages);
                    // §770
                    if (self.pdf_page_group_val > 0i32) {
                        {
                            self.pdf_print(1174i32);
                            self.pdf_print_int(((self.pdf_page_group_val) as i64));
                            {
                                self.pdf_print(1087i32);
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
                    // §772
                    if ((self.pdf_annot_list != null) || (self.pdf_link_list != null)) {
                        {
                            self.pdf_print(1176i32);
                            k = self.pdf_annot_list;
                            while (k != null) {
                                {
                                    self.pdf_print_int(((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1160i32);
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                            k = self.pdf_link_list;
                            while (k != null) {
                                {
                                    self.pdf_print_int(((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1160i32);
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
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
                    }
                    if (self.pdf_bead_list != null) {
                        {
                            k = self.pdf_bead_list;
                            self.pdf_print(1177i32);
                            while (k != null) {
                                {
                                    self.pdf_print_int(((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1160i32);
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
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
                    }
                    // §770
                    self.pdf_end_dict();
                }
            }
            // §774
            if (self.pdf_obj_list != null) {
                {
                    k = self.pdf_obj_list;
                    while (k != null) {
                        {
                            if (!(self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                self.pdf_write_obj(__av_mem[crate::ix::U((k) as usize)].hh().lh());
                            }
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §780
            if (self.pdf_ximage_list != null) {
                {
                    k = self.pdf_ximage_list;
                    while (k != null) {
                        {
                            if (!(self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                self.pdf_write_image(__av_mem[crate::ix::U((k) as usize)].hh().lh());
                            }
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §776
            if (self.pdf_xform_list != null) {
                {
                    k = self.pdf_xform_list;
                    while (k != null) {
                        {
                            if (!(self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                {
                                    self.saved_pdf_cur_form = self.pdf_cur_form;
                                    self.pdf_cur_form = __av_mem[crate::ix::U((k) as usize)].hh().lh();
                                    // §777
                                    save_font_list = self.pdf_font_list;
                                    save_obj_list = self.pdf_obj_list;
                                    save_xform_list = self.pdf_xform_list;
                                    save_ximage_list = self.pdf_ximage_list;
                                    save_text_procset = ((self.pdf_text_procset) as i32);
                                    save_image_procset = self.pdf_image_procset;
                                    // §754
                                    self.pdf_font_list = null;
                                    self.pdf_obj_list = null;
                                    self.pdf_xform_list = null;
                                    self.pdf_ximage_list = null;
                                    self.pdf_text_procset = false;
                                    self.pdf_image_procset = 0i32;
                                    // §776
                                    self.pdf_ship_out(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(3i32)) as usize)], false);
                                    self.pdf_cur_form = self.saved_pdf_cur_form;
                                    // §778
                                    self.pdf_font_list = save_font_list;
                                    self.pdf_obj_list = save_obj_list;
                                    self.pdf_xform_list = save_xform_list;
                                    self.pdf_ximage_list = save_ximage_list;
                                    self.pdf_text_procset = ((save_text_procset) != 0);
                                    self.pdf_image_procset = save_image_procset;
                                }
                            }
                            // §776
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §760
            if shipping_page {
                {
                    // §781
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = self.cur_page_height;
                    // §782
                    if (self.pdf_annot_list != null) {
                        {
                            k = self.pdf_annot_list;
                            while (k != null) {
                                {
                                    i = self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    self.pdf_begin_dict(__av_mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                    {
                                        self.pdf_print(1183i32);
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
                                    self.pdf_print_toks_ln(__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh());
                                    self.pdf_rectangle(__av_mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(3i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(4i32)) as usize)].int());
                                    self.pdf_end_dict();
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §783
                    if (self.pdf_link_list != null) {
                        {
                            k = self.pdf_link_list;
                            while (k != null) {
                                {
                                    i = self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    self.pdf_begin_dict(__av_mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                    {
                                        self.pdf_print(1183i32);
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
                                    if (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() != pdf_action_user) {
                                        {
                                            self.pdf_print(1184i32);
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
                                    if (__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh() != null) {
                                        self.pdf_print_toks_ln(__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh());
                                    }
                                    self.pdf_rectangle(__av_mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(3i32)) as usize)].int(), __av_mem[crate::ix::U(((i).wrapping_add(4i32)) as usize)].int());
                                    if (__av_mem[crate::ix::U((__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() != pdf_action_user) {
                                        self.pdf_print(1185i32);
                                    }
                                    self.write_action(__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh());
                                    self.pdf_end_dict();
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                            // §784
                            k = self.pdf_link_list;
                            while (k != null) {
                                {
                                    i = self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    if (__av_mem[crate::ix::U((i) as usize)].hh().lh() == max_halfword) {
                                        self.flush_whatsit_node(i, pdf_start_link_node);
                                    }
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §785
                    if (self.pdf_dest_list != null) {
                        {
                            k = self.pdf_dest_list;
                            while (k != null) {
                                {
                                    if (self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64)) {
                                        self.pdf_error(1179i32, 1186i32);
                                    } else {
                                        {
                                            i = self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                            if ((__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) && (__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == null)) {
                                                {
                                                    self.pdf_begin_dict(__av_mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                                    self.pdf_print(1187i32);
                                                }
                                            } else {
                                                self.pdf_begin_obj(__av_mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
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
                                                    self.pdf_buf_set(self.pdf_ptr, 91i32);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            if (__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == null) {
                                                self.pdf_print_int(((self.pdf_last_page) as i64));
                                            } else {
                                                self.pdf_print_int(((__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh()) as i64));
                                            }
                                            self.pdf_print(1160i32);
                                            match __av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b0() {
                                                pdf_dest_xyz => {
                                                    {
                                                        self.pdf_print(1188i32);
                                                        self.pdf_print_mag_bp((__av_mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(__av_mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
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
                                                        if (__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() == null) {
                                                            self.pdf_print(1189i32);
                                                        } else {
                                                            {
                                                                self.pdf_print_int((((__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() / 1000i32)) as i64));
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
                                                                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                                self.pdf_print_int((((__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() % 1000i32)) as i64));
                                                            }
                                                        }
                                                    }
                                                }
                                                pdf_dest_fit => {
                                                    self.pdf_print(1190i32);
                                                }
                                                pdf_dest_fith => {
                                                    {
                                                        self.pdf_print(1191i32);
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(__av_mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
                                                    }
                                                }
                                                pdf_dest_fitv => {
                                                    {
                                                        self.pdf_print(1192i32);
                                                        self.pdf_print_mag_bp((__av_mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
                                                    }
                                                }
                                                pdf_dest_fitb => {
                                                    self.pdf_print(1193i32);
                                                }
                                                pdf_dest_fitbh => {
                                                    {
                                                        self.pdf_print(1194i32);
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(__av_mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
                                                    }
                                                }
                                                pdf_dest_fitbv => {
                                                    {
                                                        self.pdf_print(1195i32);
                                                        self.pdf_print_mag_bp((__av_mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
                                                    }
                                                }
                                                pdf_dest_fitr => {
                                                    {
                                                        self.pdf_print(1196i32);
                                                        self.pdf_print_rect_spec(i);
                                                    }
                                                }
                                                _ => {
                                                    self.pdf_error(1179i32, 1197i32);
                                                }
                                            }
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
                                            if ((__av_mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) && (__av_mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == null)) {
                                                self.pdf_end_dict();
                                            } else {
                                                self.pdf_end_obj();
                                            }
                                        }
                                    }
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §787
                    if (self.pdf_bead_list != null) {
                        {
                            k = self.pdf_bead_list;
                            while (k != null) {
                                {
                                    self.pdf_new_obj(obj_type_others, 0i32, 1i32);
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
                                    i = self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4) as usize)];
                                    self.pdf_print_rect_spec(i);
                                    if (__av_mem[crate::ix::U((i) as usize)].hh().lh() == max_halfword) {
                                        self.flush_whatsit_node(i, pdf_start_thread_node);
                                    }
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
                                    { let __ix907 = self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4; let __v908 = self.obj_ptr; self.pdf_mem[crate::ix::U((__ix907) as usize)] = __v908; }
                                    self.pdf_end_obj();
                                    k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                }
            }
            // §763
            self.pdf_begin_dict(pdf_last_resources, 1i32);
            // §764
            if shipping_page {
                {
                    if (__av_eqtb[crate::ix::U(((pdf_page_resources_loc) - 1) as usize)].hh().rh() != null) {
                        self.pdf_print_toks_ln(__av_eqtb[crate::ix::U(((pdf_page_resources_loc) - 1) as usize)].hh().rh());
                    }
                }
            } else {
                {
                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)] != null) {
                        {
                            self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)]);
                            {
                                self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)]);
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)] = null;
                            }
                        }
                    }
                }
            }
            // §767
            if (self.pdf_font_list != null) {
                {
                    self.pdf_print(1159i32);
                    k = self.pdf_font_list;
                    while (k != null) {
                        {
                            self.pdf_print(1055i32);
                            {
                                if (self.pdf_font_num[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)] < 0i32) {
                                    self.ff = (self.pdf_font_num[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)]).wrapping_neg();
                                } else {
                                    self.ff = __av_mem[crate::ix::U((k) as usize)].hh().lh();
                                }
                            }
                            self.pdf_print_int(((self.ff) as i64));
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
                            self.pdf_print_int(((self.pdf_font_num[crate::ix::U((self.ff) as usize)]) as i64));
                            self.pdf_print(1160i32);
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
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
                    self.pdf_text_procset = true;
                }
            }
            // §768
            if ((self.pdf_xform_list != null) || (self.pdf_ximage_list != null)) {
                {
                    self.pdf_print(1161i32);
                    k = self.pdf_xform_list;
                    while (k != null) {
                        {
                            self.pdf_print(1162i32);
                            self.pdf_print_int(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int0) as i64));
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
                            self.pdf_print_int(((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                            self.pdf_print(1160i32);
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                    k = self.pdf_ximage_list;
                    while (k != null) {
                        {
                            self.pdf_print(1163i32);
                            self.pdf_print_int(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int0) as i64));
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
                            self.pdf_print_int(((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                            self.pdf_print(1160i32);
                            self.update_image_procset(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((__av_mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4).wrapping_add(4i32)) as usize)]);
                            k = __av_mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
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
            // §769
            if ((__av_eqtb[crate::ix::U(((29377i32) - 1) as usize)].int() < 0i32) || ((__av_eqtb[crate::ix::U(((29377i32) - 1) as usize)].int() == 0i32) && (__av_eqtb[crate::ix::U(((29351i32) - 1) as usize)].int() < 2i32))) {
                {
                    self.pdf_print(1164i32);
                    if self.pdf_text_procset {
                        self.pdf_print(1165i32);
                    }
                    if self.check_image_b(self.pdf_image_procset) {
                        self.pdf_print(1166i32);
                    }
                    if self.check_image_c(self.pdf_image_procset) {
                        self.pdf_print(1167i32);
                    }
                    if self.check_image_i(self.pdf_image_procset) {
                        self.pdf_print(1168i32);
                    }
                    {
                        self.pdf_print(1169i32);
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
            // §763
            self.pdf_end_dict();
            // §765
            self.flush_list(self.pdf_font_list);
            self.flush_list(self.pdf_obj_list);
            self.flush_list(self.pdf_xform_list);
            self.flush_list(self.pdf_ximage_list);
            // §760
            if shipping_page {
                {
                    // §766
                    self.flush_list(self.pdf_annot_list);
                    self.flush_list(self.pdf_link_list);
                    self.flush_list(self.pdf_dest_list);
                    self.flush_list(self.pdf_bead_list);
                }
            }
        }
        // §752
        // §751
        if (self.eTeX_mode == 1i32) {
            // §1731
            {
                if (self.LR_problems > 0i32) {
                    {
                        // §1714
                        {
                            self.print_ln();
                            self.print_nl(2053i32);
                            self.print_int((((self.LR_problems / 10000i32)) as i64));
                            self.print(2054i32);
                            self.print_int((((self.LR_problems % 10000i32)) as i64));
                            self.print(2055i32);
                            self.LR_problems = 0i32;
                        }
                        // §1731
                        self.print_char(41i32);
                        self.print_ln();
                    }
                }
                if ((self.LR_ptr != null) || (self.cur_dir != left_to_right)) {
                    self.confusion(2057i32);
                }
            }
        }
        // §751
        if ((__av_eqtb[crate::ix::U(((29311i32) - 1) as usize)].int() <= 0i32) && shipping_page) {
            self.print_char(93i32);
        }
        self.dead_cycles = 0i32;
        crate::system::break_out(&mut self.term_out);
        // §668
        if (__av_eqtb[crate::ix::U(((29308i32) - 1) as usize)].int() > 1i32) {
            {
                self.print_nl(1004i32);
                self.print_int(((self.var_used) as i64));
                self.print_char(38i32);
                self.print_int(((self.dyn_used) as i64));
                self.print_char(59i32);
            }
        }
        self.flush_node_list(p);
        if (__av_eqtb[crate::ix::U(((29308i32) - 1) as usize)].int() > 1i32) {
            {
                self.print(1005i32);
                self.print_int(((self.var_used) as i64));
                self.print_char(38i32);
                self.print_int(((self.dyn_used) as i64));
                self.print(1006i32);
                self.print_int(((((self.hi_mem_min).wrapping_sub(self.lo_mem_max)).wrapping_sub(1i32)) as i64));
                self.print_ln();
            }
        }
    }

    /// Now we are ready to declare our new procedure `ship_out`.  It will call
    /// `pdf_ship_out` if the integer parameter `pdf_output` is positive; otherwise it
    /// will call `dvi_ship_out`, which is the \TeX\ original `ship_out`.
    // §792
    pub fn ship_out(&mut self, mut p: halfword) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.fix_pdfoutput();
        if (__av_eqtb[crate::ix::U(((29342i32) - 1) as usize)].int() > 0i32) {
            self.pdf_ship_out(p, true);
        } else {
            self.dvi_ship_out(p);
        }
        if (self.ckpt_on_shipout != 0i32) {
            self.ckpt_request = self.ckpt_on_shipout;
        }
    }

    /// Finishing the PDF output file.
    /// The following procedures sort the table of destination names.
    // §794
    pub fn str_less_str(&mut self, mut s1: str_number, mut s2: str_number) -> bool {
        let mut str_less_str: bool = false;
        let mut j1: pool_pointer = 0; // §794
        let mut j2: pool_pointer = 0; // §794
        let mut e1: pool_pointer = 0; // §794
        let mut e2: pool_pointer = 0; // §794
        let mut c1: packed_ASCII_code = 0; // §794
        let mut c2: packed_ASCII_code = 0; // §794
        'l_exit_f: {
            j1 = self.str_start[crate::ix::U((s1) as usize)];
            j2 = self.str_start[crate::ix::U((s2) as usize)];
            e1 = (j1).wrapping_add((self.str_start[crate::ix::U(((s1).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s1) as usize)]));
            e2 = (j2).wrapping_add((self.str_start[crate::ix::U(((s2).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s2) as usize)]));
            while ((j1 < e1) && (j2 < e2)) {
                {
                    c1 = self.str_pool[crate::ix::U((j1) as usize)];
                    j1 = (j1).wrapping_add(1i32);
                    if ((c1 == 92i32) && (j1 < e1)) {
                        {
                            c1 = self.str_pool[crate::ix::U((j1) as usize)];
                            j1 = (j1).wrapping_add(1i32);
                            if ((c1 >= 48i32) && (c1 <= 55i32)) {
                                {
                                    c1 = (c1).wrapping_sub(48i32);
                                    if (((j1 < e1) && (self.str_pool[crate::ix::U((j1) as usize)] >= 48i32)) && (self.str_pool[crate::ix::U((j1) as usize)] <= 55i32)) {
                                        {
                                            c1 = (((8i32).wrapping_mul(c1)).wrapping_add(self.str_pool[crate::ix::U((j1) as usize)])).wrapping_sub(48i32);
                                            j1 = (j1).wrapping_add(1i32);
                                            if ((((j1 < e1) && (self.str_pool[crate::ix::U((j1) as usize)] >= 48i32)) && (self.str_pool[crate::ix::U((j1) as usize)] <= 55i32)) && (c1 < 32i32)) {
                                                {
                                                    c1 = (((8i32).wrapping_mul(c1)).wrapping_add(self.str_pool[crate::ix::U((j1) as usize)])).wrapping_sub(48i32);
                                                    j1 = (j1).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    match c1 {
                                        98 => {
                                            c1 = 8i32;
                                        }
                                        102 => {
                                            c1 = 12i32;
                                        }
                                        110 => {
                                            c1 = 10i32;
                                        }
                                        114 => {
                                            c1 = 13i32;
                                        }
                                        116 => {
                                            c1 = 9i32;
                                        }
                                        _ => {
                                        }
                                    }
                                }
                            }
                        }
                    }
                    c2 = self.str_pool[crate::ix::U((j2) as usize)];
                    j2 = (j2).wrapping_add(1i32);
                    if ((c2 == 92i32) && (j2 < e2)) {
                        {
                            c2 = self.str_pool[crate::ix::U((j2) as usize)];
                            j2 = (j2).wrapping_add(1i32);
                            if ((c2 >= 48i32) && (c2 <= 55i32)) {
                                {
                                    c2 = (c2).wrapping_sub(48i32);
                                    if (((j2 < e2) && (self.str_pool[crate::ix::U((j2) as usize)] >= 48i32)) && (self.str_pool[crate::ix::U((j2) as usize)] <= 55i32)) {
                                        {
                                            c2 = (((8i32).wrapping_mul(c2)).wrapping_add(self.str_pool[crate::ix::U((j2) as usize)])).wrapping_sub(48i32);
                                            j2 = (j2).wrapping_add(1i32);
                                            if ((((j2 < e2) && (self.str_pool[crate::ix::U((j2) as usize)] >= 48i32)) && (self.str_pool[crate::ix::U((j2) as usize)] <= 55i32)) && (c2 < 32i32)) {
                                                {
                                                    c2 = (((8i32).wrapping_mul(c2)).wrapping_add(self.str_pool[crate::ix::U((j2) as usize)])).wrapping_sub(48i32);
                                                    j2 = (j2).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    match c2 {
                                        98 => {
                                            c2 = 8i32;
                                        }
                                        102 => {
                                            c2 = 12i32;
                                        }
                                        110 => {
                                            c2 = 10i32;
                                        }
                                        114 => {
                                            c2 = 13i32;
                                        }
                                        116 => {
                                            c2 = 9i32;
                                        }
                                        _ => {
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if (c1 < c2) {
                        {
                            str_less_str = true;
                            break 'l_exit_f;
                        }
                    } else {
                        if (c1 > c2) {
                            {
                                str_less_str = false;
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            if ((j1 >= e1) && (j2 < e2)) {
                str_less_str = true;
            } else {
                str_less_str = false;
            }
        }
        str_less_str
    }

    /// Finishing the PDF output file.
    /// The following procedures sort the table of destination names.
    // §794
    pub fn sort_dest_names(&mut self, mut l: i32, mut r: i32) {
        let mut i: i32 = 0; // §794
        let mut j: i32 = 0; // §794
        let mut s: str_number = 0; // §794
        let mut e: dest_name_entry = dest_name_entry::default(); // §794
        i = l;
        j = r;
        s = self.dest_names[crate::ix::U((((l).wrapping_add(r) / 2i32)) as usize)].objname;
        loop {
            while self.str_less_str(self.dest_names[crate::ix::U((i) as usize)].objname, s) {
                i = (i).wrapping_add(1i32);
            }
            while self.str_less_str(s, self.dest_names[crate::ix::U((j) as usize)].objname) {
                j = (j).wrapping_sub(1i32);
            }
            if (i <= j) {
                {
                    e = self.dest_names[crate::ix::U((i) as usize)];
                    { let __v909 = self.dest_names[crate::ix::U((j) as usize)]; self.dest_names[crate::ix::U((i) as usize)] = __v909; }
                    self.dest_names[crate::ix::U((j) as usize)] = e;
                    i = (i).wrapping_add(1i32);
                    j = (j).wrapping_sub(1i32);
                }
            }
            if (i > j) { break; }
        }
        if (l < j) {
            self.sort_dest_names(l, j);
        }
        if (i < r) {
            self.sort_dest_names(i, r);
        }
    }

    /// Destinations that have been referenced but don't exists have
    /// `obj_dest_ptr=null`. Leaving them undefined might cause troubles for
    /// PDF browsers, so we need to fix them.
    // §796
    pub fn pdf_fix_dest(&mut self, mut k: i32) {
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != null) {
            return;
        }
        self.pdf_warning(1206i32, 345i32, true, false);
        if (self.obj_tab[crate::ix::U((k) as usize)].int0 < 0i32) {
            {
                self.print(1207i32);
                self.print((self.obj_tab[crate::ix::U((k) as usize)].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1208i32);
                self.print_int(((self.obj_tab[crate::ix::U((k) as usize)].int0) as i64));
            }
        }
        self.print(1209i32);
        self.print_ln();
        self.print_ln();
        self.pdf_begin_obj(k, 1i32);
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
        self.pdf_print_int(((self.head_tab[crate::ix::U(((obj_type_page) - 1) as usize)]) as i64));
        {
            self.pdf_print(1210i32);
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
    }

    /// The same for structure destinations, except that there is no sensible default
    /// object to point to.
    // §798
    pub fn pdf_fix_struct_dest(&mut self, mut k: i32) {
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != null) {
            return;
        }
        self.pdf_warning(1211i32, 345i32, false, false);
        if (self.obj_tab[crate::ix::U((k) as usize)].int0 < 0i32) {
            {
                self.print(1207i32);
                self.print((self.obj_tab[crate::ix::U((k) as usize)].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1208i32);
                self.print_int(((self.obj_tab[crate::ix::U((k) as usize)].int0) as i64));
            }
        }
        self.print(1212i32);
        self.print_ln();
        self.print_ln();
    }

    /// If the same keys in a dictionary are given several times, then it is not
    /// defined which value is chosen by an application.  Therefore the keys
    /// `/Producer` and `/Creator` are only set if the token list
    /// `pdf_info_toks` converted to a string does not contain these key strings.
    // §808
    pub fn pdf_print_info(&mut self) {
        let mut s: str_number = 0; // §808
        let mut creator_given: bool = false; // §808
        let mut producer_given: bool = false; // §808
        let mut creationdate_given: bool = false; // §808
        let mut moddate_given: bool = false; // §808
        let mut trapped_given: bool = false; // §808
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        self.pdf_new_dict(obj_type_others, 0i32, 3i32);
        creator_given = false;
        producer_given = false;
        creationdate_given = false;
        moddate_given = false;
        trapped_given = false;
        if (self.pdf_info_toks != null) {
            {
                s = self.tokens_to_string(self.pdf_info_toks);
                creator_given = self.substr_of_str(1230i32, s);
                producer_given = self.substr_of_str(1231i32, s);
                creationdate_given = self.substr_of_str(1232i32, s);
                moddate_given = self.substr_of_str(1233i32, s);
                trapped_given = self.substr_of_str(1234i32, s);
            }
        }
        if (!producer_given) {
            {
                // §809
                self.pdf_print(1240i32);
                self.pdf_print_int((((pdftex_version / 100i32)) as i64));
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
                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print_int((((pdftex_version % 100i32)) as i64));
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
                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print(pdftex_revision);
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
        }
        // §808
        if (self.pdf_info_toks != null) {
            {
                if ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) > 0i32) {
                    {
                        {
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
                                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
                self.flush_str(s);
                {
                    self.delete_token_ref(self.pdf_info_toks);
                    self.pdf_info_toks = null;
                }
            }
        }
        if (!creator_given) {
            self.pdf_str_entry_ln(1235i32, 1236i32);
        }
        if (__av_eqtb[crate::ix::U(((29373i32) - 1) as usize)].int() == 0i32) {
            {
                if (!creationdate_given) {
                    {
                        // §810
                        self.print_creation_date();
                    }
                }
                // §808
                if (!moddate_given) {
                    {
                        // §811
                        self.print_mod_date();
                    }
                }
            }
        }
        // §808
        if (!trapped_given) {
            {
                {
                    self.pdf_print(1237i32);
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
        if ((__av_eqtb[crate::ix::U(((29374i32) - 1) as usize)].int() % 2i32) == 0i32) {
            {
                if self.get_ptex_use_underscore() {
                    self.pdf_str_entry_ln(1238i32, self.pdftex_banner);
                } else {
                    self.pdf_str_entry_ln(1239i32, self.pdftex_banner);
                }
            }
        }
        self.pdf_end_dict();
    }

    /// The parameters to `hpack` and `vpack` correspond to \TeX's primitives
    /// like `\.{\\hbox} \.{to} \.{300pt}', `\.{\\hbox} \.{spread} \.{10pt}'; note
    /// that `\.{\\hbox}' with no dimension following it is equivalent to
    /// `\.{\\hbox} \.{spread} \.{0pt}'.  The `scan_spec` subroutine scans such
    /// constructions in the user's input, including the mandatory left brace that
    /// follows them, and it puts the specification onto `save_stack` so that the
    /// desired box can later be obtained by executing the following code:
    /// $$\vbox{\halign{#\hfil\cr
    /// `save_ptr:=save_ptr-2;`\cr
    /// `hpack(p,saved(1),saved(0)).`\cr}}$$
    /// Special care is necessary to ensure that the special `save_stack` codes
    /// are placed just below the new group code, because scanning can change
    /// `save_stack` when \.{\\csname} appears.
    // §818
    pub fn scan_spec(&mut self, mut c: group_code, mut three_codes: bool) {
        let mut s: i32 = 0; // §818
        let mut spec_code: i32 = 0; // §818
        'l_found_f: {
            if three_codes {
                s = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int();
            }
            if self.scan_keyword(1258i32) {
                spec_code = exactly;
            } else {
                if self.scan_keyword(1259i32) {
                    spec_code = additional;
                } else {
                    {
                        spec_code = additional;
                        self.cur_val = 0i32;
                        break 'l_found_f;
                    }
                }
            }
            self.scan_dimen(false, false, false);
        }
        if three_codes {
            {
                { let __ix910 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix910) as usize)].set_int(s); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix911 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix911) as usize)].set_int(spec_code); }
        { let __ix912 = (self.save_ptr).wrapping_add(1i32); let __v913 = self.cur_val; self.save_stack[crate::ix::U((__ix912) as usize)].set_int(__v913); }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(c);
        self.scan_left_brace();
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn check_expand_pars(&mut self, mut f: internal_font_number) -> bool {
        let mut check_expand_pars: bool = false;
        let mut k: internal_font_number = 0; // §824
        check_expand_pars = false;
        if ((self.pdf_font_step[crate::ix::U((f) as usize)] == 0i32) || ((self.pdf_font_stretch[crate::ix::U((f) as usize)] == null_font) && (self.pdf_font_shrink[crate::ix::U((f) as usize)] == null_font))) {
            return check_expand_pars;
        }
        if (self.cur_font_step < 0i32) {
            self.cur_font_step = self.pdf_font_step[crate::ix::U((f) as usize)];
        } else {
            if (self.cur_font_step != self.pdf_font_step[crate::ix::U((f) as usize)]) {
                self.pdf_error(1053i32, 1260i32);
            }
        }
        k = self.pdf_font_stretch[crate::ix::U((f) as usize)];
        if (k != null_font) {
            {
                if (self.max_stretch_ratio < 0i32) {
                    self.max_stretch_ratio = self.pdf_font_expand_ratio[crate::ix::U((k) as usize)];
                } else {
                    if (self.max_stretch_ratio != self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]) {
                        self.pdf_error(1053i32, 1261i32);
                    }
                }
            }
        }
        k = self.pdf_font_shrink[crate::ix::U((f) as usize)];
        if (k != null_font) {
            {
                if (self.max_shrink_ratio < 0i32) {
                    self.max_shrink_ratio = (self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]).wrapping_neg();
                } else {
                    if (self.max_shrink_ratio != (self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]).wrapping_neg()) {
                        self.pdf_error(1053i32, 1261i32);
                    }
                }
            }
        }
        check_expand_pars = true;
        check_expand_pars
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn char_stretch(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut char_stretch: scaled = 0;
        let mut k: internal_font_number = 0; // §824
        let mut dw: scaled = 0; // §824
        let mut ef: i32 = 0; // §824
        char_stretch = 0i32;
        k = self.pdf_font_stretch[crate::ix::U((f) as usize)];
        ef = self.get_ef_code(f, c);
        if ((k != null_font) && (ef > 0i32)) {
            {
                dw = ({ let __s915 = ((self.width_base[crate::ix::U((k) as usize)]).wrapping_add({ let __s914 = ((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(self.effective_char(true, k, c))) as usize; self.font_info[crate::ix::U(__s914)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s915)] }.int()).wrapping_sub({ let __s917 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s916 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s916)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s917)] }.int());
                if (dw > 0i32) {
                    char_stretch = self.round_xn_over_d(dw, ef, 1000i32);
                }
            }
        }
        char_stretch
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn char_shrink(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut char_shrink: scaled = 0;
        let mut k: internal_font_number = 0; // §824
        let mut dw: scaled = 0; // §824
        let mut ef: i32 = 0; // §824
        char_shrink = 0i32;
        k = self.pdf_font_shrink[crate::ix::U((f) as usize)];
        ef = self.get_ef_code(f, c);
        if ((k != null_font) && (ef > 0i32)) {
            {
                dw = ({ let __s919 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s918 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s918)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s919)] }.int()).wrapping_sub({ let __s921 = ((self.width_base[crate::ix::U((k) as usize)]).wrapping_add({ let __s920 = ((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(self.effective_char(true, k, c))) as usize; self.font_info[crate::ix::U(__s920)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s921)] }.int());
                if (dw > 0i32) {
                    char_shrink = self.round_xn_over_d(dw, ef, 1000i32);
                }
            }
        }
        char_shrink
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn get_kern(&mut self, mut f: internal_font_number, mut lc: eight_bits, mut rc: eight_bits) -> scaled {
        let mut get_kern: scaled = 0;
        let mut i: four_quarters = four_quarters::default(); // §824
        let mut j: four_quarters = four_quarters::default(); // §824
        let mut k: font_index = 0; // §824
        // goto labels: continue, L23
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                get_kern = 0i32;
                i = { let __s922 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, lc))) as usize; self.font_info[crate::ix::U(__s922)] }.qqqq();
                if ((i.b2() % 4i32) != lig_tag) {
                    return get_kern;
                }
                k = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add(i.b3());
                j = self.font_info[crate::ix::U((k) as usize)].qqqq();
                if (j.b0() <= stop_flag) {
                    { __goto_1 = 2; continue 'l_dispatch_1; }
                }
                k = ((((self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(j.b2()))).wrapping_add(j.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
            }
            if __goto_1 <= 1 { // continue
                j = self.font_info[crate::ix::U((k) as usize)].qqqq();
            }
            if __goto_1 <= 2 { // L23
                if (((j.b1() == rc) && (j.b0() <= stop_flag)) && (j.b2() >= kern_flag)) {
                    {
                        get_kern = self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(j.b2()))).wrapping_add(j.b3())) as usize)].int();
                        return get_kern;
                    }
                }
                if (j.b0() == 0i32) {
                    k = (k).wrapping_add(1i32);
                } else {
                    {
                        if (j.b0() >= stop_flag) {
                            return get_kern;
                        }
                        k = ((k).wrapping_add(j.b0())).wrapping_add(1i32);
                    }
                }
                { __goto_1 = 1; continue 'l_dispatch_1; }
            }
            break 'l_dispatch_1;
        }
        get_kern
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn kern_stretch(&mut self, mut p: halfword) -> scaled {
        let mut kern_stretch: scaled = 0;
        let mut l: halfword = 0; // §824
        let mut r: halfword = 0; // §824
        let mut d: scaled = 0; // §824
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        kern_stretch = 0i32;
        if (((self.prev_char_p == null) || (__av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().rh() != p)) || (__av_mem[crate::ix::U((p) as usize)].hh().rh() == null)) {
            return kern_stretch;
        }
        l = self.prev_char_p;
        r = __av_mem[crate::ix::U((p) as usize)].hh().rh();
        if (!(l >= self.hi_mem_min)) {
            if (__av_mem[crate::ix::U((l) as usize)].hh().b0() == ligature_node) {
                l = (l).wrapping_add(1i32);
            } else {
                return kern_stretch;
            }
        }
        if (!(r >= self.hi_mem_min)) {
            if (__av_mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node) {
                r = (r).wrapping_add(1i32);
            } else {
                return kern_stretch;
            }
        }
        if (!((__av_mem[crate::ix::U((l) as usize)].hh().b0() == __av_mem[crate::ix::U((r) as usize)].hh().b0()) && (self.pdf_font_stretch[crate::ix::U((__av_mem[crate::ix::U((l) as usize)].hh().b0()) as usize)] != null_font))) {
            return kern_stretch;
        }
        d = self.get_kern(self.pdf_font_stretch[crate::ix::U((__av_mem[crate::ix::U((l) as usize)].hh().b0()) as usize)], __av_mem[crate::ix::U((l) as usize)].hh().b1(), __av_mem[crate::ix::U((r) as usize)].hh().b1());
        kern_stretch = { let __a923_0 = (d).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()); let __a923_1 = self.get_ef_code(__av_mem[crate::ix::U((l) as usize)].hh().b0(), __av_mem[crate::ix::U((l) as usize)].hh().b1()); let __a923_2 = 1000i32; self.round_xn_over_d(__a923_0, __a923_1, __a923_2) };
        kern_stretch
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn kern_shrink(&mut self, mut p: halfword) -> scaled {
        let mut kern_shrink: scaled = 0;
        let mut l: halfword = 0; // §824
        let mut r: halfword = 0; // §824
        let mut d: scaled = 0; // §824
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        kern_shrink = 0i32;
        if (((self.prev_char_p == null) || (__av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().rh() != p)) || (__av_mem[crate::ix::U((p) as usize)].hh().rh() == null)) {
            return kern_shrink;
        }
        l = self.prev_char_p;
        r = __av_mem[crate::ix::U((p) as usize)].hh().rh();
        if (!(l >= self.hi_mem_min)) {
            if (__av_mem[crate::ix::U((l) as usize)].hh().b0() == ligature_node) {
                l = (l).wrapping_add(1i32);
            } else {
                return kern_shrink;
            }
        }
        if (!(r >= self.hi_mem_min)) {
            if (__av_mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node) {
                r = (r).wrapping_add(1i32);
            } else {
                return kern_shrink;
            }
        }
        if (!((__av_mem[crate::ix::U((l) as usize)].hh().b0() == __av_mem[crate::ix::U((r) as usize)].hh().b0()) && (self.pdf_font_shrink[crate::ix::U((__av_mem[crate::ix::U((l) as usize)].hh().b0()) as usize)] != null_font))) {
            return kern_shrink;
        }
        d = self.get_kern(self.pdf_font_shrink[crate::ix::U((__av_mem[crate::ix::U((l) as usize)].hh().b0()) as usize)], __av_mem[crate::ix::U((l) as usize)].hh().b1(), __av_mem[crate::ix::U((r) as usize)].hh().b1());
        kern_shrink = { let __a924_0 = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_sub(d); let __a924_1 = self.get_ef_code(__av_mem[crate::ix::U((l) as usize)].hh().b0(), __av_mem[crate::ix::U((l) as usize)].hh().b1()); let __a924_2 = 1000i32; self.round_xn_over_d(__a924_0, __a924_1, __a924_2) };
        kern_shrink
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn do_subst_font(&mut self, mut p: halfword, mut ex_ratio: i32) {
        let mut f: internal_font_number = 0; // §824
        let mut k: internal_font_number = 0; // §824
        let mut r: halfword = 0; // §824
        let mut ef: i32 = 0; // §824
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if ((!(p >= self.hi_mem_min)) && (__av_mem[crate::ix::U((p) as usize)].hh().b0() == disc_node)) {
            {
                r = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                while (r != null) {
                    {
                        if ((r >= self.hi_mem_min) || (__av_mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node)) {
                            self.do_subst_font(r, ex_ratio);
                        }
                        r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                r = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                while (r != null) {
                    {
                        if ((r >= self.hi_mem_min) || (__av_mem[crate::ix::U((r) as usize)].hh().b0() == ligature_node)) {
                            self.do_subst_font(r, ex_ratio);
                        }
                        r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                return;
            }
        }
        if (p >= self.hi_mem_min) {
            r = p;
        } else {
            if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == ligature_node) {
                r = (p).wrapping_add(1i32);
            } else {
                {
                    self.pdf_error(1053i32, 1262i32);
                }
            }
        }
        f = __av_mem[crate::ix::U((r) as usize)].hh().b0();
        ef = self.get_ef_code(f, __av_mem[crate::ix::U((r) as usize)].hh().b1());
        if (ef == 0i32) {
            return;
        }
        if ((self.pdf_font_stretch[crate::ix::U((f) as usize)] != null_font) && (ex_ratio > 0i32)) {
            k = { let __a925_0 = f; let __a925_1 = self.ext_xn_over_d((ex_ratio).wrapping_mul(ef), self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)], 1000000i32); self.expand_font(__a925_0, __a925_1) };
        } else {
            if ((self.pdf_font_shrink[crate::ix::U((f) as usize)] != null_font) && (ex_ratio < 0i32)) {
                k = { let __a926_0 = f; let __a926_1 = self.ext_xn_over_d((ex_ratio).wrapping_mul(ef), (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg(), 1000000i32); self.expand_font(__a926_0, __a926_1) };
            } else {
                k = f;
            }
        }
        if (k != f) {
            {
                __av_mem[crate::ix::U((r) as usize)].set_hh_b0(k);
                if (!(p >= self.hi_mem_min)) {
                    {
                        r = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                        while (r != null) {
                            {
                                __av_mem[crate::ix::U((r) as usize)].set_hh_b0(k);
                                r = __av_mem[crate::ix::U((r) as usize)].hh().rh();
                            }
                        }
                    }
                }
            }
        }
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn char_pw(&mut self, mut p: halfword, mut side: small_number) -> scaled {
        let mut char_pw: scaled = 0;
        let mut f: internal_font_number = 0; // §824
        let mut c: i32 = 0; // §824
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        char_pw = 0i32;
        if (side == left_side) {
            self.last_leftmost_char = null;
        } else {
            self.last_rightmost_char = null;
        }
        if (p == null) {
            return char_pw;
        }
        if (!(p >= self.hi_mem_min)) {
            {
                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == ligature_node) {
                    p = (p).wrapping_add(1i32);
                } else {
                    return char_pw;
                }
            }
        }
        f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
        if (side == left_side) {
            {
                c = self.get_lp_code(f, __av_mem[crate::ix::U((p) as usize)].hh().b1());
                self.last_leftmost_char = p;
            }
        } else {
            {
                c = self.get_rp_code(f, __av_mem[crate::ix::U((p) as usize)].hh().b1());
                self.last_rightmost_char = p;
            }
        }
        if (c == 0i32) {
            return char_pw;
        }
        char_pw = self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), c, 1000i32);
        char_pw
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn new_margin_kern(&mut self, mut w: scaled, mut p: halfword, mut side: small_number) -> halfword {
        let mut new_margin_kern: halfword = 0;
        let mut k: halfword = 0; // §824
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        k = self.get_node(margin_kern_node_size);
        __av_mem[crate::ix::U((k) as usize)].set_hh_b0(margin_kern_node);
        __av_mem[crate::ix::U((k) as usize)].set_hh_b1(side);
        __av_mem[crate::ix::U(((k).wrapping_add(1i32)) as usize)].set_int(w);
        if (p == null) {
            self.pdf_error(1263i32, 1264i32);
        }
        {
            { let __v927 = self.avail; __av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].set_hh_lh(__v927); }
            if (__av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh() == null) {
                { let __v928 = self.get_avail(); __av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].set_hh_lh(__v928); }
            } else {
                {
                    self.avail = __av_mem[crate::ix::U((__av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().rh();
                    { let __ix929 = __av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); __av_mem[crate::ix::U((__ix929) as usize)].set_hh_rh(null); }
                    self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                    self.dl_new_node(__av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh());
                }
            }
        }
        { let __ix930 = __av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); let __v931 = __av_mem[crate::ix::U((p) as usize)].hh().b1(); __av_mem[crate::ix::U((__ix930) as usize)].set_hh_b1(__v931); }
        { let __ix932 = __av_mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); let __v933 = __av_mem[crate::ix::U((p) as usize)].hh().b0(); __av_mem[crate::ix::U((__ix932) as usize)].set_hh_b0(__v933); }
        new_margin_kern = k;
        new_margin_kern
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §824
    pub fn hpack(&mut self, mut p: halfword, mut w: scaled, mut m: small_number) -> halfword {
        let mut hpack: halfword = 0;
        let mut r: halfword = 0; // §824
        let mut q: halfword = 0; // §824
        let mut h: scaled = 0; // §824
        let mut d: scaled = 0; // §824
        let mut x: scaled = 0; // §824
        let mut s: scaled = 0; // §824
        let mut g: halfword = 0; // §824
        let mut o: glue_ord = 0; // §824
        let mut f: internal_font_number = 0; // §824
        let mut i: four_quarters = four_quarters::default(); // §824
        let mut hd: eight_bits = 0; // §824
        let mut font_stretch: scaled = 0; // §824
        let mut font_shrink: scaled = 0; // §824
        let mut k: scaled = 0; // §824
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        // goto labels: common_ending, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.last_badness = 0i32;
                if self.bm_rec_on {
                    self.flashtex_bm_packed();
                }
                r = self.get_node(box_node_size);
                __av_mem[crate::ix::U((r) as usize)].set_hh_b0(hlist_node);
                __av_mem[crate::ix::U((r) as usize)].set_hh_b1(min_quarterword);
                __av_mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                q = (r).wrapping_add(5i32);
                __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                if (m == cal_expand_ratio) {
                    {
                        self.prev_char_p = null;
                        font_stretch = 0i32;
                        font_shrink = 0i32;
                        self.font_expand_ratio = 0i32;
                    }
                }
                h = 0i32;
                // §825
                d = 0i32;
                x = 0i32;
                self.total_stretch[crate::ix::U((normal) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((normal) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((fil) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((fil) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((fill) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((fill) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((filll) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((filll) as usize)] = 0i32;
                // §824
                if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                    // §1711
                    {
                        self.temp_ptr = self.get_avail();
                        { let __ix934 = self.temp_ptr; __av_mem[crate::ix::U((__ix934) as usize)].set_hh_lh(before); }
                        { let __ix935 = self.temp_ptr; let __v936 = self.LR_ptr; __av_mem[crate::ix::U((__ix935) as usize)].set_hh_rh(__v936); }
                        self.LR_ptr = self.temp_ptr;
                    }
                }
                // §824
                while (p != null) {
                    // §826
                    {
                        'l_reswitch_b: loop {
                            while (p >= self.hi_mem_min) {
                                // §829
                                {
                                    if (m >= cal_expand_ratio) {
                                        {
                                            self.prev_char_p = p;
                                            match m {
                                                cal_expand_ratio => {
                                                    {
                                                        f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                                        font_stretch = (font_stretch).wrapping_add(self.char_stretch(f, __av_mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        font_shrink = (font_shrink).wrapping_add(self.char_shrink(f, __av_mem[crate::ix::U((p) as usize)].hh().b1()));
                                                    }
                                                }
                                                subst_ex_font => {
                                                    self.do_subst_font(p, self.font_expand_ratio);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                                    i = { let __s937 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, __av_mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s937)] }.qqqq();
                                    hd = i.b1();
                                    x = (x).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int());
                                    s = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int();
                                    if (s > h) {
                                        h = s;
                                    }
                                    s = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int();
                                    if (s > d) {
                                        d = s;
                                    }
                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            // §826
                            if (p != null) {
                                {
                                    match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node | rule_node | unset_node => {
                                            // §828
                                            {
                                                x = (x).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b0() >= rule_node) {
                                                    s = 0i32;
                                                } else {
                                                    s = __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int();
                                                }
                                                if ((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_sub(s) > h) {
                                                    h = (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_sub(s);
                                                }
                                                if ((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(s) > d) {
                                                    d = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(s);
                                                }
                                            }
                                        }
                                        ins_node | mark_node | adjust_node => {
                                            // §826
                                            if ((self.adjust_tail != null) || (self.pre_adjust_tail != null)) {
                                                // §832
                                                {
                                                    while (__av_mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                                                        q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node) {
                                                        {
                                                            if (__av_mem[crate::ix::U((p) as usize)].hh().b1() != 0i32) {
                                                                {
                                                                    if (self.pre_adjust_tail == null) {
                                                                        self.confusion(1265i32);
                                                                    }
                                                                    { let __ix938 = self.pre_adjust_tail; let __v939 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix938) as usize)].set_hh_rh(__v939); }
                                                                    while (__av_mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh() != null) {
                                                                        self.pre_adjust_tail = __av_mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.adjust_tail == null) {
                                                                        self.confusion(1265i32);
                                                                    }
                                                                    { let __ix940 = self.adjust_tail; let __v941 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U((__ix940) as usize)].set_hh_rh(__v941); }
                                                                    while (__av_mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh() != null) {
                                                                        self.adjust_tail = __av_mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                            self.free_node(__av_mem[crate::ix::U((q) as usize)].hh().rh(), small_node_size);
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix942 = self.adjust_tail; __av_mem[crate::ix::U((__ix942) as usize)].set_hh_rh(p); }
                                                            self.adjust_tail = p;
                                                            p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                                        }
                                                    }
                                                    __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                    p = q;
                                                }
                                            }
                                        }
                                        whatsit_node => {
                                            // §1608
                                            if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                                {
                                                    x = (x).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    s = 0i32;
                                                    if ((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(s) > h) {
                                                        h = (__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(s);
                                                    }
                                                    if ((__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(s) > d) {
                                                        d = (__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(s);
                                                    }
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §833
                                            {
                                                g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                x = (x).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                                o = __av_mem[crate::ix::U((g) as usize)].hh().b0();
                                                { let __v943 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v943; }
                                                o = __av_mem[crate::ix::U((g) as usize)].hh().b1();
                                                { let __v944 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v944; }
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    {
                                                        g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (__av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int() > h) {
                                                            h = __av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int();
                                                        }
                                                        if (__av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int() > d) {
                                                            d = __av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        margin_kern_node => {
                                            // §826
                                            {
                                                if (m == cal_expand_ratio) {
                                                    {
                                                        f = __av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0();
                                                        self.do_subst_font(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), 1000i32);
                                                        if (f != __av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0()) {
                                                            font_stretch = ((font_stretch).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int())).wrapping_sub(self.char_pw(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), __av_mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        }
                                                        { let __ix945 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); __av_mem[crate::ix::U((__ix945) as usize)].set_hh_b0(f); }
                                                        self.do_subst_font(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), (1000i32).wrapping_neg());
                                                        if (f != __av_mem[crate::ix::U((__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0()) {
                                                            font_shrink = ((font_shrink).wrapping_sub(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int())).wrapping_sub(self.char_pw(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), __av_mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        }
                                                        { let __ix946 = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); __av_mem[crate::ix::U((__ix946) as usize)].set_hh_b0(f); }
                                                    }
                                                } else {
                                                    if (m == subst_ex_font) {
                                                        {
                                                            self.do_subst_font(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.font_expand_ratio);
                                                            { let __v947 = (self.char_pw(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), __av_mem[crate::ix::U((p) as usize)].hh().b1())).wrapping_neg(); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v947); }
                                                        }
                                                    }
                                                }
                                                x = (x).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        kern_node => {
                                            {
                                                if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == normal) {
                                                    {
                                                        if (m == cal_expand_ratio) {
                                                            {
                                                                font_stretch = (font_stretch).wrapping_add(self.kern_stretch(p));
                                                                font_shrink = (font_shrink).wrapping_add(self.kern_shrink(p));
                                                            }
                                                        } else {
                                                            if (m == subst_ex_font) {
                                                                {
                                                                    if (self.font_expand_ratio > 0i32) {
                                                                        k = self.kern_stretch(p);
                                                                    } else {
                                                                        if (self.font_expand_ratio < 0i32) {
                                                                            k = self.kern_shrink(p);
                                                                        } else {
                                                                            self.pdfassert(((0i32) != 0));
                                                                        }
                                                                    }
                                                                    if (k != 0i32) {
                                                                        {
                                                                            if (__av_mem[crate::ix::U((p) as usize)].hh().rh() >= self.hi_mem_min) {
                                                                                { let __v948 = self.get_kern(__av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().b0(), __av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().b1(), __av_mem[crate::ix::U((__av_mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b1()); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v948); }
                                                                            } else {
                                                                                if (__av_mem[crate::ix::U((__av_mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b0() == ligature_node) {
                                                                                    { let __v949 = self.get_kern(__av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().b0(), __av_mem[crate::ix::U((self.prev_char_p) as usize)].hh().b1(), __av_mem[crate::ix::U(((__av_mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().b1()); __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v949); }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                x = (x).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        math_node => {
                                            {
                                                x = (x).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                                                    // §1712
                                                    if (((__av_mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                        if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                            {
                                                                self.temp_ptr = self.LR_ptr;
                                                                self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                {
                                                                    { let __ix950 = self.temp_ptr; let __v951 = self.avail; __av_mem[crate::ix::U((__ix950) as usize)].set_hh_rh(__v951); }
                                                                    self.avail = self.temp_ptr;
                                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                __av_mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                __av_mem[crate::ix::U((p) as usize)].set_hh_b1(explicit);
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            self.temp_ptr = self.get_avail();
                                                            { let __ix952 = self.temp_ptr; let __v953 = ((L_code).wrapping_mul((__av_mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); __av_mem[crate::ix::U((__ix952) as usize)].set_hh_lh(__v953); }
                                                            { let __ix954 = self.temp_ptr; let __v955 = self.LR_ptr; __av_mem[crate::ix::U((__ix954) as usize)].set_hh_rh(__v955); }
                                                            self.LR_ptr = self.temp_ptr;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        ligature_node => {
                                            // §826
                                            {
                                                if (m == subst_ex_font) {
                                                    self.do_subst_font(p, self.font_expand_ratio);
                                                }
                                                // §827
                                                {
                                                    { let __v956 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U((lig_trick) as usize)] = __v956; }
                                                    { let __v957 = __av_mem[crate::ix::U((p) as usize)].hh().rh(); __av_mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v957); }
                                                    p = lig_trick;
                                                    continue 'l_reswitch_b;
                                                }
                                            }
                                        }
                                        disc_node => {
                                            // §826
                                            if (m == subst_ex_font) {
                                                self.do_subst_font(p, self.font_expand_ratio);
                                            }
                                        }
                                        _ => {
                                        }
                                    }
                                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            break 'l_reswitch_b;
                        }
                    }
                }
                // §824
                if (self.adjust_tail != null) {
                    { let __ix958 = self.adjust_tail; __av_mem[crate::ix::U((__ix958) as usize)].set_hh_rh(null); }
                }
                if (self.pre_adjust_tail != null) {
                    { let __ix959 = self.pre_adjust_tail; __av_mem[crate::ix::U((__ix959) as usize)].set_hh_rh(null); }
                }
                __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                // §834
                if (m == additional) {
                    w = (x).wrapping_add(w);
                }
                __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                x = (w).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                        __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                        __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        { __goto_1 = 2; continue 'l_dispatch_1; }
                    }
                } else {
                    if (x > 0i32) {
                        // §835
                        {
                            // §836
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
                            // §835
                            if (((m == cal_expand_ratio) && (o == normal)) && (font_stretch > 0i32)) {
                                {
                                    self.font_expand_ratio = self.divide_scaled(x, font_stretch, 3i32);
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v960 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v960); }
                            } else {
                                {
                                    __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == normal) {
                                if (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null) {
                                    // §837
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((normal) as usize)]);
                                        if (self.last_badness > __av_eqtb[crate::ix::U(((29303i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(1266i32);
                                                } else {
                                                    self.print_nl(1267i32);
                                                }
                                                self.print(1268i32);
                                                self.print_int(((self.last_badness) as i64));
                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                            }
                                        }
                                    }
                                }
                            }
                            // §835
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    } else {
                        // §841
                        {
                            // §842
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
                            // §841
                            if (((m == cal_expand_ratio) && (o == normal)) && (font_shrink > 0i32)) {
                                {
                                    self.font_expand_ratio = self.divide_scaled(x, font_shrink, 3i32);
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v961 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v961); }
                            } else {
                                {
                                    __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == normal)) && (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null)) {
                                {
                                    self.last_badness = 1000000i32;
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §843
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > __av_eqtb[crate::ix::U(((29911i32) - 1) as usize)].int()) || (__av_eqtb[crate::ix::U(((29303i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            if ((__av_eqtb[crate::ix::U(((29919i32) - 1) as usize)].int() > 0i32) && (((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > __av_eqtb[crate::ix::U(((29911i32) - 1) as usize)].int())) {
                                                {
                                                    while (__av_mem[crate::ix::U((q) as usize)].hh().rh() != null) {
                                                        q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    { let __v962 = self.new_rule(); __av_mem[crate::ix::U((q) as usize)].set_hh_rh(__v962); }
                                                    { let __ix963 = (__av_mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); let __v964 = __av_eqtb[crate::ix::U(((29919i32) - 1) as usize)].int(); __av_mem[crate::ix::U((__ix963) as usize)].set_int(__v964); }
                                                }
                                            }
                                            self.print_ln();
                                            self.print_nl(1274i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]));
                                            self.print(1275i32);
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            } else {
                                // §841
                                if (o == normal) {
                                    if (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null) {
                                        // §844
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((normal) as usize)]);
                                            if (self.last_badness > __av_eqtb[crate::ix::U(((29303i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(1276i32);
                                                    self.print_int(((self.last_badness) as i64));
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §841
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // common_ending
                // §824
                self.dg_box_begin(r);
                // §840
                if self.output_active {
                    self.print(1269i32);
                } else {
                    {
                        self.ls_box_lines();
                        if (self.pack_begin_line != 0i32) {
                            {
                                if (self.pack_begin_line > 0i32) {
                                    self.print(1270i32);
                                } else {
                                    self.print(1271i32);
                                }
                                self.print_int((((self.pack_begin_line).wrapping_abs()) as i64));
                                self.print(1272i32);
                            }
                        } else {
                            self.print(1273i32);
                        }
                        self.print_int(((self.line) as i64));
                    }
                }
                self.print_ln();
                self.font_in_short_display = null_font;
                self.short_display(__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh());
                self.print_ln();
                self.dg_box_end(r);
                self.begin_diagnostic();
                self.show_box(r);
                self.end_diagnostic(true);
            }
            if __goto_1 <= 2 { // exit
                // §824
                if (__av_eqtb[crate::ix::U(((29389i32) - 1) as usize)].int() > 0i32) {
                    // §1713
                    {
                        if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != before) {
                            {
                                while (__av_mem[crate::ix::U((q) as usize)].hh().rh() != null) {
                                    q = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                                }
                                loop {
                                    self.temp_ptr = q;
                                    q = self.new_math(0i32, __av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh());
                                    { let __ix965 = self.temp_ptr; __av_mem[crate::ix::U((__ix965) as usize)].set_hh_rh(q); }
                                    self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                        {
                                            { let __ix966 = self.temp_ptr; let __v967 = self.avail; __av_mem[crate::ix::U((__ix966) as usize)].set_hh_rh(__v967); }
                                            self.avail = self.temp_ptr;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                    if (__av_mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == before) { break; }
                                }
                            }
                        }
                        if (self.LR_problems > 0i32) {
                            {
                                // §1714
                                {
                                    self.print_ln();
                                    self.print_nl(2053i32);
                                    self.print_int((((self.LR_problems / 10000i32)) as i64));
                                    self.print(2054i32);
                                    self.print_int((((self.LR_problems % 10000i32)) as i64));
                                    self.print(2055i32);
                                    self.LR_problems = 0i32;
                                }
                                // §1713
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                        {
                            self.temp_ptr = self.LR_ptr;
                            self.LR_ptr = __av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                            {
                                { let __ix968 = self.temp_ptr; let __v969 = self.avail; __av_mem[crate::ix::U((__ix968) as usize)].set_hh_rh(__v969); }
                                self.avail = self.temp_ptr;
                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                            }
                        }
                        if (self.LR_ptr != null) {
                            self.confusion(2052i32);
                        }
                    }
                }
                // §824
                if ((m == cal_expand_ratio) && (self.font_expand_ratio != 0i32)) {
                    {
                        self.font_expand_ratio = self.fix_int(self.font_expand_ratio, (1000i32).wrapping_neg(), 1000i32);
                        q = __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh();
                        self.free_node(r, box_node_size);
                        r = self.hpack(q, w, subst_ex_font);
                    }
                }
                hpack = r;
            }
            break 'l_dispatch_1;
        }
        hpack
    }

    /// The `vpack` subroutine is actually a special case of a slightly more
    /// general routine called `vpackage`, which has four parameters. The fourth
    /// parameter, which is `max_dimen` in the case of `vpack`, specifies the
    /// maximum depth of the page box that is constructed. The depth is first
    /// computed by the normal rules; if it exceeds this limit, the reference
    /// point is simply moved down until the limiting depth is attained.
    // §845
    pub fn vpackage(&mut self, mut p: halfword, mut h: scaled, mut m: small_number, mut l: scaled) -> halfword {
        let mut vpackage: halfword = 0;
        let mut r: halfword = 0; // §845
        let mut w: scaled = 0; // §845
        let mut d: scaled = 0; // §845
        let mut x: scaled = 0; // §845
        let mut s: scaled = 0; // §845
        let mut g: halfword = 0; // §845
        let mut o: glue_ord = 0; // §845
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_exit_f: {
            'l_common_ending_f: {
                self.last_badness = 0i32;
                if self.bm_rec_on {
                    self.flashtex_bm_packed();
                }
                r = self.get_node(box_node_size);
                __av_mem[crate::ix::U((r) as usize)].set_hh_b0(vlist_node);
                __av_mem[crate::ix::U((r) as usize)].set_hh_b1(min_quarterword);
                __av_mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                w = 0i32;
                // §825
                d = 0i32;
                x = 0i32;
                self.total_stretch[crate::ix::U((normal) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((normal) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((fil) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((fil) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((fill) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((fill) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((filll) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((filll) as usize)] = 0i32;
                // §845
                while (p != null) {
                    // §846
                    {
                        if (p >= self.hi_mem_min) {
                            self.confusion(1277i32);
                        } else {
                            match __av_mem[crate::ix::U((p) as usize)].hh().b0() {
                                hlist_node | vlist_node | rule_node | unset_node => {
                                    // §847
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                        d = __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() >= rule_node) {
                                            s = 0i32;
                                        } else {
                                            s = __av_mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int();
                                        }
                                        if ((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s) > w) {
                                            w = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s);
                                        }
                                    }
                                }
                                whatsit_node => {
                                    // §1607
                                    if ((__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refxform_node) || (__av_mem[crate::ix::U((p) as usize)].hh().b1() == pdf_refximage_node)) {
                                        {
                                            x = ((x).wrapping_add(d)).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            d = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                            s = 0i32;
                                            if ((__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s) > w) {
                                                w = (__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s);
                                            }
                                        }
                                    }
                                }
                                glue_node => {
                                    // §848
                                    {
                                        x = (x).wrapping_add(d);
                                        d = 0i32;
                                        g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                        x = (x).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                        o = __av_mem[crate::ix::U((g) as usize)].hh().b0();
                                        { let __v970 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v970; }
                                        o = __av_mem[crate::ix::U((g) as usize)].hh().b1();
                                        { let __v971 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(__av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v971; }
                                        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                            {
                                                g = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                if (__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int() > w) {
                                                    w = __av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int();
                                                }
                                            }
                                        }
                                    }
                                }
                                kern_node => {
                                    // §846
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        d = 0i32;
                                    }
                                }
                                _ => {
                                }
                            }
                        }
                        p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
                // §845
                __av_mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                if (d > l) {
                    {
                        x = ((x).wrapping_add(d)).wrapping_sub(l);
                        __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(l);
                    }
                } else {
                    __av_mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                }
                // §849
                if (m == additional) {
                    h = (x).wrapping_add(h);
                }
                __av_mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                x = (h).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                        __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                        __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        break 'l_exit_f;
                    }
                } else {
                    if (x > 0i32) {
                        // §850
                        {
                            // §836
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
                            // §850
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v972 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v972); }
                            } else {
                                {
                                    __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == normal) {
                                if (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null) {
                                    // §851
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((normal) as usize)]);
                                        if (self.last_badness > __av_eqtb[crate::ix::U(((29304i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(1266i32);
                                                } else {
                                                    self.print_nl(1267i32);
                                                }
                                                self.print(1278i32);
                                                self.print_int(((self.last_badness) as i64));
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                            // §850
                            break 'l_exit_f;
                        }
                    } else {
                        // §853
                        {
                            // §842
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
                            // §853
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v973 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v973); }
                            } else {
                                {
                                    __av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == normal)) && (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null)) {
                                {
                                    self.last_badness = 1000000i32;
                                    __av_mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §854
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > __av_eqtb[crate::ix::U(((29912i32) - 1) as usize)].int()) || (__av_eqtb[crate::ix::U(((29304i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            self.print_ln();
                                            self.print_nl(1279i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]));
                                            self.print(1280i32);
                                            break 'l_common_ending_f;
                                        }
                                    }
                                }
                            } else {
                                // §853
                                if (o == normal) {
                                    if (__av_mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != null) {
                                        // §855
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((normal) as usize)]);
                                            if (self.last_badness > __av_eqtb[crate::ix::U(((29304i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(1281i32);
                                                    self.print_int(((self.last_badness) as i64));
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §853
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §845
            self.dg_box_begin(r);
            // §852
            if self.output_active {
                self.print(1269i32);
            } else {
                {
                    self.ls_box_lines();
                    if (self.pack_begin_line != 0i32) {
                        {
                            self.print(1271i32);
                            self.print_int((((self.pack_begin_line).wrapping_abs()) as i64));
                            self.print(1272i32);
                        }
                    } else {
                        self.print(1273i32);
                    }
                    self.print_int(((self.line) as i64));
                    self.print_ln();
                }
            }
            self.dg_box_end(r);
            self.begin_diagnostic();
            self.show_box(r);
            self.end_diagnostic(true);
        }
        // §845
        vpackage = r;
        vpackage
    }

    /// When a box is being appended to the current vertical list, the
    /// baselineskip calculation is handled by the `append_to_vlist` routine.
    // §856
    pub fn append_to_vlist(&mut self, mut b: halfword) {
        let mut d: scaled = 0; // §856
        let mut p: halfword = 0; // §856
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (self.cur_list.aux_field.int() > __av_eqtb[crate::ix::U(((29935i32) - 1) as usize)].int()) {
            {
                d = ((__av_mem[crate::ix::U(((__av_eqtb[crate::ix::U(((26629i32) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_list.aux_field.int())).wrapping_sub(__av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int());
                if (d < __av_eqtb[crate::ix::U(((29905i32) - 1) as usize)].int()) {
                    p = self.new_param_glue(line_skip_code);
                } else {
                    {
                        p = self.new_skip_param(baseline_skip_code);
                        { let __ix974 = (self.temp_ptr).wrapping_add(1i32); __av_mem[crate::ix::U((__ix974) as usize)].set_int(d); }
                    }
                }
                { let __ix975 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix975) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
        { let __ix976 = self.cur_list.tail_field; __av_mem[crate::ix::U((__ix976) as usize)].set_hh_rh(b); }
        self.cur_list.tail_field = b;
        { let __v977 = __av_mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int(); self.cur_list.aux_field.set_int(__v977); }
    }

    /// The `new_noad` function creates an `ord_noad` that is completely null.
    // §863
    pub fn new_noad(&mut self) -> halfword {
        let mut new_noad: halfword = 0;
        let mut p: halfword = 0; // §863
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(noad_size);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(ord_noad);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        { let __v978 = self.empty_field; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh(__v978); }
        { let __v979 = self.empty_field; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh(__v979); }
        { let __v980 = self.empty_field; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh(__v980); }
        new_noad = p;
        new_noad
    }

    /// Math formulas can also contain instructions like \.{\\textstyle} that
    /// override \TeX's normal style rules. A `style_node` is inserted into the
    /// data structure to record such instructions; it is three words long, so it
    /// is considered a node instead of a noad. The `subtype` is either `display_style`
    /// or `text_style` or `script_style` or `script_script_style`. The
    /// second and third words of a `style_node` are not used, but they are
    /// present because a `choice_node` is converted to a `style_node`.
    /// \TeX\ uses even numbers 0, 2, 4, 6 to encode the basic styles
    /// `display_style`, \dots, `script_script_style`, and adds~1 to get the
    /// ``cramped'' versions of these styles. This gives a numerical order that
    /// is backwards from the convention of Appendix~G in {\sl The \TeX book\/};
    /// i.e., a smaller style has a larger numerical value.
    // §865
    pub fn new_style(&mut self, mut s: small_number) -> halfword {
        let mut new_style: halfword = 0;
        let mut p: halfword = 0; // §865
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(style_node_size);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(style_node);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(0i32);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        new_style = p;
        new_style
    }

    /// Finally, the \.{\\mathchoice} primitive creates a `choice_node`, which
    /// has special subfields `display_mlist`, `text_mlist`, `script_mlist`,
    /// and `script_script_mlist` pointing to the mlists for each style.
    // §866
    pub fn new_choice(&mut self) -> halfword {
        let mut new_choice: halfword = 0;
        let mut p: halfword = 0; // §866
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.get_node(style_node_size);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(choice_node);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(null);
        __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(null);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh(null);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh(null);
        new_choice = p;
        new_choice
    }

    /// The inelegant introduction of `show_info` in the code above seems better
    /// than the alternative of using \PASCAL's strange `forward` declaration for a
    /// procedure with parameters. The \PASCAL\ convention about dropping parameters
    /// from a post-`forward` procedure is, frankly, so intolerable to the author
    /// of \TeX\ that he would rather stoop to communication via a global temporary
    /// variable. (A similar stupidity occurred with respect to `hlist_out` and
    /// `vlist_out` above, and it will occur with respect to `mlist_to_hlist` below.)
    // §870
    pub fn show_info(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.show_node_list(__av_mem[crate::ix::U((self.temp_ptr) as usize)].hh().lh());
    }

    /// Here is a function that returns a pointer to a rule node having a given
    /// thickness `t`. The rule will extend horizontally to the boundary of the vlist
    /// that eventually contains it.
    // §881
    pub fn fraction_rule(&mut self, mut t: scaled) -> halfword {
        let mut fraction_rule: halfword = 0;
        let mut p: halfword = 0; // §881
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.new_rule();
        __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(t);
        __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        fraction_rule = p;
        fraction_rule
    }

    /// The `overbar` function returns a pointer to a vlist box that consists of
    /// a given box `b`, above which has been placed a kern of height `k` under a
    /// fraction rule of thickness `t` under additional space of height `t`.
    // §882
    pub fn overbar(&mut self, mut b: halfword, mut k: scaled, mut t: scaled) -> halfword {
        let mut overbar: halfword = 0;
        let mut p: halfword = 0; // §882
        let mut q: halfword = 0; // §882
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.new_kern(k);
        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(b);
        q = self.fraction_rule(t);
        __av_mem[crate::ix::U((q) as usize)].set_hh_rh(p);
        p = self.new_kern(t);
        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(q);
        overbar = self.vpackage(p, 0i32, additional, max_dimen);
        overbar
    }

    /// Here is a subroutine that creates a new box, whose list contains a
    /// single character, and whose width includes the italic correction for
    /// that character. The height or depth of the box will be negative, if
    /// the height or depth of the character is negative; thus, this routine
    /// may deliver a slightly different result than `hpack` would produce.
    /// @<Declare subprocedures for `var_delimiter`
    // §886
    pub fn char_box(&mut self, mut f: internal_font_number, mut c: quarterword) -> halfword {
        let mut char_box: halfword = 0;
        let mut q: four_quarters = four_quarters::default(); // §886
        let mut hd: eight_bits = 0; // §886
        let mut b: halfword = 0; // §886
        let mut p: halfword = 0; // §886
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        q = { let __s981 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s981)] }.qqqq();
        hd = q.b1();
        b = self.new_null_box();
        { let __v982 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add((q.b2() / 4i32))) as usize)].int()); __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v982); }
        { let __v983 = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v983); }
        { let __v984 = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v984); }
        p = self.get_avail();
        __av_mem[crate::ix::U((p) as usize)].set_hh_b1(c);
        __av_mem[crate::ix::U((p) as usize)].set_hh_b0(f);
        __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        char_box = b;
        char_box
    }

    /// When we build an extensible character, it's handy to have the
    /// following subroutine, which puts a given character on top
    /// of the characters already in box `b`:
    /// @<Declare subprocedures for `var_delimiter`
    // §888
    pub fn stack_into_box(&mut self, mut b: halfword, mut f: internal_font_number, mut c: quarterword) {
        let mut p: halfword = 0; // §888
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        p = self.char_box(f, c);
        { let __v985 = __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v985); }
        __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        { let __v986 = __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v986); }
    }

    /// Another handy subroutine computes the height plus depth of
    /// a given character:
    /// @<Declare subprocedures for `var_delimiter`
    // §889
    pub fn height_plus_depth(&mut self, mut f: internal_font_number, mut c: quarterword) -> scaled {
        let mut height_plus_depth: scaled = 0;
        let mut q: four_quarters = four_quarters::default(); // §889
        let mut hd: eight_bits = 0; // §889
        q = { let __s987 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s987)] }.qqqq();
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
    // §883
    pub fn var_delimiter(&mut self, mut d: halfword, mut s: small_number, mut v: scaled) -> halfword {
        let mut var_delimiter: halfword = 0;
        let mut b: halfword = 0; // §883
        let mut f: internal_font_number = 0; // §883
        let mut g: internal_font_number = 0; // §883
        let mut c: quarterword = 0; // §883
        let mut x: quarterword = 0; // §883
        let mut y: quarterword = 0; // §883
        let mut m: i32 = 0; // §883
        let mut n: i32 = 0; // §883
        let mut u: scaled = 0; // §883
        let mut w: scaled = 0; // §883
        let mut q: four_quarters = four_quarters::default(); // §883
        let mut hd: eight_bits = 0; // §883
        let mut r: four_quarters = four_quarters::default(); // §883
        let mut z: small_number = 0; // §883
        let mut large_attempt: bool = false; // §883
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_found_f: {
            f = null_font;
            w = 0i32;
            large_attempt = false;
            z = __av_mem[crate::ix::U((d) as usize)].qqqq().b0();
            x = __av_mem[crate::ix::U((d) as usize)].qqqq().b1();
            while true {
                {
                    // §884
                    if ((z != 0i32) || (x != min_quarterword)) {
                        {
                            z = ((z).wrapping_add(s)).wrapping_add(16i32);
                            loop {
                                z = (z).wrapping_sub(16i32);
                                g = __av_eqtb[crate::ix::U((((math_font_base).wrapping_add(z)) - 1) as usize)].hh().rh();
                                if (g != null_font) {
                                    // §885
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
                                if (z < 16i32) { break; }
                            }
                        }
                    }
                    // §883
                    if large_attempt {
                        break 'l_found_f;
                    }
                    large_attempt = true;
                    z = __av_mem[crate::ix::U((d) as usize)].qqqq().b2();
                    x = __av_mem[crate::ix::U((d) as usize)].qqqq().b3();
                }
            }
        }
        if (f != null_font) {
            // §887
            if ((q.b2() % 4i32) == ext_tag) {
                // §890
                {
                    b = self.new_null_box();
                    __av_mem[crate::ix::U((b) as usize)].set_hh_b0(vlist_node);
                    r = self.font_info[crate::ix::U(((self.exten_base[crate::ix::U((f) as usize)]).wrapping_add(q.b3())) as usize)].qqqq();
                    // §891
                    c = r.b3();
                    u = self.height_plus_depth(f, c);
                    w = 0i32;
                    q = { let __s988 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, c))) as usize; self.font_info[crate::ix::U(__s988)] }.qqqq();
                    { let __v989 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add((q.b2() / 4i32))) as usize)].int()); __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v989); }
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
                    // §890
                    c = r.b2();
                    if (c != min_quarterword) {
                        self.stack_into_box(b, f, c);
                    }
                    c = r.b3();
                    {
                        let __for_end_5 = n;
                        m = 1i32;
                        while m <= __for_end_5 {
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
                                let __for_end_7 = n;
                                m = 1i32;
                                while m <= __for_end_7 {
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
                    { let __v990 = (w).wrapping_sub(__av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v990); }
                }
            } else {
                // §887
                b = self.char_box(f, c);
            }
        } else {
            // §883
            {
                b = self.new_null_box();
                { let __v991 = __av_eqtb[crate::ix::U(((29914i32) - 1) as usize)].int(); __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v991); }
            }
        }
        { let __v992 = (self.half((__av_mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()).wrapping_sub(__av_mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(s)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); __av_mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int(__v992); }
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
    // §892
    pub fn rebox(&mut self, mut b: halfword, mut w: scaled) -> halfword {
        let mut rebox: halfword = 0;
        let mut p: halfword = 0; // §892
        let mut f: internal_font_number = 0; // §892
        let mut v: scaled = 0; // §892
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if ((__av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int() != w) && (__av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh() != null)) {
            {
                if (__av_mem[crate::ix::U((b) as usize)].hh().b0() == vlist_node) {
                    b = self.hpack(b, 0i32, additional);
                }
                p = __av_mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                if ((p >= self.hi_mem_min) && (__av_mem[crate::ix::U((p) as usize)].hh().rh() == null)) {
                    {
                        f = __av_mem[crate::ix::U((p) as usize)].hh().b0();
                        v = { let __s994 = ((self.width_base[crate::ix::U((f) as usize)]).wrapping_add({ let __s993 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, __av_mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s993)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s994)] }.int();
                        if (v != __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()) {
                            { let __v995 = self.new_kern((__av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()).wrapping_sub(v)); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v995); }
                        }
                    }
                }
                self.free_node(b, box_node_size);
                b = self.new_glue(ss_glue);
                __av_mem[crate::ix::U((b) as usize)].set_hh_rh(p);
                while (__av_mem[crate::ix::U((p) as usize)].hh().rh() != null) {
                    p = __av_mem[crate::ix::U((p) as usize)].hh().rh();
                }
                { let __v996 = self.new_glue(ss_glue); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v996); }
                rebox = self.hpack(b, w, exactly);
            }
        } else {
            {
                __av_mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(w);
                rebox = b;
            }
        }
        rebox
    }

    /// Here is a subroutine that creates a new glue specification from another
    /// one that is expressed in `\.{mu}', given the value of the math unit.
    // §893
    pub fn math_glue(&mut self, mut g: halfword, mut m: scaled) -> halfword {
        let mut math_glue: halfword = 0;
        let mut p: halfword = 0; // §893
        let mut n: i32 = 0; // §893
        let mut f: scaled = 0; // §893
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        n = self.x_over_n(m, 65536i32);
        f = self.remainder;
        if (f < 0i32) {
            {
                n = (n).wrapping_sub(1i32);
                f = (f).wrapping_add(65536i32);
            }
        }
        p = self.get_node(glue_spec_size);
        { let __v997 = { let __a998_0 = n; let __a998_1 = __av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(); let __a998_2 = self.xn_over_d(__av_mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a998_3 = 1073741823i32; self.mult_and_add(__a998_0, __a998_1, __a998_2, __a998_3) }; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v997); }
        { let __v999 = __av_mem[crate::ix::U((g) as usize)].hh().b0(); __av_mem[crate::ix::U((p) as usize)].set_hh_b0(__v999); }
        if (__av_mem[crate::ix::U((p) as usize)].hh().b0() == normal) {
            { let __v1000 = { let __a1001_0 = n; let __a1001_1 = __av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); let __a1001_2 = self.xn_over_d(__av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(), f, 65536i32); let __a1001_3 = 1073741823i32; self.mult_and_add(__a1001_0, __a1001_1, __a1001_2, __a1001_3) }; __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v1000); }
        } else {
            { let __v1002 = __av_mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v1002); }
        }
        { let __v1003 = __av_mem[crate::ix::U((g) as usize)].hh().b1(); __av_mem[crate::ix::U((p) as usize)].set_hh_b1(__v1003); }
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == normal) {
            { let __v1004 = { let __a1005_0 = n; let __a1005_1 = __av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); let __a1005_2 = self.xn_over_d(__av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(), f, 65536i32); let __a1005_3 = 1073741823i32; self.mult_and_add(__a1005_0, __a1005_1, __a1005_2, __a1005_3) }; __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v1004); }
        } else {
            { let __v1006 = __av_mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v1006); }
        }
        math_glue = p;
        math_glue
    }

    /// The `math_kern` subroutine removes `mu_glue` from a kern node, given
    /// the value of the math unit.
    // §894
    pub fn math_kern(&mut self, mut p: halfword, mut m: scaled) {
        let mut n: i32 = 0; // §894
        let mut f: scaled = 0; // §894
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (__av_mem[crate::ix::U((p) as usize)].hh().b1() == mu_glue) {
            {
                n = self.x_over_n(m, 65536i32);
                f = self.remainder;
                if (f < 0i32) {
                    {
                        n = (n).wrapping_sub(1i32);
                        f = (f).wrapping_add(65536i32);
                    }
                }
                { let __v1007 = { let __a1008_0 = n; let __a1008_1 = __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); let __a1008_2 = self.xn_over_d(__av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a1008_3 = 1073741823i32; self.mult_and_add(__a1008_0, __a1008_1, __a1008_2, __a1008_3) }; __av_mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v1007); }
                __av_mem[crate::ix::U((p) as usize)].set_hh_b1(explicit);
            }
        }
    }

    /// Sometimes it is necessary to destroy an mlist. The following
    /// subroutine empties the current list, assuming that `abs(mode)=mmode`.
    // §895
    pub fn flush_math(&mut self) {
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.flush_node_list(__av_mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh());
        self.flush_node_list(self.cur_list.aux_field.int());
        { let __ix1009 = self.cur_list.head_field; __av_mem[crate::ix::U((__ix1009) as usize)].set_hh_rh(null); }
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.aux_field.set_int(null);
    }

    /// The recursion in `mlist_to_hlist` is due primarily to a subroutine
    /// called `clean_box` that puts a given noad field into a box using a given
    /// math style; `mlist_to_hlist` can call `clean_box`, which can call
    /// `mlist_to_hlist`.
    /// The box returned by `clean_box` is ``clean'' in the
    /// sense that its `shift_amount` is zero.
    // §897
    pub fn clean_box(&mut self, mut p: halfword, mut s: small_number) -> halfword {
        let mut clean_box: halfword = 0;
        let mut q: halfword = 0; // §897
        let mut save_style: small_number = 0; // §897
        let mut x: halfword = 0; // §897
        let mut r: halfword = 0; // §897
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        'l_found_f: {
            match __av_mem[crate::ix::U((p) as usize)].hh().rh() {
                math_char => {
                    {
                        self.cur_mlist = self.new_noad();
                        { let __ix1010 = (self.cur_mlist).wrapping_add(1i32); let __v1011 = __av_mem[crate::ix::U((p) as usize)]; __av_mem[crate::ix::U((__ix1010) as usize)] = __v1011; }
                    }
                }
                sub_box => {
                    {
                        q = __av_mem[crate::ix::U((p) as usize)].hh().lh();
                        break 'l_found_f;
                    }
                }
                sub_mlist => {
                    self.cur_mlist = __av_mem[crate::ix::U((p) as usize)].hh().lh();
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
            q = __av_mem[crate::ix::U((temp_head) as usize)].hh().rh();
            self.cur_style = save_style;
            // §880
            {
                if (self.cur_style < script_style) {
                    self.cur_size = text_size;
                } else {
                    self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                }
                self.cur_mu = self.x_over_n(self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(), 18i32);
            }
        }
        // §897
        if ((q >= self.hi_mem_min) || (q == null)) {
            x = self.hpack(q, 0i32, additional);
        } else {
            if (((__av_mem[crate::ix::U((q) as usize)].hh().rh() == null) && (__av_mem[crate::ix::U((q) as usize)].hh().b0() <= vlist_node)) && (__av_mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].int() == 0i32)) {
                x = q;
            } else {
                x = self.hpack(q, 0i32, additional);
            }
        }
        // §898
        q = __av_mem[crate::ix::U(((x).wrapping_add(5i32)) as usize)].hh().rh();
        if (q >= self.hi_mem_min) {
            {
                r = __av_mem[crate::ix::U((q) as usize)].hh().rh();
                if (r != null) {
                    if (__av_mem[crate::ix::U((r) as usize)].hh().rh() == null) {
                        if (!(r >= self.hi_mem_min)) {
                            if (__av_mem[crate::ix::U((r) as usize)].hh().b0() == kern_node) {
                                {
                                    self.free_node(r, medium_node_size);
                                    __av_mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                                }
                            }
                        }
                    }
                }
            }
        }
        // §897
        clean_box = x;
        clean_box
    }

    /// It is convenient to have a procedure that converts a `math_char`
    /// field to an ``unpacked'' form. The `fetch` routine sets `cur_f`, `cur_c`,
    /// and `cur_i` to the font code, character code, and character information bytes of
    /// a given noad field. It also takes care of issuing error messages for
    /// nonexistent characters; in such cases, `char_exists(cur_i)` will be `false`
    /// after `fetch` has acted, and the field will also have been reset to `empty`.
    // §899
    pub fn fetch(&mut self, mut a: halfword) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.cur_c = __av_mem[crate::ix::U((a) as usize)].hh().b1();
        self.cur_f = __av_eqtb[crate::ix::U(((((math_font_base).wrapping_add(__av_mem[crate::ix::U((a) as usize)].hh().b0())).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if (self.cur_f == null_font) {
            // §900
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
                self.print_size(self.cur_size);
                self.print_char(32i32);
                self.print_int(((__av_mem[crate::ix::U((a) as usize)].hh().b0()) as i64));
                self.print(1307i32);
                self.print(self.cur_c);
                self.print_char(41i32);
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 1308i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1309i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1310i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1311i32;
                }
                self.error();
                self.cur_i = self.null_character;
                __av_mem[crate::ix::U((a) as usize)].set_hh_rh(empty);
            }
        } else {
            // §899
            {
                if ((self.cur_c >= self.font_bc[crate::ix::U((self.cur_f) as usize)]) && (self.cur_c <= self.font_ec[crate::ix::U((self.cur_f) as usize)])) {
                    self.cur_i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_c)) as usize)].qqqq();
                } else {
                    self.cur_i = self.null_character;
                }
                if (!(self.cur_i.b0() > min_quarterword)) {
                    {
                        self.char_warning(self.cur_f, self.cur_c);
                        __av_mem[crate::ix::U((a) as usize)].set_hh_rh(empty);
                        self.cur_i = self.null_character;
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
    // §911
    pub fn make_over(&mut self, mut q: halfword) {
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        { let __v1012 = { let __a1013_0 = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32)); let __a1013_1 = (3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); let __a1013_2 = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(); self.overbar(__a1013_0, __a1013_1, __a1013_2) }; __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1012); }
        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// @<Declare math...
    // §912
    pub fn make_under(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §912
        let mut x: halfword = 0; // §912
        let mut y: halfword = 0; // §912
        let mut delta: scaled = 0; // §912
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
        p = self.new_kern((3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()));
        __av_mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        { let __v1014 = self.fraction_rule(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v1014); }
        y = self.vpackage(x, 0i32, additional, max_dimen);
        delta = ((__av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
        { let __v1015 = __av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(__v1015); }
        { let __v1016 = (delta).wrapping_sub(__av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(__v1016); }
        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// @<Declare math...
    // §913
    pub fn make_vcenter(&mut self, mut q: halfword) {
        let mut v: halfword = 0; // §913
        let mut delta: scaled = 0; // §913
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        v = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
        if (__av_mem[crate::ix::U((v) as usize)].hh().b0() != vlist_node) {
            self.confusion(628i32);
        }
        delta = (__av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int());
        { let __v1017 = (self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(self.half(delta)); __av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v1017); }
        { let __v1018 = (delta).wrapping_sub(__av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v1018); }
    }

    /// According to the rules in the \.{DVI} file specifications, we ensure alignment
    /// between a square root sign and the rule above its nucleus by assuming that the
    /// baseline of the square-root symbol is the same as the bottom of the rule. The
    /// height of the square-root symbol will be the thickness of the rule, and the
    /// depth of the square-root symbol should exceed or equal the height-plus-depth
    /// of the nucleus plus a certain minimum clearance~`clr`. The symbol will be
    /// placed so that the actual clearance is `clr` plus half the excess.
    /// @<Declare math...
    // §914
    pub fn make_radical(&mut self, mut q: halfword) {
        let mut x: halfword = 0; // §914
        let mut y: halfword = 0; // §914
        let mut delta: scaled = 0; // §914
        let mut clr: scaled = 0; // §914
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
        if (self.cur_style < text_style) {
            clr = (self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(((self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_abs() / 4i32));
        } else {
            {
                clr = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                clr = (clr).wrapping_add(((clr).wrapping_abs() / 4i32));
            }
        }
        y = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, (((__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr)).wrapping_add(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()));
        delta = (__av_mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int()).wrapping_sub(((__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr));
        if (delta > 0i32) {
            clr = (clr).wrapping_add(self.half(delta));
        }
        { let __v1019 = ((__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(clr)).wrapping_neg(); __av_mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v1019); }
        { let __v1020 = self.overbar(x, clr, __av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U((y) as usize)].set_hh_rh(__v1020); }
        { let __v1021 = self.hpack(y, 0i32, additional); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v1021); }
        __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
    }

    /// Slants are not considered when placing accents in math mode. The accenter is
    /// centered over the accentee, and the accent width is treated as zero with
    /// respect to the size of the final box.
    /// @<Declare math...
    // §915
    pub fn make_math_accent(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §915
        let mut x: halfword = 0; // §915
        let mut y: halfword = 0; // §915
        let mut a: i32 = 0; // §915
        let mut c: quarterword = 0; // §915
        let mut f: internal_font_number = 0; // §915
        let mut i: four_quarters = four_quarters::default(); // §915
        let mut s: scaled = 0; // §915
        let mut h: scaled = 0; // §915
        let mut delta: scaled = 0; // §915
        let mut w: scaled = 0; // §915
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        self.fetch((q).wrapping_add(4i32));
        if (self.cur_i.b0() > min_quarterword) {
            {
                'l_done_f: {
                    'l_done1_f: {
                        i = self.cur_i;
                        c = self.cur_c;
                        f = self.cur_f;
                        // §918
                        s = 0i32;
                        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
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
                    // §915
                    x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
                    w = __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int();
                    h = __av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                    // §917
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
                // §915
                if (h < self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) {
                    delta = h;
                } else {
                    delta = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
                }
                if ((__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() != empty) || (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != empty)) {
                    if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
                        // §919
                        {
                            self.flush_node_list(x);
                            x = self.new_noad();
                            { let __v1022 = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]; __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)] = __v1022; }
                            { let __v1023 = __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)]; __av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)] = __v1023; }
                            { let __v1024 = __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)]; __av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)] = __v1024; }
                            { let __v1025 = self.empty_field; __av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh(__v1025); }
                            { let __v1026 = self.empty_field; __av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_hh(__v1026); }
                            __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_mlist);
                            __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
                            x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                            delta = ((delta).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_sub(h);
                            h = __av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                        }
                    }
                }
                // §915
                y = self.char_box(f, c);
                { let __v1027 = (s).wrapping_add(self.half((w).wrapping_sub(__av_mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int()))); __av_mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v1027); }
                __av_mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(0i32);
                p = self.new_kern((delta).wrapping_neg());
                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                __av_mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                y = self.vpackage(y, 0i32, additional, max_dimen);
                { let __v1028 = __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(__v1028); }
                if (__av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int() < h) {
                    // §916
                    {
                        p = self.new_kern((h).wrapping_sub(__av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()));
                        { let __v1029 = __av_mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh(); __av_mem[crate::ix::U((p) as usize)].set_hh_rh(__v1029); }
                        __av_mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                        __av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(h);
                    }
                }
                // §915
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
            }
        }
    }

    /// The `make_fraction` procedure is a bit different because it sets
    /// `new_hlist(q)` directly rather than making a sub-box.
    /// @<Declare math...
    // §920
    pub fn make_fraction(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §920
        let mut v: halfword = 0; // §920
        let mut x: halfword = 0; // §920
        let mut y: halfword = 0; // §920
        let mut z: halfword = 0; // §920
        let mut delta: scaled = 0; // §920
        let mut delta1: scaled = 0; // §920
        let mut delta2: scaled = 0; // §920
        let mut shift_up: scaled = 0; // §920
        let mut shift_down: scaled = 0; // §920
        let mut clr: scaled = 0; // §920
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == default_code) {
            { let __v1030 = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1030); }
        }
        // §921
        x = self.clean_box((q).wrapping_add(2i32), ((self.cur_style).wrapping_add(2i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        z = self.clean_box((q).wrapping_add(3i32), (((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(3i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        if (__av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() < __av_mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int()) {
            x = self.rebox(x, __av_mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int());
        } else {
            z = self.rebox(z, __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int());
        }
        if (self.cur_style < text_style) {
            {
                shift_up = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                shift_down = self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
            }
        } else {
            {
                shift_down = self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() != 0i32) {
                    shift_up = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                } else {
                    shift_up = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                }
            }
        }
        // §920
        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            // §922
            {
                if (self.cur_style < text_style) {
                    clr = (7i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                } else {
                    clr = (3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                }
                delta = self.half((clr).wrapping_sub(((shift_up).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down))));
                if (delta > 0i32) {
                    {
                        shift_up = (shift_up).wrapping_add(delta);
                        shift_down = (shift_down).wrapping_add(delta);
                    }
                }
            }
        } else {
            // §923
            {
                if (self.cur_style < text_style) {
                    clr = (3i32).wrapping_mul(__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                } else {
                    clr = __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                }
                delta = self.half(__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                delta1 = (clr).wrapping_sub(((shift_up).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(delta)));
                delta2 = (clr).wrapping_sub(((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(delta)).wrapping_sub((__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                if (delta1 > 0i32) {
                    shift_up = (shift_up).wrapping_add(delta1);
                }
                if (delta2 > 0i32) {
                    shift_down = (shift_down).wrapping_add(delta2);
                }
            }
        }
        // §924
        v = self.new_null_box();
        __av_mem[crate::ix::U((v) as usize)].set_hh_b0(vlist_node);
        { let __v1031 = (shift_up).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()); __av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v1031); }
        { let __v1032 = (__av_mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int()).wrapping_add(shift_down); __av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v1032); }
        { let __v1033 = __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v1033); }
        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            {
                p = self.new_kern(((shift_up).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(z);
            }
        } else {
            {
                y = self.fraction_rule(__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                p = self.new_kern(((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(delta)).wrapping_sub((__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                __av_mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                p = self.new_kern(((shift_up).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(delta)));
                __av_mem[crate::ix::U((p) as usize)].set_hh_rh(y);
            }
        }
        __av_mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        __av_mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(x);
        // §925
        if (self.cur_style < text_style) {
            delta = self.font_info[crate::ix::U(((20i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
        } else {
            delta = self.font_info[crate::ix::U(((21i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
        }
        x = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, delta);
        __av_mem[crate::ix::U((x) as usize)].set_hh_rh(v);
        z = self.var_delimiter((q).wrapping_add(5i32), self.cur_size, delta);
        __av_mem[crate::ix::U((v) as usize)].set_hh_rh(z);
        { let __v1034 = self.hpack(x, 0i32, additional); __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v1034); }
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
    // §926
    pub fn make_op(&mut self, mut q: halfword) -> scaled {
        let mut make_op: scaled = 0;
        let mut delta: scaled = 0; // §926
        let mut p: halfword = 0; // §926
        let mut v: halfword = 0; // §926
        let mut x: halfword = 0; // §926
        let mut y: halfword = 0; // §926
        let mut z: halfword = 0; // §926
        let mut c: quarterword = 0; // §926
        let mut i: four_quarters = four_quarters::default(); // §926
        let mut shift_up: scaled = 0; // §926
        let mut shift_down: scaled = 0; // §926
        #[allow(unused_mut)]
        let mut __av_eqtb = self.eqtb.view();
        #[allow(unused_mut)]
        let mut __av_mem = self.mem.view();
        if ((__av_mem[crate::ix::U((q) as usize)].hh().b1() == normal) && (self.cur_style < text_style)) {
            __av_mem[crate::ix::U((q) as usize)].set_hh_b1(limits);
        }
        if (__av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == math_char) {
            {
                self.fetch((q).wrapping_add(1i32));
                if ((self.cur_style < text_style) && ((self.cur_i.b2() % 4i32) == list_tag)) {
                    {
                        c = self.cur_i.b3();
                        i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(c)) as usize)].qqqq();
                        if (i.b0() > min_quarterword) {
                            {
                                self.cur_c = c;
                                self.cur_i = i;
                                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_b1(c);
                            }
                        }
                    }
                }
                delta = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((self.cur_i.b2() / 4i32))) as usize)].int();
                x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                if ((__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != empty) && (__av_mem[crate::ix::U((q) as usize)].hh().b1() != limits)) {
                    { let __v1035 = (__av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int()).wrapping_sub(delta); __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v1035); }
                }
                { let __v1036 = (self.half((__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27695i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); __av_mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v1036); }
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(sub_box);
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
            }
        } else {
            delta = 0i32;
        }
        if (__av_mem[crate::ix::U((q) as usize)].hh().b1() == limits) {
            // §927
            {
                x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                y = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                z = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                v = self.new_null_box();
                __av_mem[crate::ix::U((v) as usize)].set_hh_b0(vlist_node);
                { let __v1037 = __av_mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v1037); }
                if (__av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() > __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v1038 = __av_mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v1038); }
                }
                if (__av_mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int() > __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v1039 = __av_mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v1039); }
                }
                x = self.rebox(x, __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                y = self.rebox(y, __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                z = self.rebox(z, __av_mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                { let __v1040 = self.half(delta); __av_mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v1040); }
                { let __v1041 = (__av_mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].int()).wrapping_neg(); __av_mem[crate::ix::U(((z).wrapping_add(4i32)) as usize)].set_int(__v1041); }
                { let __v1042 = __av_mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v1042); }
                { let __v1043 = __av_mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int(); __av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v1043); }
                // §928
                if (__av_mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == empty) {
                    {
                        self.free_node(x, box_node_size);
                        __av_mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(y);
                    }
                } else {
                    {
                        shift_up = (self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int());
                        if (shift_up < self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()) {
                            shift_up = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                        }
                        p = self.new_kern(shift_up);
                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(y);
                        __av_mem[crate::ix::U((x) as usize)].set_hh_rh(p);
                        p = self.new_kern(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                        __av_mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                        { let __v1044 = ((((__av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_up); __av_mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v1044); }
                    }
                }
                if (__av_mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == empty) {
                    self.free_node(z, box_node_size);
                } else {
                    {
                        shift_down = (self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int());
                        if (shift_down < self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()) {
                            shift_down = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                        }
                        p = self.new_kern(shift_down);
                        __av_mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                        __av_mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                        p = self.new_kern(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                        __av_mem[crate::ix::U((z) as usize)].set_hh_rh(p);
                        { let __v1045 = ((((__av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((__av_eqtb[crate::ix::U((((27696i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int())).wrapping_add(__av_mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_down); __av_mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v1045); }
                    }
                }
                // §927
                __av_mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(v);
            }
        }
        // §926
        make_op = delta;
        make_op
    }

}
