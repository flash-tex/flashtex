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
    // §727
    pub fn pdf_print_toks(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §727
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
    // §727
    pub fn pdf_print_toks_ln(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §727
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
        self.flush_str(s);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §772
    pub fn pdf_write_obj(&mut self, mut n: i32) {
        let mut s: str_number = 0; // §772
        let mut f: crate::system::ByteFile = Default::default(); // §772
        s = self.tokens_to_string(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)]);
        {
            self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)]);
            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(0i32)) as usize)] = 0i32;
        }
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
            {
                self.pdf_begin_dict(n, 0i32);
                if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] != 0i32) {
                    {
                        self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]);
                        {
                            self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)]);
                            self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(2i32)) as usize)] = 0i32;
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
                self.cur_area = 348i32;
                self.cur_ext = 348i32;
                self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                if (!{ let mut __f0 = ::core::mem::take(&mut f); let __r = self.tex_b_openin(&mut __f0); f = __f0; __r }) {
                    {
                        self.print_nl(264i32);
                        self.print(s);
                        self.print(1164i32);
                        self.pdf_error(1165i32, 1166i32);
                    }
                }
                self.print(1069i32);
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
                                            self.overflow(1004i32, pdf_op_buf_size);
                                        } else {
                                            if ((!self.pdf_os_mode) && ((1i32).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                                self.pdf_flush();
                                            }
                                        }
                                    }
                                }
                                {
                                    { let __a730_0 = self.pdf_ptr; let __a730_1 = { let mut __f0 = ::core::mem::take(&mut f); let __r = self.getc(&mut __f0); f = __f0; __r }; self.pdf_buf_set(__a730_0, __a730_1) };
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
                self.print(1021i32);
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
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
            self.pdf_end_stream();
        } else {
            self.pdf_end_obj();
        }
        self.flush_str(s);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §772
    pub fn flush_whatsit_node(&mut self, mut p: halfword, mut s: small_number) {
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(8i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        if (self.mem[crate::ix::U((p) as usize)].hh().rh() != 0i32) {
            self.pdf_error(1167i32, 1168i32);
        }
        self.flush_node_list(p);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §778
    pub fn pdf_write_image(&mut self, mut n: i32) {
        self.pdf_begin_dict(n, 0i32);
        if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)] != 0i32) {
            {
                self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)]);
                {
                    self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)]);
                    self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(3i32)) as usize)] = 0i32;
                }
            }
        }
        if (self.fixed_pdf_draftmode == 0i32) {
            self.write_image(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(4i32)) as usize)]);
        }
        self.delete_image(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((n) as usize)].int4).wrapping_add(4i32)) as usize)]);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §785
    pub fn pdf_print_rect_spec(&mut self, mut r: halfword) {
        self.pdf_print_mag_bp((self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].int()));
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
        self.pdf_print_mag_bp((self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].int()));
    }

    /// When a destination is created we need to check whether another destination
    /// with the same identifier already exists and give a warning if needed.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1564
    pub fn warn_dest_dup(&mut self, mut id: i32, mut byname: small_number, mut s1: str_number, mut s2: str_number) {
        if (self.eqtb[crate::ix::U(((629107i32) - 1) as usize)].int() > 0i32) {
            return;
        }
        self.pdf_warning(s1, 1831i32, true, false);
        if (byname > 0i32) {
            {
                self.print(1815i32);
                self.print_mark(id);
            }
        } else {
            {
                self.print(1194i32);
                self.print_int(((id) as i64));
            }
        }
        self.print(1832i32);
        self.print(s2);
        self.print_ln();
        self.show_context();
    }

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1630
    pub fn write_action(&mut self, mut p: halfword) {
        let mut s: str_number = 0; // §1630
        let mut d: i32 = 0; // §1630
        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 3i32) {
            {
                self.pdf_print_toks_ln(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                return;
            }
        }
        self.pdf_print(1240i32);
        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
            {
                self.pdf_print(1919i32);
                s = self.tokens_to_string(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                if ((self.str_pool[crate::ix::U((self.str_start[crate::ix::U((s) as usize)]) as usize)] == 40i32) && (self.str_pool[crate::ix::U((((self.str_start[crate::ix::U((s) as usize)]).wrapping_add((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]))).wrapping_sub(1i32)) as usize)] == 41i32)) {
                    self.pdf_print(s);
                } else {
                    {
                        self.pdf_print_str(s);
                    }
                }
                self.flush_str(s);
                self.pdf_print(32i32);
                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() > 0i32) {
                    {
                        self.pdf_print(1920i32);
                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == 1i32) {
                            self.pdf_print(1921i32);
                        } else {
                            self.pdf_print(1922i32);
                        }
                    }
                }
            }
        }
        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
            0 => {
                {
                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                        {
                            self.pdf_print(1923i32);
                            { let __a731_0 = ((self.get_obj(1i32, self.mem[crate::ix::U((p) as usize)].hh().rh(), false)) as i64); self.pdf_print_int(__a731_0) };
                            self.pdf_print(1073i32);
                        }
                    } else {
                        {
                            self.pdf_print(1924i32);
                            self.pdf_print_int((((self.mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_sub(1i32)) as i64));
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
                            self.pdf_buf_set(self.pdf_ptr, 32i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                    { let __a732_0 = self.tokens_to_string(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()); self.pdf_print(__a732_0) };
                    self.flush_str(self.last_tokens_string);
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
                            self.pdf_buf_set(self.pdf_ptr, 93i32);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
            1 => {
                {
                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                        {
                            self.pdf_print(1925i32);
                            d = self.get_obj(5i32, self.mem[crate::ix::U((p) as usize)].hh().rh(), (((self.mem[crate::ix::U((p) as usize)].hh().b1() % 2i32)) != 0));
                        }
                    } else {
                        self.pdf_print(1926i32);
                    }
                    if ((self.mem[crate::ix::U((p) as usize)].hh().b1() % 2i32) == 1i32) {
                        {
                            { let __a733_0 = 68i32; let __a733_1 = self.tokens_to_string(self.mem[crate::ix::U((p) as usize)].hh().rh()); self.pdf_str_entry(__a733_0, __a733_1) };
                            self.flush_str(self.last_tokens_string);
                        }
                    } else {
                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                            self.pdf_indirect(68i32, d);
                        } else {
                            self.pdf_error(1845i32, 1820i32);
                        }
                    }
                }
            }
            2 => {
                {
                    self.pdf_print(1927i32);
                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                        d = self.get_obj(10i32, self.mem[crate::ix::U((p) as usize)].hh().rh(), (((self.mem[crate::ix::U((p) as usize)].hh().b1() % 2i32)) != 0));
                    }
                    if ((self.mem[crate::ix::U((p) as usize)].hh().b1() % 2i32) == 1i32) {
                        {
                            { let __a734_0 = 68i32; let __a734_1 = self.tokens_to_string(self.mem[crate::ix::U((p) as usize)].hh().rh()); self.pdf_str_entry(__a734_0, __a734_1) };
                            self.flush_str(self.last_tokens_string);
                        }
                    } else {
                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                            self.pdf_indirect(68i32, d);
                        } else {
                            self.pdf_int_entry(68i32, self.mem[crate::ix::U((p) as usize)].hh().rh());
                        }
                    }
                }
            }
            _ => {}
        }
        if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh() != 0i32) {
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
                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 0i32) {
                    { let __a735_0 = 1928i32; let __a735_1 = self.get_obj(6i32, self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh(), ((((self.mem[crate::ix::U((p) as usize)].hh().b1() / 2i32) % 2i32)) != 0)); self.pdf_indirect(__a735_0, __a735_1) };
                } else {
                    {
                        self.pdf_print(1929i32);
                        { let __a736_0 = self.tokens_to_string(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].hh().rh()); self.pdf_print(__a736_0) };
                        self.flush_str(self.last_tokens_string);
                    }
                }
            }
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

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1630
    pub fn set_rect_dimens(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled, mut w: scaled, mut h: scaled, mut d: scaled, mut margin: scaled) {
        { let __v737 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v737); }
        if (w == (1073741824i32).wrapping_neg()) {
            { let __v738 = (x).wrapping_add(self.mem[crate::ix::U(((parent_box).wrapping_add(1i32)) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v738); }
        } else {
            { let __v739 = (self.cur_h).wrapping_add(w); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v739); }
        }
        if (h == (1073741824i32).wrapping_neg()) {
            { let __v740 = (y).wrapping_sub(self.mem[crate::ix::U(((parent_box).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v740); }
        } else {
            { let __v741 = (self.cur_v).wrapping_sub(h); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v741); }
        }
        if (d == (1073741824i32).wrapping_neg()) {
            { let __v742 = (y).wrapping_add(self.mem[crate::ix::U(((parent_box).wrapping_add(2i32)) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v742); }
        } else {
            { let __v743 = (self.cur_v).wrapping_add(d); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v743); }
        }
        if (self.is_shipping_page && self.matrixused()) {
            {
                self.matrixtransformrect(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), (self.cur_page_height).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int()), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), (self.cur_page_height).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()));
                { let __v744 = self.getllx(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v744); }
                { let __v745 = (self.cur_page_height).wrapping_sub(self.getlly()); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v745); }
                { let __v746 = self.geturx(); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v746); }
                { let __v747 = (self.cur_page_height).wrapping_sub(self.getury()); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v747); }
            }
        }
        { let __v748 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_sub(margin); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v748); }
        { let __v749 = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(margin); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v749); }
        { let __v750 = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(margin); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v750); }
        { let __v751 = (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int()).wrapping_add(margin); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v751); }
    }

    /// The following procedures are needed for outputting whatsit nodes for
    /// \pdfTeX{}.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1630
    pub fn do_annot(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        if (!self.is_shipping_page) {
            self.pdf_error(1845i32, 1930i32);
        }
        if self.doing_leaders {
            return;
        }
        if (self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 > (((2i32).wrapping_neg()) as i64)) {
            { let __v752 = self.pdf_new_objnum(); self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(__v752); }
        }
        self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), 0i32);
        self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int();
            self.pdf_annot_list = self.append_ptr(self.pdf_annot_list, self.pdf_append_list_arg);
        }
        if (self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 == (((2i32).wrapping_neg()) as i64)) {
            self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 = (((1i32).wrapping_neg()) as i64);
        }
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1635
    pub fn push_link_level(&mut self, mut p: halfword) {
        if (self.pdf_link_stack_ptr >= pdf_max_link_level) {
            self.overflow(1931i32, pdf_max_link_level);
        }
        self.pdfassert(((self.mem[crate::ix::U((p) as usize)].hh().b0() == 8i32) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 16i32)));
        self.pdf_link_stack_ptr = (self.pdf_link_stack_ptr).wrapping_add(1i32);
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].nesting_level = self.cur_s;
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node = self.copy_node_list(p);
        self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].ref_link_node = p;
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1635
    pub fn pop_link_level(&mut self) {
        self.pdfassert((self.pdf_link_stack_ptr > 0i32));
        self.flush_node_list(self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node);
        self.pdf_link_stack_ptr = (self.pdf_link_stack_ptr).wrapping_sub(1i32);
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1635
    pub fn do_link(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        if (!self.is_shipping_page) {
            self.pdf_error(1845i32, 1932i32);
        }
        self.pdfassert((self.mem[crate::ix::U((parent_box) as usize)].hh().b0() == 0i32));
        if (self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 > (((2i32).wrapping_neg()) as i64)) {
            { let __v753 = self.pdf_new_objnum(); self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_int(__v753); }
        }
        self.push_link_level(p);
        self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int());
        self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int();
            self.pdf_link_list = self.append_ptr(self.pdf_link_list, self.pdf_append_list_arg);
        }
        if (self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 == (((2i32).wrapping_neg()) as i64)) {
            self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()) as usize)].int2 = (((1i32).wrapping_neg()) as i64);
        }
    }

    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1635
    pub fn end_link(&mut self) {
        let mut p: halfword = 0; // §1635
        if (self.pdf_link_stack_ptr < 1i32) {
            self.pdf_error(1845i32, 1933i32);
        }
        if (self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].nesting_level != self.cur_s) {
            self.pdf_warning(0i32, 1934i32, true, true);
        }
        if (self.mem[crate::ix::U(((self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].link_node).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()) {
            {
                p = self.pdf_link_stack[crate::ix::U(((self.pdf_link_stack_ptr) - 1) as usize)].ref_link_node;
                if (self.is_shipping_page && self.matrixused()) {
                    {
                        self.matrixrecalculate((self.cur_h).wrapping_add(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()));
                        { let __v754 = (self.getllx()).wrapping_sub(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v754); }
                        { let __v755 = ((self.cur_page_height).wrapping_sub(self.getury())).wrapping_sub(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v755); }
                        { let __v756 = (self.geturx()).wrapping_add(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v756); }
                        { let __v757 = ((self.cur_page_height).wrapping_sub(self.getlly())).wrapping_add(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_int(__v757); }
                    }
                } else {
                    { let __v758 = (self.cur_h).wrapping_add(self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int()); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v758); }
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
    // §1636
    pub fn append_link(&mut self, mut parent_box: halfword, mut x: scaled, mut y: scaled, mut i: small_number) {
        let mut p: halfword = 0; // §1636
        self.pdfassert((self.mem[crate::ix::U((parent_box) as usize)].hh().b0() == 0i32));
        p = self.copy_node_list(self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].link_node);
        self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].ref_link_node = p;
        self.mem[crate::ix::U((p) as usize)].set_hh_lh(268435455i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(0i32);
        self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629665i32) - 1) as usize)].int());
        self.pdf_create_obj(0i32, 0i32);
        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = self.obj_ptr;
            self.pdf_link_list = self.append_ptr(self.pdf_link_list, self.pdf_append_list_arg);
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn append_bead(&mut self, mut p: halfword) {
        let mut a: i32 = 0; // §1637
        let mut b: i32 = 0; // §1637
        let mut c: i32 = 0; // §1637
        let mut t: i32 = 0; // §1637
        if (!self.is_shipping_page) {
            self.pdf_error(1845i32, 1935i32);
        }
        t = self.get_obj(10i32, self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        b = self.pdf_new_objnum();
        self.obj_tab[crate::ix::U((b) as usize)].int4 = self.pdf_get_mem(5i32);
        { let __ix759 = (self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(1i32); let __v760 = self.pdf_last_page; self.pdf_mem[crate::ix::U((__ix759) as usize)] = __v760; }
        self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((b) as usize)].int4) as usize)] = p;
        if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != 0i32) {
            { let __ix761 = (self.obj_tab[crate::ix::U((b) as usize)].int4).wrapping_add(4i32); let __v762 = self.tokens_to_string(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh()); self.pdf_mem[crate::ix::U((__ix761) as usize)] = __v762; }
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
    // §1637
    pub fn do_thread(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        if self.doing_leaders {
            return;
        }
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 21i32) {
            {
                self.pdf_thread_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                self.pdf_thread_ht = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                self.pdf_thread_dp = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                self.pdf_last_thread_id = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                self.pdf_last_thread_named_id = (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32);
                if self.pdf_last_thread_named_id {
                    { let __ix763 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v764 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix763) as usize)].set_hh_lh(__v764); }
                }
                self.pdf_thread_level = self.cur_s;
            }
        }
        self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629667i32) - 1) as usize)].int());
        self.append_bead(p);
        self.last_thread = p;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn append_thread(&mut self, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        let mut p: halfword = 0; // §1637
        p = self.get_node(7i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_lh(268435455i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(0i32);
        { let __v765 = self.pdf_thread_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v765); }
        { let __v766 = self.pdf_thread_ht; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v766); }
        { let __v767 = self.pdf_thread_dp; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v767); }
        self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].set_hh_lh(0i32);
        { let __v768 = self.pdf_last_thread_id; self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_rh(__v768); }
        if self.pdf_last_thread_named_id {
            {
                { let __ix769 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v770 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix769) as usize)].set_hh_lh(__v770); }
                self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b1(1i32);
            }
        } else {
            self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_hh_b1(0i32);
        }
        self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629667i32) - 1) as usize)].int());
        self.append_bead(p);
        self.last_thread = p;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn end_thread(&mut self) {
        if (self.pdf_thread_level != self.cur_s) {
            self.pdf_error(1845i32, 1936i32);
        }
        if ((self.pdf_thread_dp == (1073741824i32).wrapping_neg()) && (self.last_thread != 0i32)) {
            { let __ix771 = (self.last_thread).wrapping_add(4i32); let __v772 = (self.cur_v).wrapping_add(self.eqtb[crate::ix::U(((629667i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix771) as usize)].set_int(__v772); }
        }
        if self.pdf_last_thread_named_id {
            self.delete_token_ref(self.pdf_last_thread_id);
        }
        self.last_thread = 0i32;
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn open_subentries(&mut self, mut p: halfword) -> i32 {
        let mut open_subentries: i32 = 0;
        let mut k: i32 = 0; // §1637
        let mut c: i32 = 0; // §1637
        let mut l: i32 = 0; // §1637
        let mut r: i32 = 0; // §1637
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
    // §1637
    pub fn do_dest(&mut self, mut p: halfword, mut parent_box: halfword, mut x: scaled, mut y: scaled) {
        let mut k: i32 = 0; // §1637
        if (!self.is_shipping_page) {
            self.pdf_error(1845i32, 1937i32);
        }
        if self.doing_leaders {
            return;
        }
        if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().rh() == 0i32) {
            k = self.get_obj(5i32, self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        } else {
            k = self.get_obj(6i32, self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), ((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1()) != 0));
        }
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != 0i32) {
            {
                self.warn_dest_dup(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 1845i32, 1846i32);
                return;
            }
        }
        self.obj_tab[crate::ix::U((k) as usize)].int4 = p;
        {
            self.pdf_append_list_arg = k;
            self.pdf_dest_list = self.append_ptr(self.pdf_dest_list, self.pdf_append_list_arg);
        }
        match self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() {
            0 => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629666i32) - 1) as usize)].int());
                } else {
                    {
                        { let __v773 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v773); }
                        { let __v774 = self.cur_v; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v774); }
                    }
                }
            }
            2 | 5 => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629666i32) - 1) as usize)].int());
                } else {
                    { let __v775 = self.cur_v; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v775); }
                }
            }
            3 | 6 => {
                if self.matrixused() {
                    self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629666i32) - 1) as usize)].int());
                } else {
                    { let __v776 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v776); }
                }
            }
            1 | 4 => {
            }
            7 => {
                self.set_rect_dimens(p, parent_box, x, y, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.eqtb[crate::ix::U(((629666i32) - 1) as usize)].int());
            }
            _ => {}
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn out_form(&mut self, mut p: halfword) {
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
        if (self.pdf_lookup_list(self.pdf_xform_list, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) == 0i32) {
            {
                self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh();
                self.pdf_xform_list = self.append_ptr(self.pdf_xform_list, self.pdf_append_list_arg);
            }
        }
        self.cur_v = (self.cur_v).wrapping_add(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)]);
        self.pdf_print(1032i32);
        self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
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
        {
            self.pdf_print(1033i32);
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
        self.pdf_print(1148i32);
        self.pdf_print_int(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int0) as i64));
        if (self.pdf_resname_prefix != 0i32) {
            self.pdf_print(self.pdf_resname_prefix);
        }
        {
            self.pdf_print(1938i32);
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

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn out_image(&mut self, mut p: halfword) {
        let mut image: i32 = 0; // §1637
        let mut groupref: i32 = 0; // §1637
        let mut img_w: i32 = 0; // §1637
        let mut img_h: i32 = 0; // §1637
        image = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(4i32)) as usize)];
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
        if (self.pdf_lookup_list(self.pdf_ximage_list, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) == 0i32) {
            {
                self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh();
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
                { let __a777_0 = self.ext_xn_over_d(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.ten_pow[crate::ix::U((6i32) as usize)], self.one_hundred_bp); let __a777_1 = 4i32; self.pdf_print_real(__a777_0, __a777_1) };
                self.pdf_print(1143i32);
                { let __a778_0 = self.ext_xn_over_d((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()), self.ten_pow[crate::ix::U((6i32) as usize)], self.one_hundred_bp); let __a778_1 = 4i32; self.pdf_print_real(__a778_0, __a778_1) };
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
                self.pdf_print_bp((self.cur_h).wrapping_sub(self.pdf_origin_h));
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
                { let __a779_0 = self.ext_xn_over_d(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), self.ten_pow[crate::ix::U((6i32) as usize)], img_w); let __a779_1 = 6i32; self.pdf_print_real(__a779_0, __a779_1) };
                self.pdf_print(1143i32);
                { let __a780_0 = self.ext_xn_over_d((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()), self.ten_pow[crate::ix::U((6i32) as usize)], img_h); let __a780_1 = 6i32; self.pdf_print_real(__a780_0, __a780_1) };
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
                { let __a781_0 = ((self.cur_h).wrapping_sub(self.pdf_origin_h)).wrapping_sub({ let __a782_0 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); let __a782_1 = self.epdf_orig_x(image); let __a782_2 = img_w; self.ext_xn_over_d(__a782_0, __a782_1, __a782_2) }); self.pdf_print_bp(__a781_0) };
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
                { let __a783_0 = ((self.pdf_origin_v).wrapping_sub(self.cur_v)).wrapping_sub({ let __a784_0 = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()); let __a784_1 = self.epdf_orig_y(image); let __a784_2 = img_h; self.ext_xn_over_d(__a784_0, __a784_1, __a784_2) }); self.pdf_print_bp(__a783_0) };
            }
        }
        {
            self.pdf_print(1033i32);
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
        self.pdf_print(1149i32);
        self.pdf_print_int(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int0) as i64));
        if (self.pdf_resname_prefix != 0i32) {
            self.pdf_print(self.pdf_resname_prefix);
        }
        {
            self.pdf_print(1938i32);
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

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn gap_amount(&mut self, mut p: halfword, mut cur_pos: scaled) -> scaled {
        let mut gap_amount: scaled = 0;
        let mut snap_unit: scaled = 0; // §1637
        let mut stretch_amount: scaled = 0; // §1637
        let mut shrink_amount: scaled = 0; // §1637
        let mut last_pos: scaled = 0; // §1637
        let mut next_pos: scaled = 0; // §1637
        let mut g: scaled = 0; // §1637
        let mut g2: scaled = 0; // §1637
        snap_unit = self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(1i32)) as usize)].int();
        if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b0() > 0i32) {
            stretch_amount = 1073741823i32;
        } else {
            stretch_amount = self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(2i32)) as usize)].int();
        }
        if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().b1() > 0i32) {
            shrink_amount = 1073741823i32;
        } else {
            shrink_amount = self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()).wrapping_add(3i32)) as usize)].int();
        }
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 37i32) {
            last_pos = (self.pdf_snapy_refpos).wrapping_add((snap_unit).wrapping_mul(((cur_pos).wrapping_sub(self.pdf_snapy_refpos) / snap_unit)));
        } else {
            self.pdf_error(1939i32, 1940i32);
        }
        next_pos = (last_pos).wrapping_add(snap_unit);
        g = 1073741823i32;
        g2 = 1073741823i32;
        gap_amount = 0i32;
        if ((cur_pos).wrapping_sub(last_pos) < shrink_amount) {
            g = (cur_pos).wrapping_sub(last_pos);
        }
        if ((next_pos).wrapping_sub(cur_pos) < stretch_amount) {
            g2 = (next_pos).wrapping_sub(cur_pos);
        }
        if ((g == 1073741823i32) && (g2 == 1073741823i32)) {
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
    // §1637
    pub fn get_vpos(&mut self, mut p: halfword, mut q: halfword, mut b: halfword) -> halfword {
        let mut get_vpos: halfword = 0;
        let mut tmp_v: scaled = 0; // §1637
        let mut g_order: glue_ord = 0; // §1637
        let mut g_sign: i32 = 0; // §1637
        let mut glue_temp: f64 = 0.0; // §1637
        let mut cur_glue: f64 = 0.0; // §1637
        let mut cur_g: scaled = 0; // §1637
        let mut this_box: halfword = 0; // §1637
        tmp_v = self.cur_v;
        this_box = b;
        cur_g = 0i32;
        cur_glue = 0.0f64;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        while ((p != q) && (p != 0i32)) {
            {
                if (p >= self.hi_mem_min) {
                    self.confusion(1949i32);
                } else {
                    {
                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                            0 | 1 | 2 => {
                                tmp_v = ((tmp_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                            }
                            8 => {
                                if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == 12i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == 14i32)) {
                                    tmp_v = ((tmp_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                }
                            }
                            10 => {
                                {
                                    // §1638
                                    {
                                        self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                        self.rule_ht = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                        if (g_sign != 0i32) {
                                            {
                                                if (g_sign == 1i32) {
                                                    {
                                                        if (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                            {
                                                                cur_glue = (cur_glue + ((self.mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                                    if (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                        {
                                                            cur_glue = (cur_glue - ((self.mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                            glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                    // §1637
                                    tmp_v = (tmp_v).wrapping_add(self.rule_ht);
                                }
                            }
                            11 => {
                                tmp_v = (tmp_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                            }
                            _ => {
                            }
                        }
                    }
                }
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        get_vpos = tmp_v;
        get_vpos
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn do_snapy_comp(&mut self, mut p: halfword, mut b: halfword) {
        let mut q: halfword = 0; // §1637
        let mut tmp_v: scaled = 0; // §1637
        let mut g: scaled = 0; // §1637
        let mut g2: scaled = 0; // §1637
        if (!(((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == 8i32)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 38i32))) {
            self.pdf_error(1939i32, 1950i32);
        }
        q = p;
        while (q != 0i32) {
            {
                if (((!(q >= self.hi_mem_min)) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == 8i32)) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == 37i32)) {
                    {
                        tmp_v = self.get_vpos(p, q, b);
                        g = self.gap_amount(q, tmp_v);
                        g2 = self.round_xn_over_d(g, self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), 1000i32);
                        self.cur_v = (self.cur_v).wrapping_add(g2);
                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int((g).wrapping_sub(g2));
                        if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                            self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(1i32);
                        }
                        return;
                    }
                }
                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
            }
        }
    }

    /// Threads are handled in similar way as link annotations.
    /// @<Declare procedures needed in `pdf_hlist_out`, `pdf_vlist_out`
    // §1637
    pub fn do_snapy(&mut self, mut p: halfword) {
        self.count_do_snapy = (self.count_do_snapy).wrapping_add(1i32);
        if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() != 0i32) {
            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
        } else {
            self.cur_v = (self.cur_v).wrapping_add(self.gap_amount(p, self.cur_v));
        }
    }

    /// The implementation of procedure `pdf_hlist_out` is similar to `hlist_out`.
    // §729
    pub fn pdf_hlist_out(&mut self) {
        let mut base_line: scaled = 0; // §729
        let mut left_edge: scaled = 0; // §729
        let mut save_h: scaled = 0; // §729
        let mut this_box: halfword = 0; // §729
        let mut g_order: glue_ord = 0; // §729
        let mut g_sign: i32 = 0; // §729
        let mut p: halfword = 0; // §729
        let mut leader_box: halfword = 0; // §729
        let mut leader_wd: scaled = 0; // §729
        let mut lx: scaled = 0; // §729
        let mut outer_doing_leaders: bool = false; // §729
        let mut edge: scaled = 0; // §729
        let mut prev_p: halfword = 0; // §729
        let mut glue_temp: f64 = 0.0; // §729
        let mut cur_glue: f64 = 0.0; // §729
        let mut cur_g: scaled = 0; // §729
        let mut i: small_number = 0; // §729
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        base_line = self.cur_v;
        prev_p = (this_box).wrapping_add(5i32);
        // §1714
        if (self.eTeX_mode == 1i32) {
            {
                // §1710
                {
                    self.temp_ptr = self.get_avail();
                    { let __ix785 = self.temp_ptr; self.mem[crate::ix::U((__ix785) as usize)].set_hh_lh(0i32); }
                    { let __ix786 = self.temp_ptr; let __v787 = self.LR_ptr; self.mem[crate::ix::U((__ix786) as usize)].set_hh_rh(__v787); }
                    self.LR_ptr = self.temp_ptr;
                }
                // §1714
                if ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) == 2i32) {
                    if (self.cur_dir == 1i32) {
                        {
                            self.cur_dir = 0i32;
                            self.cur_h = (self.cur_h).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(0i32);
                    }
                }
                if ((self.cur_dir == 1i32) && ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) != 1i32)) {
                    // §1721
                    {
                        save_h = self.cur_h;
                        self.temp_ptr = p;
                        p = self.new_kern(0i32);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        self.cur_h = 0i32;
                        { let __v788 = { let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, 0i32, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v788); }
                        { let __v789 = (self.cur_h).wrapping_neg(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v789); }
                        self.cur_h = save_h;
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(1i32);
                    }
                }
            }
        }
        // §729
        left_edge = self.cur_h;
        // §730
        {
            let __for_end_2 = self.pdf_link_stack_ptr;
            i = 1i32;
            while i <= __for_end_2 {
                {
                    self.pdfassert((self.mem[crate::ix::U(((self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].link_node).wrapping_add(1i32)) as usize)].int() == (1073741824i32).wrapping_neg()));
                    if ((self.pdf_link_stack[crate::ix::U(((i) - 1) as usize)].nesting_level == self.cur_s) && self.gen_running_link) {
                        self.append_link(this_box, left_edge, base_line, i);
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        // §729
        while (p != 0i32) {
            'l_reswitch_b: loop {
                // §731
                if (p >= self.hi_mem_min) {
                    {
                        loop {
                            self.dl_node(p);
                            self.f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                            self.c = self.mem[crate::ix::U((p) as usize)].hh().b1();
                            if (((self.font_bc[crate::ix::U((self.f) as usize)] <= self.c) && (self.c <= self.font_ec[crate::ix::U((self.f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0() > 0i32)) {
                                {
                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == 0i32) {
                                        self.do_vf(self.f);
                                    }
                                    if (self.pdf_font_type[crate::ix::U((self.f) as usize)] == 1i32) {
                                        self.do_vf_packet(self.f, self.c);
                                    } else {
                                        {
                                            self.pdf_begin_string(self.f);
                                            self.pdf_print_char(self.f, self.c);
                                            self.adv_char_width(self.f, self.c);
                                        }
                                    }
                                }
                            } else {
                                self.char_warning(self.f, self.c);
                            }
                            self.cur_h = (self.cur_h).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0())) as usize)].int());
                            prev_p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            if (!(p >= self.hi_mem_min)) { break; }
                        }
                    }
                } else {
                    // §732
                    {
                        'l_L15_f: {
                            'l_L13_f: {
                                'l_L14_f: {
                                    self.dl_node(p);
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        0 | 1 => {
                                            // §733
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == 0i32) {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            } else {
                                                {
                                                    self.cur_v = (base_line).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    self.temp_ptr = p;
                                                    edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    if (self.cur_dir == 1i32) {
                                                        self.cur_h = edge;
                                                    }
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 1i32) {
                                                        self.pdf_vlist_out();
                                                    } else {
                                                        self.pdf_hlist_out();
                                                    }
                                                    self.cur_h = edge;
                                                    self.cur_v = base_line;
                                                }
                                            }
                                        }
                                        2 => {
                                            // §732
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        8 => {
                                            // §1645
                                            match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                                7 | 8 => {
                                                    self.pdf_out_literal(p);
                                                }
                                                40 => {
                                                    self.pdf_out_colorstack(p);
                                                }
                                                41 => {
                                                    self.pdf_out_setmatrix(p);
                                                }
                                                42 => {
                                                    self.pdf_out_save(p);
                                                }
                                                43 => {
                                                    self.pdf_out_restore(p);
                                                }
                                                10 => {
                                                    {
                                                        self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                        self.pdf_obj_list = self.append_ptr(self.pdf_obj_list, self.pdf_append_list_arg);
                                                    }
                                                }
                                                12 => {
                                                    // §1647
                                                    {
                                                        self.cur_v = base_line;
                                                        edge = self.cur_h;
                                                        self.out_form(p);
                                                        self.cur_h = (edge).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                        self.cur_v = base_line;
                                                    }
                                                }
                                                14 => {
                                                    // §1646
                                                    {
                                                        self.cur_v = (base_line).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        edge = self.cur_h;
                                                        self.out_image(p);
                                                        self.cur_h = (edge).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                        self.cur_v = base_line;
                                                    }
                                                }
                                                15 => {
                                                    // §1645
                                                    self.do_annot(p, this_box, left_edge, base_line);
                                                }
                                                16 => {
                                                    self.do_link(p, this_box, left_edge, base_line);
                                                }
                                                17 => {
                                                    self.end_link();
                                                }
                                                19 => {
                                                    self.do_dest(p, this_box, left_edge, base_line);
                                                }
                                                20 => {
                                                    self.do_thread(p, this_box, left_edge, base_line);
                                                }
                                                21 => {
                                                    self.pdf_error(1845i32, 1961i32);
                                                }
                                                22 => {
                                                    self.pdf_error(1845i32, 1962i32);
                                                }
                                                23 => {
                                                    // §1641
                                                    {
                                                        self.pdf_last_x_pos = self.cur_h;
                                                        if self.is_shipping_page {
                                                            self.pdf_last_y_pos = (self.cur_page_height).wrapping_sub(self.cur_v);
                                                        } else {
                                                            self.pdf_last_y_pos = ((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth)).wrapping_sub(self.cur_v);
                                                        }
                                                    }
                                                }
                                                3 | 4 => {
                                                    // §1645
                                                    self.pdf_special(p);
                                                }
                                                36 => {
                                                    // §1642
                                                    {
                                                        self.pdf_snapx_refpos = self.cur_h;
                                                        self.pdf_snapy_refpos = self.cur_v;
                                                    }
                                                }
                                                38 | 37 => {
                                                    // §1645
                                                }
                                                45 => {
                                                    self.gen_faked_interword_space = true;
                                                }
                                                46 => {
                                                    self.gen_faked_interword_space = false;
                                                }
                                                47 => {
                                                    self.pdf_insert_fake_space();
                                                }
                                                48 => {
                                                    self.gen_running_link = false;
                                                }
                                                49 => {
                                                    self.gen_running_link = true;
                                                }
                                                _ => {
                                                    self.out_what(p);
                                                }
                                            }
                                        }
                                        10 => {
                                            // §735
                                            {
                                                self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_wd = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != 0i32) {
                                                    {
                                                        if (g_sign == 1i32) {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((self.mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                        glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                                            if (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((self.mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                                    glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                                    // §1699
                                                    if (((g_sign == 1i32) && (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == 2i32) && (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().rh() == 0i32) {
                                                                    self.free_node(self.g, 4i32);
                                                                } else {
                                                                    { let __ix790 = self.g; let __v791 = (self.mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix790) as usize)].set_hh_rh(__v791); }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() < 100i32) {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(11i32);
                                                                    { let __v792 = self.rule_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v792); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(4i32);
                                                                    { let __ix793 = self.g; self.mem[crate::ix::U((__ix793) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix794 = self.g; self.mem[crate::ix::U((__ix794) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix795 = (self.g).wrapping_add(1i32); let __v796 = self.rule_wd; self.mem[crate::ix::U((__ix795) as usize)].set_int(__v796); }
                                                                    { let __ix797 = (self.g).wrapping_add(2i32); self.mem[crate::ix::U((__ix797) as usize)].set_int(0i32); }
                                                                    { let __ix798 = (self.g).wrapping_add(3i32); self.mem[crate::ix::U((__ix798) as usize)].set_int(0i32); }
                                                                    { let __v799 = self.g; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v799); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // §735
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= 100i32) {
                                                    // §736
                                                    {
                                                        leader_box = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == 2i32) {
                                                            {
                                                                self.rule_ht = self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int();
                                                                self.rule_dp = self.mem[crate::ix::U(((leader_box).wrapping_add(2i32)) as usize)].int();
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_wd = self.mem[crate::ix::U(((leader_box).wrapping_add(1i32)) as usize)].int();
                                                        if ((leader_wd > 0i32) && (self.rule_wd > 0i32)) {
                                                            {
                                                                self.rule_wd = (self.rule_wd).wrapping_add(10i32);
                                                                if (self.cur_dir == 1i32) {
                                                                    self.cur_h = (self.cur_h).wrapping_sub(10i32);
                                                                }
                                                                edge = (self.cur_h).wrapping_add(self.rule_wd);
                                                                lx = 0i32;
                                                                // §655
                                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 100i32) {
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.cur_h = (left_edge).wrapping_add((leader_wd).wrapping_mul(((self.cur_h).wrapping_sub(left_edge) / leader_wd)));
                                                                        if (self.cur_h < save_h) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_wd / leader_wd);
                                                                        self.lr = (self.rule_wd % leader_wd);
                                                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 101i32) {
                                                                            self.cur_h = (self.cur_h).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_h = (self.cur_h).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §736
                                                                while ((self.cur_h).wrapping_add(leader_wd) <= edge) {
                                                                    // §737
                                                                    {
                                                                        self.cur_v = (base_line).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = leader_box;
                                                                        if (self.cur_dir == 1i32) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == 1i32) {
                                                                            self.pdf_vlist_out();
                                                                        } else {
                                                                            self.pdf_hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.cur_v = base_line;
                                                                        self.cur_h = ((save_h).wrapping_add(leader_wd)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §736
                                                                if (self.cur_dir == 1i32) {
                                                                    self.cur_h = edge;
                                                                } else {
                                                                    self.cur_h = (edge).wrapping_sub(10i32);
                                                                }
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §735
                                                break 'l_L13_f;
                                            }
                                        }
                                        40 | 11 => {
                                            // §732
                                            self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        9 => {
                                            // §1716
                                            {
                                                if (self.eTeX_mode == 1i32) {
                                                    // §1717
                                                    {
                                                        if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                            if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((4i32).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / 4i32))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = self.LR_ptr;
                                                                    self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix800 = self.temp_ptr; let __v801 = self.avail; self.mem[crate::ix::U((__ix800) as usize)].set_hh_rh(__v801); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() > 4i32) {
                                                                        self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    self.temp_ptr = self.get_avail();
                                                                    { let __ix802 = self.temp_ptr; let __v803 = ((4i32).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / 4i32))).wrapping_add(3i32); self.mem[crate::ix::U((__ix802) as usize)].set_hh_lh(__v803); }
                                                                    { let __ix804 = self.temp_ptr; let __v805 = self.LR_ptr; self.mem[crate::ix::U((__ix804) as usize)].set_hh_rh(__v805); }
                                                                    self.LR_ptr = self.temp_ptr;
                                                                }
                                                                if ((self.mem[crate::ix::U((p) as usize)].hh().b1() / 8i32) != self.cur_dir) {
                                                                    // §1722
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                        self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                                        self.free_node(p, 2i32);
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        p = self.new_edge(self.cur_dir, self.rule_wd);
                                                                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                                                                        self.cur_h = ((self.cur_h).wrapping_sub(left_edge)).wrapping_add(self.rule_wd);
                                                                        { let __v806 = { let __a1 = self.new_edge((1i32).wrapping_sub(self.cur_dir), 0i32); let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, __a1, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v806); }
                                                                        { let __v807 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v807); }
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        self.cur_h = save_h;
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1717
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(11i32);
                                                    }
                                                }
                                                // §1716
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        6 => {
                                            // §826
                                            {
                                                { let __v808 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((4999987i32) as usize)] = __v808; }
                                                { let __v809 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((4999987i32) as usize)].set_hh_rh(__v809); }
                                                p = 4999987i32;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        14 => {
                                            // §1720
                                            {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                left_edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                self.cur_dir = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                            }
                                        }
                                        _ => {
                                            // §732
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_ht == (1073741824i32).wrapping_neg()) {
                                    // §734
                                    self.rule_ht = self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int();
                                }
                                if (self.rule_dp == (1073741824i32).wrapping_neg()) {
                                    self.rule_dp = self.mem[crate::ix::U(((this_box).wrapping_add(2i32)) as usize)].int();
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
                            // §732
                            self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                        }
                        prev_p = p;
                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
                break 'l_reswitch_b;
            }
        }
        // §1715
        if (self.eTeX_mode == 1i32) {
            {
                // §1718
                {
                    while (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != 0i32) {
                        {
                            if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() > 4i32) {
                                self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                            }
                            {
                                self.temp_ptr = self.LR_ptr;
                                self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                {
                                    { let __ix810 = self.temp_ptr; let __v811 = self.avail; self.mem[crate::ix::U((__ix810) as usize)].set_hh_rh(__v811); }
                                    self.avail = self.temp_ptr;
                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                }
                            }
                        }
                    }
                    {
                        self.temp_ptr = self.LR_ptr;
                        self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                        {
                            { let __ix812 = self.temp_ptr; let __v813 = self.avail; self.mem[crate::ix::U((__ix812) as usize)].set_hh_rh(__v813); }
                            self.avail = self.temp_ptr;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                    }
                }
                // §1715
                if ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) == 2i32) {
                    self.cur_dir = 1i32;
                }
            }
        }
        // §729
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `pdf_vlist_out` routine is similar to `pdf_hlist_out`, but a bit simpler.
    // §738
    pub fn pdf_vlist_out(&mut self) {
        let mut left_edge: scaled = 0; // §738
        let mut top_edge: scaled = 0; // §738
        let mut save_v: scaled = 0; // §738
        let mut this_box: halfword = 0; // §738
        let mut g_order: glue_ord = 0; // §738
        let mut g_sign: i32 = 0; // §738
        let mut p: halfword = 0; // §738
        let mut leader_box: halfword = 0; // §738
        let mut leader_ht: scaled = 0; // §738
        let mut lx: scaled = 0; // §738
        let mut outer_doing_leaders: bool = false; // §738
        let mut edge: scaled = 0; // §738
        let mut glue_temp: f64 = 0.0; // §738
        let mut cur_glue: f64 = 0.0; // §738
        let mut cur_g: scaled = 0; // §738
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        left_edge = self.cur_h;
        self.cur_v = (self.cur_v).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int());
        top_edge = self.cur_v;
        // §739
        if (((self.last_thread != 0i32) && (self.pdf_thread_dp == (1073741824i32).wrapping_neg())) && (self.pdf_thread_level == self.cur_s)) {
            self.append_thread(this_box, left_edge, (top_edge).wrapping_add(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
        }
        // §738
        while (p != 0i32) {
            // §740
            {
                'l_L15_f: {
                    if (p >= self.hi_mem_min) {
                        self.confusion(1134i32);
                    } else {
                        // §741
                        {
                            'l_L13_f: {
                                'l_L14_f: {
                                    self.dl_node(p);
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        0 | 1 => {
                                            // §742
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == 0i32) {
                                                self.cur_v = ((self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            } else {
                                                {
                                                    self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    save_v = self.cur_v;
                                                    if (self.cur_dir == 1i32) {
                                                        self.cur_h = (left_edge).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    } else {
                                                        self.cur_h = (left_edge).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    }
                                                    self.temp_ptr = p;
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 1i32) {
                                                        self.pdf_vlist_out();
                                                    } else {
                                                        self.pdf_hlist_out();
                                                    }
                                                    self.cur_v = (save_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    self.cur_h = left_edge;
                                                }
                                            }
                                        }
                                        2 => {
                                            // §741
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        8 => {
                                            // §1639
                                            match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                                7 | 8 => {
                                                    self.pdf_out_literal(p);
                                                }
                                                40 => {
                                                    self.pdf_out_colorstack(p);
                                                }
                                                41 => {
                                                    self.pdf_out_setmatrix(p);
                                                }
                                                42 => {
                                                    self.pdf_out_save(p);
                                                }
                                                43 => {
                                                    self.pdf_out_restore(p);
                                                }
                                                10 => {
                                                    {
                                                        self.pdf_append_list_arg = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                        self.pdf_obj_list = self.append_ptr(self.pdf_obj_list, self.pdf_append_list_arg);
                                                    }
                                                }
                                                12 => {
                                                    // §1644
                                                    {
                                                        self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                        save_v = self.cur_v;
                                                        self.cur_h = left_edge;
                                                        self.out_form(p);
                                                        self.cur_v = (save_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        self.cur_h = left_edge;
                                                    }
                                                }
                                                14 => {
                                                    // §1643
                                                    {
                                                        self.cur_v = ((self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        save_v = self.cur_v;
                                                        self.cur_h = left_edge;
                                                        self.out_image(p);
                                                        self.cur_v = save_v;
                                                        self.cur_h = left_edge;
                                                    }
                                                }
                                                15 => {
                                                    // §1639
                                                    self.do_annot(p, this_box, left_edge, (top_edge).wrapping_add(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                16 => {
                                                    self.pdf_error(1845i32, 1959i32);
                                                }
                                                17 => {
                                                    self.pdf_error(1845i32, 1960i32);
                                                }
                                                19 => {
                                                    self.do_dest(p, this_box, left_edge, (top_edge).wrapping_add(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                20 | 21 => {
                                                    self.do_thread(p, this_box, left_edge, (top_edge).wrapping_add(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int()));
                                                }
                                                22 => {
                                                    self.end_thread();
                                                }
                                                23 => {
                                                    // §1641
                                                    {
                                                        self.pdf_last_x_pos = self.cur_h;
                                                        if self.is_shipping_page {
                                                            self.pdf_last_y_pos = (self.cur_page_height).wrapping_sub(self.cur_v);
                                                        } else {
                                                            self.pdf_last_y_pos = ((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth)).wrapping_sub(self.cur_v);
                                                        }
                                                    }
                                                }
                                                3 | 4 => {
                                                    // §1639
                                                    self.pdf_special(p);
                                                }
                                                36 => {
                                                    // §1642
                                                    {
                                                        self.pdf_snapx_refpos = self.cur_h;
                                                        self.pdf_snapy_refpos = self.cur_v;
                                                    }
                                                }
                                                38 => {
                                                    // §1639
                                                    self.do_snapy_comp(p, this_box);
                                                }
                                                37 => {
                                                    self.do_snapy(p);
                                                }
                                                45 => {
                                                    self.gen_faked_interword_space = true;
                                                }
                                                46 => {
                                                    self.gen_faked_interword_space = false;
                                                }
                                                47 => {
                                                    self.pdf_insert_fake_space();
                                                }
                                                48 => {
                                                    self.gen_running_link = false;
                                                }
                                                49 => {
                                                    self.gen_running_link = true;
                                                }
                                                _ => {
                                                    self.out_what(p);
                                                }
                                            }
                                        }
                                        10 => {
                                            // §744
                                            {
                                                self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_ht = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != 0i32) {
                                                    {
                                                        if (g_sign == 1i32) {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                    {
                                                                        cur_glue = (cur_glue + ((self.mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                        glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                                            if (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                {
                                                                    cur_glue = (cur_glue - ((self.mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                                    glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * cur_glue);
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
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= 100i32) {
                                                    // §745
                                                    {
                                                        leader_box = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == 2i32) {
                                                            {
                                                                self.rule_wd = self.mem[crate::ix::U(((leader_box).wrapping_add(1i32)) as usize)].int();
                                                                self.rule_dp = 0i32;
                                                                break 'l_L14_f;
                                                            }
                                                        }
                                                        leader_ht = (self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(2i32)) as usize)].int());
                                                        if ((leader_ht > 0i32) && (self.rule_ht > 0i32)) {
                                                            {
                                                                self.rule_ht = (self.rule_ht).wrapping_add(10i32);
                                                                edge = (self.cur_v).wrapping_add(self.rule_ht);
                                                                lx = 0i32;
                                                                // §664
                                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 100i32) {
                                                                    {
                                                                        save_v = self.cur_v;
                                                                        self.cur_v = (top_edge).wrapping_add((leader_ht).wrapping_mul(((self.cur_v).wrapping_sub(top_edge) / leader_ht)));
                                                                        if (self.cur_v < save_v) {
                                                                            self.cur_v = (self.cur_v).wrapping_add(leader_ht);
                                                                        }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.lq = (self.rule_ht / leader_ht);
                                                                        self.lr = (self.rule_ht % leader_ht);
                                                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 101i32) {
                                                                            self.cur_v = (self.cur_v).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_v = (self.cur_v).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §745
                                                                while ((self.cur_v).wrapping_add(leader_ht) <= edge) {
                                                                    // §746
                                                                    {
                                                                        if (self.cur_dir == 1i32) {
                                                                            self.cur_h = (left_edge).wrapping_sub(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        } else {
                                                                            self.cur_h = (left_edge).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        }
                                                                        self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int());
                                                                        save_v = self.cur_v;
                                                                        self.temp_ptr = leader_box;
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == 1i32) {
                                                                            self.pdf_vlist_out();
                                                                        } else {
                                                                            self.pdf_hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.cur_h = left_edge;
                                                                        self.cur_v = (((save_v).wrapping_sub(self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int())).wrapping_add(leader_ht)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §745
                                                                self.cur_v = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §744
                                                break 'l_L13_f;
                                            }
                                        }
                                        11 => {
                                            // §741
                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        _ => {
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_wd == (1073741824i32).wrapping_neg()) {
                                    // §743
                                    self.rule_wd = self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_dir == 1i32) {
                                            self.cur_h = (self.cur_h).wrapping_sub(self.rule_wd);
                                        }
                                        self.pdf_set_rule(self.cur_h, self.cur_v, self.rule_wd, self.rule_ht);
                                        self.cur_h = left_edge;
                                    }
                                }
                                break 'l_L15_f;
                            }
                            // §741
                            self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                        }
                    }
                }
                // §740
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        // §738
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// `fix_pdfoutput` freezes `pdfoutput` when something has been written to
    /// the output.
    // §747
    pub fn fix_pdfoutput(&mut self) {
        if (!self.fixed_pdfoutput_set) {
            {
                self.fixed_pdfoutput = self.eqtb[crate::ix::U(((629079i32) - 1) as usize)].int();
                self.fixed_pdfoutput_set = true;
            }
        } else {
            if (self.fixed_pdfoutput != self.eqtb[crate::ix::U(((629079i32) - 1) as usize)].int()) {
                self.pdf_error(1014i32, 1135i32);
            }
        }
        if self.fixed_pdfoutput_set {
            self.fix_pdf_draftmode();
        }
    }

    /// `fix_pdf_draftmode` freezes `pdfdraftmode` when something has been written to
    /// the output and also switches some things off when draftmode is on.
    // §748
    pub fn fix_pdf_draftmode(&mut self) {
        if (!self.fixed_pdf_draftmode_set) {
            {
                self.fixed_pdf_draftmode = self.eqtb[crate::ix::U(((629105i32) - 1) as usize)].int();
                self.fixed_pdf_draftmode_set = true;
            }
        } else {
            if (self.fixed_pdf_draftmode != self.eqtb[crate::ix::U(((629105i32) - 1) as usize)].int()) {
                self.pdf_error(1014i32, 1136i32);
            }
        }
        if (self.fixed_pdf_draftmode_set && (self.fixed_pdf_draftmode > 0i32)) {
            {
                self.fixed_pdf_draftmode_set = true;
                self.eqtb[crate::ix::U(((629080i32) - 1) as usize)].set_int(0i32);
                self.fixed_pdf_objcompresslevel = 0i32;
            }
        }
    }

    /// `substr_of_str` is used in `pdf_ship_out` and `pdf_print_info`.
    // §749
    pub fn substr_of_str(&mut self, mut s: str_number, mut t: str_number) -> bool {
        let mut substr_of_str: bool = false;
        let mut j: pool_pointer = 0; // §749
        let mut k: pool_pointer = 0; // §749
        let mut kk: pool_pointer = 0; // §749
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
    // §750
    pub fn pdf_ship_out(&mut self, mut p: halfword, mut shipping_page: bool) {
        let mut i: i32 = 0; // §750
        let mut j: i32 = 0; // §750
        let mut k: i32 = 0; // §750
        let mut s: pool_pointer = 0; // §750
        let mut mediabox_given: bool = false; // §750
        let mut save_font_list: halfword = 0; // §750
        let mut save_obj_list: halfword = 0; // §750
        let mut save_ximage_list: halfword = 0; // §750
        let mut save_xform_list: halfword = 0; // §750
        let mut save_image_procset: i32 = 0; // §750
        let mut save_text_procset: i32 = 0; // §750
        let mut pdf_last_resources: i32 = 0; // §750
        'l_done_f: {
            if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_nl(348i32);
                    self.print_ln();
                    self.print(990i32);
                }
            }
            if (!self.init_pdf_output) {
                {
                    // §792
                    self.check_pdfversion();
                    self.prepare_mag();
                    self.fixed_decimal_digits = self.fix_int(self.eqtb[crate::ix::U(((629081i32) - 1) as usize)].int(), 0i32, 4i32);
                    self.min_bp_val = self.divide_scaled(self.one_hundred_bp, self.ten_pow[crate::ix::U(((self.fixed_decimal_digits).wrapping_add(2i32)) as usize)], 0i32);
                    if (self.eqtb[crate::ix::U(((629084i32) - 1) as usize)].int() == 0i32) {
                        { let __v814 = self.pk_dpi; self.eqtb[crate::ix::U(((629084i32) - 1) as usize)].set_int(__v814); }
                    }
                    self.fixed_pk_resolution = self.fix_int(self.eqtb[crate::ix::U(((629084i32) - 1) as usize)].int(), 72i32, 8000i32);
                    self.pk_scale_factor = self.divide_scaled(72i32, self.fixed_pk_resolution, (5i32).wrapping_add(self.fixed_decimal_digits));
                    if (self.eqtb[crate::ix::U(((627171i32) - 1) as usize)].hh().rh() != 0i32) {
                        {
                            { let __a815_0 = self.fixed_pk_resolution; let __a815_1 = self.tokens_to_string(self.eqtb[crate::ix::U(((627171i32) - 1) as usize)].hh().rh()); self.pk_init(__a815_0, __a815_1) };
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                            }
                        }
                    } else {
                        self.pk_init(self.fixed_pk_resolution, 0i32);
                    }
                    self.set_job_id(self.eqtb[crate::ix::U(((629041i32) - 1) as usize)].int(), self.eqtb[crate::ix::U(((629040i32) - 1) as usize)].int(), self.eqtb[crate::ix::U(((629039i32) - 1) as usize)].int(), self.eqtb[crate::ix::U(((629038i32) - 1) as usize)].int());
                    if ((self.eqtb[crate::ix::U(((629085i32) - 1) as usize)].int() > 0i32) && (self.pdf_resname_prefix == 0i32)) {
                        self.pdf_resname_prefix = self.get_resname_prefix();
                    }
                    // §750
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
                    while ((self.eqtb[crate::ix::U((((629128i32).wrapping_add(j)) - 1) as usize)].int() == 0i32) && (j > 0i32)) {
                        j = (j).wrapping_sub(1i32);
                    }
                    {
                        let __for_end_5 = j;
                        k = 0i32;
                        while k <= __for_end_5 {
                            {
                                self.print_int(((self.eqtb[crate::ix::U((((629128i32).wrapping_add(k)) - 1) as usize)].int()) as i64));
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
            if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() > 0i32) {
                {
                    if shipping_page {
                        self.print_char(93i32);
                    }
                    self.begin_diagnostic();
                    self.show_box(p);
                    self.end_diagnostic(true);
                }
            }
            // §669
            if ((((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > 1073741823i32) || (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > 1073741823i32)) || (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int()) > 1073741823i32)) || ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int()) > 1073741823i32)) {
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(994i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 995i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 996i32;
                    }
                    self.error();
                    if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() <= 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(997i32);
                            self.show_box(p);
                            self.end_diagnostic(true);
                        }
                    }
                    break 'l_done_f;
                }
            }
            if (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int()) > self.max_v) {
                self.max_v = ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int());
            }
            if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int()) > self.max_h) {
                self.max_h = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int());
            }
            // §752
            self.fix_pdfoutput();
            self.temp_ptr = p;
            self.prepare_mag();
            pdf_last_resources = self.pdf_new_objnum();
            self.pdf_page_group_val = 0i32;
            // §753
            self.pdf_font_list = 0i32;
            self.pdf_obj_list = 0i32;
            self.pdf_xform_list = 0i32;
            self.pdf_ximage_list = 0i32;
            self.pdf_text_procset = false;
            self.pdf_image_procset = 0i32;
            // §752
            if (!shipping_page) {
                {
                    self.pdf_xform_width = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                    self.pdf_xform_height = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                    self.pdf_xform_depth = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                    self.pdf_begin_dict(self.pdf_cur_form, 0i32);
                    self.pdf_last_stream = self.pdf_cur_form;
                    self.cur_v = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                    self.cur_h = 0i32;
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = (self.pdf_xform_height).wrapping_add(self.pdf_xform_depth);
                }
            } else {
                {
                    // §755
                    self.cur_h_offset = (self.eqtb[crate::ix::U(((629661i32) - 1) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int());
                    self.cur_v_offset = (self.eqtb[crate::ix::U(((629662i32) - 1) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int());
                    if (self.eqtb[crate::ix::U(((629663i32) - 1) as usize)].int() != 0i32) {
                        self.cur_page_width = self.eqtb[crate::ix::U(((629663i32) - 1) as usize)].int();
                    } else {
                        self.cur_page_width = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add((2i32).wrapping_mul(self.cur_h_offset));
                    }
                    if (self.eqtb[crate::ix::U(((629664i32) - 1) as usize)].int() != 0i32) {
                        self.cur_page_height = self.eqtb[crate::ix::U(((629664i32) - 1) as usize)].int();
                    } else {
                        self.cur_page_height = ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add((2i32).wrapping_mul(self.cur_v_offset));
                    }
                    // §752
                    self.pdf_last_page = self.get_obj(1i32, (self.total_pages).wrapping_add(1i32), ((0i32) != 0));
                    self.obj_tab[crate::ix::U((self.pdf_last_page) as usize)].int4 = 1i32;
                    self.pdf_new_dict(0i32, 0i32, 0i32);
                    self.pdf_last_stream = self.obj_ptr;
                    self.cur_h = self.cur_h_offset;
                    self.cur_v = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.cur_v_offset);
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = self.cur_page_height;
                    // §754
                    self.pdf_annot_list = 0i32;
                    self.pdf_link_list = 0i32;
                    self.pdf_dest_list = 0i32;
                    self.pdf_bead_list = 0i32;
                    self.last_thread = 0i32;
                }
            }
            // §752
            if (!shipping_page) {
                {
                    // §756
                    {
                        self.pdf_print(1137i32);
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
                        self.pdf_print(1138i32);
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
                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)] != 0i32) {
                        {
                            self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)]);
                            {
                                self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)]);
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(4i32)) as usize)] = 0i32;
                            }
                        }
                    }
                    self.pdf_print(1139i32);
                    self.pdf_print(1052i32);
                    self.pdf_print_bp(self.pdf_xform_width);
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
                    self.pdf_print_bp((self.pdf_xform_height).wrapping_add(self.pdf_xform_depth));
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
                    {
                        self.pdf_print(1140i32);
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
                        self.pdf_print(1141i32);
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
                    self.pdf_indirect_ln(1142i32, pdf_last_resources);
                }
            }
            // §757
            self.pdf_begin_stream();
            if shipping_page {
                {
                    // §758
                    self.prepare_mag();
                    if (self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int() != 1000i32) {
                        {
                            self.pdf_print_real(self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int(), 3i32);
                            self.pdf_print(1143i32);
                            self.pdf_print_real(self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int(), 3i32);
                            {
                                self.pdf_print(1144i32);
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
            }
            // §757
            self.pdfshipoutbegin(shipping_page);
            if shipping_page {
                self.pdf_out_colorstack_startpage();
            }
            // §751
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 1i32) {
                self.pdf_vlist_out();
            } else {
                self.pdf_hlist_out();
            }
            if shipping_page {
                self.total_pages = (self.total_pages).wrapping_add(1i32);
            }
            self.cur_s = (1i32).wrapping_neg();
            // §760
            self.pdf_end_text();
            self.pdfshipoutend(shipping_page);
            self.pdf_end_stream();
            // §759
            if shipping_page {
                {
                    // §769
                    self.pdf_begin_dict(self.pdf_last_page, 1i32);
                    {
                        self.pdf_print(1156i32);
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
                    self.pdf_indirect_ln(1157i32, self.pdf_last_stream);
                    self.pdf_indirect_ln(1142i32, pdf_last_resources);
                    mediabox_given = false;
                    if (self.eqtb[crate::ix::U(((627169i32) - 1) as usize)].hh().rh() != 0i32) {
                        {
                            s = self.tokens_to_string(self.eqtb[crate::ix::U(((627169i32) - 1) as usize)].hh().rh());
                            mediabox_given = self.substr_of_str(1158i32, s);
                            self.flush_str(s);
                        }
                    }
                    if (!mediabox_given) {
                        {
                            self.pdf_print(1159i32);
                            self.pdf_print_mag_bp(self.cur_page_width);
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
                            self.pdf_print_mag_bp(self.cur_page_height);
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
                    }
                    if (self.eqtb[crate::ix::U(((627169i32) - 1) as usize)].hh().rh() != 0i32) {
                        self.pdf_print_toks_ln(self.eqtb[crate::ix::U(((627169i32) - 1) as usize)].hh().rh());
                    }
                    // §770
                    if ((self.total_pages % 6i32) == 1i32) {
                        {
                            self.pdf_create_obj(2i32, 6i32);
                            self.pdf_last_pages = self.obj_ptr;
                        }
                    }
                    self.pdf_indirect_ln(1161i32, self.pdf_last_pages);
                    // §769
                    if (self.pdf_page_group_val > 0i32) {
                        {
                            self.pdf_print(1160i32);
                            self.pdf_print_int(((self.pdf_page_group_val) as i64));
                            {
                                self.pdf_print(1073i32);
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
                    // §771
                    if ((self.pdf_annot_list != 0i32) || (self.pdf_link_list != 0i32)) {
                        {
                            self.pdf_print(1162i32);
                            k = self.pdf_annot_list;
                            while (k != 0i32) {
                                {
                                    self.pdf_print_int(((self.mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1146i32);
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                            k = self.pdf_link_list;
                            while (k != 0i32) {
                                {
                                    self.pdf_print_int(((self.mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1146i32);
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
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
                    if (self.pdf_bead_list != 0i32) {
                        {
                            k = self.pdf_bead_list;
                            self.pdf_print(1163i32);
                            while (k != 0i32) {
                                {
                                    self.pdf_print_int(((self.mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                                    self.pdf_print(1146i32);
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
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
                    // §769
                    self.pdf_end_dict();
                }
            }
            // §773
            if (self.pdf_obj_list != 0i32) {
                {
                    k = self.pdf_obj_list;
                    while (k != 0i32) {
                        {
                            if (!(self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                self.pdf_write_obj(self.mem[crate::ix::U((k) as usize)].hh().lh());
                            }
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §779
            if (self.pdf_ximage_list != 0i32) {
                {
                    k = self.pdf_ximage_list;
                    while (k != 0i32) {
                        {
                            if (!(self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                self.pdf_write_image(self.mem[crate::ix::U((k) as usize)].hh().lh());
                            }
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §775
            if (self.pdf_xform_list != 0i32) {
                {
                    k = self.pdf_xform_list;
                    while (k != 0i32) {
                        {
                            if (!(self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64))) {
                                {
                                    self.saved_pdf_cur_form = self.pdf_cur_form;
                                    self.pdf_cur_form = self.mem[crate::ix::U((k) as usize)].hh().lh();
                                    // §776
                                    save_font_list = self.pdf_font_list;
                                    save_obj_list = self.pdf_obj_list;
                                    save_xform_list = self.pdf_xform_list;
                                    save_ximage_list = self.pdf_ximage_list;
                                    save_text_procset = ((self.pdf_text_procset) as i32);
                                    save_image_procset = self.pdf_image_procset;
                                    // §753
                                    self.pdf_font_list = 0i32;
                                    self.pdf_obj_list = 0i32;
                                    self.pdf_xform_list = 0i32;
                                    self.pdf_ximage_list = 0i32;
                                    self.pdf_text_procset = false;
                                    self.pdf_image_procset = 0i32;
                                    // §775
                                    self.pdf_ship_out(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(3i32)) as usize)], false);
                                    self.pdf_cur_form = self.saved_pdf_cur_form;
                                    // §777
                                    self.pdf_font_list = save_font_list;
                                    self.pdf_obj_list = save_obj_list;
                                    self.pdf_xform_list = save_xform_list;
                                    self.pdf_ximage_list = save_ximage_list;
                                    self.pdf_text_procset = ((save_text_procset) != 0);
                                    self.pdf_image_procset = save_image_procset;
                                }
                            }
                            // §775
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                }
            }
            // §759
            if shipping_page {
                {
                    // §780
                    self.pdf_origin_h = 0i32;
                    self.pdf_origin_v = self.cur_page_height;
                    // §781
                    if (self.pdf_annot_list != 0i32) {
                        {
                            k = self.pdf_annot_list;
                            while (k != 0i32) {
                                {
                                    i = self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    self.pdf_begin_dict(self.mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                    {
                                        self.pdf_print(1169i32);
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
                                    self.pdf_print_toks_ln(self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh());
                                    self.pdf_rectangle(self.mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(4i32)) as usize)].int());
                                    self.pdf_end_dict();
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §782
                    if (self.pdf_link_list != 0i32) {
                        {
                            k = self.pdf_link_list;
                            while (k != 0i32) {
                                {
                                    i = self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    self.pdf_begin_dict(self.mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                    {
                                        self.pdf_print(1169i32);
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
                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() != 3i32) {
                                        {
                                            self.pdf_print(1170i32);
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
                                    if (self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh() != 0i32) {
                                        self.pdf_print_toks_ln(self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().lh());
                                    }
                                    self.pdf_rectangle(self.mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U(((i).wrapping_add(4i32)) as usize)].int());
                                    if (self.mem[crate::ix::U((self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() != 3i32) {
                                        self.pdf_print(1171i32);
                                    }
                                    self.write_action(self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().rh());
                                    self.pdf_end_dict();
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                            // §783
                            k = self.pdf_link_list;
                            while (k != 0i32) {
                                {
                                    i = self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                    if (self.mem[crate::ix::U((i) as usize)].hh().lh() == 268435455i32) {
                                        self.flush_whatsit_node(i, 16i32);
                                    }
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §784
                    if (self.pdf_dest_list != 0i32) {
                        {
                            k = self.pdf_dest_list;
                            while (k != 0i32) {
                                {
                                    if (self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int2 > (((1i32).wrapping_neg()) as i64)) {
                                        self.pdf_error(1165i32, 1172i32);
                                    } else {
                                        {
                                            i = self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4;
                                            if ((self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) && (self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == 0i32)) {
                                                {
                                                    self.pdf_begin_dict(self.mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
                                                    self.pdf_print(1173i32);
                                                }
                                            } else {
                                                self.pdf_begin_obj(self.mem[crate::ix::U((k) as usize)].hh().lh(), 1i32);
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
                                                    self.pdf_buf_set(self.pdf_ptr, 91i32);
                                                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == 0i32) {
                                                self.pdf_print_int(((self.pdf_last_page) as i64));
                                            } else {
                                                self.pdf_print_int(((self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh()) as i64));
                                            }
                                            self.pdf_print(1146i32);
                                            match self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b0() {
                                                0 => {
                                                    {
                                                        self.pdf_print(1174i32);
                                                        self.pdf_print_mag_bp((self.mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
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
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(self.mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
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
                                                        if (self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() == 0i32) {
                                                            self.pdf_print(1175i32);
                                                        } else {
                                                            {
                                                                self.pdf_print_int((((self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() / 1000i32)) as i64));
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
                                                                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                                                                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                                self.pdf_print_int((((self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().lh() % 1000i32)) as i64));
                                                            }
                                                        }
                                                    }
                                                }
                                                1 => {
                                                    self.pdf_print(1176i32);
                                                }
                                                2 => {
                                                    {
                                                        self.pdf_print(1177i32);
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(self.mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
                                                    }
                                                }
                                                3 => {
                                                    {
                                                        self.pdf_print(1178i32);
                                                        self.pdf_print_mag_bp((self.mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
                                                    }
                                                }
                                                4 => {
                                                    self.pdf_print(1179i32);
                                                }
                                                5 => {
                                                    {
                                                        self.pdf_print(1180i32);
                                                        self.pdf_print_mag_bp((self.pdf_origin_v).wrapping_sub(self.mem[crate::ix::U(((i).wrapping_add(2i32)) as usize)].int()));
                                                    }
                                                }
                                                6 => {
                                                    {
                                                        self.pdf_print(1181i32);
                                                        self.pdf_print_mag_bp((self.mem[crate::ix::U(((i).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.pdf_origin_h));
                                                    }
                                                }
                                                7 => {
                                                    {
                                                        self.pdf_print(1182i32);
                                                        self.pdf_print_rect_spec(i);
                                                    }
                                                }
                                                _ => {
                                                    self.pdf_error(1165i32, 1183i32);
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
                                            if ((self.mem[crate::ix::U(((i).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) && (self.mem[crate::ix::U(((i).wrapping_add(6i32)) as usize)].hh().rh() == 0i32)) {
                                                self.pdf_end_dict();
                                            } else {
                                                self.pdf_end_obj();
                                            }
                                        }
                                    }
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                    // §786
                    if (self.pdf_bead_list != 0i32) {
                        {
                            k = self.pdf_bead_list;
                            while (k != 0i32) {
                                {
                                    self.pdf_new_obj(0i32, 0i32, 1i32);
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
                                    i = self.pdf_mem[crate::ix::U((self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4) as usize)];
                                    self.pdf_print_rect_spec(i);
                                    if (self.mem[crate::ix::U((i) as usize)].hh().lh() == 268435455i32) {
                                        self.flush_whatsit_node(i, 21i32);
                                    }
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
                                    { let __ix816 = self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4; let __v817 = self.obj_ptr; self.pdf_mem[crate::ix::U((__ix816) as usize)] = __v817; }
                                    self.pdf_end_obj();
                                    k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                                }
                            }
                        }
                    }
                }
            }
            // §762
            self.pdf_begin_dict(pdf_last_resources, 1i32);
            // §763
            if shipping_page {
                {
                    if (self.eqtb[crate::ix::U(((627170i32) - 1) as usize)].hh().rh() != 0i32) {
                        self.pdf_print_toks_ln(self.eqtb[crate::ix::U(((627170i32) - 1) as usize)].hh().rh());
                    }
                }
            } else {
                {
                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)] != 0i32) {
                        {
                            self.pdf_print_toks_ln(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)]);
                            {
                                self.delete_token_ref(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)]);
                                self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.pdf_cur_form) as usize)].int4).wrapping_add(5i32)) as usize)] = 0i32;
                            }
                        }
                    }
                }
            }
            // §766
            if (self.pdf_font_list != 0i32) {
                {
                    self.pdf_print(1145i32);
                    k = self.pdf_font_list;
                    while (k != 0i32) {
                        {
                            self.pdf_print(1041i32);
                            {
                                if (self.pdf_font_num[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)] < 0i32) {
                                    self.ff = (self.pdf_font_num[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)]).wrapping_neg();
                                } else {
                                    self.ff = self.mem[crate::ix::U((k) as usize)].hh().lh();
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
                            self.pdf_print_int(((self.pdf_font_num[crate::ix::U((self.ff) as usize)]) as i64));
                            self.pdf_print(1146i32);
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
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
                    self.pdf_text_procset = true;
                }
            }
            // §767
            if ((self.pdf_xform_list != 0i32) || (self.pdf_ximage_list != 0i32)) {
                {
                    self.pdf_print(1147i32);
                    k = self.pdf_xform_list;
                    while (k != 0i32) {
                        {
                            self.pdf_print(1148i32);
                            self.pdf_print_int(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int0) as i64));
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
                            self.pdf_print_int(((self.mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                            self.pdf_print(1146i32);
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
                    k = self.pdf_ximage_list;
                    while (k != 0i32) {
                        {
                            self.pdf_print(1149i32);
                            self.pdf_print_int(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int0) as i64));
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
                            self.pdf_print_int(((self.mem[crate::ix::U((k) as usize)].hh().lh()) as i64));
                            self.pdf_print(1146i32);
                            self.update_image_procset(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U((k) as usize)].hh().lh()) as usize)].int4).wrapping_add(4i32)) as usize)]);
                            k = self.mem[crate::ix::U((k) as usize)].hh().rh();
                        }
                    }
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
            // §768
            if ((self.eqtb[crate::ix::U(((629114i32) - 1) as usize)].int() < 0i32) || ((self.eqtb[crate::ix::U(((629114i32) - 1) as usize)].int() == 0i32) && (self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int() < 2i32))) {
                {
                    self.pdf_print(1150i32);
                    if self.pdf_text_procset {
                        self.pdf_print(1151i32);
                    }
                    if self.check_image_b(self.pdf_image_procset) {
                        self.pdf_print(1152i32);
                    }
                    if self.check_image_c(self.pdf_image_procset) {
                        self.pdf_print(1153i32);
                    }
                    if self.check_image_i(self.pdf_image_procset) {
                        self.pdf_print(1154i32);
                    }
                    {
                        self.pdf_print(1155i32);
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
            // §762
            self.pdf_end_dict();
            // §764
            self.flush_list(self.pdf_font_list);
            self.flush_list(self.pdf_obj_list);
            self.flush_list(self.pdf_xform_list);
            self.flush_list(self.pdf_ximage_list);
            // §759
            if shipping_page {
                {
                    // §765
                    self.flush_list(self.pdf_annot_list);
                    self.flush_list(self.pdf_link_list);
                    self.flush_list(self.pdf_dest_list);
                    self.flush_list(self.pdf_bead_list);
                }
            }
        }
        // §751
        // §750
        if (self.eTeX_mode == 1i32) {
            // §1730
            {
                if (self.LR_problems > 0i32) {
                    {
                        // §1713
                        {
                            self.print_ln();
                            self.print_nl(2022i32);
                            self.print_int((((self.LR_problems / 10000i32)) as i64));
                            self.print(2023i32);
                            self.print_int((((self.LR_problems % 10000i32)) as i64));
                            self.print(2024i32);
                            self.LR_problems = 0i32;
                        }
                        // §1730
                        self.print_char(41i32);
                        self.print_ln();
                    }
                }
                if ((self.LR_ptr != 0i32) || (self.cur_dir != 0i32)) {
                    self.confusion(2026i32);
                }
            }
        }
        // §750
        if ((self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() <= 0i32) && shipping_page) {
            self.print_char(93i32);
        }
        self.dead_cycles = 0i32;
        crate::system::break_out(&mut self.term_out);
        // §667
        if (self.eqtb[crate::ix::U(((629049i32) - 1) as usize)].int() > 1i32) {
            {
                self.print_nl(991i32);
                self.print_int(((self.var_used) as i64));
                self.print_char(38i32);
                self.print_int(((self.dyn_used) as i64));
                self.print_char(59i32);
            }
        }
        self.flush_node_list(p);
        if (self.eqtb[crate::ix::U(((629049i32) - 1) as usize)].int() > 1i32) {
            {
                self.print(992i32);
                self.print_int(((self.var_used) as i64));
                self.print_char(38i32);
                self.print_int(((self.dyn_used) as i64));
                self.print(993i32);
                self.print_int(((((self.hi_mem_min).wrapping_sub(self.lo_mem_max)).wrapping_sub(1i32)) as i64));
                self.print_ln();
            }
        }
    }

    /// Now we are ready to declare our new procedure `ship_out`.  It will call
    /// `pdf_ship_out` if the integer parameter `pdf_output` is positive; otherwise it
    /// will call `dvi_ship_out`, which is the \TeX\ original `ship_out`.
    // §791
    pub fn ship_out(&mut self, mut p: halfword) {
        self.fix_pdfoutput();
        if (self.eqtb[crate::ix::U(((629079i32) - 1) as usize)].int() > 0i32) {
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
    // §793
    pub fn str_less_str(&mut self, mut s1: str_number, mut s2: str_number) -> bool {
        let mut str_less_str: bool = false;
        let mut j1: pool_pointer = 0; // §793
        let mut j2: pool_pointer = 0; // §793
        let mut e1: pool_pointer = 0; // §793
        let mut e2: pool_pointer = 0; // §793
        let mut c1: packed_ASCII_code = 0; // §793
        let mut c2: packed_ASCII_code = 0; // §793
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
    // §793
    pub fn sort_dest_names(&mut self, mut l: i32, mut r: i32) {
        let mut i: i32 = 0; // §793
        let mut j: i32 = 0; // §793
        let mut s: str_number = 0; // §793
        let mut e: dest_name_entry = dest_name_entry::default(); // §793
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
                    { let __v818 = self.dest_names[crate::ix::U((j) as usize)]; self.dest_names[crate::ix::U((i) as usize)] = __v818; }
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
    // §795
    pub fn pdf_fix_dest(&mut self, mut k: i32) {
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != 0i32) {
            return;
        }
        self.pdf_warning(1192i32, 348i32, true, false);
        if (self.obj_tab[crate::ix::U((k) as usize)].int0 < 0i32) {
            {
                self.print(1193i32);
                self.print((self.obj_tab[crate::ix::U((k) as usize)].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1194i32);
                self.print_int(((self.obj_tab[crate::ix::U((k) as usize)].int0) as i64));
            }
        }
        self.print(1195i32);
        self.print_ln();
        self.print_ln();
        self.pdf_begin_obj(k, 1i32);
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
        self.pdf_print_int(((self.head_tab[crate::ix::U(((1i32) - 1) as usize)]) as i64));
        {
            self.pdf_print(1196i32);
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
    }

    /// The same for structure destinations, except that there is no sensible default
    /// object to point to.
    // §797
    pub fn pdf_fix_struct_dest(&mut self, mut k: i32) {
        if (self.obj_tab[crate::ix::U((k) as usize)].int4 != 0i32) {
            return;
        }
        self.pdf_warning(1197i32, 348i32, false, false);
        if (self.obj_tab[crate::ix::U((k) as usize)].int0 < 0i32) {
            {
                self.print(1193i32);
                self.print((self.obj_tab[crate::ix::U((k) as usize)].int0).wrapping_neg());
                self.print(125i32);
            }
        } else {
            {
                self.print(1194i32);
                self.print_int(((self.obj_tab[crate::ix::U((k) as usize)].int0) as i64));
            }
        }
        self.print(1198i32);
        self.print_ln();
        self.print_ln();
    }

    /// If the same keys in a dictionary are given several times, then it is not
    /// defined which value is chosen by an application.  Therefore the keys
    /// `/Producer` and `/Creator` are only set if the token list
    /// `pdf_info_toks` converted to a string does not contain these key strings.
    // §807
    pub fn pdf_print_info(&mut self) {
        let mut s: str_number = 0; // §807
        let mut creator_given: bool = false; // §807
        let mut producer_given: bool = false; // §807
        let mut creationdate_given: bool = false; // §807
        let mut moddate_given: bool = false; // §807
        let mut trapped_given: bool = false; // §807
        self.pdf_new_dict(0i32, 0i32, 3i32);
        creator_given = false;
        producer_given = false;
        creationdate_given = false;
        moddate_given = false;
        trapped_given = false;
        if (self.pdf_info_toks != 0i32) {
            {
                s = self.tokens_to_string(self.pdf_info_toks);
                creator_given = self.substr_of_str(1216i32, s);
                producer_given = self.substr_of_str(1217i32, s);
                creationdate_given = self.substr_of_str(1218i32, s);
                moddate_given = self.substr_of_str(1219i32, s);
                trapped_given = self.substr_of_str(1220i32, s);
            }
        }
        if (!producer_given) {
            {
                // §808
                self.pdf_print(1226i32);
                self.pdf_print_int((((140i32 / 100i32)) as i64));
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
                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print_int((((140i32 % 100i32)) as i64));
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
                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print(257i32);
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
        }
        // §807
        if (self.pdf_info_toks != 0i32) {
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
                self.flush_str(s);
                {
                    self.delete_token_ref(self.pdf_info_toks);
                    self.pdf_info_toks = 0i32;
                }
            }
        }
        if (!creator_given) {
            self.pdf_str_entry_ln(1221i32, 1222i32);
        }
        if (self.eqtb[crate::ix::U(((629110i32) - 1) as usize)].int() == 0i32) {
            {
                if (!creationdate_given) {
                    {
                        // §809
                        self.print_creation_date();
                    }
                }
                // §807
                if (!moddate_given) {
                    {
                        // §810
                        self.print_mod_date();
                    }
                }
            }
        }
        // §807
        if (!trapped_given) {
            {
                {
                    self.pdf_print(1223i32);
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
        if ((self.eqtb[crate::ix::U(((629111i32) - 1) as usize)].int() % 2i32) == 0i32) {
            {
                if self.get_ptex_use_underscore() {
                    self.pdf_str_entry_ln(1224i32, self.pdftex_banner);
                } else {
                    self.pdf_str_entry_ln(1225i32, self.pdftex_banner);
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
    // §817
    pub fn scan_spec(&mut self, mut c: group_code, mut three_codes: bool) {
        let mut s: i32 = 0; // §817
        let mut spec_code: i32 = 0; // §817
        'l_found_f: {
            if three_codes {
                s = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int();
            }
            if self.scan_keyword(1244i32) {
                spec_code = 0i32;
            } else {
                if self.scan_keyword(1245i32) {
                    spec_code = 1i32;
                } else {
                    {
                        spec_code = 1i32;
                        self.cur_val = 0i32;
                        break 'l_found_f;
                    }
                }
            }
            self.scan_dimen(false, false, false);
        }
        if three_codes {
            {
                { let __ix819 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix819) as usize)].set_int(s); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix820 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix820) as usize)].set_int(spec_code); }
        { let __ix821 = (self.save_ptr).wrapping_add(1i32); let __v822 = self.cur_val; self.save_stack[crate::ix::U((__ix821) as usize)].set_int(__v822); }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(c);
        self.scan_left_brace();
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §823
    pub fn check_expand_pars(&mut self, mut f: internal_font_number) -> bool {
        let mut check_expand_pars: bool = false;
        let mut k: internal_font_number = 0; // §823
        check_expand_pars = false;
        if ((self.pdf_font_step[crate::ix::U((f) as usize)] == 0i32) || ((self.pdf_font_stretch[crate::ix::U((f) as usize)] == 0i32) && (self.pdf_font_shrink[crate::ix::U((f) as usize)] == 0i32))) {
            return check_expand_pars;
        }
        if (self.cur_font_step < 0i32) {
            self.cur_font_step = self.pdf_font_step[crate::ix::U((f) as usize)];
        } else {
            if (self.cur_font_step != self.pdf_font_step[crate::ix::U((f) as usize)]) {
                self.pdf_error(1039i32, 1246i32);
            }
        }
        k = self.pdf_font_stretch[crate::ix::U((f) as usize)];
        if (k != 0i32) {
            {
                if (self.max_stretch_ratio < 0i32) {
                    self.max_stretch_ratio = self.pdf_font_expand_ratio[crate::ix::U((k) as usize)];
                } else {
                    if (self.max_stretch_ratio != self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]) {
                        self.pdf_error(1039i32, 1247i32);
                    }
                }
            }
        }
        k = self.pdf_font_shrink[crate::ix::U((f) as usize)];
        if (k != 0i32) {
            {
                if (self.max_shrink_ratio < 0i32) {
                    self.max_shrink_ratio = (self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]).wrapping_neg();
                } else {
                    if (self.max_shrink_ratio != (self.pdf_font_expand_ratio[crate::ix::U((k) as usize)]).wrapping_neg()) {
                        self.pdf_error(1039i32, 1247i32);
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
    // §823
    pub fn char_stretch(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut char_stretch: scaled = 0;
        let mut k: internal_font_number = 0; // §823
        let mut dw: scaled = 0; // §823
        let mut ef: i32 = 0; // §823
        char_stretch = 0i32;
        k = self.pdf_font_stretch[crate::ix::U((f) as usize)];
        ef = self.get_ef_code(f, c);
        if ((k != 0i32) && (ef > 0i32)) {
            {
                dw = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((k) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int()).wrapping_sub(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int());
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
    // §823
    pub fn char_shrink(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut char_shrink: scaled = 0;
        let mut k: internal_font_number = 0; // §823
        let mut dw: scaled = 0; // §823
        let mut ef: i32 = 0; // §823
        char_shrink = 0i32;
        k = self.pdf_font_shrink[crate::ix::U((f) as usize)];
        ef = self.get_ef_code(f, c);
        if ((k != 0i32) && (ef > 0i32)) {
            {
                dw = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int()).wrapping_sub(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((k) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((k) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int());
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
    // §823
    pub fn get_kern(&mut self, mut f: internal_font_number, mut lc: eight_bits, mut rc: eight_bits) -> scaled {
        let mut get_kern: scaled = 0;
        let mut i: four_quarters = four_quarters::default(); // §823
        let mut j: four_quarters = four_quarters::default(); // §823
        let mut k: font_index = 0; // §823
        // goto labels: continue, L23
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                get_kern = 0i32;
                i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(lc)) as usize)].qqqq();
                if (((i.b2()).wrapping_sub(0i32) % 4i32) != 1i32) {
                    return get_kern;
                }
                k = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add(i.b3());
                j = self.font_info[crate::ix::U((k) as usize)].qqqq();
                if (j.b0() <= 128i32) {
                    { __goto_1 = 2; continue 'l_dispatch_1; }
                }
                k = ((((self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(j.b2()))).wrapping_add(j.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
            }
            if __goto_1 <= 1 { // continue
                j = self.font_info[crate::ix::U((k) as usize)].qqqq();
            }
            if __goto_1 <= 2 { // L23
                if (((j.b1() == rc) && (j.b0() <= 128i32)) && (j.b2() >= 128i32)) {
                    {
                        get_kern = self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(j.b2()))).wrapping_add(j.b3())) as usize)].int();
                        return get_kern;
                    }
                }
                if (j.b0() == 0i32) {
                    k = (k).wrapping_add(1i32);
                } else {
                    {
                        if (j.b0() >= 128i32) {
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
    // §823
    pub fn kern_stretch(&mut self, mut p: halfword) -> scaled {
        let mut kern_stretch: scaled = 0;
        let mut l: halfword = 0; // §823
        let mut r: halfword = 0; // §823
        let mut d: scaled = 0; // §823
        kern_stretch = 0i32;
        if (((self.prev_char_p == 0i32) || (self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().rh() != p)) || (self.mem[crate::ix::U((p) as usize)].hh().rh() == 0i32)) {
            return kern_stretch;
        }
        l = self.prev_char_p;
        r = self.mem[crate::ix::U((p) as usize)].hh().rh();
        if (!(l >= self.hi_mem_min)) {
            if (self.mem[crate::ix::U((l) as usize)].hh().b0() == 6i32) {
                l = (l).wrapping_add(1i32);
            } else {
                return kern_stretch;
            }
        }
        if (!(r >= self.hi_mem_min)) {
            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == 6i32) {
                r = (r).wrapping_add(1i32);
            } else {
                return kern_stretch;
            }
        }
        if (!((self.mem[crate::ix::U((l) as usize)].hh().b0() == self.mem[crate::ix::U((r) as usize)].hh().b0()) && (self.pdf_font_stretch[crate::ix::U((self.mem[crate::ix::U((l) as usize)].hh().b0()) as usize)] != 0i32))) {
            return kern_stretch;
        }
        d = self.get_kern(self.pdf_font_stretch[crate::ix::U((self.mem[crate::ix::U((l) as usize)].hh().b0()) as usize)], self.mem[crate::ix::U((l) as usize)].hh().b1(), self.mem[crate::ix::U((r) as usize)].hh().b1());
        kern_stretch = { let __a823_0 = (d).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()); let __a823_1 = self.get_ef_code(self.mem[crate::ix::U((l) as usize)].hh().b0(), self.mem[crate::ix::U((l) as usize)].hh().b1()); let __a823_2 = 1000i32; self.round_xn_over_d(__a823_0, __a823_1, __a823_2) };
        kern_stretch
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §823
    pub fn kern_shrink(&mut self, mut p: halfword) -> scaled {
        let mut kern_shrink: scaled = 0;
        let mut l: halfword = 0; // §823
        let mut r: halfword = 0; // §823
        let mut d: scaled = 0; // §823
        kern_shrink = 0i32;
        if (((self.prev_char_p == 0i32) || (self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().rh() != p)) || (self.mem[crate::ix::U((p) as usize)].hh().rh() == 0i32)) {
            return kern_shrink;
        }
        l = self.prev_char_p;
        r = self.mem[crate::ix::U((p) as usize)].hh().rh();
        if (!(l >= self.hi_mem_min)) {
            if (self.mem[crate::ix::U((l) as usize)].hh().b0() == 6i32) {
                l = (l).wrapping_add(1i32);
            } else {
                return kern_shrink;
            }
        }
        if (!(r >= self.hi_mem_min)) {
            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == 6i32) {
                r = (r).wrapping_add(1i32);
            } else {
                return kern_shrink;
            }
        }
        if (!((self.mem[crate::ix::U((l) as usize)].hh().b0() == self.mem[crate::ix::U((r) as usize)].hh().b0()) && (self.pdf_font_shrink[crate::ix::U((self.mem[crate::ix::U((l) as usize)].hh().b0()) as usize)] != 0i32))) {
            return kern_shrink;
        }
        d = self.get_kern(self.pdf_font_shrink[crate::ix::U((self.mem[crate::ix::U((l) as usize)].hh().b0()) as usize)], self.mem[crate::ix::U((l) as usize)].hh().b1(), self.mem[crate::ix::U((r) as usize)].hh().b1());
        kern_shrink = { let __a824_0 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_sub(d); let __a824_1 = self.get_ef_code(self.mem[crate::ix::U((l) as usize)].hh().b0(), self.mem[crate::ix::U((l) as usize)].hh().b1()); let __a824_2 = 1000i32; self.round_xn_over_d(__a824_0, __a824_1, __a824_2) };
        kern_shrink
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §823
    pub fn do_subst_font(&mut self, mut p: halfword, mut ex_ratio: i32) {
        let mut f: internal_font_number = 0; // §823
        let mut k: internal_font_number = 0; // §823
        let mut r: halfword = 0; // §823
        let mut ef: i32 = 0; // §823
        if ((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == 7i32)) {
            {
                r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                while (r != 0i32) {
                    {
                        if ((r >= self.hi_mem_min) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == 6i32)) {
                            self.do_subst_font(r, ex_ratio);
                        }
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                while (r != 0i32) {
                    {
                        if ((r >= self.hi_mem_min) || (self.mem[crate::ix::U((r) as usize)].hh().b0() == 6i32)) {
                            self.do_subst_font(r, ex_ratio);
                        }
                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                    }
                }
                return;
            }
        }
        if (p >= self.hi_mem_min) {
            r = p;
        } else {
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 6i32) {
                r = (p).wrapping_add(1i32);
            } else {
                {
                    self.pdf_error(1039i32, 1248i32);
                }
            }
        }
        f = self.mem[crate::ix::U((r) as usize)].hh().b0();
        ef = self.get_ef_code(f, self.mem[crate::ix::U((r) as usize)].hh().b1());
        if (ef == 0i32) {
            return;
        }
        if ((self.pdf_font_stretch[crate::ix::U((f) as usize)] != 0i32) && (ex_ratio > 0i32)) {
            k = { let __a825_0 = f; let __a825_1 = self.ext_xn_over_d((ex_ratio).wrapping_mul(ef), self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_stretch[crate::ix::U((f) as usize)]) as usize)], 1000000i32); self.expand_font(__a825_0, __a825_1) };
        } else {
            if ((self.pdf_font_shrink[crate::ix::U((f) as usize)] != 0i32) && (ex_ratio < 0i32)) {
                k = { let __a826_0 = f; let __a826_1 = self.ext_xn_over_d((ex_ratio).wrapping_mul(ef), (self.pdf_font_expand_ratio[crate::ix::U((self.pdf_font_shrink[crate::ix::U((f) as usize)]) as usize)]).wrapping_neg(), 1000000i32); self.expand_font(__a826_0, __a826_1) };
            } else {
                k = f;
            }
        }
        if (k != f) {
            {
                self.mem[crate::ix::U((r) as usize)].set_hh_b0(k);
                if (!(p >= self.hi_mem_min)) {
                    {
                        r = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                        while (r != 0i32) {
                            {
                                self.mem[crate::ix::U((r) as usize)].set_hh_b0(k);
                                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
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
    // §823
    pub fn char_pw(&mut self, mut p: halfword, mut side: small_number) -> scaled {
        let mut char_pw: scaled = 0;
        let mut f: internal_font_number = 0; // §823
        let mut c: i32 = 0; // §823
        char_pw = 0i32;
        if (side == 0i32) {
            self.last_leftmost_char = 0i32;
        } else {
            self.last_rightmost_char = 0i32;
        }
        if (p == 0i32) {
            return char_pw;
        }
        if (!(p >= self.hi_mem_min)) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 6i32) {
                    p = (p).wrapping_add(1i32);
                } else {
                    return char_pw;
                }
            }
        }
        f = self.mem[crate::ix::U((p) as usize)].hh().b0();
        if (side == 0i32) {
            {
                c = self.get_lp_code(f, self.mem[crate::ix::U((p) as usize)].hh().b1());
                self.last_leftmost_char = p;
            }
        } else {
            {
                c = self.get_rp_code(f, self.mem[crate::ix::U((p) as usize)].hh().b1());
                self.last_rightmost_char = p;
            }
        }
        if (c == 0i32) {
            return char_pw;
        }
        char_pw = self.round_xn_over_d(self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), c, 1000i32);
        char_pw
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §823
    pub fn new_margin_kern(&mut self, mut w: scaled, mut p: halfword, mut side: small_number) -> halfword {
        let mut new_margin_kern: halfword = 0;
        let mut k: halfword = 0; // §823
        k = self.get_node(3i32);
        self.mem[crate::ix::U((k) as usize)].set_hh_b0(40i32);
        self.mem[crate::ix::U((k) as usize)].set_hh_b1(side);
        self.mem[crate::ix::U(((k).wrapping_add(1i32)) as usize)].set_int(w);
        if (p == 0i32) {
            self.pdf_error(1249i32, 1250i32);
        }
        {
            { let __v827 = self.avail; self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].set_hh_lh(__v827); }
            if (self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh() == 0i32) {
                { let __v828 = self.get_avail(); self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].set_hh_lh(__v828); }
            } else {
                {
                    self.avail = self.mem[crate::ix::U((self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().rh();
                    { let __ix829 = self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U((__ix829) as usize)].set_hh_rh(0i32); }
                    self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                    self.dl_new_node(self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh());
                }
            }
        }
        { let __ix830 = self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); let __v831 = self.mem[crate::ix::U((p) as usize)].hh().b1(); self.mem[crate::ix::U((__ix830) as usize)].set_hh_b1(__v831); }
        { let __ix832 = self.mem[crate::ix::U(((k).wrapping_add(2i32)) as usize)].hh().lh(); let __v833 = self.mem[crate::ix::U((p) as usize)].hh().b0(); self.mem[crate::ix::U((__ix832) as usize)].set_hh_b0(__v833); }
        new_margin_kern = k;
        new_margin_kern
    }

    /// Here is `hpack`, which is place where we do font substituting when
    /// font expansion is being used. We define some constants used when calling
    /// `hpack` to deal with font expansion.
    // §823
    pub fn hpack(&mut self, mut p: halfword, mut w: scaled, mut m: small_number) -> halfword {
        let mut hpack: halfword = 0;
        let mut r: halfword = 0; // §823
        let mut q: halfword = 0; // §823
        let mut h: scaled = 0; // §823
        let mut d: scaled = 0; // §823
        let mut x: scaled = 0; // §823
        let mut s: scaled = 0; // §823
        let mut g: halfword = 0; // §823
        let mut o: glue_ord = 0; // §823
        let mut f: internal_font_number = 0; // §823
        let mut i: four_quarters = four_quarters::default(); // §823
        let mut hd: eight_bits = 0; // §823
        let mut font_stretch: scaled = 0; // §823
        let mut font_shrink: scaled = 0; // §823
        let mut k: scaled = 0; // §823
        // goto labels: common_ending, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.last_badness = 0i32;
                r = self.get_node(7i32);
                self.mem[crate::ix::U((r) as usize)].set_hh_b0(0i32);
                self.mem[crate::ix::U((r) as usize)].set_hh_b1(0i32);
                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                q = (r).wrapping_add(5i32);
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                if (m == 2i32) {
                    {
                        self.prev_char_p = 0i32;
                        font_stretch = 0i32;
                        font_shrink = 0i32;
                        self.font_expand_ratio = 0i32;
                    }
                }
                h = 0i32;
                // §824
                d = 0i32;
                x = 0i32;
                self.total_stretch[crate::ix::U((0i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((0i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((1i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((1i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((2i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((2i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((3i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((3i32) as usize)] = 0i32;
                // §823
                if (self.eqtb[crate::ix::U(((629126i32) - 1) as usize)].int() > 0i32) {
                    // §1710
                    {
                        self.temp_ptr = self.get_avail();
                        { let __ix834 = self.temp_ptr; self.mem[crate::ix::U((__ix834) as usize)].set_hh_lh(0i32); }
                        { let __ix835 = self.temp_ptr; let __v836 = self.LR_ptr; self.mem[crate::ix::U((__ix835) as usize)].set_hh_rh(__v836); }
                        self.LR_ptr = self.temp_ptr;
                    }
                }
                // §823
                while (p != 0i32) {
                    // §825
                    {
                        'l_reswitch_b: loop {
                            while (p >= self.hi_mem_min) {
                                // §828
                                {
                                    if (m >= 2i32) {
                                        {
                                            self.prev_char_p = p;
                                            match m {
                                                2 => {
                                                    {
                                                        f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                                        font_stretch = (font_stretch).wrapping_add(self.char_stretch(f, self.mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        font_shrink = (font_shrink).wrapping_add(self.char_shrink(f, self.mem[crate::ix::U((p) as usize)].hh().b1()));
                                                    }
                                                }
                                                3 => {
                                                    self.do_subst_font(p, self.font_expand_ratio);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b1())) as usize)].qqqq();
                                    hd = (i.b1()).wrapping_sub(0i32);
                                    x = (x).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int());
                                    s = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int();
                                    if (s > h) {
                                        h = s;
                                    }
                                    s = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int();
                                    if (s > d) {
                                        d = s;
                                    }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            // §825
                            if (p != 0i32) {
                                {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        0 | 1 | 2 | 13 => {
                                            // §827
                                            {
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= 2i32) {
                                                    s = 0i32;
                                                } else {
                                                    s = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int();
                                                }
                                                if ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_sub(s) > h) {
                                                    h = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_sub(s);
                                                }
                                                if ((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(s) > d) {
                                                    d = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_add(s);
                                                }
                                            }
                                        }
                                        3 | 4 | 5 => {
                                            // §825
                                            if ((self.adjust_tail != 0i32) || (self.pre_adjust_tail != 0i32)) {
                                                // §831
                                                {
                                                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 5i32) {
                                                        {
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() != 0i32) {
                                                                {
                                                                    if (self.pre_adjust_tail == 0i32) {
                                                                        self.confusion(1251i32);
                                                                    }
                                                                    { let __ix837 = self.pre_adjust_tail; let __v838 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix837) as usize)].set_hh_rh(__v838); }
                                                                    while (self.mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh() != 0i32) {
                                                                        self.pre_adjust_tail = self.mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.adjust_tail == 0i32) {
                                                                        self.confusion(1251i32);
                                                                    }
                                                                    { let __ix839 = self.adjust_tail; let __v840 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix839) as usize)].set_hh_rh(__v840); }
                                                                    while (self.mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh() != 0i32) {
                                                                        self.adjust_tail = self.mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            self.free_node(self.mem[crate::ix::U((q) as usize)].hh().rh(), 2i32);
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix841 = self.adjust_tail; self.mem[crate::ix::U((__ix841) as usize)].set_hh_rh(p); }
                                                            self.adjust_tail = p;
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                        }
                                                    }
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                    p = q;
                                                }
                                            }
                                        }
                                        8 => {
                                            // §1607
                                            if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == 12i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == 14i32)) {
                                                {
                                                    x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    s = 0i32;
                                                    if ((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(s) > h) {
                                                        h = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(s);
                                                    }
                                                    if ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(s) > d) {
                                                        d = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(s);
                                                    }
                                                }
                                            }
                                        }
                                        10 => {
                                            // §832
                                            {
                                                g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                                o = self.mem[crate::ix::U((g) as usize)].hh().b0();
                                                { let __v842 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v842; }
                                                o = self.mem[crate::ix::U((g) as usize)].hh().b1();
                                                { let __v843 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v843; }
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= 100i32) {
                                                    {
                                                        g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int() > h) {
                                                            h = self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int();
                                                        }
                                                        if (self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int() > d) {
                                                            d = self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        40 => {
                                            // §825
                                            {
                                                if (m == 2i32) {
                                                    {
                                                        f = self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0();
                                                        self.do_subst_font(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), 1000i32);
                                                        if (f != self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0()) {
                                                            font_stretch = ((font_stretch).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int())).wrapping_sub(self.char_pw(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        }
                                                        { let __ix844 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U((__ix844) as usize)].set_hh_b0(f); }
                                                        self.do_subst_font(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), (1000i32).wrapping_neg());
                                                        if (f != self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0()) {
                                                            font_shrink = ((font_shrink).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int())).wrapping_sub(self.char_pw(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.mem[crate::ix::U((p) as usize)].hh().b1()));
                                                        }
                                                        { let __ix845 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U((__ix845) as usize)].set_hh_b0(f); }
                                                    }
                                                } else {
                                                    if (m == 3i32) {
                                                        {
                                                            self.do_subst_font(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.font_expand_ratio);
                                                            { let __v846 = (self.char_pw(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.mem[crate::ix::U((p) as usize)].hh().b1())).wrapping_neg(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v846); }
                                                        }
                                                    }
                                                }
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        11 => {
                                            {
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32) {
                                                    {
                                                        if (m == 2i32) {
                                                            {
                                                                font_stretch = (font_stretch).wrapping_add(self.kern_stretch(p));
                                                                font_shrink = (font_shrink).wrapping_add(self.kern_shrink(p));
                                                            }
                                                        } else {
                                                            if (m == 3i32) {
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
                                                                            if (self.mem[crate::ix::U((p) as usize)].hh().rh() >= self.hi_mem_min) {
                                                                                { let __v847 = self.get_kern(self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().b0(), self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().b1(), self.mem[crate::ix::U((self.mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b1()); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v847); }
                                                                            } else {
                                                                                if (self.mem[crate::ix::U((self.mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b0() == 6i32) {
                                                                                    { let __v848 = self.get_kern(self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().b0(), self.mem[crate::ix::U((self.prev_char_p) as usize)].hh().b1(), self.mem[crate::ix::U(((self.mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().b1()); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v848); }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        9 => {
                                            {
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (self.eqtb[crate::ix::U(((629126i32) - 1) as usize)].int() > 0i32) {
                                                    // §1711
                                                    if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                        if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((4i32).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / 4i32))).wrapping_add(3i32)) {
                                                            {
                                                                self.temp_ptr = self.LR_ptr;
                                                                self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                {
                                                                    { let __ix849 = self.temp_ptr; let __v850 = self.avail; self.mem[crate::ix::U((__ix849) as usize)].set_hh_rh(__v850); }
                                                                    self.avail = self.temp_ptr;
                                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                self.mem[crate::ix::U((p) as usize)].set_hh_b0(11i32);
                                                                self.mem[crate::ix::U((p) as usize)].set_hh_b1(1i32);
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            self.temp_ptr = self.get_avail();
                                                            { let __ix851 = self.temp_ptr; let __v852 = ((4i32).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / 4i32))).wrapping_add(3i32); self.mem[crate::ix::U((__ix851) as usize)].set_hh_lh(__v852); }
                                                            { let __ix853 = self.temp_ptr; let __v854 = self.LR_ptr; self.mem[crate::ix::U((__ix853) as usize)].set_hh_rh(__v854); }
                                                            self.LR_ptr = self.temp_ptr;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        6 => {
                                            // §825
                                            {
                                                if (m == 3i32) {
                                                    self.do_subst_font(p, self.font_expand_ratio);
                                                }
                                                // §826
                                                {
                                                    { let __v855 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((4999987i32) as usize)] = __v855; }
                                                    { let __v856 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((4999987i32) as usize)].set_hh_rh(__v856); }
                                                    p = 4999987i32;
                                                    continue 'l_reswitch_b;
                                                }
                                            }
                                        }
                                        7 => {
                                            // §825
                                            if (m == 3i32) {
                                                self.do_subst_font(p, self.font_expand_ratio);
                                            }
                                        }
                                        _ => {
                                        }
                                    }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            break 'l_reswitch_b;
                        }
                    }
                }
                // §823
                if (self.adjust_tail != 0i32) {
                    { let __ix857 = self.adjust_tail; self.mem[crate::ix::U((__ix857) as usize)].set_hh_rh(0i32); }
                }
                if (self.pre_adjust_tail != 0i32) {
                    { let __ix858 = self.pre_adjust_tail; self.mem[crate::ix::U((__ix858) as usize)].set_hh_rh(0i32); }
                }
                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                // §833
                if (m == 1i32) {
                    w = (x).wrapping_add(w);
                }
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                x = (w).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(0i32);
                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        { __goto_1 = 2; continue 'l_dispatch_1; }
                    }
                } else {
                    if (x > 0i32) {
                        // §834
                        {
                            // §835
                            if (self.total_stretch[crate::ix::U((3i32) as usize)] != 0i32) {
                                o = 3i32;
                            } else {
                                if (self.total_stretch[crate::ix::U((2i32) as usize)] != 0i32) {
                                    o = 2i32;
                                } else {
                                    if (self.total_stretch[crate::ix::U((1i32) as usize)] != 0i32) {
                                        o = 1i32;
                                    } else {
                                        o = 0i32;
                                    }
                                }
                            }
                            // §834
                            if (((m == 2i32) && (o == 0i32)) && (font_stretch > 0i32)) {
                                {
                                    self.font_expand_ratio = self.divide_scaled(x, font_stretch, 3i32);
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(1i32);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v859 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v859); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == 0i32) {
                                if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32) {
                                    // §836
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((0i32) as usize)]);
                                        if (self.last_badness > self.eqtb[crate::ix::U(((629044i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(1252i32);
                                                } else {
                                                    self.print_nl(1253i32);
                                                }
                                                self.print(1254i32);
                                                self.print_int(((self.last_badness) as i64));
                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                            }
                                        }
                                    }
                                }
                            }
                            // §834
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    } else {
                        // §840
                        {
                            // §841
                            if (self.total_shrink[crate::ix::U((3i32) as usize)] != 0i32) {
                                o = 3i32;
                            } else {
                                if (self.total_shrink[crate::ix::U((2i32) as usize)] != 0i32) {
                                    o = 2i32;
                                } else {
                                    if (self.total_shrink[crate::ix::U((1i32) as usize)] != 0i32) {
                                        o = 1i32;
                                    } else {
                                        o = 0i32;
                                    }
                                }
                            }
                            // §840
                            if (((m == 2i32) && (o == 0i32)) && (font_shrink > 0i32)) {
                                {
                                    self.font_expand_ratio = self.divide_scaled(x, font_shrink, 3i32);
                                    { __goto_1 = 2; continue 'l_dispatch_1; }
                                }
                            }
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(2i32);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v860 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v860); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32)) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §842
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((0i32) as usize)]) > self.eqtb[crate::ix::U(((629648i32) - 1) as usize)].int()) || (self.eqtb[crate::ix::U(((629044i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            if ((self.eqtb[crate::ix::U(((629656i32) - 1) as usize)].int() > 0i32) && (((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((0i32) as usize)]) > self.eqtb[crate::ix::U(((629648i32) - 1) as usize)].int())) {
                                                {
                                                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != 0i32) {
                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    { let __v861 = self.new_rule(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v861); }
                                                    { let __ix862 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); let __v863 = self.eqtb[crate::ix::U(((629656i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix862) as usize)].set_int(__v863); }
                                                }
                                            }
                                            self.print_ln();
                                            self.print_nl(1260i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((0i32) as usize)]));
                                            self.print(1261i32);
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            } else {
                                // §840
                                if (o == 0i32) {
                                    if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32) {
                                        // §843
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((0i32) as usize)]);
                                            if (self.last_badness > self.eqtb[crate::ix::U(((629044i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(1262i32);
                                                    self.print_int(((self.last_badness) as i64));
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §840
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // common_ending
                // §823
                if self.output_active {
                    // §839
                    self.print(1255i32);
                } else {
                    {
                        if (self.pack_begin_line != 0i32) {
                            {
                                if (self.pack_begin_line > 0i32) {
                                    self.print(1256i32);
                                } else {
                                    self.print(1257i32);
                                }
                                self.print_int((((self.pack_begin_line).wrapping_abs()) as i64));
                                self.print(1258i32);
                            }
                        } else {
                            self.print(1259i32);
                        }
                        self.print_int(((self.line) as i64));
                    }
                }
                self.print_ln();
                self.font_in_short_display = 0i32;
                self.short_display(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh());
                self.print_ln();
                self.begin_diagnostic();
                self.show_box(r);
                self.end_diagnostic(true);
            }
            if __goto_1 <= 2 { // exit
                // §823
                if (self.eqtb[crate::ix::U(((629126i32) - 1) as usize)].int() > 0i32) {
                    // §1712
                    {
                        if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != 0i32) {
                            {
                                while (self.mem[crate::ix::U((q) as usize)].hh().rh() != 0i32) {
                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                }
                                loop {
                                    self.temp_ptr = q;
                                    q = self.new_math(0i32, self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh());
                                    { let __ix864 = self.temp_ptr; self.mem[crate::ix::U((__ix864) as usize)].set_hh_rh(q); }
                                    self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                        {
                                            { let __ix865 = self.temp_ptr; let __v866 = self.avail; self.mem[crate::ix::U((__ix865) as usize)].set_hh_rh(__v866); }
                                            self.avail = self.temp_ptr;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                    if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == 0i32) { break; }
                                }
                            }
                        }
                        if (self.LR_problems > 0i32) {
                            {
                                // §1713
                                {
                                    self.print_ln();
                                    self.print_nl(2022i32);
                                    self.print_int((((self.LR_problems / 10000i32)) as i64));
                                    self.print(2023i32);
                                    self.print_int((((self.LR_problems % 10000i32)) as i64));
                                    self.print(2024i32);
                                    self.LR_problems = 0i32;
                                }
                                // §1712
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                        {
                            self.temp_ptr = self.LR_ptr;
                            self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                            {
                                { let __ix867 = self.temp_ptr; let __v868 = self.avail; self.mem[crate::ix::U((__ix867) as usize)].set_hh_rh(__v868); }
                                self.avail = self.temp_ptr;
                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                            }
                        }
                        if (self.LR_ptr != 0i32) {
                            self.confusion(2021i32);
                        }
                    }
                }
                // §823
                if ((m == 2i32) && (self.font_expand_ratio != 0i32)) {
                    {
                        self.font_expand_ratio = self.fix_int(self.font_expand_ratio, (1000i32).wrapping_neg(), 1000i32);
                        q = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh();
                        self.free_node(r, 7i32);
                        r = self.hpack(q, w, 3i32);
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
    // §844
    pub fn vpackage(&mut self, mut p: halfword, mut h: scaled, mut m: small_number, mut l: scaled) -> halfword {
        let mut vpackage: halfword = 0;
        let mut r: halfword = 0; // §844
        let mut w: scaled = 0; // §844
        let mut d: scaled = 0; // §844
        let mut x: scaled = 0; // §844
        let mut s: scaled = 0; // §844
        let mut g: halfword = 0; // §844
        let mut o: glue_ord = 0; // §844
        'l_exit_f: {
            'l_common_ending_f: {
                self.last_badness = 0i32;
                r = self.get_node(7i32);
                self.mem[crate::ix::U((r) as usize)].set_hh_b0(1i32);
                self.mem[crate::ix::U((r) as usize)].set_hh_b1(0i32);
                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                w = 0i32;
                // §824
                d = 0i32;
                x = 0i32;
                self.total_stretch[crate::ix::U((0i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((0i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((1i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((1i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((2i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((2i32) as usize)] = 0i32;
                self.total_stretch[crate::ix::U((3i32) as usize)] = 0i32;
                self.total_shrink[crate::ix::U((3i32) as usize)] = 0i32;
                // §844
                while (p != 0i32) {
                    // §845
                    {
                        if (p >= self.hi_mem_min) {
                            self.confusion(1263i32);
                        } else {
                            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                0 | 1 | 2 | 13 => {
                                    // §846
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                        d = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= 2i32) {
                                            s = 0i32;
                                        } else {
                                            s = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int();
                                        }
                                        if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s) > w) {
                                            w = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s);
                                        }
                                    }
                                }
                                8 => {
                                    // §1606
                                    if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == 12i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == 14i32)) {
                                        {
                                            x = ((x).wrapping_add(d)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            d = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                            s = 0i32;
                                            if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s) > w) {
                                                w = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s);
                                            }
                                        }
                                    }
                                }
                                10 => {
                                    // §847
                                    {
                                        x = (x).wrapping_add(d);
                                        d = 0i32;
                                        g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                        x = (x).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                        o = self.mem[crate::ix::U((g) as usize)].hh().b0();
                                        { let __v869 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v869; }
                                        o = self.mem[crate::ix::U((g) as usize)].hh().b1();
                                        { let __v870 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v870; }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= 100i32) {
                                            {
                                                g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                if (self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int() > w) {
                                                    w = self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int();
                                                }
                                            }
                                        }
                                    }
                                }
                                11 => {
                                    // §845
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        d = 0i32;
                                    }
                                }
                                _ => {
                                }
                            }
                        }
                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
                // §844
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                if (d > l) {
                    {
                        x = ((x).wrapping_add(d)).wrapping_sub(l);
                        self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(l);
                    }
                } else {
                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                }
                // §848
                if (m == 1i32) {
                    h = (x).wrapping_add(h);
                }
                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                x = (h).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(0i32);
                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        break 'l_exit_f;
                    }
                } else {
                    if (x > 0i32) {
                        // §849
                        {
                            // §835
                            if (self.total_stretch[crate::ix::U((3i32) as usize)] != 0i32) {
                                o = 3i32;
                            } else {
                                if (self.total_stretch[crate::ix::U((2i32) as usize)] != 0i32) {
                                    o = 2i32;
                                } else {
                                    if (self.total_stretch[crate::ix::U((1i32) as usize)] != 0i32) {
                                        o = 1i32;
                                    } else {
                                        o = 0i32;
                                    }
                                }
                            }
                            // §849
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(1i32);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v871 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v871); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == 0i32) {
                                if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32) {
                                    // §850
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((0i32) as usize)]);
                                        if (self.last_badness > self.eqtb[crate::ix::U(((629045i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(1252i32);
                                                } else {
                                                    self.print_nl(1253i32);
                                                }
                                                self.print(1264i32);
                                                self.print_int(((self.last_badness) as i64));
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                            // §849
                            break 'l_exit_f;
                        }
                    } else {
                        // §852
                        {
                            // §841
                            if (self.total_shrink[crate::ix::U((3i32) as usize)] != 0i32) {
                                o = 3i32;
                            } else {
                                if (self.total_shrink[crate::ix::U((2i32) as usize)] != 0i32) {
                                    o = 2i32;
                                } else {
                                    if (self.total_shrink[crate::ix::U((1i32) as usize)] != 0i32) {
                                        o = 1i32;
                                    } else {
                                        o = 0i32;
                                    }
                                }
                            }
                            // §852
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(2i32);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v872 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v872); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(0i32);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == 0i32)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32)) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §853
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((0i32) as usize)]) > self.eqtb[crate::ix::U(((629649i32) - 1) as usize)].int()) || (self.eqtb[crate::ix::U(((629045i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            self.print_ln();
                                            self.print_nl(1265i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((0i32) as usize)]));
                                            self.print(1266i32);
                                            break 'l_common_ending_f;
                                        }
                                    }
                                }
                            } else {
                                // §852
                                if (o == 0i32) {
                                    if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != 0i32) {
                                        // §854
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((0i32) as usize)]);
                                            if (self.last_badness > self.eqtb[crate::ix::U(((629045i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(1267i32);
                                                    self.print_int(((self.last_badness) as i64));
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §852
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §844
            if self.output_active {
                // §851
                self.print(1255i32);
            } else {
                {
                    if (self.pack_begin_line != 0i32) {
                        {
                            self.print(1257i32);
                            self.print_int((((self.pack_begin_line).wrapping_abs()) as i64));
                            self.print(1258i32);
                        }
                    } else {
                        self.print(1259i32);
                    }
                    self.print_int(((self.line) as i64));
                    self.print_ln();
                }
            }
            self.begin_diagnostic();
            self.show_box(r);
            self.end_diagnostic(true);
        }
        // §844
        vpackage = r;
        vpackage
    }

    /// When a box is being appended to the current vertical list, the
    /// baselineskip calculation is handled by the `append_to_vlist` routine.
    // §855
    pub fn append_to_vlist(&mut self, mut b: halfword) {
        let mut d: scaled = 0; // §855
        let mut p: halfword = 0; // §855
        if (self.cur_list.aux_field.int() > self.eqtb[crate::ix::U(((629672i32) - 1) as usize)].int()) {
            {
                d = ((self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((626629i32) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_list.aux_field.int())).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int());
                if (d < self.eqtb[crate::ix::U(((629642i32) - 1) as usize)].int()) {
                    p = self.new_param_glue(0i32);
                } else {
                    {
                        p = self.new_skip_param(1i32);
                        { let __ix873 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix873) as usize)].set_int(d); }
                    }
                }
                { let __ix874 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix874) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
        { let __ix875 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix875) as usize)].set_hh_rh(b); }
        self.cur_list.tail_field = b;
        { let __v876 = self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int(); self.cur_list.aux_field.set_int(__v876); }
    }

    /// The `new_noad` function creates an `ord_noad` that is completely null.
    // §862
    pub fn new_noad(&mut self) -> halfword {
        let mut new_noad: halfword = 0;
        let mut p: halfword = 0; // §862
        p = self.get_node(4i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(16i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        { let __v877 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh(__v877); }
        { let __v878 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh(__v878); }
        { let __v879 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh(__v879); }
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
    // §864
    pub fn new_style(&mut self, mut s: small_number) -> halfword {
        let mut new_style: halfword = 0;
        let mut p: halfword = 0; // §864
        p = self.get_node(3i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(14i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        new_style = p;
        new_style
    }

    /// Finally, the \.{\\mathchoice} primitive creates a `choice_node`, which
    /// has special subfields `display_mlist`, `text_mlist`, `script_mlist`,
    /// and `script_script_mlist` pointing to the mlists for each style.
    // §865
    pub fn new_choice(&mut self) -> halfword {
        let mut new_choice: halfword = 0;
        let mut p: halfword = 0; // §865
        p = self.get_node(3i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(15i32);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh(0i32);
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
    // §869
    pub fn show_info(&mut self) {
        self.show_node_list(self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().lh());
    }

    /// Here is a function that returns a pointer to a rule node having a given
    /// thickness `t`. The rule will extend horizontally to the boundary of the vlist
    /// that eventually contains it.
    // §880
    pub fn fraction_rule(&mut self, mut t: scaled) -> halfword {
        let mut fraction_rule: halfword = 0;
        let mut p: halfword = 0; // §880
        p = self.new_rule();
        self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(t);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        fraction_rule = p;
        fraction_rule
    }

    /// The `overbar` function returns a pointer to a vlist box that consists of
    /// a given box `b`, above which has been placed a kern of height `k` under a
    /// fraction rule of thickness `t` under additional space of height `t`.
    // §881
    pub fn overbar(&mut self, mut b: halfword, mut k: scaled, mut t: scaled) -> halfword {
        let mut overbar: halfword = 0;
        let mut p: halfword = 0; // §881
        let mut q: halfword = 0; // §881
        p = self.new_kern(k);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(b);
        q = self.fraction_rule(t);
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
        p = self.new_kern(t);
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
        overbar = self.vpackage(p, 0i32, 1i32, 1073741823i32);
        overbar
    }

    /// Here is a subroutine that creates a new box, whose list contains a
    /// single character, and whose width includes the italic correction for
    /// that character. The height or depth of the box will be negative, if
    /// the height or depth of the character is negative; thus, this routine
    /// may deliver a slightly different result than `hpack` would produce.
    /// @<Declare subprocedures for `var_delimiter`
    // §885
    pub fn char_box(&mut self, mut f: internal_font_number, mut c: quarterword) -> halfword {
        let mut char_box: halfword = 0;
        let mut q: four_quarters = four_quarters::default(); // §885
        let mut hd: eight_bits = 0; // §885
        let mut b: halfword = 0; // §885
        let mut p: halfword = 0; // §885
        q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq();
        hd = (q.b1()).wrapping_sub(0i32);
        b = self.new_null_box();
        { let __v880 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(((q.b2()).wrapping_sub(0i32) / 4i32))) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v880); }
        { let __v881 = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add((hd / 16i32))) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v881); }
        { let __v882 = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add((hd % 16i32))) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v882); }
        p = self.get_avail();
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(c);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(f);
        self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        char_box = b;
        char_box
    }

    /// When we build an extensible character, it's handy to have the
    /// following subroutine, which puts a given character on top
    /// of the characters already in box `b`:
    /// @<Declare subprocedures for `var_delimiter`
    // §887
    pub fn stack_into_box(&mut self, mut b: halfword, mut f: internal_font_number, mut c: quarterword) {
        let mut p: halfword = 0; // §887
        p = self.char_box(f, c);
        { let __v883 = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v883); }
        self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].set_hh_rh(p);
        { let __v884 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].set_int(__v884); }
    }

    /// Another handy subroutine computes the height plus depth of
    /// a given character:
    /// @<Declare subprocedures for `var_delimiter`
    // §888
    pub fn height_plus_depth(&mut self, mut f: internal_font_number, mut c: quarterword) -> scaled {
        let mut height_plus_depth: scaled = 0;
        let mut q: four_quarters = four_quarters::default(); // §888
        let mut hd: eight_bits = 0; // §888
        q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq();
        hd = (q.b1()).wrapping_sub(0i32);
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
    // §882
    pub fn var_delimiter(&mut self, mut d: halfword, mut s: small_number, mut v: scaled) -> halfword {
        let mut var_delimiter: halfword = 0;
        let mut b: halfword = 0; // §882
        let mut f: internal_font_number = 0; // §882
        let mut g: internal_font_number = 0; // §882
        let mut c: quarterword = 0; // §882
        let mut x: quarterword = 0; // §882
        let mut y: quarterword = 0; // §882
        let mut m: i32 = 0; // §882
        let mut n: i32 = 0; // §882
        let mut u: scaled = 0; // §882
        let mut w: scaled = 0; // §882
        let mut q: four_quarters = four_quarters::default(); // §882
        let mut hd: eight_bits = 0; // §882
        let mut r: four_quarters = four_quarters::default(); // §882
        let mut z: small_number = 0; // §882
        let mut large_attempt: bool = false; // §882
        'l_found_f: {
            f = 0i32;
            w = 0i32;
            large_attempt = false;
            z = self.mem[crate::ix::U((d) as usize)].qqqq().b0();
            x = self.mem[crate::ix::U((d) as usize)].qqqq().b1();
            while true {
                {
                    // §883
                    if ((z != 0i32) || (x != 0i32)) {
                        {
                            z = ((z).wrapping_add(s)).wrapping_add(16i32);
                            loop {
                                z = (z).wrapping_sub(16i32);
                                g = self.eqtb[crate::ix::U((((627690i32).wrapping_add(z)) - 1) as usize)].hh().rh();
                                if (g != 0i32) {
                                    // §884
                                    {
                                        y = x;
                                        if (((y).wrapping_sub(0i32) >= self.font_bc[crate::ix::U((g) as usize)]) && ((y).wrapping_sub(0i32) <= self.font_ec[crate::ix::U((g) as usize)])) {
                                            {
                                                'l_continue_b: loop {
                                                    q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((g) as usize)]).wrapping_add(y)) as usize)].qqqq();
                                                    if (q.b0() > 0i32) {
                                                        {
                                                            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 3i32) {
                                                                {
                                                                    f = g;
                                                                    c = y;
                                                                    break 'l_found_f;
                                                                }
                                                            }
                                                            hd = (q.b1()).wrapping_sub(0i32);
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
                                                            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 2i32) {
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
                    // §882
                    if large_attempt {
                        break 'l_found_f;
                    }
                    large_attempt = true;
                    z = self.mem[crate::ix::U((d) as usize)].qqqq().b2();
                    x = self.mem[crate::ix::U((d) as usize)].qqqq().b3();
                }
            }
        }
        if (f != 0i32) {
            // §886
            if (((q.b2()).wrapping_sub(0i32) % 4i32) == 3i32) {
                // §889
                {
                    b = self.new_null_box();
                    self.mem[crate::ix::U((b) as usize)].set_hh_b0(1i32);
                    r = self.font_info[crate::ix::U(((self.exten_base[crate::ix::U((f) as usize)]).wrapping_add(q.b3())) as usize)].qqqq();
                    // §890
                    c = r.b3();
                    u = self.height_plus_depth(f, c);
                    w = 0i32;
                    q = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq();
                    { let __v885 = (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(q.b0())) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(((q.b2()).wrapping_sub(0i32) / 4i32))) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v885); }
                    c = r.b2();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    c = r.b1();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    c = r.b0();
                    if (c != 0i32) {
                        w = (w).wrapping_add(self.height_plus_depth(f, c));
                    }
                    n = 0i32;
                    if (u > 0i32) {
                        while (w < v) {
                            {
                                w = (w).wrapping_add(u);
                                n = (n).wrapping_add(1i32);
                                if (r.b1() != 0i32) {
                                    w = (w).wrapping_add(u);
                                }
                            }
                        }
                    }
                    // §889
                    c = r.b2();
                    if (c != 0i32) {
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
                    if (c != 0i32) {
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
                    if (c != 0i32) {
                        self.stack_into_box(b, f, c);
                    }
                    { let __v886 = (w).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].set_int(__v886); }
                }
            } else {
                // §886
                b = self.char_box(f, c);
            }
        } else {
            // §882
            {
                b = self.new_null_box();
                { let __v887 = self.eqtb[crate::ix::U(((629651i32) - 1) as usize)].int(); self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].set_int(__v887); }
            }
        }
        { let __v888 = (self.half((self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(s)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); self.mem[crate::ix::U(((b).wrapping_add(4i32)) as usize)].set_int(__v888); }
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
    // §891
    pub fn rebox(&mut self, mut b: halfword, mut w: scaled) -> halfword {
        let mut rebox: halfword = 0;
        let mut p: halfword = 0; // §891
        let mut f: internal_font_number = 0; // §891
        let mut v: scaled = 0; // §891
        if ((self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int() != w) && (self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh() != 0i32)) {
            {
                if (self.mem[crate::ix::U((b) as usize)].hh().b0() == 1i32) {
                    b = self.hpack(b, 0i32, 1i32);
                }
                p = self.mem[crate::ix::U(((b).wrapping_add(5i32)) as usize)].hh().rh();
                if ((p >= self.hi_mem_min) && (self.mem[crate::ix::U((p) as usize)].hh().rh() == 0i32)) {
                    {
                        f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                        v = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().b1())) as usize)].qqqq().b0())) as usize)].int();
                        if (v != self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()) {
                            { let __v889 = self.new_kern((self.mem[crate::ix::U(((b).wrapping_add(1i32)) as usize)].int()).wrapping_sub(v)); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v889); }
                        }
                    }
                }
                self.free_node(b, 7i32);
                b = self.new_glue(12i32);
                self.mem[crate::ix::U((b) as usize)].set_hh_rh(p);
                while (self.mem[crate::ix::U((p) as usize)].hh().rh() != 0i32) {
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
                { let __v890 = self.new_glue(12i32); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v890); }
                rebox = self.hpack(b, w, 0i32);
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
    // §892
    pub fn math_glue(&mut self, mut g: halfword, mut m: scaled) -> halfword {
        let mut math_glue: halfword = 0;
        let mut p: halfword = 0; // §892
        let mut n: i32 = 0; // §892
        let mut f: scaled = 0; // §892
        n = self.x_over_n(m, 65536i32);
        f = self.remainder;
        if (f < 0i32) {
            {
                n = (n).wrapping_sub(1i32);
                f = (f).wrapping_add(65536i32);
            }
        }
        p = self.get_node(4i32);
        { let __v891 = { let __a892_0 = n; let __a892_1 = self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(); let __a892_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a892_3 = 1073741823i32; self.mult_and_add(__a892_0, __a892_1, __a892_2, __a892_3) }; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v891); }
        { let __v893 = self.mem[crate::ix::U((g) as usize)].hh().b0(); self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v893); }
        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == 0i32) {
            { let __v894 = { let __a895_0 = n; let __a895_1 = self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); let __a895_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(), f, 65536i32); let __a895_3 = 1073741823i32; self.mult_and_add(__a895_0, __a895_1, __a895_2, __a895_3) }; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v894); }
        } else {
            { let __v896 = self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v896); }
        }
        { let __v897 = self.mem[crate::ix::U((g) as usize)].hh().b1(); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v897); }
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32) {
            { let __v898 = { let __a899_0 = n; let __a899_1 = self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); let __a899_2 = self.xn_over_d(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(), f, 65536i32); let __a899_3 = 1073741823i32; self.mult_and_add(__a899_0, __a899_1, __a899_2, __a899_3) }; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v898); }
        } else {
            { let __v900 = self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_int(__v900); }
        }
        math_glue = p;
        math_glue
    }

    /// The `math_kern` subroutine removes `mu_glue` from a kern node, given
    /// the value of the math unit.
    // §893
    pub fn math_kern(&mut self, mut p: halfword, mut m: scaled) {
        let mut n: i32 = 0; // §893
        let mut f: scaled = 0; // §893
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == 99i32) {
            {
                n = self.x_over_n(m, 65536i32);
                f = self.remainder;
                if (f < 0i32) {
                    {
                        n = (n).wrapping_sub(1i32);
                        f = (f).wrapping_add(65536i32);
                    }
                }
                { let __v901 = { let __a902_0 = n; let __a902_1 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); let __a902_2 = self.xn_over_d(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(), f, 65536i32); let __a902_3 = 1073741823i32; self.mult_and_add(__a902_0, __a902_1, __a902_2, __a902_3) }; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v901); }
                self.mem[crate::ix::U((p) as usize)].set_hh_b1(1i32);
            }
        }
    }

    /// Sometimes it is necessary to destroy an mlist. The following
    /// subroutine empties the current list, assuming that `abs(mode)=mmode`.
    // §894
    pub fn flush_math(&mut self) {
        self.flush_node_list(self.mem[crate::ix::U((self.cur_list.head_field) as usize)].hh().rh());
        self.flush_node_list(self.cur_list.aux_field.int());
        { let __ix903 = self.cur_list.head_field; self.mem[crate::ix::U((__ix903) as usize)].set_hh_rh(0i32); }
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.aux_field.set_int(0i32);
    }

    /// The recursion in `mlist_to_hlist` is due primarily to a subroutine
    /// called `clean_box` that puts a given noad field into a box using a given
    /// math style; `mlist_to_hlist` can call `clean_box`, which can call
    /// `mlist_to_hlist`.
    /// The box returned by `clean_box` is ``clean'' in the
    /// sense that its `shift_amount` is zero.
    // §896
    pub fn clean_box(&mut self, mut p: halfword, mut s: small_number) -> halfword {
        let mut clean_box: halfword = 0;
        let mut q: halfword = 0; // §896
        let mut save_style: small_number = 0; // §896
        let mut x: halfword = 0; // §896
        let mut r: halfword = 0; // §896
        'l_found_f: {
            match self.mem[crate::ix::U((p) as usize)].hh().rh() {
                1 => {
                    {
                        self.cur_mlist = self.new_noad();
                        { let __ix904 = (self.cur_mlist).wrapping_add(1i32); let __v905 = self.mem[crate::ix::U((p) as usize)]; self.mem[crate::ix::U((__ix904) as usize)] = __v905; }
                    }
                }
                2 => {
                    {
                        q = self.mem[crate::ix::U((p) as usize)].hh().lh();
                        break 'l_found_f;
                    }
                }
                3 => {
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
            q = self.mem[crate::ix::U((4999996i32) as usize)].hh().rh();
            self.cur_style = save_style;
            // §879
            {
                if (self.cur_style < 4i32) {
                    self.cur_size = 0i32;
                } else {
                    self.cur_size = (16i32).wrapping_mul(((self.cur_style).wrapping_sub(2i32) / 2i32));
                }
                self.cur_mu = self.x_over_n(self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(), 18i32);
            }
        }
        // §896
        if ((q >= self.hi_mem_min) || (q == 0i32)) {
            x = self.hpack(q, 0i32, 1i32);
        } else {
            if (((self.mem[crate::ix::U((q) as usize)].hh().rh() == 0i32) && (self.mem[crate::ix::U((q) as usize)].hh().b0() <= 1i32)) && (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].int() == 0i32)) {
                x = q;
            } else {
                x = self.hpack(q, 0i32, 1i32);
            }
        }
        // §897
        q = self.mem[crate::ix::U(((x).wrapping_add(5i32)) as usize)].hh().rh();
        if (q >= self.hi_mem_min) {
            {
                r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                if (r != 0i32) {
                    if (self.mem[crate::ix::U((r) as usize)].hh().rh() == 0i32) {
                        if (!(r >= self.hi_mem_min)) {
                            if (self.mem[crate::ix::U((r) as usize)].hh().b0() == 11i32) {
                                {
                                    self.free_node(r, 2i32);
                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(0i32);
                                }
                            }
                        }
                    }
                }
            }
        }
        // §896
        clean_box = x;
        clean_box
    }

    /// It is convenient to have a procedure that converts a `math_char`
    /// field to an ``unpacked'' form. The `fetch` routine sets `cur_f`, `cur_c`,
    /// and `cur_i` to the font code, character code, and character information bytes of
    /// a given noad field. It also takes care of issuing error messages for
    /// nonexistent characters; in such cases, `char_exists(cur_i)` will be `false`
    /// after `fetch` has acted, and the field will also have been reset to `empty`.
    // §898
    pub fn fetch(&mut self, mut a: halfword) {
        self.cur_c = self.mem[crate::ix::U((a) as usize)].hh().b1();
        self.cur_f = self.eqtb[crate::ix::U(((((627690i32).wrapping_add(self.mem[crate::ix::U((a) as usize)].hh().b0())).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh();
        if (self.cur_f == 0i32) {
            // §899
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
                self.print_size(self.cur_size);
                self.print_char(32i32);
                self.print_int(((self.mem[crate::ix::U((a) as usize)].hh().b0()) as i64));
                self.print(1293i32);
                self.print((self.cur_c).wrapping_sub(0i32));
                self.print_char(41i32);
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 1294i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 1295i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 1296i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1297i32;
                }
                self.error();
                self.cur_i = self.null_character;
                self.mem[crate::ix::U((a) as usize)].set_hh_rh(0i32);
            }
        } else {
            // §898
            {
                if (((self.cur_c).wrapping_sub(0i32) >= self.font_bc[crate::ix::U((self.cur_f) as usize)]) && ((self.cur_c).wrapping_sub(0i32) <= self.font_ec[crate::ix::U((self.cur_f) as usize)])) {
                    self.cur_i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_c)) as usize)].qqqq();
                } else {
                    self.cur_i = self.null_character;
                }
                if (!(self.cur_i.b0() > 0i32)) {
                    {
                        self.char_warning(self.cur_f, (self.cur_c).wrapping_sub(0i32));
                        self.mem[crate::ix::U((a) as usize)].set_hh_rh(0i32);
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
    // §910
    pub fn make_over(&mut self, mut q: halfword) {
        { let __v906 = { let __a907_0 = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32)); let __a907_1 = (3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); let __a907_2 = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(); self.overbar(__a907_0, __a907_1, __a907_2) }; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v906); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
    }

    /// @<Declare math...
    // §911
    pub fn make_under(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §911
        let mut x: halfword = 0; // §911
        let mut y: halfword = 0; // §911
        let mut delta: scaled = 0; // §911
        x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
        p = self.new_kern((3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()));
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        { let __v908 = self.fraction_rule(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v908); }
        y = self.vpackage(x, 0i32, 1i32, 1073741823i32);
        delta = ((self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
        { let __v909 = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(__v909); }
        { let __v910 = (delta).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].set_int(__v910); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
    }

    /// @<Declare math...
    // §912
    pub fn make_vcenter(&mut self, mut q: halfword) {
        let mut v: halfword = 0; // §912
        let mut delta: scaled = 0; // §912
        v = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
        if (self.mem[crate::ix::U((v) as usize)].hh().b0() != 1i32) {
            self.confusion(618i32);
        }
        delta = (self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int());
        { let __v911 = (self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(self.half(delta)); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v911); }
        { let __v912 = (delta).wrapping_sub(self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v912); }
    }

    /// According to the rules in the \.{DVI} file specifications, we ensure alignment
    /// between a square root sign and the rule above its nucleus by assuming that the
    /// baseline of the square-root symbol is the same as the bottom of the rule. The
    /// height of the square-root symbol will be the thickness of the rule, and the
    /// depth of the square-root symbol should exceed or equal the height-plus-depth
    /// of the nucleus plus a certain minimum clearance~`clr`. The symbol will be
    /// placed so that the actual clearance is `clr` plus half the excess.
    /// @<Declare math...
    // §913
    pub fn make_radical(&mut self, mut q: halfword) {
        let mut x: halfword = 0; // §913
        let mut y: halfword = 0; // §913
        let mut delta: scaled = 0; // §913
        let mut clr: scaled = 0; // §913
        x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
        if (self.cur_style < 2i32) {
            clr = (self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(((self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_abs() / 4i32));
        } else {
            {
                clr = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                clr = (clr).wrapping_add(((clr).wrapping_abs() / 4i32));
            }
        }
        y = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, (((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr)).wrapping_add(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()));
        delta = (self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int()).wrapping_sub(((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(clr));
        if (delta > 0i32) {
            clr = (clr).wrapping_add(self.half(delta));
        }
        { let __v913 = ((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_add(clr)).wrapping_neg(); self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v913); }
        { let __v914 = self.overbar(x, clr, self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U((y) as usize)].set_hh_rh(__v914); }
        { let __v915 = self.hpack(y, 0i32, 1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v915); }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
    }

    /// Slants are not considered when placing accents in math mode. The accenter is
    /// centered over the accentee, and the accent width is treated as zero with
    /// respect to the size of the final box.
    /// @<Declare math...
    // §914
    pub fn make_math_accent(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §914
        let mut x: halfword = 0; // §914
        let mut y: halfword = 0; // §914
        let mut a: i32 = 0; // §914
        let mut c: quarterword = 0; // §914
        let mut f: internal_font_number = 0; // §914
        let mut i: four_quarters = four_quarters::default(); // §914
        let mut s: scaled = 0; // §914
        let mut h: scaled = 0; // §914
        let mut delta: scaled = 0; // §914
        let mut w: scaled = 0; // §914
        self.fetch((q).wrapping_add(4i32));
        if (self.cur_i.b0() > 0i32) {
            {
                'l_done_f: {
                    'l_done1_f: {
                        i = self.cur_i;
                        c = self.cur_c;
                        f = self.cur_f;
                        // §917
                        s = 0i32;
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == 1i32) {
                            {
                                self.fetch((q).wrapping_add(1i32));
                                if (((self.cur_i.b2()).wrapping_sub(0i32) % 4i32) == 1i32) {
                                    {
                                        a = (self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(self.cur_i.b3());
                                        self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                        if (self.cur_i.b0() > 128i32) {
                                            {
                                                a = ((((self.lig_kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())).wrapping_add(32768i32)).wrapping_sub((256i32).wrapping_mul(128i32));
                                                self.cur_i = self.font_info[crate::ix::U((a) as usize)].qqqq();
                                            }
                                        }
                                        while true {
                                            {
                                                if ((self.cur_i.b1()).wrapping_sub(0i32) == self.skew_char[crate::ix::U((self.cur_f) as usize)]) {
                                                    {
                                                        if (self.cur_i.b2() >= 128i32) {
                                                            if (self.cur_i.b0() <= 128i32) {
                                                                s = self.font_info[crate::ix::U((((self.kern_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add((256i32).wrapping_mul(self.cur_i.b2()))).wrapping_add(self.cur_i.b3())) as usize)].int();
                                                            }
                                                        }
                                                        break 'l_done1_f;
                                                    }
                                                }
                                                if (self.cur_i.b0() >= 128i32) {
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
                    // §914
                    x = self.clean_box((q).wrapping_add(1i32), ((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(1i32));
                    w = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int();
                    h = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                    // §916
                    while true {
                        {
                            if (((i.b2()).wrapping_sub(0i32) % 4i32) != 2i32) {
                                break 'l_done_f;
                            }
                            y = i.b3();
                            i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(y)) as usize)].qqqq();
                            if (!(i.b0() > 0i32)) {
                                break 'l_done_f;
                            }
                            if (self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(i.b0())) as usize)].int() > w) {
                                break 'l_done_f;
                            }
                            c = y;
                        }
                    }
                }
                // §914
                if (h < self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int()) {
                    delta = h;
                } else {
                    delta = self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
                }
                if ((self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() != 0i32) || (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != 0i32)) {
                    if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == 1i32) {
                        // §918
                        {
                            self.flush_node_list(x);
                            x = self.new_noad();
                            { let __v916 = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)] = __v916; }
                            { let __v917 = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)] = __v917; }
                            { let __v918 = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)]; self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)] = __v918; }
                            { let __v919 = self.empty_field; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_hh(__v919); }
                            { let __v920 = self.empty_field; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_hh(__v920); }
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(3i32);
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
                            x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                            delta = ((delta).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_sub(h);
                            h = self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int();
                        }
                    }
                }
                // §914
                y = self.char_box(f, c);
                { let __v921 = (s).wrapping_add(self.half((w).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int()))); self.mem[crate::ix::U(((y).wrapping_add(4i32)) as usize)].set_int(__v921); }
                self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(0i32);
                p = self.new_kern((delta).wrapping_neg());
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                y = self.vpackage(y, 0i32, 1i32, 1073741823i32);
                { let __v922 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].set_int(__v922); }
                if (self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int() < h) {
                    // §915
                    {
                        p = self.new_kern((h).wrapping_sub(self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int()));
                        { let __v923 = self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v923); }
                        self.mem[crate::ix::U(((y).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].set_int(h);
                    }
                }
                // §914
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(y);
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
            }
        }
    }

    /// The `make_fraction` procedure is a bit different because it sets
    /// `new_hlist(q)` directly rather than making a sub-box.
    /// @<Declare math...
    // §919
    pub fn make_fraction(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §919
        let mut v: halfword = 0; // §919
        let mut x: halfword = 0; // §919
        let mut y: halfword = 0; // §919
        let mut z: halfword = 0; // §919
        let mut delta: scaled = 0; // §919
        let mut delta1: scaled = 0; // §919
        let mut delta2: scaled = 0; // §919
        let mut shift_up: scaled = 0; // §919
        let mut shift_down: scaled = 0; // §919
        let mut clr: scaled = 0; // §919
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 1073741824i32) {
            { let __v924 = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v924); }
        }
        // §920
        x = self.clean_box((q).wrapping_add(2i32), ((self.cur_style).wrapping_add(2i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        z = self.clean_box((q).wrapping_add(3i32), (((2i32).wrapping_mul((self.cur_style / 2i32))).wrapping_add(3i32)).wrapping_sub((2i32).wrapping_mul((self.cur_style / 6i32))));
        if (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() < self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int()) {
            x = self.rebox(x, self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int());
        } else {
            z = self.rebox(z, self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int());
        }
        if (self.cur_style < 2i32) {
            {
                shift_up = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                shift_down = self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
            }
        } else {
            {
                shift_down = self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() != 0i32) {
                    shift_up = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                } else {
                    shift_up = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                }
            }
        }
        // §919
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            // §921
            {
                if (self.cur_style < 2i32) {
                    clr = (7i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                } else {
                    clr = (3i32).wrapping_mul(self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
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
            // §922
            {
                if (self.cur_style < 2i32) {
                    clr = (3i32).wrapping_mul(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                } else {
                    clr = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                }
                delta = self.half(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                delta1 = (clr).wrapping_sub(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(delta)));
                delta2 = (clr).wrapping_sub(((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(delta)).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                if (delta1 > 0i32) {
                    shift_up = (shift_up).wrapping_add(delta1);
                }
                if (delta2 > 0i32) {
                    shift_down = (shift_down).wrapping_add(delta2);
                }
            }
        }
        // §923
        v = self.new_null_box();
        self.mem[crate::ix::U((v) as usize)].set_hh_b0(1i32);
        { let __v925 = (shift_up).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v925); }
        { let __v926 = (self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int()).wrapping_add(shift_down); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v926); }
        { let __v927 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v927); }
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int() == 0i32) {
            {
                p = self.new_kern(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
            }
        } else {
            {
                y = self.fraction_rule(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                p = self.new_kern(((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(delta)).wrapping_sub((self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int()).wrapping_sub(shift_down)));
                self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                p = self.new_kern(((shift_up).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_sub((self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_add(delta)));
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(y);
            }
        }
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(x);
        // §924
        if (self.cur_style < 2i32) {
            delta = self.font_info[crate::ix::U(((20i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
        } else {
            delta = self.font_info[crate::ix::U(((21i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
        }
        x = self.var_delimiter((q).wrapping_add(4i32), self.cur_size, delta);
        self.mem[crate::ix::U((x) as usize)].set_hh_rh(v);
        z = self.var_delimiter((q).wrapping_add(5i32), self.cur_size, delta);
        self.mem[crate::ix::U((v) as usize)].set_hh_rh(z);
        { let __v928 = self.hpack(x, 0i32, 1i32); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v928); }
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
    // §925
    pub fn make_op(&mut self, mut q: halfword) -> scaled {
        let mut make_op: scaled = 0;
        let mut delta: scaled = 0; // §925
        let mut p: halfword = 0; // §925
        let mut v: halfword = 0; // §925
        let mut x: halfword = 0; // §925
        let mut y: halfword = 0; // §925
        let mut z: halfword = 0; // §925
        let mut c: quarterword = 0; // §925
        let mut i: four_quarters = four_quarters::default(); // §925
        let mut shift_up: scaled = 0; // §925
        let mut shift_down: scaled = 0; // §925
        if ((self.mem[crate::ix::U((q) as usize)].hh().b1() == 0i32) && (self.cur_style < 2i32)) {
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(1i32);
        }
        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh() == 1i32) {
            {
                self.fetch((q).wrapping_add(1i32));
                if ((self.cur_style < 2i32) && (((self.cur_i.b2()).wrapping_sub(0i32) % 4i32) == 2i32)) {
                    {
                        c = self.cur_i.b3();
                        i = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(c)) as usize)].qqqq();
                        if (i.b0() > 0i32) {
                            {
                                self.cur_c = c;
                                self.cur_i = i;
                                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_b1(c);
                            }
                        }
                    }
                }
                delta = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((self.cur_f) as usize)]).wrapping_add(((self.cur_i.b2()).wrapping_sub(0i32) / 4i32))) as usize)].int();
                x = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                if ((self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() != 0i32) && (self.mem[crate::ix::U((q) as usize)].hh().b1() != 1i32)) {
                    { let __v929 = (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int()).wrapping_sub(delta); self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].set_int(__v929); }
                }
                { let __v930 = (self.half((self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int()))).wrapping_sub(self.font_info[crate::ix::U(((22i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627692i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()); self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v930); }
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(2i32);
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(x);
            }
        } else {
            delta = 0i32;
        }
        if (self.mem[crate::ix::U((q) as usize)].hh().b1() == 1i32) {
            // §926
            {
                x = self.clean_box((q).wrapping_add(2i32), (((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(4i32)).wrapping_add((self.cur_style % 2i32)));
                y = self.clean_box((q).wrapping_add(1i32), self.cur_style);
                z = self.clean_box((q).wrapping_add(3i32), ((2i32).wrapping_mul((self.cur_style / 4i32))).wrapping_add(5i32));
                v = self.new_null_box();
                self.mem[crate::ix::U((v) as usize)].set_hh_b0(1i32);
                { let __v931 = self.mem[crate::ix::U(((y).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v931); }
                if (self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v932 = self.mem[crate::ix::U(((x).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v932); }
                }
                if (self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int() > self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int()) {
                    { let __v933 = self.mem[crate::ix::U(((z).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].set_int(__v933); }
                }
                x = self.rebox(x, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                y = self.rebox(y, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                z = self.rebox(z, self.mem[crate::ix::U(((v).wrapping_add(1i32)) as usize)].int());
                { let __v934 = self.half(delta); self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].set_int(__v934); }
                { let __v935 = (self.mem[crate::ix::U(((x).wrapping_add(4i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U(((z).wrapping_add(4i32)) as usize)].set_int(__v935); }
                { let __v936 = self.mem[crate::ix::U(((y).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v936); }
                { let __v937 = self.mem[crate::ix::U(((y).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v937); }
                // §927
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].hh().rh() == 0i32) {
                    {
                        self.free_node(x, 7i32);
                        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(y);
                    }
                } else {
                    {
                        shift_up = (self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int());
                        if (shift_up < self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()) {
                            shift_up = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                        }
                        p = self.new_kern(shift_up);
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(y);
                        self.mem[crate::ix::U((x) as usize)].set_hh_rh(p);
                        p = self.new_kern(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(x);
                        self.mem[crate::ix::U(((v).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                        { let __v938 = ((((self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((x).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_up); self.mem[crate::ix::U(((v).wrapping_add(3i32)) as usize)].set_int(__v938); }
                    }
                }
                if (self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].hh().rh() == 0i32) {
                    self.free_node(z, 7i32);
                } else {
                    {
                        shift_down = (self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()).wrapping_sub(self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int());
                        if (shift_down < self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int()) {
                            shift_down = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                        }
                        p = self.new_kern(shift_down);
                        self.mem[crate::ix::U((y) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(z);
                        p = self.new_kern(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int());
                        self.mem[crate::ix::U((z) as usize)].set_hh_rh(p);
                        { let __v939 = ((((self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].int()).wrapping_add(self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U((((627693i32).wrapping_add(self.cur_size)) - 1) as usize)].hh().rh()) as usize)])) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((z).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((z).wrapping_add(2i32)) as usize)].int())).wrapping_add(shift_down); self.mem[crate::ix::U(((v).wrapping_add(2i32)) as usize)].set_int(__v939); }
                    }
                }
                // §926
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(v);
            }
        }
        // §925
        make_op = delta;
        make_op
    }

}
