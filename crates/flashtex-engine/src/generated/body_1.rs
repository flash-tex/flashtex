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
    /// Here is a function that returns a pointer to a copy of a glue spec.
    /// The reference count in the copy is `null`, because there is assumed
    /// to be exactly one reference to the new specification.
    // §169
    pub fn new_spec(&mut self, mut p: halfword) -> halfword {
        let mut new_spec: halfword = 0;
        let mut q: halfword = 0; // §169
        q = self.get_node(glue_spec_size);
        { let __v58 = self.mem[crate::ix::U((p) as usize)]; self.mem[crate::ix::U((q) as usize)] = __v58; }
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
        { let __v59 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v59); }
        { let __v60 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v60); }
        { let __v61 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v61); }
        new_spec = q;
        new_spec
    }

    /// And here's a function that creates a glue node for a given parameter
    /// identified by its code number; for example,
    /// `new_param_glue(line_skip_code)` returns a pointer to a glue node for the
    /// current \.{\\lineskip}.
    // §170
    pub fn new_param_glue(&mut self, mut n: small_number) -> halfword {
        let mut new_param_glue: halfword = 0;
        let mut p: halfword = 0; // §170
        let mut q: halfword = 0; // §170
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(glue_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1((n).wrapping_add(1i32));
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(null);
        q = self.eqtb[crate::ix::U((((glue_base).wrapping_add(n)) - 1) as usize)].hh().rh();
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(q);
        { let __v62 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v62); }
        new_param_glue = p;
        new_param_glue
    }

    /// Glue nodes that are more or less anonymous are created by `new_glue`,
    /// whose argument points to a glue specification.
    // §171
    pub fn new_glue(&mut self, mut q: halfword) -> halfword {
        let mut new_glue: halfword = 0;
        let mut p: halfword = 0; // §171
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(glue_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(null);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(q);
        { let __v63 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v63); }
        new_glue = p;
        new_glue
    }

    /// Still another subroutine is needed: This one is sort of a combination
    /// of `new_param_glue` and `new_glue`. It creates a glue node for one of
    /// the current glue parameters, but it makes a fresh copy of the glue
    /// specification, since that specification will probably be subject to change,
    /// while the parameter will stay put. The global variable `temp_ptr` is
    /// set to the address of the new spec.
    // §172
    pub fn new_skip_param(&mut self, mut n: small_number) -> halfword {
        let mut new_skip_param: halfword = 0;
        let mut p: halfword = 0; // §172
        self.temp_ptr = self.new_spec(self.eqtb[crate::ix::U((((glue_base).wrapping_add(n)) - 1) as usize)].hh().rh());
        p = self.new_glue(self.temp_ptr);
        { let __ix64 = self.temp_ptr; self.mem[crate::ix::U((__ix64) as usize)].set_hh_rh(null); }
        self.mem[crate::ix::U((p) as usize)].set_hh_b1((n).wrapping_add(1i32));
        new_skip_param = p;
        new_skip_param
    }

    /// The `new_kern` function creates a kern node having a given width.
    // §174
    pub fn new_kern(&mut self, mut w: scaled) -> halfword {
        let mut new_kern: halfword = 0;
        let mut p: halfword = 0; // §174
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(w);
        new_kern = p;
        new_kern
    }

    /// Anyone who has been reading the last few sections of the program will
    /// be able to guess what comes next.
    // §176
    pub fn new_penalty(&mut self, mut m: i32) -> halfword {
        let mut new_penalty: halfword = 0;
        let mut p: halfword = 0; // §176
        p = self.get_node(small_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(penalty_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(m);
        new_penalty = p;
        new_penalty
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_error(&mut self, mut t: str_number, mut p: str_number) {
        self.normalize_selector();
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(1024i32);
        }
        if (t != 0i32) {
            {
                self.print(288i32);
                self.print(t);
                self.print(41i32);
            }
        }
        self.print(650i32);
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

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_warning(&mut self, mut t: str_number, mut p: str_number, mut prepend_nl: bool, mut append_nl: bool) {
        if (self.interaction == error_stop_mode) {
        }
        if prepend_nl {
            self.print_ln();
        }
        self.print(1025i32);
        if (t != 0i32) {
            {
                self.print(288i32);
                self.print(t);
                self.print(41i32);
            }
        }
        self.print(650i32);
        self.print(p);
        if append_nl {
            self.print_ln();
        }
        if (self.history == spotless) {
            self.history = warning_issued;
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_os_get_os_buf(&mut self, mut s: i32) {
        let mut a: i32 = 0; // §686
        if (s > (sup_pdf_os_buf_size).wrapping_sub(self.pdf_ptr)) {
            self.overflow(1026i32, self.pdf_os_buf_size);
        }
        if ((self.pdf_ptr).wrapping_add(s) > self.pdf_os_buf_size) {
            {
                a = (((0.2f64 * ((self.pdf_os_buf_size) as f64))) as i32);
                if ((self.pdf_ptr).wrapping_add(s) > (self.pdf_os_buf_size).wrapping_add(a)) {
                    self.pdf_os_buf_size = (self.pdf_ptr).wrapping_add(s);
                } else {
                    if (self.pdf_os_buf_size < (sup_pdf_os_buf_size).wrapping_sub(a)) {
                        self.pdf_os_buf_size = (self.pdf_os_buf_size).wrapping_add(a);
                    } else {
                        self.pdf_os_buf_size = sup_pdf_os_buf_size;
                    }
                }
                self.pdf_os_buf.resize_len(((self.pdf_os_buf_size) as usize) + 1);
                self.pdf_buf_is_os = true;
                self.pdf_buf_size = self.pdf_os_buf_size;
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn remove_last_space(&mut self) {
        if ((self.pdf_ptr > 0i32) && (self.pdf_buf_get((self.pdf_ptr).wrapping_sub(1i32)) == 32i32)) {
            self.pdf_ptr = (self.pdf_ptr).wrapping_sub(1i32);
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_print_octal(&mut self, mut n: i32) {
        let mut k: i32 = 0; // §686
        k = 0i32;
        loop {
            self.dig[crate::ix::U((k) as usize)] = (n % 8i32);
            n = (n / 8i32);
            k = (k).wrapping_add(1i32);
            if (n == 0i32) { break; }
        }
        if (k == 1i32) {
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
                        self.pdf_buf_set(self.pdf_ptr, 48i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
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
                        self.pdf_buf_set(self.pdf_ptr, 48i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
        if (k == 2i32) {
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
                    self.pdf_buf_set(self.pdf_ptr, 48i32);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
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
                        self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add(self.dig[crate::ix::U((k) as usize)]));
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_print_char(&mut self, mut f: internal_font_number, mut c: i32) {
        self.pdf_mark_char(f, c);
        if (((((c <= 32i32) || (c == 92i32)) || (c == 40i32)) || (c == 41i32)) || (c > 127i32)) {
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
                        self.pdf_buf_set(self.pdf_ptr, 92i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                self.pdf_print_octal(c);
            }
        } else {
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
                    self.pdf_buf_set(self.pdf_ptr, c);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_print(&mut self, mut s: str_number) {
        let mut j: pool_pointer = 0; // §686
        let mut c: i32 = 0; // §686
        j = self.str_start[crate::ix::U((s) as usize)];
        while (j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) {
            {
                c = self.str_pool[crate::ix::U((j) as usize)];
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
                        self.pdf_buf_set(self.pdf_ptr, c);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                j = (j).wrapping_add(1i32);
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn str_in_str(&mut self, mut s: str_number, mut r: str_number, mut i: i32) -> bool {
        let mut str_in_str: bool = false;
        let mut j: pool_pointer = 0; // §686
        let mut k: pool_pointer = 0; // §686
        str_in_str = false;
        if ((self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]) < (i).wrapping_add((self.str_start[crate::ix::U(((r).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((r) as usize)]))) {
            return str_in_str;
        }
        j = (i).wrapping_add(self.str_start[crate::ix::U((s) as usize)]);
        k = self.str_start[crate::ix::U((r) as usize)];
        while ((j < self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]) && (k < self.str_start[crate::ix::U(((r).wrapping_add(1i32)) as usize)])) {
            {
                if (self.str_pool[crate::ix::U((j) as usize)] != self.str_pool[crate::ix::U((k) as usize)]) {
                    return str_in_str;
                }
                j = (j).wrapping_add(1i32);
                k = (k).wrapping_add(1i32);
            }
        }
        str_in_str = true;
        str_in_str
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_print_int(&mut self, mut n: longinteger) {
        let mut k: i32 = 0; // §686
        let mut m: longinteger = 0; // §686
        k = 0i32;
        if (n < ((0i32) as i64)) {
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
                        self.pdf_buf_set(self.pdf_ptr, 45i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                if (n > (((100000000i32).wrapping_neg()) as i64)) {
                    n = (n).wrapping_neg();
                } else {
                    {
                        m = ((((1i32).wrapping_neg()) as i64)).wrapping_sub(n);
                        n = (m / ((10i32) as i64));
                        m = ((m % ((10i32) as i64))).wrapping_add(((1i32) as i64));
                        k = 1i32;
                        if (m < ((10i32) as i64)) {
                            self.dig[crate::ix::U((0i32) as usize)] = ((m) as i32);
                        } else {
                            {
                                self.dig[crate::ix::U((0i32) as usize)] = 0i32;
                                n = (n).wrapping_add(((1i32) as i64));
                            }
                        }
                    }
                }
            }
        }
        loop {
            self.dig[crate::ix::U((k) as usize)] = (((n % ((10i32) as i64))) as i32);
            n = (n / ((10i32) as i64));
            k = (k).wrapping_add(1i32);
            if (n == ((0i32) as i64)) { break; }
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
                    self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add(self.dig[crate::ix::U((k) as usize)]));
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn pdf_print_two(&mut self, mut n: i32) {
        n = ((n).wrapping_abs() % 100i32);
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
                self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add((n / 10i32)));
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
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
                self.pdf_buf_set(self.pdf_ptr, (48i32).wrapping_add((n % 10i32)));
                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
            }
        }
    }

    /// Basic printing procedures for PDF output are very similar to \TeX\ basic
    /// printing ones but the output is going to PDF buffer. Subroutines with
    /// suffix `_ln` append a new-line character to the PDF output.
    // §686
    pub fn tokens_to_string(&mut self, mut p: halfword) -> str_number {
        let mut tokens_to_string: str_number = 0;
        if (self.selector == new_string) {
            self.pdf_error(1027i32, 1028i32);
        }
        self.old_setting = self.selector;
        self.selector = new_string;
        self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), null, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = self.old_setting;
        self.last_tokens_string = self.make_string();
        tokens_to_string = self.last_tokens_string;
        tokens_to_string
    }

    /// The following function divides `s` by `m`. `dd` is number of decimal digits.
    /// The result is $r = (s/m) \times 10^{dd}$, and the remainder after
    /// the division is $s - `scaled_out`$
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §689
    pub fn divide_scaled(&mut self, mut s: scaled, mut m: scaled, mut dd: i32) -> scaled {
        let mut divide_scaled: scaled = 0;
        let mut q: scaled = 0; // §689
        let mut r: scaled = 0; // §689
        let mut sign: i32 = 0; // §689
        let mut i: i32 = 0; // §689
        sign = 1i32;
        if (s < 0i32) {
            {
                sign = (sign).wrapping_neg();
                s = (s).wrapping_neg();
            }
        }
        if (m < 0i32) {
            {
                sign = (sign).wrapping_neg();
                m = (m).wrapping_neg();
            }
        }
        if (m == 0i32) {
            self.pdf_error(1029i32, 1030i32);
        } else {
            if (m >= (max_integer / 10i32)) {
                self.pdf_error(1029i32, 1031i32);
            }
        }
        q = (s / m);
        r = (s % m);
        {
            let __for_end_2 = dd;
            i = 1i32;
            while i <= __for_end_2 {
                {
                    q = ((10i32).wrapping_mul(q)).wrapping_add(((10i32).wrapping_mul(r) / m));
                    r = ((10i32).wrapping_mul(r) % m);
                }
                i = i.wrapping_add(1);
            }
        }
        if ((2i32).wrapping_mul(r) >= m) {
            {
                q = (q).wrapping_add(1i32);
                r = (r).wrapping_sub(m);
            }
        }
        self.scaled_out = (sign).wrapping_mul((s).wrapping_sub((r / self.ten_pow[crate::ix::U((dd) as usize)])));
        divide_scaled = (sign).wrapping_mul(q);
        divide_scaled
    }

    /// The following function divides `s` by `m`. `dd` is number of decimal digits.
    /// The result is $r = (s/m) \times 10^{dd}$, and the remainder after
    /// the division is $s - `scaled_out`$
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §689
    pub fn round_xn_over_d(&mut self, mut x: scaled, mut n: i32, mut d: i32) -> scaled {
        let mut round_xn_over_d: scaled = 0;
        let mut positive: bool = false; // §689
        let mut t: nonnegative_integer = 0; // §689
        let mut u: nonnegative_integer = 0; // §689
        let mut v: nonnegative_integer = 0; // §689
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

    /// The following function divides `s` by `m`. `dd` is number of decimal digits.
    /// The result is $r = (s/m) \times 10^{dd}$, and the remainder after
    /// the division is $s - `scaled_out`$
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §689
    pub fn is_bit_set(&mut self, mut n: i32, mut s: small_number) -> bool {
        let mut is_bit_set: bool = false;
        let mut m: i32 = 0; // §689
        let mut i: i32 = 0; // §689
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

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn append_dest_name(&mut self, mut s: str_number, mut n: i32) {
        let mut a: i32 = 0; // §698
        if (self.pdf_dest_names_ptr == sup_dest_names_size) {
            self.overflow(1064i32, self.dest_names_size);
        }
        if (self.pdf_dest_names_ptr == self.dest_names_size) {
            {
                a = (((0.2f64 * ((self.dest_names_size) as f64))) as i32);
                if (self.dest_names_size < (sup_dest_names_size).wrapping_sub(a)) {
                    self.dest_names_size = (self.dest_names_size).wrapping_add(a);
                } else {
                    self.dest_names_size = sup_dest_names_size;
                }
                self.dest_names.resize_len(((self.dest_names_size) as usize) + 1);
            }
        }
        self.dest_names[crate::ix::U((self.pdf_dest_names_ptr) as usize)].objname = s;
        self.dest_names[crate::ix::U((self.pdf_dest_names_ptr) as usize)].objnum = n;
        self.pdf_dest_names_ptr = (self.pdf_dest_names_ptr).wrapping_add(1i32);
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_create_obj(&mut self, mut t: i32, mut i: i32) {
        let mut a: i32 = 0; // §698
        let mut p: i32 = 0; // §698
        let mut q: i32 = 0; // §698
        if (self.sys_obj_ptr == sup_obj_tab_size) {
            self.overflow(1065i32, self.obj_tab_size);
        }
        if (self.sys_obj_ptr == self.obj_tab_size) {
            {
                a = (((0.2f64 * ((self.obj_tab_size) as f64))) as i32);
                if (self.obj_tab_size < (sup_obj_tab_size).wrapping_sub(a)) {
                    self.obj_tab_size = (self.obj_tab_size).wrapping_add(a);
                } else {
                    self.obj_tab_size = sup_obj_tab_size;
                }
                self.obj_tab.resize_len(((self.obj_tab_size) as usize) + 1);
            }
        }
        self.sys_obj_ptr = (self.sys_obj_ptr).wrapping_add(1i32);
        self.obj_ptr = self.sys_obj_ptr;
        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int0 = i;
        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int2 = (((2i32).wrapping_neg()) as i64);
        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int4 = 0i32;
        self.avl_put_obj(self.obj_ptr, t);
        if (t == obj_type_page) {
            {
                p = self.head_tab[crate::ix::U(((t) - 1) as usize)];
                if ((p == 0i32) || (self.obj_tab[crate::ix::U((p) as usize)].int0 < i)) {
                    {
                        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int1 = p;
                        { let __v65 = self.obj_ptr; self.head_tab[crate::ix::U(((t) - 1) as usize)] = __v65; }
                    }
                } else {
                    {
                        'l_done_f: {
                            while (p != 0i32) {
                                {
                                    if (self.obj_tab[crate::ix::U((p) as usize)].int0 < i) {
                                        break 'l_done_f;
                                    }
                                    q = p;
                                    p = self.obj_tab[crate::ix::U((p) as usize)].int1;
                                }
                            }
                        }
                        self.obj_tab[crate::ix::U((q) as usize)].int1 = self.obj_ptr;
                        self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int1 = p;
                    }
                }
            }
        } else {
            if (t != obj_type_others) {
                {
                    self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int1 = self.head_tab[crate::ix::U(((t) - 1) as usize)];
                    { let __v66 = self.obj_ptr; self.head_tab[crate::ix::U(((t) - 1) as usize)] = __v66; }
                    if ((t == obj_type_dest) && (i < 0i32)) {
                        self.append_dest_name((self.obj_tab[crate::ix::U((self.obj_ptr) as usize)].int0).wrapping_neg(), self.obj_ptr);
                    }
                }
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_new_objnum(&mut self) -> i32 {
        let mut pdf_new_objnum: i32 = 0;
        self.pdf_create_obj(obj_type_others, 0i32);
        pdf_new_objnum = self.obj_ptr;
        pdf_new_objnum
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_os_switch(&mut self, mut pdf_os: bool) {
        if (pdf_os && self.pdf_os_enable) {
            {
                if (!self.pdf_os_mode) {
                    {
                        self.pdf_op_ptr = self.pdf_ptr;
                        self.pdf_ptr = self.pdf_os_ptr;
                        self.pdf_buf_is_os = true;
                        self.pdf_buf_size = self.pdf_os_buf_size;
                        self.pdf_os_mode = true;
                    }
                }
            }
        } else {
            {
                if self.pdf_os_mode {
                    {
                        self.pdf_os_ptr = self.pdf_ptr;
                        self.pdf_ptr = self.pdf_op_ptr;
                        self.pdf_buf_is_os = false;
                        self.pdf_buf_size = pdf_op_buf_size;
                        self.pdf_os_mode = false;
                    }
                }
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_os_prepare_obj(&mut self, mut i: i32, mut pdf_os_level: i32) {
        self.pdf_os_switch(((pdf_os_level > 0i32) && (self.fixed_pdf_objcompresslevel >= pdf_os_level)));
        if self.pdf_os_mode {
            {
                if (self.pdf_os_cur_objnum == 0i32) {
                    {
                        self.pdf_os_cur_objnum = self.pdf_new_objnum();
                        self.obj_ptr = (self.obj_ptr).wrapping_sub(1i32);
                        self.pdf_os_cntr = (self.pdf_os_cntr).wrapping_add(1i32);
                        self.pdf_os_objidx = 0i32;
                        self.pdf_ptr = 0i32;
                    }
                } else {
                    self.pdf_os_objidx = (self.pdf_os_objidx).wrapping_add(1i32);
                }
                self.obj_tab[crate::ix::U((i) as usize)].int3 = self.pdf_os_objidx;
                self.obj_tab[crate::ix::U((i) as usize)].int2 = ((self.pdf_os_cur_objnum) as i64);
                self.pdf_os_objnum[crate::ix::U((self.pdf_os_objidx) as usize)] = i;
                { let __ix67 = self.pdf_os_objidx; let __v68 = self.pdf_ptr; self.pdf_os_objoff[crate::ix::U((__ix67) as usize)] = __v68; }
            }
        } else {
            {
                self.obj_tab[crate::ix::U((i) as usize)].int2 = (self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64));
                self.obj_tab[crate::ix::U((i) as usize)].int3 = (1i32).wrapping_neg();
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_begin_obj(&mut self, mut i: i32, mut pdf_os_level: i32) {
        self.check_pdfversion();
        self.pdf_os_prepare_obj(i, pdf_os_level);
        if (!self.pdf_os_mode) {
            {
                self.pdf_print_int(((i) as i64));
                {
                    self.pdf_print(1066i32);
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        } else {
            if (self.eqtb[crate::ix::U(((629080i32) - 1) as usize)].int() == 0i32) {
                {
                    self.pdf_print(1067i32);
                    self.pdf_print_int(((i) as i64));
                    {
                        self.pdf_print(1066i32);
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
                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_new_obj(&mut self, mut t: i32, mut i: i32, mut pdf_os: i32) {
        self.pdf_create_obj(t, i);
        self.pdf_begin_obj(self.obj_ptr, pdf_os);
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_end_obj(&mut self) {
        if self.pdf_os_mode {
            {
                if (self.pdf_os_objidx == (pdf_os_max_objs).wrapping_sub(1i32)) {
                    self.pdf_os_write_objstream();
                }
            }
        } else {
            {
                self.pdf_print(1068i32);
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
                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_begin_dict(&mut self, mut i: i32, mut pdf_os_level: i32) {
        self.check_pdfversion();
        self.pdf_os_prepare_obj(i, pdf_os_level);
        if (!self.pdf_os_mode) {
            {
                self.pdf_print_int(((i) as i64));
                {
                    self.pdf_print(1066i32);
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        } else {
            if (self.eqtb[crate::ix::U(((629080i32) - 1) as usize)].int() == 0i32) {
                {
                    self.pdf_print(1067i32);
                    self.pdf_print_int(((i) as i64));
                    {
                        self.pdf_print(1066i32);
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
                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                }
            }
        }
        {
            self.pdf_print(1069i32);
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
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_new_dict(&mut self, mut t: i32, mut i: i32, mut pdf_os: i32) {
        self.pdf_create_obj(t, i);
        self.pdf_begin_dict(self.obj_ptr, pdf_os);
    }

    /// Here we implement subroutines for work with objects and related things.
    /// Some of them are used in former parts too, so we need to declare them
    /// forward.
    // §698
    pub fn pdf_end_dict(&mut self) {
        if self.pdf_os_mode {
            {
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
                if (self.pdf_os_objidx == (pdf_os_max_objs).wrapping_sub(1i32)) {
                    self.pdf_os_write_objstream();
                }
            }
        } else {
            {
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
                {
                    self.pdf_print(1068i32);
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
    }

    /// Write out an accumulated object stream.
    /// First the object number and byte offset pairs are generated
    /// and appended to the ready buffered object stream.
    /// By this the value of \.{/First} can be calculated.
    /// Then a new \.{/ObjStm} object is generated, and everything is
    /// copied to the PDF output buffer, where also compression is done.
    /// When calling this procedure, `pdf_os_mode` must be `true`.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §699
    pub fn pdf_os_write_objstream(&mut self) {
        let mut i: halfword = 0; // §699
        let mut j: halfword = 0; // §699
        let mut p: halfword = 0; // §699
        let mut q: halfword = 0; // §699
        if (self.pdf_os_cur_objnum == 0i32) {
            return;
        }
        p = self.pdf_ptr;
        i = 0i32;
        j = 0i32;
        while (i <= self.pdf_os_objidx) {
            {
                self.pdf_print_int(((self.pdf_os_objnum[crate::ix::U((i) as usize)]) as i64));
                self.pdf_print(32i32);
                self.pdf_print_int(((self.pdf_os_objoff[crate::ix::U((i) as usize)]) as i64));
                if (j == 9i32) {
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
                                self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                                self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                            }
                        }
                        j = 0i32;
                    }
                } else {
                    {
                        self.pdf_print(32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
                i = (i).wrapping_add(1i32);
            }
        }
        self.pdf_buf_set((self.pdf_ptr).wrapping_sub(1i32), pdf_new_line_char);
        q = self.pdf_ptr;
        self.pdf_begin_dict(self.pdf_os_cur_objnum, 0i32);
        {
            self.pdf_print(1070i32);
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
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_print(1071i32);
        {
            self.pdf_print_int((((self.pdf_os_objidx).wrapping_add(1i32)) as i64));
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
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_print(1072i32);
        {
            self.pdf_print_int((((q).wrapping_sub(p)) as i64));
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
                    self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
            }
        }
        self.pdf_begin_stream();
        {
            if (self.pdf_os_mode && (((q).wrapping_sub(p)).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                self.pdf_os_get_os_buf((q).wrapping_sub(p));
            } else {
                if ((!self.pdf_os_mode) && ((q).wrapping_sub(p) > self.pdf_buf_size)) {
                    self.overflow(1004i32, pdf_op_buf_size);
                } else {
                    if ((!self.pdf_os_mode) && (((q).wrapping_sub(p)).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_flush();
                    }
                }
            }
        }
        i = p;
        while (i < q) {
            {
                {
                    self.pdf_buf_set(self.pdf_ptr, self.pdf_os_buf[crate::ix::U((i) as usize)]);
                    self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                }
                i = (i).wrapping_add(1i32);
            }
        }
        i = 0i32;
        while (i < p) {
            {
                q = (i).wrapping_add(self.pdf_buf_size);
                if (q > p) {
                    q = p;
                }
                {
                    if (self.pdf_os_mode && (((q).wrapping_sub(i)).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                        self.pdf_os_get_os_buf((q).wrapping_sub(i));
                    } else {
                        if ((!self.pdf_os_mode) && ((q).wrapping_sub(i) > self.pdf_buf_size)) {
                            self.overflow(1004i32, pdf_op_buf_size);
                        } else {
                            if ((!self.pdf_os_mode) && (((q).wrapping_sub(i)).wrapping_add(self.pdf_ptr) > self.pdf_buf_size)) {
                                self.pdf_flush();
                            }
                        }
                    }
                }
                while (i < q) {
                    {
                        {
                            self.pdf_buf_set(self.pdf_ptr, self.pdf_os_buf[crate::ix::U((i) as usize)]);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                        i = (i).wrapping_add(1i32);
                    }
                }
            }
        }
        self.pdf_end_stream();
        self.pdf_os_cur_objnum = 0i32;
    }

    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §700
    pub fn append_ptr(&mut self, mut p: halfword, mut i: i32) -> halfword {
        let mut append_ptr: halfword = 0;
        let mut q: halfword = 0; // §700
        append_ptr = p;
        {
            q = self.avail;
            if (q == null) {
                q = self.get_avail();
            } else {
                {
                    self.avail = self.mem[crate::ix::U((q) as usize)].hh().rh();
                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
                    self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                    self.dl_new_node(q);
                }
            }
        }
        self.mem[crate::ix::U((q) as usize)].set_hh_lh(i);
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
        if (p == null) {
            {
                append_ptr = q;
                return append_ptr;
            }
        }
        while (self.mem[crate::ix::U((p) as usize)].hh().rh() != null) {
            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
        append_ptr
    }

    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §700
    pub fn pdf_lookup_list(&mut self, mut p: halfword, mut i: i32) -> halfword {
        let mut pdf_lookup_list: halfword = 0;
        pdf_lookup_list = null;
        while (p != null) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().lh() == i) {
                    {
                        pdf_lookup_list = p;
                        return pdf_lookup_list;
                    }
                }
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        pdf_lookup_list
    }

    /// We need to check whether the referenced object exists.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1545
    pub fn prev_rightmost(&mut self, mut s: halfword, mut e: halfword) -> halfword {
        let mut prev_rightmost: halfword = 0;
        let mut p: halfword = 0; // §1545
        prev_rightmost = null;
        p = s;
        if (p == null) {
            return prev_rightmost;
        }
        while (self.mem[crate::ix::U((p) as usize)].hh().rh() != e) {
            {
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                if (p == null) {
                    return prev_rightmost;
                }
            }
        }
        prev_rightmost = p;
        prev_rightmost
    }

    /// We need to check whether the referenced object exists.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1545
    pub fn pdf_check_obj(&mut self, mut t: i32, mut n: i32) {
        let mut k: i32 = 0; // §1545
        k = self.head_tab[crate::ix::U(((t) - 1) as usize)];
        while ((k != 0i32) && (k != n)) {
            k = self.obj_tab[crate::ix::U((k) as usize)].int1;
        }
        if (k == 0i32) {
            self.pdf_error(1762i32, 1787i32);
        }
    }

    /// The following function finds object with identifier `i` and type `t`.
    /// `i < 0` indicates that `-i` should be treated as a string number. If no
    /// such object exists then it will be created. This function is used mainly to
    /// find destination for link annotations and outlines; however it is also used
    /// in `pdf_ship_out` (to check whether a Page object already exists) so we need
    /// to declare it together with subroutines needed in `pdf_hlist_out` and
    /// `pdf_vlist_out`.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1555
    pub fn find_obj(&mut self, mut t: i32, mut i: i32, mut byname: bool) -> i32 {
        let mut find_obj: i32 = 0;
        find_obj = self.avl_find_obj(t, i, ((byname) as i32));
        find_obj
    }

    /// The following function finds object with identifier `i` and type `t`.
    /// `i < 0` indicates that `-i` should be treated as a string number. If no
    /// such object exists then it will be created. This function is used mainly to
    /// find destination for link annotations and outlines; however it is also used
    /// in `pdf_ship_out` (to check whether a Page object already exists) so we need
    /// to declare it together with subroutines needed in `pdf_hlist_out` and
    /// `pdf_vlist_out`.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1555
    pub fn flush_str(&mut self, mut s: str_number) {
        if (s == (self.str_ptr).wrapping_sub(1i32)) {
            {
                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
            }
        }
    }

    /// The following function finds object with identifier `i` and type `t`.
    /// `i < 0` indicates that `-i` should be treated as a string number. If no
    /// such object exists then it will be created. This function is used mainly to
    /// find destination for link annotations and outlines; however it is also used
    /// in `pdf_ship_out` (to check whether a Page object already exists) so we need
    /// to declare it together with subroutines needed in `pdf_hlist_out` and
    /// `pdf_vlist_out`.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1555
    pub fn get_obj(&mut self, mut t: i32, mut i: i32, mut byname: bool) -> i32 {
        let mut get_obj: i32 = 0;
        let mut r: i32 = 0; // §1555
        let mut s: str_number = 0; // §1555
        if (((byname) as i32) > 0i32) {
            {
                s = self.tokens_to_string(i);
                r = self.find_obj(t, s, true);
            }
        } else {
            {
                s = 0i32;
                r = self.find_obj(t, i, false);
            }
        }
        if (r == 0i32) {
            {
                if (((byname) as i32) > 0i32) {
                    {
                        self.pdf_create_obj(t, (s).wrapping_neg());
                        s = 0i32;
                    }
                } else {
                    self.pdf_create_obj(t, i);
                }
                r = self.obj_ptr;
                if ((t == obj_type_dest) || (t == obj_type_struct_dest)) {
                    self.obj_tab[crate::ix::U((r) as usize)].int4 = null;
                }
            }
        }
        if (s != 0i32) {
            self.flush_str(s);
        }
        get_obj = r;
        get_obj
    }

    /// The following function finds object with identifier `i` and type `t`.
    /// `i < 0` indicates that `-i` should be treated as a string number. If no
    /// such object exists then it will be created. This function is used mainly to
    /// find destination for link annotations and outlines; however it is also used
    /// in `pdf_ship_out` (to check whether a Page object already exists) so we need
    /// to declare it together with subroutines needed in `pdf_hlist_out` and
    /// `pdf_vlist_out`.
    /// @<Declare procedures that need to be declared forward for \pdfTeX
    // §1555
    pub fn get_microinterval(&mut self) -> i32 {
        let mut get_microinterval: i32 = 0;
        let mut s: i32 = 0; // §1555
        let mut m: i32 = 0; // §1555
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
    // §192
    pub fn print_font_identifier(&mut self, mut f: internal_font_number) {
        if (self.pdf_font_blink[crate::ix::U((f) as usize)] == null_font) {
            self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 514) as usize)].rh());
        } else {
            self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.pdf_font_blink[crate::ix::U((f) as usize)])) - 514) as usize)].rh());
        }
        if (self.eqtb[crate::ix::U(((629099i32) - 1) as usize)].int() > 0i32) {
            {
                self.print(288i32);
                self.print(self.font_name[crate::ix::U((f) as usize)]);
                if (self.font_size[crate::ix::U((f) as usize)] != self.font_dsize[crate::ix::U((f) as usize)]) {
                    {
                        self.print(64i32);
                        self.print_scaled(self.font_size[crate::ix::U((f) as usize)]);
                        self.print(314i32);
                    }
                }
                self.print(41i32);
            }
        } else {
            if (self.pdf_font_expand_ratio[crate::ix::U((f) as usize)] != 0i32) {
                {
                    self.print(288i32);
                    if (self.pdf_font_expand_ratio[crate::ix::U((f) as usize)] > 0i32) {
                        self.print(43i32);
                    }
                    self.print_int(((self.pdf_font_expand_ratio[crate::ix::U((f) as usize)]) as i64));
                    self.print(41i32);
                }
            }
        }
    }

    /// Boxes, rules, inserts, whatsits, marks, and things in general that are
    /// sort of ``complicated'' are indicated only by printing `\.{[]}'.
    // §192
    pub fn short_display(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §192
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
                                            self.print_font_identifier(self.mem[crate::ix::U((p) as usize)].hh().b0());
                                        }
                                        self.print_char(32i32);
                                        self.font_in_short_display = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    }
                                }
                                self.print((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(0i32));
                            }
                        }
                    }
                } else {
                    // §193
                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                        hlist_node | vlist_node | ins_node | whatsit_node | mark_node | adjust_node | unset_node => {
                            self.print(315i32);
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
                                self.print(315i32);
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
                                        if (self.mem[crate::ix::U((p) as usize)].hh().rh() != null) {
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
                // §192
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §194
    pub fn print_font_and_char(&mut self, mut p: i32) {
        if (p > self.mem_end) {
            self.print_esc(316i32);
        } else {
            {
                if ((self.mem[crate::ix::U((p) as usize)].hh().b0() < font_base) || (self.mem[crate::ix::U((p) as usize)].hh().b0() > font_max)) {
                    self.print_char(42i32);
                } else {
                    self.print_font_identifier(self.mem[crate::ix::U((p) as usize)].hh().b0());
                }
                self.print_char(32i32);
                self.print((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(0i32));
            }
        }
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §194
    pub fn print_mark(&mut self, mut p: i32) {
        self.print_char(123i32);
        if ((p < self.hi_mem_min) || (p > self.mem_end)) {
            self.print_esc(316i32);
        } else {
            self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), null, (self.max_print_line).wrapping_sub(10i32));
        }
        self.print_char(125i32);
    }

    /// The `show_node_list` routine requires some auxiliary subroutines: one to
    /// print a font-and-character combination, one to print a token list without
    /// its reference count, and one to print a rule dimension.
    // §194
    pub fn print_rule_dimen(&mut self, mut d: scaled) {
        if (d == (1073741824i32).wrapping_neg()) {
            self.print_char(42i32);
        } else {
            self.print_scaled(d);
        }
    }

    /// Then there is a subroutine that prints glue stretch and shrink, possibly
    /// followed by the name of finite units:
    // §195
    pub fn print_glue(&mut self, mut d: scaled, mut order: i32, mut s: str_number) {
        self.print_scaled(d);
        if ((order < normal) || (order > filll)) {
            self.print(317i32);
        } else {
            if (order > normal) {
                {
                    self.print(318i32);
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
    // §196
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
                        self.print(319i32);
                        self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U((p) as usize)].hh().b0(), s);
                    }
                }
                if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() != 0i32) {
                    {
                        self.print(320i32);
                        self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U((p) as usize)].hh().b1(), s);
                    }
                }
            }
        }
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §867
    pub fn print_fam_and_char(&mut self, mut p: halfword) {
        self.print_esc(480i32);
        self.print_int(((self.mem[crate::ix::U((p) as usize)].hh().b0()) as i64));
        self.print_char(32i32);
        self.print((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(0i32));
    }

    /// Here are some simple routines used in the display of noads.
    /// @<Declare procedures needed for displaying the elements of mlists
    // §867
    pub fn print_delimiter(&mut self, mut p: halfword) {
        let mut a: i32 = 0; // §867
        a = (((self.mem[crate::ix::U((p) as usize)].qqqq().b0()).wrapping_mul(256i32)).wrapping_add(self.mem[crate::ix::U((p) as usize)].qqqq().b1())).wrapping_sub(0i32);
        a = ((((a).wrapping_mul(4096i32)).wrapping_add((self.mem[crate::ix::U((p) as usize)].qqqq().b2()).wrapping_mul(256i32))).wrapping_add(self.mem[crate::ix::U((p) as usize)].qqqq().b3())).wrapping_sub(0i32);
        if (a < 0i32) {
            self.print_int(((a) as i64));
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
    // §868
    pub fn print_subsidiary_data(&mut self, mut p: halfword, mut c: ASCII_code) {
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) >= self.depth_threshold) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().rh() != empty) {
                    self.print(321i32);
                }
            }
        } else {
            {
                {
                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = c;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
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
                        if (self.mem[crate::ix::U((p) as usize)].hh().lh() == null) {
                            {
                                self.print_ln();
                                self.print_current_string();
                                self.print(1268i32);
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
    // §870
    pub fn print_style(&mut self, mut c: i32) {
        match (c / 2i32) {
            0 => {
                self.print_esc(1269i32);
            }
            1 => {
                self.print_esc(1270i32);
            }
            2 => {
                self.print_esc(1271i32);
            }
            3 => {
                self.print_esc(1272i32);
            }
            _ => {
                self.print(1273i32);
            }
        }
    }

    /// Sometimes we need to convert \TeX's internal code numbers into symbolic
    /// form. The `print_skip_param` routine gives the symbolic name of a glue
    /// parameter.
    /// @<Declare the procedure called `print_skip_param`
    // §243
    pub fn print_skip_param(&mut self, mut n: i32) {
        match n {
            line_skip_code => {
                self.print_esc(389i32);
            }
            baseline_skip_code => {
                self.print_esc(390i32);
            }
            par_skip_code => {
                self.print_esc(391i32);
            }
            above_display_skip_code => {
                self.print_esc(392i32);
            }
            below_display_skip_code => {
                self.print_esc(393i32);
            }
            above_display_short_skip_code => {
                self.print_esc(394i32);
            }
            below_display_short_skip_code => {
                self.print_esc(395i32);
            }
            left_skip_code => {
                self.print_esc(396i32);
            }
            right_skip_code => {
                self.print_esc(397i32);
            }
            top_skip_code => {
                self.print_esc(398i32);
            }
            split_top_skip_code => {
                self.print_esc(399i32);
            }
            tab_skip_code => {
                self.print_esc(400i32);
            }
            space_skip_code => {
                self.print_esc(401i32);
            }
            xspace_skip_code => {
                self.print_esc(402i32);
            }
            par_fill_skip_code => {
                self.print_esc(403i32);
            }
            thin_mu_skip_code => {
                self.print_esc(404i32);
            }
            med_mu_skip_code => {
                self.print_esc(405i32);
            }
            thick_mu_skip_code => {
                self.print_esc(406i32);
            }
            _ => {
                self.print(407i32);
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
    // §200
    pub fn show_node_list(&mut self, mut p: i32) {
        let mut n: i32 = 0; // §200
        let mut g: f64 = 0.0; // §200
        'l_exit_f: {
            if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) > self.depth_threshold) {
                {
                    if (p > null) {
                        self.print(321i32);
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
                            self.print(322i32);
                            break 'l_exit_f;
                        }
                    }
                    n = (n).wrapping_add(1i32);
                    if (n > self.breadth_max) {
                        {
                            self.print(323i32);
                            break 'l_exit_f;
                        }
                    }
                    // §201
                    if (p >= self.hi_mem_min) {
                        self.print_font_and_char(p);
                    } else {
                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                            hlist_node | vlist_node | unset_node => {
                                // §202
                                {
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) {
                                        self.print_esc(104i32);
                                    } else {
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                            self.print_esc(118i32);
                                        } else {
                                            self.print_esc(328i32);
                                        }
                                    }
                                    self.print(329i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print_char(43i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(330i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == unset_node) {
                                        // §203
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() != min_quarterword) {
                                                {
                                                    self.print(288i32);
                                                    self.print_int((((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32)) as i64));
                                                    self.print(332i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(333i32);
                                                    self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(334i32);
                                                    self.print_glue(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int(), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0(), 0i32);
                                                }
                                            }
                                        }
                                    } else {
                                        // §202
                                        {
                                            // §204
                                            g = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].gr();
                                            if ((g != 0.0f64) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() != normal)) {
                                                {
                                                    self.print(335i32);
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() == shrinking) {
                                                        self.print(336i32);
                                                    }
                                                    if ((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int()).wrapping_abs() < 1048576i32) {
                                                        self.print(337i32);
                                                    } else {
                                                        if ((g).abs() > 20000.0f64) {
                                                            {
                                                                if (g > 0.0f64) {
                                                                    self.print_char(62i32);
                                                                } else {
                                                                    self.print(338i32);
                                                                }
                                                                self.print_glue((20000i32).wrapping_mul(unity), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                            }
                                                        } else {
                                                            self.print_glue(crate::system::pas_round((((unity) as f64) * g)), self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1(), 0i32);
                                                        }
                                                    }
                                                }
                                            }
                                            // §202
                                            if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int() != 0i32) {
                                                {
                                                    self.print(331i32);
                                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                }
                                            }
                                            if (self.eTeX_mode == 1i32) {
                                                // §1704
                                                if ((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && ((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(0i32) == dlist)) {
                                                    self.print(2020i32);
                                                }
                                            }
                                        }
                                    }
                                    // §202
                                    {
                                        {
                                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            rule_node => {
                                // §205
                                {
                                    self.print_esc(339i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print_char(43i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(330i32);
                                    self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                }
                            }
                            ins_node => {
                                // §206
                                {
                                    self.print_esc(340i32);
                                    self.print_int((((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(0i32)) as i64));
                                    self.print(341i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                    self.print(342i32);
                                    self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(), 0i32);
                                    self.print_char(44i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                    self.print(343i32);
                                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()) as i64));
                                    {
                                        {
                                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            whatsit_node => {
                                // §1603
                                match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                    open_node => {
                                        {
                                            self.print_write_whatsit(1714i32, p);
                                            self.print_char(61i32);
                                            self.print_file_name(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(), self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                        }
                                    }
                                    write_node => {
                                        {
                                            self.print_write_whatsit(678i32, p);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    close_node => {
                                        self.print_write_whatsit(1715i32, p);
                                    }
                                    special_node => {
                                        {
                                            self.print_esc(1716i32);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    latespecial_node => {
                                        {
                                            self.print_esc(1716i32);
                                            self.print(1875i32);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    language_node => {
                                        {
                                            self.print_esc(1718i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as i64));
                                            self.print(1876i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b0()) as i64));
                                            self.print_char(44i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().b1()) as i64));
                                            self.print_char(41i32);
                                        }
                                    }
                                    pdf_literal_node | pdf_lateliteral_node => {
                                        {
                                            self.print_esc(1719i32);
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_lateliteral_node) {
                                                self.print(1875i32);
                                            }
                                            match self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() {
                                                set_origin => {
                                                }
                                                direct_page => {
                                                    self.print(1000i32);
                                                }
                                                direct_always => {
                                                    self.print(1877i32);
                                                }
                                                _ => {
                                                    self.confusion(1878i32);
                                                }
                                            }
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    pdf_colorstack_node => {
                                        {
                                            self.print_esc(1879i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as i64));
                                            match self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() {
                                                colorstack_set => {
                                                    self.print(1880i32);
                                                }
                                                colorstack_push => {
                                                    self.print(1881i32);
                                                }
                                                colorstack_pop => {
                                                    self.print(1882i32);
                                                }
                                                colorstack_current => {
                                                    self.print(1883i32);
                                                }
                                                _ => {
                                                    self.confusion(1131i32);
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() <= colorstack_data) {
                                                self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                            }
                                        }
                                    }
                                    pdf_setmatrix_node => {
                                        {
                                            self.print_esc(1720i32);
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                    pdf_save_node => {
                                        {
                                            self.print_esc(1721i32);
                                        }
                                    }
                                    pdf_restore_node => {
                                        {
                                            self.print_esc(1722i32);
                                        }
                                    }
                                    pdf_refobj_node => {
                                        {
                                            self.print_esc(1724i32);
                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(1i32)) as usize)] > 0i32) {
                                                {
                                                    if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)] != null) {
                                                        {
                                                            self.print(1884i32);
                                                            self.print_mark(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)]);
                                                        }
                                                    }
                                                    self.print(1885i32);
                                                }
                                            }
                                            if (self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(3i32)) as usize)] > 0i32) {
                                                self.print(1886i32);
                                            }
                                            self.print_mark(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(0i32)) as usize)]);
                                        }
                                    }
                                    pdf_refxform_node => {
                                        {
                                            self.print_esc(1726i32);
                                            self.print(40i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(1i32)) as usize)]);
                                            self.print_char(43i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)]);
                                            self.print(330i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(0i32)) as usize)]);
                                        }
                                    }
                                    pdf_refximage_node => {
                                        {
                                            self.print_esc(1728i32);
                                            self.print(40i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(1i32)) as usize)]);
                                            self.print_char(43i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(2i32)) as usize)]);
                                            self.print(330i32);
                                            self.print_scaled(self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()) as usize)].int4).wrapping_add(0i32)) as usize)]);
                                        }
                                    }
                                    pdf_annot_node => {
                                        {
                                            self.print_esc(1729i32);
                                            // §1601
                                            self.print(40i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            self.print_char(43i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                            self.print(330i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            // §1603
                                            self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh());
                                        }
                                    }
                                    pdf_start_link_node => {
                                        {
                                            self.print_esc(1730i32);
                                            // §1601
                                            self.print(40i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            self.print_char(43i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                            self.print(330i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            // §1603
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh() != null) {
                                                {
                                                    self.print(1884i32);
                                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh());
                                                }
                                            }
                                            self.print(1887i32);
                                            if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() == pdf_action_user) {
                                                {
                                                    self.print(1888i32);
                                                    self.print_mark(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().lh());
                                                }
                                            } else {
                                                {
                                                    if (self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                                        {
                                                            self.print(1886i32);
                                                            self.print_mark(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh());
                                                        }
                                                    }
                                                    match self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() {
                                                        pdf_action_goto => {
                                                            {
                                                                if ((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b1() % 2i32) == 1i32) {
                                                                    {
                                                                        self.print(1889i32);
                                                                        self.print_mark(self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh());
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.print(1890i32);
                                                                        self.print_int(((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh()) as i64));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        pdf_action_page => {
                                                            {
                                                                self.print(1000i32);
                                                                self.print_int(((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh()) as i64));
                                                                self.print_mark(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().lh());
                                                            }
                                                        }
                                                        pdf_action_thread => {
                                                            {
                                                                if ((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b1() % 2i32) == 1i32) {
                                                                    {
                                                                        self.print(1891i32);
                                                                        self.print_mark(self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh());
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.print(1892i32);
                                                                        self.print_int(((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh()) as i64));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            self.pdf_error(1893i32, 1894i32);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    pdf_end_link_node => {
                                        self.print_esc(1731i32);
                                    }
                                    pdf_dest_node => {
                                        {
                                            self.print_esc(1733i32);
                                            if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().rh() != null) {
                                                {
                                                    self.print(1895i32);
                                                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().rh()) as i64));
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                {
                                                    self.print(1896i32);
                                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                                }
                                            } else {
                                                {
                                                    self.print(1897i32);
                                                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as i64));
                                                }
                                            }
                                            self.print(32i32);
                                            match self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b0() {
                                                pdf_dest_xyz => {
                                                    {
                                                        self.print(1835i32);
                                                        if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
                                                            {
                                                                self.print(1898i32);
                                                                self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh()) as i64));
                                                            }
                                                        }
                                                    }
                                                }
                                                pdf_dest_fitbh => {
                                                    self.print(1837i32);
                                                }
                                                pdf_dest_fitbv => {
                                                    self.print(1838i32);
                                                }
                                                pdf_dest_fitb => {
                                                    self.print(1839i32);
                                                }
                                                pdf_dest_fith => {
                                                    self.print(1840i32);
                                                }
                                                pdf_dest_fitv => {
                                                    self.print(1841i32);
                                                }
                                                pdf_dest_fitr => {
                                                    {
                                                        self.print(1842i32);
                                                        // §1601
                                                        self.print(40i32);
                                                        self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                        self.print_char(43i32);
                                                        self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                        self.print(330i32);
                                                        self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    }
                                                }
                                                pdf_dest_fit => {
                                                    // §1603
                                                    self.print(1843i32);
                                                }
                                                _ => {
                                                    self.print(1899i32);
                                                }
                                            }
                                        }
                                    }
                                    pdf_thread_node | pdf_start_thread_node => {
                                        {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_thread_node) {
                                                self.print_esc(1734i32);
                                            } else {
                                                self.print_esc(1735i32);
                                            }
                                            self.print(40i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            self.print_char(43i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                            self.print(330i32);
                                            self.print_rule_dimen(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
                                                {
                                                    self.print(1884i32);
                                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh());
                                                }
                                            }
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                {
                                                    self.print(1896i32);
                                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                                }
                                            } else {
                                                {
                                                    self.print(1897i32);
                                                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as i64));
                                                }
                                            }
                                        }
                                    }
                                    pdf_end_thread_node => {
                                        self.print_esc(1736i32);
                                    }
                                    pdf_save_pos_node => {
                                        self.print_esc(1737i32);
                                    }
                                    pdf_snap_ref_point_node => {
                                        self.print_esc(1738i32);
                                    }
                                    pdf_snapy_node => {
                                        {
                                            self.print_esc(1739i32);
                                            self.print_char(32i32);
                                            self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 0i32);
                                            self.print_char(32i32);
                                            self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(), 0i32);
                                        }
                                    }
                                    pdf_snapy_comp_node => {
                                        {
                                            self.print_esc(1740i32);
                                            self.print_char(32i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()) as i64));
                                        }
                                    }
                                    pdf_interword_space_on_node => {
                                        self.print_esc(1755i32);
                                    }
                                    pdf_interword_space_off_node => {
                                        self.print_esc(1756i32);
                                    }
                                    pdf_fake_space_node => {
                                        self.print_esc(1757i32);
                                    }
                                    pdf_running_link_off_node => {
                                        self.print_esc(1758i32);
                                    }
                                    pdf_running_link_on_node => {
                                        self.print_esc(1759i32);
                                    }
                                    _ => {
                                        self.print(1900i32);
                                    }
                                }
                            }
                            glue_node => {
                                // §207
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                    // §208
                                    {
                                        self.print_esc(348i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                            self.print_char(99i32);
                                        } else {
                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == x_leaders) {
                                                self.print_char(120i32);
                                            }
                                        }
                                        self.print(349i32);
                                        self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 0i32);
                                        {
                                            {
                                                self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                        }
                                    }
                                } else {
                                    // §207
                                    {
                                        self.print_esc(344i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() != normal) {
                                            {
                                                self.print_char(40i32);
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() < cond_math_glue) {
                                                    self.print_skip_param((self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(1i32));
                                                } else {
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() == cond_math_glue) {
                                                        self.print_esc(345i32);
                                                    } else {
                                                        self.print_esc(346i32);
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
                                                    self.print_spec(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), 347i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            margin_kern_node => {
                                // §201
                                {
                                    self.print_esc(324i32);
                                    self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() == left_side) {
                                        self.print(325i32);
                                    } else {
                                        self.print(326i32);
                                    }
                                }
                            }
                            kern_node => {
                                // §209
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() != mu_glue) {
                                    {
                                        self.print_esc(324i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() != normal) {
                                            self.print_char(32i32);
                                        }
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == acc_kern) {
                                            self.print(350i32);
                                        }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == auto_kern) {
                                            self.print(351i32);
                                        }
                                    }
                                } else {
                                    {
                                        self.print_esc(352i32);
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        self.print(347i32);
                                    }
                                }
                            }
                            math_node => {
                                // §210
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() > after) {
                                    {
                                        if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                            self.print_esc(353i32);
                                        } else {
                                            self.print_esc(354i32);
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
                                        self.print_esc(355i32);
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == before) {
                                            self.print(356i32);
                                        } else {
                                            self.print(357i32);
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() != 0i32) {
                                            {
                                                self.print(358i32);
                                                self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                    }
                                }
                            }
                            ligature_node => {
                                // §211
                                {
                                    self.print_font_and_char((p).wrapping_add(1i32));
                                    self.print(359i32);
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
                                // §212
                                {
                                    self.print_esc(360i32);
                                    self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()) as i64));
                                }
                            }
                            disc_node => {
                                // §213
                                {
                                    self.print_esc(361i32);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() > 0i32) {
                                        {
                                            self.print(362i32);
                                            self.print_int(((self.mem[crate::ix::U((p) as usize)].hh().b1()) as i64));
                                        }
                                    }
                                    {
                                        {
                                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                    {
                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 124i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            mark_node => {
                                // §214
                                {
                                    self.print_esc(363i32);
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() != 0i32) {
                                        {
                                            self.print_char(115i32);
                                            self.print_int(((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as i64));
                                        }
                                    }
                                    self.print_mark(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                }
                            }
                            adjust_node => {
                                // §215
                                {
                                    self.print_esc(364i32);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() != 0i32) {
                                        self.print(365i32);
                                    }
                                    {
                                        {
                                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 46i32;
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    }
                                }
                            }
                            style_node => {
                                // §866
                                self.print_style(self.mem[crate::ix::U((p) as usize)].hh().b1());
                            }
                            choice_node => {
                                // §871
                                {
                                    self.print_esc(603i32);
                                    {
                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 68i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 84i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 83i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                    {
                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 115i32;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.show_node_list(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                    self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                }
                            }
                            ord_noad | op_noad | bin_noad | rel_noad | open_noad | close_noad | punct_noad | inner_noad | radical_noad | over_noad | under_noad | vcenter_noad | accent_noad | left_noad | right_noad => {
                                // §872
                                {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        ord_noad => {
                                            self.print_esc(1274i32);
                                        }
                                        op_noad => {
                                            self.print_esc(1275i32);
                                        }
                                        bin_noad => {
                                            self.print_esc(1276i32);
                                        }
                                        rel_noad => {
                                            self.print_esc(1277i32);
                                        }
                                        open_noad => {
                                            self.print_esc(1278i32);
                                        }
                                        close_noad => {
                                            self.print_esc(1279i32);
                                        }
                                        punct_noad => {
                                            self.print_esc(1280i32);
                                        }
                                        inner_noad => {
                                            self.print_esc(1281i32);
                                        }
                                        over_noad => {
                                            self.print_esc(1282i32);
                                        }
                                        under_noad => {
                                            self.print_esc(1283i32);
                                        }
                                        vcenter_noad => {
                                            self.print_esc(618i32);
                                        }
                                        radical_noad => {
                                            {
                                                self.print_esc(612i32);
                                                self.print_delimiter((p).wrapping_add(4i32));
                                            }
                                        }
                                        accent_noad => {
                                            {
                                                self.print_esc(584i32);
                                                self.print_fam_and_char((p).wrapping_add(4i32));
                                            }
                                        }
                                        left_noad => {
                                            {
                                                self.print_esc(1284i32);
                                                self.print_delimiter((p).wrapping_add(1i32));
                                            }
                                        }
                                        right_noad => {
                                            {
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal) {
                                                    self.print_esc(1285i32);
                                                } else {
                                                    self.print_esc(1286i32);
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
                                                    self.print_esc(1287i32);
                                                } else {
                                                    self.print_esc(1288i32);
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
                                // §873
                                {
                                    self.print_esc(1289i32);
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == default_code) {
                                        self.print(1290i32);
                                    } else {
                                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                    }
                                    if ((((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0() != 0i32) || (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1() != min_quarterword)) || (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2() != 0i32)) || (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b3() != min_quarterword)) {
                                        {
                                            self.print(1291i32);
                                            self.print_delimiter((p).wrapping_add(4i32));
                                        }
                                    }
                                    if ((((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b0() != 0i32) || (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b1() != min_quarterword)) || (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b2() != 0i32)) || (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].qqqq().b3() != min_quarterword)) {
                                        {
                                            self.print(1292i32);
                                            self.print_delimiter((p).wrapping_add(5i32));
                                        }
                                    }
                                    self.print_subsidiary_data((p).wrapping_add(2i32), 92i32);
                                    self.print_subsidiary_data((p).wrapping_add(3i32), 47i32);
                                }
                            }
                            _ => {
                                // §201
                                self.print(327i32);
                            }
                        }
                    }
                    // §200
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
        }
    }

    /// The recursive machinery is started by calling `show_box`.
    // §216
    pub fn show_box(&mut self, mut p: halfword) {
        // §254
        self.depth_threshold = self.eqtb[crate::ix::U(((629043i32) - 1) as usize)].int();
        self.breadth_max = self.eqtb[crate::ix::U(((629042i32) - 1) as usize)].int();
        // §216
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
    // §218
    pub fn delete_token_ref(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().lh() == null) {
            self.flush_list(p);
        } else {
            { let __v69 = (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v69); }
        }
    }

    /// Similarly, `delete_glue_ref` is called when a pointer to a glue
    /// specification is being withdrawn.
    // §219
    pub fn delete_glue_ref(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().rh() == null) {
            self.free_node(p, glue_spec_size);
        } else {
            { let __v70 = (self.mem[crate::ix::U((p) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v70); }
        }
    }

    /// Now we are ready to delete any node list, recursively.
    /// In practice, the nodes deleted are usually charnodes (about 2/3 of the time),
    /// and they are glue nodes in about half of the remaining cases.
    // §220
    pub fn flush_node_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §220
        while (p != null) {
            {
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                if (p >= self.hi_mem_min) {
                    {
                        { let __v71 = self.avail; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v71); }
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
                                    // §1605
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
                                            pdf_literal_node | pdf_lateliteral_node => {
                                                {
                                                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                                    self.free_node(p, write_node_size);
                                                }
                                            }
                                            pdf_colorstack_node => {
                                                {
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() <= colorstack_data) {
                                                        {
                                                            self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh());
                                                            self.free_node(p, pdf_colorstack_setter_node_size);
                                                        }
                                                    } else {
                                                        self.free_node(p, pdf_colorstack_getter_node_size);
                                                    }
                                                }
                                            }
                                            pdf_setmatrix_node => {
                                                {
                                                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                                    self.free_node(p, pdf_setmatrix_node_size);
                                                }
                                            }
                                            pdf_save_node => {
                                                {
                                                    self.free_node(p, pdf_save_node_size);
                                                }
                                            }
                                            pdf_restore_node => {
                                                {
                                                    self.free_node(p, pdf_restore_node_size);
                                                }
                                            }
                                            pdf_refobj_node => {
                                                self.free_node(p, pdf_refobj_node_size);
                                            }
                                            pdf_refxform_node => {
                                                self.free_node(p, pdf_refxform_node_size);
                                            }
                                            pdf_refximage_node => {
                                                self.free_node(p, pdf_refximage_node_size);
                                            }
                                            pdf_annot_node => {
                                                {
                                                    self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh());
                                                    self.free_node(p, pdf_annot_node_size);
                                                }
                                            }
                                            pdf_start_link_node => {
                                                {
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh() != null) {
                                                        self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh());
                                                    }
                                                    {
                                                        if (self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().rh() == null) {
                                                            {
                                                                if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() == pdf_action_user) {
                                                                    self.delete_token_ref(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().lh());
                                                                } else {
                                                                    {
                                                                        if (self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh() != null) {
                                                                            self.delete_token_ref(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].hh().lh());
                                                                        }
                                                                        if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b0() == pdf_action_page) {
                                                                            self.delete_token_ref(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().lh());
                                                                        } else {
                                                                            if ((((((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b1()) != 0) && ((1i32) != 0))) as i32) == 1i32) {
                                                                                self.delete_token_ref(self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().rh());
                                                                            }
                                                                        }
                                                                        if ((((((self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().b1()) != 0) && ((2i32) != 0))) as i32) == 2i32) {
                                                                            self.delete_token_ref(self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(3i32)) as usize)].hh().rh());
                                                                        }
                                                                    }
                                                                }
                                                                self.free_node(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(), pdf_action_size);
                                                            }
                                                        } else {
                                                            { let __ix72 = (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32); let __v73 = (self.mem[crate::ix::U(((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix72) as usize)].set_hh_rh(__v73); }
                                                        }
                                                    }
                                                    self.free_node(p, pdf_annot_node_size);
                                                }
                                            }
                                            pdf_end_link_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_dest_node => {
                                                {
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                        self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                                    }
                                                    self.free_node(p, pdf_dest_node_size);
                                                }
                                            }
                                            pdf_thread_node | pdf_start_thread_node => {
                                                {
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                                        self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
                                                    }
                                                    if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
                                                        self.delete_token_ref(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh());
                                                    }
                                                    self.free_node(p, pdf_thread_node_size);
                                                }
                                            }
                                            pdf_end_thread_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_save_pos_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_snap_ref_point_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_snapy_node => {
                                                {
                                                    self.delete_glue_ref(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                                    self.free_node(p, snap_node_size);
                                                }
                                            }
                                            pdf_snapy_comp_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_interword_space_on_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_interword_space_off_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_fake_space_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_running_link_off_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            pdf_running_link_on_node => {
                                                self.free_node(p, small_node_size);
                                            }
                                            _ => {
                                                self.confusion(1902i32);
                                            }
                                        }
                                        break 'l_done_f;
                                    }
                                }
                                glue_node => {
                                    // §220
                                    {
                                        {
                                            if (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh() == null) {
                                                self.free_node(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(), glue_spec_size);
                                            } else {
                                                { let __ix74 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v75 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix74) as usize)].set_hh_rh(__v75); }
                                            }
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() != null) {
                                            self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                        }
                                    }
                                }
                                kern_node | math_node | penalty_node => {
                                }
                                margin_kern_node => {
                                    {
                                        {
                                            { let __ix76 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh(); let __v77 = self.avail; self.mem[crate::ix::U((__ix76) as usize)].set_hh_rh(__v77); }
                                            self.avail = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh();
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
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
                                    // §874
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
                                    // §220
                                    self.confusion(366i32);
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
    // §222
    pub fn copy_node_list(&mut self, mut p: halfword) -> halfword {
        let mut copy_node_list: halfword = 0;
        let mut h: halfword = 0; // §222
        let mut q: halfword = 0; // §222
        let mut r: halfword = 0; // §222
        let mut words: i32 = 0; // §222
        h = self.get_avail();
        q = h;
        while (p != null) {
            {
                // §223
                words = 1i32;
                if (p >= self.hi_mem_min) {
                    r = self.get_avail();
                } else {
                    // §224
                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                        hlist_node | vlist_node | unset_node => {
                            {
                                r = self.get_node(box_node_size);
                                { let __v78 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)] = __v78; }
                                { let __v79 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)] = __v79; }
                                { let __v80 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(__v80); }
                                words = 5i32;
                            }
                        }
                        rule_node => {
                            {
                                r = self.get_node(rule_node_size);
                                words = rule_node_size;
                            }
                        }
                        ins_node => {
                            {
                                r = self.get_node(ins_node_size);
                                { let __v81 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)] = __v81; }
                                { let __ix82 = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh(); let __v83 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().rh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix82) as usize)].set_hh_rh(__v83); }
                                { let __v84 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().lh()); self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_hh_lh(__v84); }
                                words = 4i32;
                            }
                        }
                        whatsit_node => {
                            // §1604
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
                                        { let __ix85 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v86 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix85) as usize)].set_hh_lh(__v86); }
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
                                        { let __ix87 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v88 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix87) as usize)].set_hh_lh(__v88); }
                                        words = write_node_size;
                                    }
                                }
                                pdf_colorstack_node => {
                                    {
                                        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() <= colorstack_data) {
                                            {
                                                r = self.get_node(pdf_colorstack_setter_node_size);
                                                { let __ix89 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh(); let __v90 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix89) as usize)].set_hh_lh(__v90); }
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
                                        { let __ix91 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v92 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix91) as usize)].set_hh_lh(__v92); }
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
                                        { let __ix93 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh(); let __v94 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix93) as usize)].set_hh_lh(__v94); }
                                        words = pdf_annot_node_size;
                                    }
                                }
                                pdf_start_link_node => {
                                    {
                                        r = self.get_node(pdf_annot_node_size);
                                        { let __v95 = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(__v95); }
                                        { let __v96 = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(__v96); }
                                        { let __v97 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v97); }
                                        { let __v98 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_lh(__v98); }
                                        if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh() != null) {
                                            { let __ix99 = self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh(); let __v100 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix99) as usize)].set_hh_lh(__v100); }
                                        }
                                        { let __v101 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(__v101); }
                                        { let __ix102 = (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32); let __v103 = (self.mem[crate::ix::U(((self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix102) as usize)].set_hh_rh(__v103); }
                                        { let __v104 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].int(); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_int(__v104); }
                                    }
                                }
                                pdf_end_link_node => {
                                    r = self.get_node(small_node_size);
                                }
                                pdf_dest_node => {
                                    {
                                        r = self.get_node(pdf_dest_node_size);
                                        if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                            { let __ix105 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v106 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix105) as usize)].set_hh_lh(__v106); }
                                        }
                                        words = pdf_dest_node_size;
                                    }
                                }
                                pdf_thread_node | pdf_start_thread_node => {
                                    {
                                        r = self.get_node(pdf_thread_node_size);
                                        if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().b1() > 0i32) {
                                            { let __ix107 = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh(); let __v108 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix107) as usize)].set_hh_lh(__v108); }
                                        }
                                        if (self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh() != null) {
                                            { let __ix109 = self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh(); let __v110 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix109) as usize)].set_hh_lh(__v110); }
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
                                        { let __ix111 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v112 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix111) as usize)].set_hh_rh(__v112); }
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
                                    self.confusion(1901i32);
                                }
                            }
                        }
                        glue_node => {
                            // §224
                            {
                                r = self.get_node(small_node_size);
                                { let __ix113 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); let __v114 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix113) as usize)].set_hh_rh(__v114); }
                                { let __v115 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh(); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v115); }
                                { let __v116 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v116); }
                            }
                        }
                        kern_node | math_node | penalty_node => {
                            {
                                r = self.get_node(small_node_size);
                                words = small_node_size;
                            }
                        }
                        margin_kern_node => {
                            {
                                r = self.get_node(margin_kern_node_size);
                                {
                                    { let __v117 = self.avail; self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(__v117); }
                                    if (self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh() == null) {
                                        { let __v118 = self.get_avail(); self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_hh_lh(__v118); }
                                    } else {
                                        {
                                            self.avail = self.mem[crate::ix::U((self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().rh();
                                            { let __ix119 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh(); self.mem[crate::ix::U((__ix119) as usize)].set_hh_rh(null); }
                                            self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                            self.dl_new_node(self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh());
                                        }
                                    }
                                }
                                { let __ix120 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh(); let __v121 = self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b0(); self.mem[crate::ix::U((__ix120) as usize)].set_hh_b0(__v121); }
                                { let __ix122 = self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].hh().lh(); let __v123 = self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh()) as usize)].hh().b1(); self.mem[crate::ix::U((__ix122) as usize)].set_hh_b1(__v123); }
                                words = small_node_size;
                            }
                        }
                        ligature_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v124 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)] = __v124; }
                                { let __v125 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v125); }
                            }
                        }
                        disc_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v126 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_lh(__v126); }
                                { let __v127 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_hh_rh(__v127); }
                            }
                        }
                        mark_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __ix128 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(); let __v129 = (self.mem[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh()) as usize)].hh().lh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix128) as usize)].set_hh_lh(__v129); }
                                words = small_node_size;
                            }
                        }
                        adjust_node => {
                            {
                                r = self.get_node(small_node_size);
                                { let __v130 = self.copy_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()); self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(__v130); }
                            }
                        }
                        _ => {
                            self.confusion(367i32);
                        }
                    }
                }
                // §223
                while (words > 0i32) {
                    {
                        words = (words).wrapping_sub(1i32);
                        { let __v131 = self.mem[crate::ix::U(((p).wrapping_add(words)) as usize)]; self.mem[crate::ix::U(((r).wrapping_add(words)) as usize)] = __v131; }
                    }
                }
                // §222
                self.dl_copy(r, p);
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                q = r;
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(null);
        q = self.mem[crate::ix::U((h) as usize)].hh().rh();
        {
            { let __v132 = self.avail; self.mem[crate::ix::U((h) as usize)].set_hh_rh(__v132); }
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
    // §229
    pub fn print_mode(&mut self, mut m: i32) {
        if (m > 0i32) {
            match (m / 104i32) {
                0 => {
                    self.print(368i32);
                }
                1 => {
                    self.print(369i32);
                }
                2 => {
                    self.print(370i32);
                }
                _ => {}
            }
        } else {
            if (m == 0i32) {
                self.print(371i32);
            } else {
                match ((m).wrapping_neg() / 104i32) {
                    0 => {
                        self.print(372i32);
                    }
                    1 => {
                        self.print(373i32);
                    }
                    2 => {
                        self.print(355i32);
                    }
                    _ => {}
                }
            }
        }
        self.print(374i32);
    }

    /// When \TeX's work on one level is interrupted, the state is saved by
    /// calling `push_nest`. This routine changes `head` and `tail` so that
    /// a new (empty) list is begun; it does not change `mode` or `aux`.
    // §234
    pub fn push_nest(&mut self) {
        if (self.nest_ptr > self.max_nest_stack) {
            {
                self.max_nest_stack = self.nest_ptr;
                if (self.nest_ptr == nest_size) {
                    self.overflow(375i32, nest_size);
                }
            }
        }
        { let __ix133 = self.nest_ptr; let __v134 = self.cur_list; self.nest[crate::ix::U((__ix133) as usize)] = __v134; }
        self.nest_ptr = (self.nest_ptr).wrapping_add(1i32);
        self.cur_list.head_field = self.get_avail();
        self.cur_list.tail_field = self.cur_list.head_field;
        self.cur_list.pg_field = 0i32;
        self.cur_list.ml_field = self.line;
        self.cur_list.eTeX_aux_field = null;
    }

    /// Conversely, when \TeX\ is finished on the current level, the former
    /// state is restored by calling `pop_nest`. This routine will never be
    /// called at the lowest semantic level, nor will it be called unless `head`
    /// is a node that should be returned to free memory.
    // §235
    pub fn pop_nest(&mut self) {
        {
            { let __ix135 = self.cur_list.head_field; let __v136 = self.avail; self.mem[crate::ix::U((__ix135) as usize)].set_hh_rh(__v136); }
            self.avail = self.cur_list.head_field;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.nest_ptr = (self.nest_ptr).wrapping_sub(1i32);
        self.cur_list = self.nest[crate::ix::U((self.nest_ptr) as usize)];
    }

    /// Here is a procedure that displays what \TeX\ is working on, at all levels.
    // §236
    pub fn show_activities(&mut self) {
        let mut p: i32 = 0; // §236
        let mut m: i32 = 0; // §236
        let mut a: memory_word = memory_word::default(); // §236
        let mut q: halfword = 0; // §236
        let mut r: halfword = 0; // §236
        let mut t: i32 = 0; // §236
        { let __ix137 = self.nest_ptr; let __v138 = self.cur_list; self.nest[crate::ix::U((__ix137) as usize)] = __v138; }
        self.print_nl(348i32);
        self.print_ln();
        {
            let __for_end_2 = 0i32;
            p = self.nest_ptr;
            while p >= __for_end_2 {
                {
                    m = self.nest[crate::ix::U((p) as usize)].mode_field;
                    a = self.nest[crate::ix::U((p) as usize)].aux_field;
                    self.print_nl(376i32);
                    self.print_mode(m);
                    self.print(377i32);
                    self.print_int((((self.nest[crate::ix::U((p) as usize)].ml_field).wrapping_abs()) as i64));
                    if (m == hmode) {
                        if (self.nest[crate::ix::U((p) as usize)].pg_field != 8585216i32) {
                            {
                                self.print(378i32);
                                self.print_int((((self.nest[crate::ix::U((p) as usize)].pg_field % 65536i32)) as i64));
                                self.print(379i32);
                                self.print_int((((self.nest[crate::ix::U((p) as usize)].pg_field / 4194304i32)) as i64));
                                self.print_char(44i32);
                                self.print_int(((((self.nest[crate::ix::U((p) as usize)].pg_field / 65536i32) % 64i32)) as i64));
                                self.print_char(41i32);
                            }
                        }
                    }
                    if (self.nest[crate::ix::U((p) as usize)].ml_field < 0i32) {
                        self.print(380i32);
                    }
                    if (p == 0i32) {
                        {
                            // §1163
                            if (page_head != self.page_tail) {
                                {
                                    self.print_nl(1395i32);
                                    if self.output_active {
                                        self.print(1396i32);
                                    }
                                    self.show_box(self.mem[crate::ix::U((page_head) as usize)].hh().rh());
                                    if (self.page_contents > empty) {
                                        {
                                            self.print_nl(1397i32);
                                            self.print_totals();
                                            self.print_nl(1398i32);
                                            self.print_scaled(self.page_so_far[crate::ix::U((0i32) as usize)]);
                                            r = self.mem[crate::ix::U((page_ins_head) as usize)].hh().rh();
                                            while (r != page_ins_head) {
                                                {
                                                    self.print_ln();
                                                    self.print_esc(340i32);
                                                    t = (self.mem[crate::ix::U((r) as usize)].hh().b1()).wrapping_sub(0i32);
                                                    self.print_int(((t) as i64));
                                                    self.print(1399i32);
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
                                                            self.print(1400i32);
                                                            self.print_int(((t) as i64));
                                                            self.print(1401i32);
                                                        }
                                                    }
                                                    r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §236
                            if (self.mem[crate::ix::U((contrib_head) as usize)].hh().rh() != null) {
                                self.print_nl(381i32);
                            }
                        }
                    }
                    self.show_box(self.mem[crate::ix::U((self.nest[crate::ix::U((p) as usize)].head_field) as usize)].hh().rh());
                    // §237
                    match ((m).wrapping_abs() / 104i32) {
                        0 => {
                            {
                                self.print_nl(382i32);
                                if (a.int() <= self.eqtb[crate::ix::U(((629672i32) - 1) as usize)].int()) {
                                    self.print(383i32);
                                } else {
                                    self.print_scaled(a.int());
                                }
                                if (self.nest[crate::ix::U((p) as usize)].pg_field != 0i32) {
                                    {
                                        self.print(384i32);
                                        self.print_int(((self.nest[crate::ix::U((p) as usize)].pg_field) as i64));
                                        self.print(385i32);
                                        if (self.nest[crate::ix::U((p) as usize)].pg_field != 1i32) {
                                            self.print_char(115i32);
                                        }
                                    }
                                }
                            }
                        }
                        1 => {
                            {
                                self.print_nl(386i32);
                                self.print_int(((a.hh().lh()) as i64));
                                if (m > 0i32) {
                                    if (a.hh().rh() > 0i32) {
                                        {
                                            self.print(387i32);
                                            self.print_int(((a.hh().rh()) as i64));
                                        }
                                    }
                                }
                            }
                        }
                        2 => {
                            if (a.int() != null) {
                                {
                                    self.print(388i32);
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
    // §255
    pub fn print_param(&mut self, mut n: i32) {
        match n {
            pretolerance_code => {
                self.print_esc(436i32);
            }
            tolerance_code => {
                self.print_esc(437i32);
            }
            line_penalty_code => {
                self.print_esc(438i32);
            }
            hyphen_penalty_code => {
                self.print_esc(439i32);
            }
            ex_hyphen_penalty_code => {
                self.print_esc(440i32);
            }
            club_penalty_code => {
                self.print_esc(441i32);
            }
            widow_penalty_code => {
                self.print_esc(442i32);
            }
            display_widow_penalty_code => {
                self.print_esc(443i32);
            }
            broken_penalty_code => {
                self.print_esc(444i32);
            }
            bin_op_penalty_code => {
                self.print_esc(445i32);
            }
            rel_penalty_code => {
                self.print_esc(446i32);
            }
            pre_display_penalty_code => {
                self.print_esc(447i32);
            }
            post_display_penalty_code => {
                self.print_esc(448i32);
            }
            inter_line_penalty_code => {
                self.print_esc(449i32);
            }
            double_hyphen_demerits_code => {
                self.print_esc(450i32);
            }
            final_hyphen_demerits_code => {
                self.print_esc(451i32);
            }
            adj_demerits_code => {
                self.print_esc(452i32);
            }
            mag_code => {
                self.print_esc(453i32);
            }
            delimiter_factor_code => {
                self.print_esc(454i32);
            }
            looseness_code => {
                self.print_esc(455i32);
            }
            time_code => {
                self.print_esc(456i32);
            }
            day_code => {
                self.print_esc(457i32);
            }
            month_code => {
                self.print_esc(458i32);
            }
            year_code => {
                self.print_esc(459i32);
            }
            show_box_breadth_code => {
                self.print_esc(460i32);
            }
            show_box_depth_code => {
                self.print_esc(461i32);
            }
            hbadness_code => {
                self.print_esc(462i32);
            }
            vbadness_code => {
                self.print_esc(463i32);
            }
            pausing_code => {
                self.print_esc(464i32);
            }
            tracing_online_code => {
                self.print_esc(465i32);
            }
            tracing_macros_code => {
                self.print_esc(466i32);
            }
            tracing_stats_code => {
                self.print_esc(467i32);
            }
            tracing_paragraphs_code => {
                self.print_esc(468i32);
            }
            tracing_pages_code => {
                self.print_esc(469i32);
            }
            tracing_output_code => {
                self.print_esc(470i32);
            }
            tracing_lost_chars_code => {
                self.print_esc(471i32);
            }
            tracing_commands_code => {
                self.print_esc(472i32);
            }
            tracing_restores_code => {
                self.print_esc(473i32);
            }
            uc_hyph_code => {
                self.print_esc(474i32);
            }
            output_penalty_code => {
                self.print_esc(475i32);
            }
            max_dead_cycles_code => {
                self.print_esc(476i32);
            }
            hang_after_code => {
                self.print_esc(477i32);
            }
            floating_penalty_code => {
                self.print_esc(478i32);
            }
            global_defs_code => {
                self.print_esc(479i32);
            }
            cur_fam_code => {
                self.print_esc(480i32);
            }
            escape_char_code => {
                self.print_esc(481i32);
            }
            default_hyphen_char_code => {
                self.print_esc(482i32);
            }
            default_skew_char_code => {
                self.print_esc(483i32);
            }
            end_line_char_code => {
                self.print_esc(484i32);
            }
            new_line_char_code => {
                self.print_esc(485i32);
            }
            language_code => {
                self.print_esc(486i32);
            }
            left_hyphen_min_code => {
                self.print_esc(487i32);
            }
            right_hyphen_min_code => {
                self.print_esc(488i32);
            }
            holding_inserts_code => {
                self.print_esc(489i32);
            }
            error_context_lines_code => {
                self.print_esc(490i32);
            }
            char_sub_def_min_code => {
                self.print_esc(491i32);
            }
            char_sub_def_max_code => {
                self.print_esc(492i32);
            }
            tracing_char_sub_def_code => {
                self.print_esc(493i32);
            }
            tracing_stack_levels_code => {
                self.print_esc(494i32);
            }
            partoken_context_code => {
                self.print_esc(495i32);
            }
            show_stream_code => {
                self.print_esc(496i32);
            }
            pdf_output_code => {
                self.print_esc(497i32);
            }
            pdf_compress_level_code => {
                self.print_esc(498i32);
            }
            pdf_objcompresslevel_code => {
                self.print_esc(499i32);
            }
            pdf_decimal_digits_code => {
                self.print_esc(500i32);
            }
            pdf_move_chars_code => {
                self.print_esc(501i32);
            }
            pdf_image_resolution_code => {
                self.print_esc(502i32);
            }
            pdf_pk_resolution_code => {
                self.print_esc(503i32);
            }
            pdf_unique_resname_code => {
                self.print_esc(504i32);
            }
            pdf_option_always_use_pdfpagebox_code => {
                self.print_esc(505i32);
            }
            pdf_option_pdf_inclusion_errorlevel_code => {
                self.print_esc(506i32);
            }
            pdf_major_version_code => {
                self.print_esc(507i32);
            }
            pdf_minor_version_code => {
                self.print_esc(508i32);
            }
            pdf_force_pagebox_code => {
                self.print_esc(509i32);
            }
            pdf_pagebox_code => {
                self.print_esc(510i32);
            }
            pdf_inclusion_errorlevel_code => {
                self.print_esc(511i32);
            }
            pdf_gamma_code => {
                self.print_esc(512i32);
            }
            pdf_image_gamma_code => {
                self.print_esc(513i32);
            }
            pdf_image_hicolor_code => {
                self.print_esc(514i32);
            }
            pdf_image_apply_gamma_code => {
                self.print_esc(515i32);
            }
            pdf_adjust_spacing_code => {
                self.print_esc(516i32);
            }
            pdf_protrude_chars_code => {
                self.print_esc(517i32);
            }
            pdf_tracing_fonts_code => {
                self.print_esc(518i32);
            }
            pdf_adjust_interword_glue_code => {
                self.print_esc(519i32);
            }
            pdf_prepend_kern_code => {
                self.print_esc(520i32);
            }
            pdf_append_kern_code => {
                self.print_esc(521i32);
            }
            pdf_gen_tounicode_code => {
                self.print_esc(522i32);
            }
            pdf_draftmode_code => {
                self.print_esc(523i32);
            }
            pdf_inclusion_copy_font_code => {
                self.print_esc(524i32);
            }
            pdf_suppress_warning_dup_dest_code => {
                self.print_esc(525i32);
            }
            pdf_suppress_warning_dup_map_code => {
                self.print_esc(526i32);
            }
            pdf_suppress_warning_page_group_code => {
                self.print_esc(527i32);
            }
            pdf_info_omit_date_code => {
                self.print_esc(528i32);
            }
            pdf_suppress_ptex_info_code => {
                self.print_esc(529i32);
            }
            pdf_omit_charset_code => {
                self.print_esc(530i32);
            }
            pdf_omit_info_dict_code => {
                self.print_esc(531i32);
            }
            pdf_omit_procset_code => {
                self.print_esc(532i32);
            }
            pdf_ptex_use_underscore_code => {
                self.print_esc(533i32);
            }
            tracing_assigns_code => {
                // §1659
                self.print_esc(1967i32);
            }
            tracing_groups_code => {
                self.print_esc(1968i32);
            }
            tracing_ifs_code => {
                self.print_esc(1969i32);
            }
            tracing_scan_tokens_code => {
                self.print_esc(1970i32);
            }
            tracing_nesting_code => {
                self.print_esc(1971i32);
            }
            pre_display_direction_code => {
                self.print_esc(1972i32);
            }
            last_line_fit_code => {
                self.print_esc(1973i32);
            }
            saving_vdiscards_code => {
                self.print_esc(1974i32);
            }
            saving_hyph_codes_code => {
                self.print_esc(1975i32);
            }
            ignore_primitive_error_code => {
                self.print_esc(1976i32);
            }
            108 => {
                // §1700
                self.print_esc(2015i32);
            }
            synctex_code => {
                // §1883
                self.print_esc(2070i32);
            }
            _ => {
                // §255
                self.print(534i32);
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
    // §259
    pub fn fix_date_and_time(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.sys_time); let mut __f1 = ::core::mem::take(&mut self.sys_day); let mut __f2 = ::core::mem::take(&mut self.sys_month); let mut __f3 = ::core::mem::take(&mut self.sys_year); let __r = self.date_and_time(&mut __f0, &mut __f1, &mut __f2, &mut __f3); self.sys_time = __f0; self.sys_day = __f1; self.sys_month = __f2; self.sys_year = __f3; __r };
        { let __v139 = self.sys_time; self.eqtb[crate::ix::U(((629038i32) - 1) as usize)].set_int(__v139); }
        { let __v140 = self.sys_day; self.eqtb[crate::ix::U(((629039i32) - 1) as usize)].set_int(__v140); }
        { let __v141 = self.sys_month; self.eqtb[crate::ix::U(((629040i32) - 1) as usize)].set_int(__v141); }
        { let __v142 = self.sys_year; self.eqtb[crate::ix::U(((629041i32) - 1) as usize)].set_int(__v142); }
    }

    /// \TeX\ is occasionally supposed to print diagnostic information that
    /// goes only into the transcript file, unless `tracing_online` is positive.
    /// Here are two routines that adjust the destination of print commands:
    // §263
    pub fn begin_diagnostic(&mut self) {
        self.old_setting = self.selector;
        if ((self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].int() <= 0i32) && (self.selector == term_and_log)) {
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
    // §263
    pub fn end_diagnostic(&mut self, mut blank_line: bool) {
        self.print_nl(348i32);
        if blank_line {
            self.print_ln();
        }
        self.selector = self.old_setting;
    }

    /// The final region of `eqtb` contains the dimension parameters defined
    /// here, and the 256 \.{\\dimen} registers.
    // §265
    pub fn print_length_param(&mut self, mut n: i32) {
        match n {
            par_indent_code => {
                self.print_esc(539i32);
            }
            math_surround_code => {
                self.print_esc(540i32);
            }
            line_skip_limit_code => {
                self.print_esc(541i32);
            }
            hsize_code => {
                self.print_esc(542i32);
            }
            vsize_code => {
                self.print_esc(543i32);
            }
            max_depth_code => {
                self.print_esc(544i32);
            }
            split_max_depth_code => {
                self.print_esc(545i32);
            }
            box_max_depth_code => {
                self.print_esc(546i32);
            }
            hfuzz_code => {
                self.print_esc(547i32);
            }
            vfuzz_code => {
                self.print_esc(548i32);
            }
            delimiter_shortfall_code => {
                self.print_esc(549i32);
            }
            null_delimiter_space_code => {
                self.print_esc(550i32);
            }
            script_space_code => {
                self.print_esc(551i32);
            }
            pre_display_size_code => {
                self.print_esc(552i32);
            }
            display_width_code => {
                self.print_esc(553i32);
            }
            display_indent_code => {
                self.print_esc(554i32);
            }
            overfull_rule_code => {
                self.print_esc(555i32);
            }
            hang_indent_code => {
                self.print_esc(556i32);
            }
            h_offset_code => {
                self.print_esc(557i32);
            }
            v_offset_code => {
                self.print_esc(558i32);
            }
            emergency_stretch_code => {
                self.print_esc(559i32);
            }
            pdf_h_origin_code => {
                self.print_esc(560i32);
            }
            pdf_v_origin_code => {
                self.print_esc(561i32);
            }
            pdf_page_width_code => {
                self.print_esc(562i32);
            }
            pdf_page_height_code => {
                self.print_esc(563i32);
            }
            pdf_link_margin_code => {
                self.print_esc(564i32);
            }
            pdf_dest_margin_code => {
                self.print_esc(565i32);
            }
            pdf_thread_margin_code => {
                self.print_esc(566i32);
            }
            pdf_first_line_height_code => {
                self.print_esc(567i32);
            }
            pdf_last_line_depth_code => {
                self.print_esc(568i32);
            }
            pdf_each_line_height_code => {
                self.print_esc(569i32);
            }
            pdf_each_line_depth_code => {
                self.print_esc(570i32);
            }
            pdf_ignored_dimen_code => {
                self.print_esc(571i32);
            }
            pdf_px_dimen_code => {
                self.print_esc(572i32);
            }
            _ => {
                self.print(573i32);
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
    // §320
    pub fn print_cmd_chr(&mut self, mut cmd: quarterword, mut chr_code: halfword) {
        let mut n: i32 = 0; // §320
        match cmd {
            left_brace => {
                {
                    self.print(639i32);
                    self.print(chr_code);
                }
            }
            right_brace => {
                {
                    self.print(640i32);
                    self.print(chr_code);
                }
            }
            math_shift => {
                {
                    self.print(641i32);
                    self.print(chr_code);
                }
            }
            mac_param => {
                {
                    self.print(642i32);
                    self.print(chr_code);
                }
            }
            sup_mark => {
                {
                    self.print(643i32);
                    self.print(chr_code);
                }
            }
            sub_mark => {
                {
                    self.print(644i32);
                    self.print(chr_code);
                }
            }
            endv => {
                self.print(645i32);
            }
            spacer => {
                {
                    self.print(646i32);
                    self.print(chr_code);
                }
            }
            letter => {
                {
                    self.print(647i32);
                    self.print(chr_code);
                }
            }
            other_char => {
                {
                    self.print(648i32);
                    self.print(chr_code);
                }
            }
            assign_glue | assign_mu_glue => {
                // §245
                if (chr_code < skip_base) {
                    self.print_skip_param((chr_code).wrapping_sub(626628i32));
                } else {
                    if (chr_code < mu_skip_base) {
                        {
                            self.print_esc(408i32);
                            self.print_int((((chr_code).wrapping_sub(626646i32)) as i64));
                        }
                    } else {
                        {
                            self.print_esc(409i32);
                            self.print_int((((chr_code).wrapping_sub(626902i32)) as i64));
                        }
                    }
                }
            }
            assign_toks => {
                // §249
                if (chr_code >= toks_base) {
                    {
                        self.print_esc(423i32);
                        self.print_int((((chr_code).wrapping_sub(627173i32)) as i64));
                    }
                } else {
                    match chr_code {
                        output_routine_loc => {
                            self.print_esc(410i32);
                        }
                        every_par_loc => {
                            self.print_esc(411i32);
                        }
                        every_math_loc => {
                            self.print_esc(412i32);
                        }
                        every_display_loc => {
                            self.print_esc(413i32);
                        }
                        every_hbox_loc => {
                            self.print_esc(414i32);
                        }
                        every_vbox_loc => {
                            self.print_esc(415i32);
                        }
                        every_job_loc => {
                            self.print_esc(416i32);
                        }
                        every_cr_loc => {
                            self.print_esc(417i32);
                        }
                        every_eof_loc => {
                            // §1658
                            self.print_esc(1966i32);
                        }
                        pdf_pages_attr_loc => {
                            // §249
                            self.print_esc(419i32);
                        }
                        pdf_page_attr_loc => {
                            self.print_esc(420i32);
                        }
                        pdf_page_resources_loc => {
                            self.print_esc(421i32);
                        }
                        pdf_pk_mode_loc => {
                            self.print_esc(422i32);
                        }
                        _ => {
                            self.print_esc(418i32);
                        }
                    }
                }
            }
            assign_int => {
                // §257
                if (chr_code < count_base) {
                    self.print_param((chr_code).wrapping_sub(629018i32));
                } else {
                    {
                        self.print_esc(537i32);
                        self.print_int((((chr_code).wrapping_sub(629128i32)) as i64));
                    }
                }
            }
            assign_dimen => {
                // §267
                if (chr_code < scaled_base) {
                    self.print_length_param((chr_code).wrapping_sub(629640i32));
                } else {
                    {
                        self.print_esc(574i32);
                        self.print_int((((chr_code).wrapping_sub(629674i32)) as i64));
                    }
                }
            }
            accent => {
                // §288
                self.print_esc(584i32);
            }
            advance => {
                self.print_esc(585i32);
            }
            after_assignment => {
                self.print_esc(586i32);
            }
            after_group => {
                self.print_esc(587i32);
            }
            assign_font_dimen => {
                self.print_esc(597i32);
            }
            begin_group => {
                self.print_esc(588i32);
            }
            break_penalty => {
                self.print_esc(610i32);
            }
            char_num => {
                self.print_esc(589i32);
            }
            cs_name => {
                self.print_esc(580i32);
            }
            def_font => {
                self.print_esc(594i32);
            }
            letterspace_font => {
                self.print_esc(595i32);
            }
            pdf_copy_font => {
                self.print_esc(596i32);
            }
            delim_num => {
                self.print_esc(590i32);
            }
            divide => {
                self.print_esc(591i32);
            }
            end_cs_name => {
                self.print_esc(581i32);
            }
            end_group => {
                self.print_esc(592i32);
            }
            ex_space => {
                self.print_esc(32i32);
            }
            expand_after => {
                if (chr_code == 0i32) {
                    self.print_esc(593i32);
                } else {
                    // §1763
                    self.print_esc(929i32);
                }
            }
            halign => {
                // §288
                self.print_esc(598i32);
            }
            hrule => {
                self.print_esc(599i32);
            }
            ignore_spaces => {
                if (chr_code == 0i32) {
                    self.print_esc(600i32);
                } else {
                    self.print_esc(577i32);
                }
            }
            insert => {
                self.print_esc(340i32);
            }
            ital_corr => {
                self.print_esc(47i32);
            }
            mark => {
                {
                    self.print_esc(363i32);
                    if (chr_code > 0i32) {
                        self.print_char(115i32);
                    }
                }
            }
            math_accent => {
                self.print_esc(601i32);
            }
            math_char_num => {
                self.print_esc(602i32);
            }
            math_choice => {
                self.print_esc(603i32);
            }
            multiply => {
                self.print_esc(604i32);
            }
            no_align => {
                self.print_esc(605i32);
            }
            no_boundary => {
                self.print_esc(606i32);
            }
            no_expand => {
                if (chr_code == 0i32) {
                    self.print_esc(607i32);
                } else {
                    self.print_esc(577i32);
                }
            }
            non_script => {
                self.print_esc(345i32);
            }
            omit => {
                self.print_esc(608i32);
            }
            radical => {
                self.print_esc(612i32);
            }
            read_to_cs => {
                if (chr_code == 0i32) {
                    self.print_esc(613i32);
                } else {
                    // §1760
                    self.print_esc(2030i32);
                }
            }
            relax => {
                // §288
                self.print_esc(614i32);
            }
            set_box => {
                self.print_esc(615i32);
            }
            set_prev_graf => {
                self.print_esc(611i32);
            }
            set_shape => {
                match chr_code {
                    par_shape_loc => {
                        self.print_esc(609i32);
                    }
                    inter_line_penalties_loc => {
                        // §1865
                        self.print_esc(2066i32);
                    }
                    club_penalties_loc => {
                        self.print_esc(2067i32);
                    }
                    widow_penalties_loc => {
                        self.print_esc(2068i32);
                    }
                    display_widow_penalties_loc => {
                        self.print_esc(2069i32);
                    }
                    _ => {}
                }
            }
            the => {
                // §288
                if (chr_code == 0i32) {
                    self.print_esc(616i32);
                } else {
                    // §1687
                    if (chr_code == 1i32) {
                        self.print_esc(2005i32);
                    } else {
                        self.print_esc(2006i32);
                    }
                }
            }
            toks_register => {
                // §1833
                {
                    self.print_esc(423i32);
                    if (chr_code != mem_bot) {
                        self.print_sa_num(chr_code);
                    }
                }
            }
            vadjust => {
                // §288
                self.print_esc(364i32);
            }
            valign => {
                if (chr_code == 0i32) {
                    self.print_esc(617i32);
                } else {
                    // §1702
                    match chr_code {
                        begin_L_code => {
                            self.print_esc(2016i32);
                        }
                        end_L_code => {
                            self.print_esc(2017i32);
                        }
                        begin_R_code => {
                            self.print_esc(2018i32);
                        }
                        _ => {
                            self.print_esc(2019i32);
                        }
                    }
                }
            }
            vcenter => {
                // §288
                self.print_esc(618i32);
            }
            vrule => {
                self.print_esc(619i32);
            }
            partoken_name => {
                self.print_esc(535i32);
            }
            par_end => {
                // §357
                self.print_esc(682i32);
            }
            input => {
                // §403
                if (chr_code == 0i32) {
                    self.print_esc(716i32);
                } else {
                    // §1748
                    if (chr_code == 2i32) {
                        self.print_esc(2028i32);
                    } else {
                        // §403
                        self.print_esc(717i32);
                    }
                }
            }
            top_bot_mark => {
                // §411
                {
                    match (chr_code % marks_code) {
                        first_mark_code => {
                            self.print_esc(719i32);
                        }
                        bot_mark_code => {
                            self.print_esc(720i32);
                        }
                        split_first_mark_code => {
                            self.print_esc(721i32);
                        }
                        split_bot_mark_code => {
                            self.print_esc(722i32);
                        }
                        _ => {
                            self.print_esc(718i32);
                        }
                    }
                    if (chr_code >= marks_code) {
                        self.print_char(115i32);
                    }
                }
            }
            register => {
                // §1832
                {
                    if ((chr_code < mem_bot) || (chr_code > lo_mem_stat_max)) {
                        cmd = (self.mem[crate::ix::U((chr_code) as usize)].hh().b0() / 16i32);
                    } else {
                        {
                            cmd = (chr_code).wrapping_sub(0i32);
                            chr_code = null;
                        }
                    }
                    if (cmd == int_val) {
                        self.print_esc(537i32);
                    } else {
                        if (cmd == dimen_val) {
                            self.print_esc(574i32);
                        } else {
                            if (cmd == glue_val) {
                                self.print_esc(408i32);
                            } else {
                                self.print_esc(409i32);
                            }
                        }
                    }
                    if (chr_code != null) {
                        self.print_sa_num(chr_code);
                    }
                }
            }
            set_aux => {
                // §443
                if (chr_code == vmode) {
                    self.print_esc(756i32);
                } else {
                    self.print_esc(755i32);
                }
            }
            set_page_int => {
                if (chr_code == 0i32) {
                    self.print_esc(757i32);
                } else {
                    // §1693
                    if (chr_code == 2i32) {
                        self.print_esc(2011i32);
                    } else {
                        // §443
                        self.print_esc(758i32);
                    }
                }
            }
            set_box_dimen => {
                if (chr_code == width_offset) {
                    self.print_esc(759i32);
                } else {
                    if (chr_code == height_offset) {
                        self.print_esc(760i32);
                    } else {
                        self.print_esc(761i32);
                    }
                }
            }
            last_item => {
                match chr_code {
                    int_val => {
                        self.print_esc(762i32);
                    }
                    dimen_val => {
                        self.print_esc(763i32);
                    }
                    glue_val => {
                        self.print_esc(764i32);
                    }
                    input_line_no_code => {
                        self.print_esc(765i32);
                    }
                    last_node_type_code => {
                        // §1650
                        self.print_esc(1963i32);
                    }
                    eTeX_version_code => {
                        self.print_esc(1964i32);
                    }
                    current_group_level_code => {
                        // §1664
                        self.print_esc(1990i32);
                    }
                    current_group_type_code => {
                        self.print_esc(1991i32);
                    }
                    current_if_level_code => {
                        // §1667
                        self.print_esc(1992i32);
                    }
                    current_if_type_code => {
                        self.print_esc(1993i32);
                    }
                    current_if_branch_code => {
                        self.print_esc(1994i32);
                    }
                    font_char_wd_code => {
                        // §1670
                        self.print_esc(1995i32);
                    }
                    font_char_ht_code => {
                        self.print_esc(1996i32);
                    }
                    font_char_dp_code => {
                        self.print_esc(1997i32);
                    }
                    font_char_ic_code => {
                        self.print_esc(1998i32);
                    }
                    par_shape_length_code => {
                        // §1673
                        self.print_esc(1999i32);
                    }
                    par_shape_indent_code => {
                        self.print_esc(2000i32);
                    }
                    par_shape_dimen_code => {
                        self.print_esc(2001i32);
                    }
                    39 => {
                        // §1779
                        self.print_esc(2042i32);
                    }
                    40 => {
                        self.print_esc(2043i32);
                    }
                    41 => {
                        self.print_esc(2044i32);
                    }
                    42 => {
                        self.print_esc(2045i32);
                    }
                    glue_stretch_order_code => {
                        // §1802
                        self.print_esc(2049i32);
                    }
                    glue_shrink_order_code => {
                        self.print_esc(2050i32);
                    }
                    glue_stretch_code => {
                        self.print_esc(2051i32);
                    }
                    glue_shrink_code => {
                        self.print_esc(2052i32);
                    }
                    mu_to_glue_code => {
                        // §1806
                        self.print_esc(2053i32);
                    }
                    glue_to_mu_code => {
                        self.print_esc(2054i32);
                    }
                    pdftex_version_code => {
                        // §443
                        self.print_esc(767i32);
                    }
                    pdf_last_obj_code => {
                        self.print_esc(768i32);
                    }
                    pdf_last_xform_code => {
                        self.print_esc(769i32);
                    }
                    pdf_last_ximage_code => {
                        self.print_esc(770i32);
                    }
                    pdf_last_ximage_pages_code => {
                        self.print_esc(771i32);
                    }
                    pdf_last_annot_code => {
                        self.print_esc(772i32);
                    }
                    pdf_last_x_pos_code => {
                        self.print_esc(773i32);
                    }
                    pdf_last_y_pos_code => {
                        self.print_esc(774i32);
                    }
                    pdf_retval_code => {
                        self.print_esc(775i32);
                    }
                    pdf_last_ximage_colordepth_code => {
                        self.print_esc(776i32);
                    }
                    elapsed_time_code => {
                        self.print_esc(777i32);
                    }
                    pdf_shell_escape_code => {
                        self.print_esc(778i32);
                    }
                    random_seed_code => {
                        self.print_esc(779i32);
                    }
                    pdf_last_link_code => {
                        self.print_esc(780i32);
                    }
                    _ => {
                        self.print_esc(766i32);
                    }
                }
            }
            convert => {
                // §495
                match chr_code {
                    number_code => {
                        self.print_esc(839i32);
                    }
                    roman_numeral_code => {
                        self.print_esc(840i32);
                    }
                    string_code => {
                        self.print_esc(841i32);
                    }
                    meaning_code => {
                        self.print_esc(842i32);
                    }
                    font_name_code => {
                        self.print_esc(843i32);
                    }
                    eTeX_revision_code => {
                        self.print_esc(872i32);
                    }
                    expanded_code => {
                        self.print_esc(844i32);
                    }
                    pdftex_revision_code => {
                        self.print_esc(845i32);
                    }
                    pdftex_banner_code => {
                        self.print_esc(846i32);
                    }
                    pdf_font_name_code => {
                        self.print_esc(847i32);
                    }
                    pdf_font_objnum_code => {
                        self.print_esc(848i32);
                    }
                    pdf_font_size_code => {
                        self.print_esc(849i32);
                    }
                    pdf_page_ref_code => {
                        self.print_esc(850i32);
                    }
                    left_margin_kern_code => {
                        self.print_esc(851i32);
                    }
                    right_margin_kern_code => {
                        self.print_esc(852i32);
                    }
                    pdf_xform_name_code => {
                        self.print_esc(853i32);
                    }
                    pdf_escape_string_code => {
                        self.print_esc(854i32);
                    }
                    pdf_escape_name_code => {
                        self.print_esc(855i32);
                    }
                    pdf_escape_hex_code => {
                        self.print_esc(856i32);
                    }
                    pdf_unescape_hex_code => {
                        self.print_esc(857i32);
                    }
                    pdf_creation_date_code => {
                        self.print_esc(858i32);
                    }
                    pdf_file_mod_date_code => {
                        self.print_esc(859i32);
                    }
                    pdf_file_size_code => {
                        self.print_esc(860i32);
                    }
                    pdf_mdfive_sum_code => {
                        self.print_esc(861i32);
                    }
                    pdf_file_dump_code => {
                        self.print_esc(862i32);
                    }
                    pdf_match_code => {
                        self.print_esc(863i32);
                    }
                    pdf_last_match_code => {
                        self.print_esc(864i32);
                    }
                    pdf_strcmp_code => {
                        self.print_esc(865i32);
                    }
                    pdf_colorstack_init_code => {
                        self.print_esc(866i32);
                    }
                    uniform_deviate_code => {
                        self.print_esc(867i32);
                    }
                    normal_deviate_code => {
                        self.print_esc(868i32);
                    }
                    pdf_insert_ht_code => {
                        self.print_esc(870i32);
                    }
                    pdf_ximage_bbox_code => {
                        self.print_esc(871i32);
                    }
                    _ => {
                        self.print_esc(869i32);
                    }
                }
            }
            if_test => {
                // §514
                {
                    if (chr_code >= unless_code) {
                        self.print_esc(929i32);
                    }
                    match (chr_code % unless_code) {
                        if_cat_code => {
                            self.print_esc(912i32);
                        }
                        if_int_code => {
                            self.print_esc(913i32);
                        }
                        if_dim_code => {
                            self.print_esc(914i32);
                        }
                        if_odd_code => {
                            self.print_esc(915i32);
                        }
                        if_vmode_code => {
                            self.print_esc(916i32);
                        }
                        if_hmode_code => {
                            self.print_esc(917i32);
                        }
                        if_mmode_code => {
                            self.print_esc(918i32);
                        }
                        if_inner_code => {
                            self.print_esc(919i32);
                        }
                        if_void_code => {
                            self.print_esc(920i32);
                        }
                        if_hbox_code => {
                            self.print_esc(921i32);
                        }
                        if_vbox_code => {
                            self.print_esc(922i32);
                        }
                        ifx_code => {
                            self.print_esc(923i32);
                        }
                        if_eof_code => {
                            self.print_esc(924i32);
                        }
                        if_true_code => {
                            self.print_esc(925i32);
                        }
                        if_false_code => {
                            self.print_esc(926i32);
                        }
                        if_case_code => {
                            self.print_esc(927i32);
                        }
                        if_pdfprimitive_code => {
                            self.print_esc(928i32);
                        }
                        if_def_code => {
                            // §1764
                            self.print_esc(2031i32);
                        }
                        if_cs_code => {
                            self.print_esc(2032i32);
                        }
                        if_font_char_code => {
                            self.print_esc(2033i32);
                        }
                        if_in_csname_code => {
                            self.print_esc(2034i32);
                        }
                        if_pdfabs_num_code => {
                            self.print_esc(2035i32);
                        }
                        if_pdfabs_dim_code => {
                            self.print_esc(2036i32);
                        }
                        _ => {
                            // §514
                            self.print_esc(911i32);
                        }
                    }
                }
            }
            fi_or_else => {
                // §518
                if (chr_code == fi_code) {
                    self.print_esc(930i32);
                } else {
                    if (chr_code == or_code) {
                        self.print_esc(931i32);
                    } else {
                        self.print_esc(932i32);
                    }
                }
            }
            tab_mark => {
                // §957
                if (chr_code == span_code) {
                    self.print_esc(1307i32);
                } else {
                    {
                        self.print(1311i32);
                        self.print(chr_code);
                    }
                }
            }
            car_ret => {
                if (chr_code == cr_code) {
                    self.print_esc(1308i32);
                } else {
                    self.print_esc(1309i32);
                }
            }
            set_page_dimen => {
                // §1161
                match chr_code {
                    0 => {
                        self.print_esc(1385i32);
                    }
                    1 => {
                        self.print_esc(1386i32);
                    }
                    2 => {
                        self.print_esc(1387i32);
                    }
                    3 => {
                        self.print_esc(1388i32);
                    }
                    4 => {
                        self.print_esc(1389i32);
                    }
                    5 => {
                        self.print_esc(1390i32);
                    }
                    6 => {
                        self.print_esc(1391i32);
                    }
                    _ => {
                        self.print_esc(1392i32);
                    }
                }
            }
            stop => {
                // §1231
                if (chr_code == 1i32) {
                    self.print_esc(1438i32);
                } else {
                    self.print_esc(353i32);
                }
            }
            hskip => {
                // §1237
                match chr_code {
                    skip_code => {
                        self.print_esc(1439i32);
                    }
                    fil_code => {
                        self.print_esc(1440i32);
                    }
                    fill_code => {
                        self.print_esc(1441i32);
                    }
                    ss_code => {
                        self.print_esc(1442i32);
                    }
                    _ => {
                        self.print_esc(1443i32);
                    }
                }
            }
            vskip => {
                match chr_code {
                    skip_code => {
                        self.print_esc(1444i32);
                    }
                    fil_code => {
                        self.print_esc(1445i32);
                    }
                    fill_code => {
                        self.print_esc(1446i32);
                    }
                    ss_code => {
                        self.print_esc(1447i32);
                    }
                    _ => {
                        self.print_esc(1448i32);
                    }
                }
            }
            mskip => {
                self.print_esc(346i32);
            }
            kern => {
                self.print_esc(324i32);
            }
            mkern => {
                self.print_esc(352i32);
            }
            hmove => {
                // §1250
                if (chr_code == 1i32) {
                    self.print_esc(1466i32);
                } else {
                    self.print_esc(1467i32);
                }
            }
            vmove => {
                if (chr_code == 1i32) {
                    self.print_esc(1468i32);
                } else {
                    self.print_esc(1469i32);
                }
            }
            make_box => {
                match chr_code {
                    box_code => {
                        self.print_esc(425i32);
                    }
                    copy_code => {
                        self.print_esc(1470i32);
                    }
                    last_box_code => {
                        self.print_esc(1471i32);
                    }
                    vsplit_code => {
                        self.print_esc(1380i32);
                    }
                    vtop_code => {
                        self.print_esc(1472i32);
                    }
                    5 => {
                        self.print_esc(1382i32);
                    }
                    _ => {
                        self.print_esc(1473i32);
                    }
                }
            }
            leader_ship => {
                if (chr_code == a_leaders) {
                    self.print_esc(1475i32);
                } else {
                    if (chr_code == c_leaders) {
                        self.print_esc(1476i32);
                    } else {
                        if (chr_code == x_leaders) {
                            self.print_esc(1477i32);
                        } else {
                            self.print_esc(1474i32);
                        }
                    }
                }
            }
            start_par => {
                // §1267
                if (chr_code == 0i32) {
                    self.print_esc(1494i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(1493i32);
                    } else {
                        self.print_esc(1495i32);
                    }
                }
            }
            remove_item => {
                // §1286
                if (chr_code == glue_node) {
                    self.print_esc(1507i32);
                } else {
                    if (chr_code == kern_node) {
                        self.print_esc(1506i32);
                    } else {
                        self.print_esc(1505i32);
                    }
                }
            }
            un_hbox => {
                if (chr_code == copy_code) {
                    self.print_esc(1509i32);
                } else {
                    self.print_esc(1508i32);
                }
            }
            un_vbox => {
                if (chr_code == copy_code) {
                    self.print_esc(1511i32);
                } else {
                    // §1862
                    if (chr_code == last_box_code) {
                        self.print_esc(2064i32);
                    } else {
                        if (chr_code == vsplit_code) {
                            self.print_esc(2065i32);
                        } else {
                            // §1286
                            self.print_esc(1510i32);
                        }
                    }
                }
            }
            discretionary => {
                // §1293
                if (chr_code == 1i32) {
                    self.print_esc(45i32);
                } else {
                    self.print_esc(361i32);
                }
            }
            eq_no => {
                // §1321
                if (chr_code == 1i32) {
                    self.print_esc(1543i32);
                } else {
                    self.print_esc(1542i32);
                }
            }
            math_comp => {
                // §1335
                match chr_code {
                    ord_noad => {
                        self.print_esc(1274i32);
                    }
                    op_noad => {
                        self.print_esc(1275i32);
                    }
                    bin_noad => {
                        self.print_esc(1276i32);
                    }
                    rel_noad => {
                        self.print_esc(1277i32);
                    }
                    open_noad => {
                        self.print_esc(1278i32);
                    }
                    close_noad => {
                        self.print_esc(1279i32);
                    }
                    punct_noad => {
                        self.print_esc(1280i32);
                    }
                    inner_noad => {
                        self.print_esc(1281i32);
                    }
                    under_noad => {
                        self.print_esc(1283i32);
                    }
                    _ => {
                        self.print_esc(1282i32);
                    }
                }
            }
            limit_switch => {
                if (chr_code == limits) {
                    self.print_esc(1287i32);
                } else {
                    if (chr_code == no_limits) {
                        self.print_esc(1288i32);
                    } else {
                        self.print_esc(1544i32);
                    }
                }
            }
            math_style => {
                // §1348
                self.print_style(chr_code);
            }
            above => {
                // §1357
                match chr_code {
                    over_code => {
                        self.print_esc(1563i32);
                    }
                    atop_code => {
                        self.print_esc(1564i32);
                    }
                    3 => {
                        self.print_esc(1565i32);
                    }
                    4 => {
                        self.print_esc(1566i32);
                    }
                    5 => {
                        self.print_esc(1567i32);
                    }
                    _ => {
                        self.print_esc(1562i32);
                    }
                }
            }
            left_right => {
                // §1367
                if (chr_code == left_noad) {
                    self.print_esc(1284i32);
                } else {
                    // §1698
                    if (chr_code == middle_noad) {
                        self.print_esc(1286i32);
                    } else {
                        // §1367
                        self.print_esc(1285i32);
                    }
                }
            }
            prefix => {
                // §1387
                if (chr_code == 1i32) {
                    self.print_esc(1587i32);
                } else {
                    if (chr_code == 2i32) {
                        self.print_esc(1588i32);
                    } else {
                        // §1771
                        if (chr_code == 8i32) {
                            self.print_esc(1601i32);
                        } else {
                            // §1387
                            self.print_esc(1589i32);
                        }
                    }
                }
            }
            def => {
                if (chr_code == 0i32) {
                    self.print_esc(1590i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(1591i32);
                    } else {
                        if (chr_code == 2i32) {
                            self.print_esc(1592i32);
                        } else {
                            self.print_esc(1593i32);
                        }
                    }
                }
            }
            let_ => {
                // §1398
                if (chr_code != normal) {
                    self.print_esc(1611i32);
                } else {
                    self.print_esc(1610i32);
                }
            }
            shorthand_def => {
                // §1401
                match chr_code {
                    char_def_code => {
                        self.print_esc(1612i32);
                    }
                    math_char_def_code => {
                        self.print_esc(1613i32);
                    }
                    count_def_code => {
                        self.print_esc(1614i32);
                    }
                    dimen_def_code => {
                        self.print_esc(1615i32);
                    }
                    skip_def_code => {
                        self.print_esc(1616i32);
                    }
                    mu_skip_def_code => {
                        self.print_esc(1617i32);
                    }
                    _ => {
                        self.print_esc(1618i32);
                    }
                }
            }
            char_given => {
                {
                    self.print_esc(589i32);
                    self.print_hex(chr_code);
                }
            }
            math_given => {
                {
                    self.print_esc(602i32);
                    self.print_hex(chr_code);
                }
            }
            def_code => {
                // §1409
                if (chr_code == cat_code_base) {
                    self.print_esc(431i32);
                } else {
                    if (chr_code == math_code_base) {
                        self.print_esc(435i32);
                    } else {
                        if (chr_code == lc_code_base) {
                            self.print_esc(432i32);
                        } else {
                            if (chr_code == uc_code_base) {
                                self.print_esc(433i32);
                            } else {
                                if (chr_code == sf_code_base) {
                                    self.print_esc(434i32);
                                } else {
                                    self.print_esc(538i32);
                                }
                            }
                        }
                    }
                }
            }
            def_family => {
                self.print_size((chr_code).wrapping_sub(627690i32));
            }
            hyph_data => {
                // §1429
                if (chr_code == 1i32) {
                    self.print_esc(1366i32);
                } else {
                    self.print_esc(1354i32);
                }
            }
            assign_font_int => {
                // §1433
                match chr_code {
                    0 => {
                        self.print_esc(1636i32);
                    }
                    1 => {
                        self.print_esc(1637i32);
                    }
                    lp_code_base => {
                        self.print_esc(1638i32);
                    }
                    rp_code_base => {
                        self.print_esc(1639i32);
                    }
                    ef_code_base => {
                        self.print_esc(1640i32);
                    }
                    tag_code => {
                        self.print_esc(1641i32);
                    }
                    kn_bs_code_base => {
                        self.print_esc(1642i32);
                    }
                    st_bs_code_base => {
                        self.print_esc(1643i32);
                    }
                    sh_bs_code_base => {
                        self.print_esc(1644i32);
                    }
                    kn_bc_code_base => {
                        self.print_esc(1645i32);
                    }
                    kn_ac_code_base => {
                        self.print_esc(1646i32);
                    }
                    no_lig_code => {
                        self.print_esc(1647i32);
                    }
                    _ => {}
                }
            }
            set_font => {
                // §1439
                {
                    self.print(1654i32);
                    self.slow_print(self.font_name[crate::ix::U((chr_code) as usize)]);
                    if (self.font_size[crate::ix::U((chr_code) as usize)] != self.font_dsize[crate::ix::U((chr_code) as usize)]) {
                        {
                            self.print(895i32);
                            self.print_scaled(self.font_size[crate::ix::U((chr_code) as usize)]);
                            self.print(314i32);
                        }
                    }
                }
            }
            set_interaction => {
                // §1441
                match chr_code {
                    batch_mode => {
                        self.print_esc(276i32);
                    }
                    nonstop_mode => {
                        self.print_esc(277i32);
                    }
                    scroll_mode => {
                        self.print_esc(278i32);
                    }
                    _ => {
                        self.print_esc(1655i32);
                    }
                }
            }
            in_stream => {
                // §1451
                if (chr_code == 0i32) {
                    self.print_esc(1657i32);
                } else {
                    self.print_esc(1656i32);
                }
            }
            message => {
                // §1456
                if (chr_code == 0i32) {
                    self.print_esc(1658i32);
                } else {
                    self.print_esc(1659i32);
                }
            }
            case_shift => {
                // §1465
                if (chr_code == lc_code_base) {
                    self.print_esc(1665i32);
                } else {
                    self.print_esc(1666i32);
                }
            }
            xray => {
                // §1470
                match chr_code {
                    show_box_code => {
                        self.print_esc(1668i32);
                    }
                    show_the_code => {
                        self.print_esc(1669i32);
                    }
                    show_lists_code => {
                        self.print_esc(1670i32);
                    }
                    show_groups => {
                        // §1676
                        self.print_esc(2002i32);
                    }
                    show_tokens => {
                        // §1685
                        self.print_esc(2004i32);
                    }
                    show_ifs => {
                        // §1690
                        self.print_esc(2007i32);
                    }
                    _ => {
                        // §1470
                        self.print_esc(1667i32);
                    }
                }
            }
            undefined_cs => {
                // §1473
                self.print(1677i32);
            }
            call | long_call | outer_call | long_outer_call => {
                {
                    n = (cmd).wrapping_sub(114i32);
                    if (self.mem[crate::ix::U((self.mem[crate::ix::U((chr_code) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
                        n = (n).wrapping_add(4i32);
                    }
                    if ((((n / 4i32)) % 2) != 0) {
                        self.print_esc(1601i32);
                    }
                    if (((n) % 2) != 0) {
                        self.print_esc(1587i32);
                    }
                    if ((((n / 2i32)) % 2) != 0) {
                        self.print_esc(1588i32);
                    }
                    if (n > 0i32) {
                        self.print_char(32i32);
                    }
                    self.print(1678i32);
                }
            }
            end_template => {
                self.print_esc(1679i32);
            }
            extension => {
                // §1526
                match chr_code {
                    open_node => {
                        self.print_esc(1714i32);
                    }
                    write_node => {
                        self.print_esc(678i32);
                    }
                    close_node => {
                        self.print_esc(1715i32);
                    }
                    special_node => {
                        self.print_esc(1716i32);
                    }
                    immediate_code => {
                        self.print_esc(1717i32);
                    }
                    set_language_code => {
                        self.print_esc(1718i32);
                    }
                    pdf_annot_node => {
                        self.print_esc(1729i32);
                    }
                    pdf_catalog_code => {
                        self.print_esc(1742i32);
                    }
                    pdf_dest_node => {
                        self.print_esc(1733i32);
                    }
                    pdf_end_link_node => {
                        self.print_esc(1731i32);
                    }
                    pdf_end_thread_node => {
                        self.print_esc(1736i32);
                    }
                    pdf_font_attr_code => {
                        self.print_esc(1745i32);
                    }
                    pdf_font_expand_code => {
                        self.print_esc(1752i32);
                    }
                    pdf_include_chars_code => {
                        self.print_esc(1744i32);
                    }
                    pdf_info_code => {
                        self.print_esc(1741i32);
                    }
                    pdf_literal_node => {
                        self.print_esc(1719i32);
                    }
                    pdf_colorstack_node => {
                        self.print_esc(1131i32);
                    }
                    pdf_setmatrix_node => {
                        self.print_esc(1720i32);
                    }
                    pdf_save_node => {
                        self.print_esc(1721i32);
                    }
                    pdf_restore_node => {
                        self.print_esc(1722i32);
                    }
                    pdf_map_file_code => {
                        self.print_esc(1746i32);
                    }
                    pdf_map_line_code => {
                        self.print_esc(1747i32);
                    }
                    pdf_names_code => {
                        self.print_esc(1743i32);
                    }
                    pdf_obj_code => {
                        self.print_esc(1723i32);
                    }
                    pdf_outline_code => {
                        self.print_esc(1732i32);
                    }
                    pdf_refobj_node => {
                        self.print_esc(1724i32);
                    }
                    pdf_refxform_node => {
                        self.print_esc(1726i32);
                    }
                    pdf_refximage_node => {
                        self.print_esc(1728i32);
                    }
                    pdf_save_pos_node => {
                        self.print_esc(1737i32);
                    }
                    pdf_snap_ref_point_node => {
                        self.print_esc(1738i32);
                    }
                    pdf_snapy_comp_node => {
                        self.print_esc(1740i32);
                    }
                    pdf_snapy_node => {
                        self.print_esc(1739i32);
                    }
                    pdf_start_link_node => {
                        self.print_esc(1730i32);
                    }
                    pdf_start_thread_node => {
                        self.print_esc(1735i32);
                    }
                    pdf_thread_node => {
                        self.print_esc(1734i32);
                    }
                    pdf_trailer_code => {
                        self.print_esc(1748i32);
                    }
                    pdf_trailer_id_code => {
                        self.print_esc(1749i32);
                    }
                    pdf_xform_code => {
                        self.print_esc(1725i32);
                    }
                    pdf_ximage_code => {
                        self.print_esc(1727i32);
                    }
                    reset_timer_code => {
                        self.print_esc(1750i32);
                    }
                    set_random_seed_code => {
                        self.print_esc(1751i32);
                    }
                    pdf_nobuiltin_tounicode_code => {
                        self.print_esc(1754i32);
                    }
                    pdf_glyph_to_unicode_code => {
                        self.print_esc(1753i32);
                    }
                    pdf_interword_space_on_node => {
                        self.print_esc(1755i32);
                    }
                    pdf_interword_space_off_node => {
                        self.print_esc(1756i32);
                    }
                    pdf_fake_space_node => {
                        self.print_esc(1757i32);
                    }
                    pdf_running_link_off_node => {
                        self.print_esc(1758i32);
                    }
                    pdf_running_link_on_node => {
                        self.print_esc(1759i32);
                    }
                    pdf_space_font_code => {
                        self.print_esc(1760i32);
                    }
                    _ => {
                        self.print(1761i32);
                    }
                }
            }
            _ => {
                // §320
                self.print(649i32);
            }
        }
    }

    /// Here is a procedure that displays the contents of `eqtb[n]`
    /// symbolically.
    // §270
    pub fn show_eqtb(&mut self, mut n: halfword) {
        if (n < active_base) {
            self.print_char(63i32);
        } else {
            if (n < glue_base) {
                // §241
                {
                    self.sprint_cs(n);
                    self.print_char(61i32);
                    self.print_cmd_chr(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().b0(), self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh());
                    if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().b0() >= call) {
                        {
                            self.print_char(58i32);
                            self.show_token_list(self.mem[crate::ix::U((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as usize)].hh().rh(), null, 32i32);
                        }
                    }
                }
            } else {
                // §270
                if (n < local_base) {
                    // §247
                    if (n < skip_base) {
                        {
                            self.print_skip_param((n).wrapping_sub(626628i32));
                            self.print_char(61i32);
                            if (n < 626643i32) {
                                self.print_spec(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(), 314i32);
                            } else {
                                self.print_spec(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(), 347i32);
                            }
                        }
                    } else {
                        if (n < mu_skip_base) {
                            {
                                self.print_esc(408i32);
                                self.print_int((((n).wrapping_sub(626646i32)) as i64));
                                self.print_char(61i32);
                                self.print_spec(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(), 314i32);
                            }
                        } else {
                            {
                                self.print_esc(409i32);
                                self.print_int((((n).wrapping_sub(626902i32)) as i64));
                                self.print_char(61i32);
                                self.print_spec(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(), 347i32);
                            }
                        }
                    }
                } else {
                    // §270
                    if (n < int_base) {
                        // §251
                        if ((n == par_shape_loc) || ((n >= etex_pen_base) && (n < etex_pens))) {
                            {
                                self.print_cmd_chr(set_shape, n);
                                self.print_char(61i32);
                                if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh() == null) {
                                    self.print_char(48i32);
                                } else {
                                    if (n > par_shape_loc) {
                                        {
                                            self.print_int(((self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()) as i64));
                                            self.print_char(32i32);
                                            self.print_int(((self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()).wrapping_add(2i32)) as usize)].int()) as i64));
                                            if (self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int() > 1i32) {
                                                self.print_esc(424i32);
                                            }
                                        }
                                    } else {
                                        self.print_int(((self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh()) as i64));
                                    }
                                }
                            }
                        } else {
                            if (n < toks_base) {
                                {
                                    self.print_cmd_chr(assign_toks, n);
                                    self.print_char(61i32);
                                    if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh() != null) {
                                        self.show_token_list(self.mem[crate::ix::U((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as usize)].hh().rh(), null, 32i32);
                                    }
                                }
                            } else {
                                if (n < box_base) {
                                    {
                                        self.print_esc(423i32);
                                        self.print_int((((n).wrapping_sub(627173i32)) as i64));
                                        self.print_char(61i32);
                                        if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh() != null) {
                                            self.show_token_list(self.mem[crate::ix::U((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as usize)].hh().rh(), null, 32i32);
                                        }
                                    }
                                } else {
                                    if (n < cur_font_loc) {
                                        {
                                            self.print_esc(425i32);
                                            self.print_int((((n).wrapping_sub(627433i32)) as i64));
                                            self.print_char(61i32);
                                            if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh() == null) {
                                                self.print(426i32);
                                            } else {
                                                {
                                                    self.depth_threshold = 0i32;
                                                    self.breadth_max = 1i32;
                                                    self.show_node_list(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh());
                                                }
                                            }
                                        }
                                    } else {
                                        if (n < cat_code_base) {
                                            // §252
                                            {
                                                if (n == cur_font_loc) {
                                                    self.print(427i32);
                                                } else {
                                                    if (n < 627706i32) {
                                                        {
                                                            self.print_esc(428i32);
                                                            self.print_int((((n).wrapping_sub(627690i32)) as i64));
                                                        }
                                                    } else {
                                                        if (n < 627722i32) {
                                                            {
                                                                self.print_esc(429i32);
                                                                self.print_int((((n).wrapping_sub(627706i32)) as i64));
                                                            }
                                                        } else {
                                                            {
                                                                self.print_esc(430i32);
                                                                self.print_int((((n).wrapping_sub(627722i32)) as i64));
                                                            }
                                                        }
                                                    }
                                                }
                                                self.print_char(61i32);
                                                self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh())) - 514) as usize)].rh());
                                            }
                                        } else {
                                            // §253
                                            if (n < math_code_base) {
                                                {
                                                    if (n < lc_code_base) {
                                                        {
                                                            self.print_esc(431i32);
                                                            self.print_int((((n).wrapping_sub(627738i32)) as i64));
                                                        }
                                                    } else {
                                                        if (n < uc_code_base) {
                                                            {
                                                                self.print_esc(432i32);
                                                                self.print_int((((n).wrapping_sub(627994i32)) as i64));
                                                            }
                                                        } else {
                                                            if (n < sf_code_base) {
                                                                {
                                                                    self.print_esc(433i32);
                                                                    self.print_int((((n).wrapping_sub(628250i32)) as i64));
                                                                }
                                                            } else {
                                                                {
                                                                    self.print_esc(434i32);
                                                                    self.print_int((((n).wrapping_sub(628506i32)) as i64));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.print_char(61i32);
                                                    self.print_int(((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as i64));
                                                }
                                            } else {
                                                {
                                                    self.print_esc(435i32);
                                                    self.print_int((((n).wrapping_sub(628762i32)) as i64));
                                                    self.print_char(61i32);
                                                    self.print_int((((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()).wrapping_sub(0i32)) as i64));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // §270
                        if (n < dimen_base) {
                            // §260
                            {
                                if (n < count_base) {
                                    self.print_param((n).wrapping_sub(629018i32));
                                } else {
                                    if (n < del_code_base) {
                                        {
                                            self.print_esc(537i32);
                                            self.print_int((((n).wrapping_sub(629128i32)) as i64));
                                        }
                                    } else {
                                        {
                                            self.print_esc(538i32);
                                            self.print_int((((n).wrapping_sub(629384i32)) as i64));
                                        }
                                    }
                                }
                                self.print_char(61i32);
                                self.print_int(((self.eqtb[crate::ix::U(((n) - 1) as usize)].int()) as i64));
                            }
                        } else {
                            // §270
                            if (n <= eqtb_size) {
                                // §269
                                {
                                    if (n < scaled_base) {
                                        self.print_length_param((n).wrapping_sub(629640i32));
                                    } else {
                                        {
                                            self.print_esc(574i32);
                                            self.print_int((((n).wrapping_sub(629674i32)) as i64));
                                        }
                                    }
                                    self.print_char(61i32);
                                    self.print_scaled(self.eqtb[crate::ix::U(((n) - 1) as usize)].int());
                                    self.print(314i32);
                                }
                            } else {
                                // §270
                                self.print_char(63i32);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Here is the subroutine that searches the hash table for an identifier
    /// that matches a given string of length `l>1` appearing in `buffer[j..
    /// (j+l-1)]`. If the identifier is found, the corresponding hash table address
    /// is returned. Otherwise, if the global variable `no_new_control_sequence`
    /// is `true`, the dummy address `undefined_control_sequence` is returned.
    /// Otherwise the identifier is inserted into the hash table and its location
    /// is returned.
    // §278
    pub fn id_lookup(&mut self, mut j: i32, mut l: i32) -> halfword {
        let mut id_lookup: halfword = 0;
        let mut h: i32 = 0; // §278
        let mut d: i32 = 0; // §278
        let mut p: halfword = 0; // §278
        let mut k: halfword = 0; // §278
        'l_found_f: {
            // §280
            h = self.buffer[crate::ix::U((j) as usize)];
            {
                let __for_end_3 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                k = (j).wrapping_add(1i32);
                while k <= __for_end_3 {
                    {
                        h = ((h).wrapping_add(h)).wrapping_add(self.buffer[crate::ix::U((k) as usize)]);
                        while (h >= hash_prime) {
                            h = (h).wrapping_sub(522749i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            // §278
            p = (h).wrapping_add(514i32);
            while true {
                {
                    if (self.hash[crate::ix::U(((p) - 514) as usize)].rh() > 0i32) {
                        if ((self.str_start[crate::ix::U(((self.hash[crate::ix::U(((p) - 514) as usize)].rh()).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.hash[crate::ix::U(((p) - 514) as usize)].rh()) as usize)]) == l) {
                            if self.str_eq_buf(self.hash[crate::ix::U(((p) - 514) as usize)].rh(), j) {
                                break 'l_found_f;
                            }
                        }
                    }
                    if (self.hash[crate::ix::U(((p) - 514) as usize)].lh() == 0i32) {
                        {
                            if self.no_new_control_sequence {
                                p = undefined_control_sequence;
                            } else {
                                // §279
                                {
                                    if (self.hash[crate::ix::U(((p) - 514) as usize)].rh() > 0i32) {
                                        {
                                            loop {
                                                if (self.hash_used == hash_base) {
                                                    self.overflow(578i32, hash_size);
                                                }
                                                self.hash_used = (self.hash_used).wrapping_sub(1i32);
                                                if (self.hash[crate::ix::U(((self.hash_used) - 514) as usize)].rh() == 0i32) { break; }
                                            }
                                            { let __v143 = self.hash_used; self.hash[crate::ix::U(((p) - 514) as usize)].set_lh(__v143); }
                                            p = self.hash_used;
                                        }
                                    }
                                    {
                                        if ((self.pool_ptr).wrapping_add(l) > pool_size) {
                                            self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                        }
                                    }
                                    d = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]);
                                    while (self.pool_ptr > self.str_start[crate::ix::U((self.str_ptr) as usize)]) {
                                        {
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                            { let __ix144 = (self.pool_ptr).wrapping_add(l); let __v145 = self.str_pool[crate::ix::U((self.pool_ptr) as usize)]; self.str_pool[crate::ix::U((__ix144) as usize)] = __v145; }
                                        }
                                    }
                                    {
                                        let __for_end_9 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                                        k = j;
                                        while k <= __for_end_9 {
                                            {
                                                { let __ix146 = self.pool_ptr; let __v147 = self.buffer[crate::ix::U((k) as usize)]; self.str_pool[crate::ix::U((__ix146) as usize)] = __v147; }
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            k = k.wrapping_add(1);
                                        }
                                    }
                                    { let __v148 = self.make_string(); self.hash[crate::ix::U(((p) - 514) as usize)].set_rh(__v148); }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(d);
                                    self.cs_count = (self.cs_count).wrapping_add(1i32);
                                    if self.intr_on {
                                        self.flashtex_intr_new_cs(p);
                                    }
                                }
                            }
                            // §278
                            break 'l_found_f;
                        }
                    }
                    p = self.hash[crate::ix::U(((p) - 514) as usize)].lh();
                }
            }
        }
        if self.rs_on {
            self.flashtex_id_read(j, l, p);
        }
        id_lookup = p;
        id_lookup
    }

    /// Here is the subroutine that searches the primitive table for an identifier:
    // §281
    pub fn prim_lookup(&mut self, mut s: str_number) -> halfword {
        let mut prim_lookup: halfword = 0;
        let mut h: i32 = 0; // §281
        let mut p: halfword = 0; // §281
        let mut k: halfword = 0; // §281
        let mut j: i32 = 0; // §281
        let mut l: i32 = 0; // §281
        'l_found_f: {
            if (s <= biggest_char) {
                {
                    if (s < 0i32) {
                        {
                            p = undefined_primitive;
                            break 'l_found_f;
                        }
                    } else {
                        p = ((s % prim_prime)).wrapping_add(1i32);
                    }
                }
            } else {
                {
                    j = self.str_start[crate::ix::U((s) as usize)];
                    if (s == self.str_ptr) {
                        l = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]);
                    } else {
                        l = (self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((s) as usize)]);
                    }
                    // §283
                    h = self.str_pool[crate::ix::U((j) as usize)];
                    {
                        let __for_end_5 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                        k = (j).wrapping_add(1i32);
                        while k <= __for_end_5 {
                            {
                                h = ((h).wrapping_add(h)).wrapping_add(self.str_pool[crate::ix::U((k) as usize)]);
                                while (h >= prim_prime) {
                                    h = (h).wrapping_sub(1777i32);
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    // §281
                    p = (h).wrapping_add(1i32);
                }
            }
            while true {
                {
                    if (self.prim[crate::ix::U((p) as usize)].rh() > 256i32) {
                        {
                            if ((self.str_start[crate::ix::U(((self.prim[crate::ix::U((p) as usize)].rh()).wrapping_add(0i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U(((self.prim[crate::ix::U((p) as usize)].rh()).wrapping_sub(1i32)) as usize)]) == l) {
                                if self.str_eq_str((self.prim[crate::ix::U((p) as usize)].rh()).wrapping_sub(1i32), s) {
                                    break 'l_found_f;
                                }
                            }
                        }
                    } else {
                        if (self.prim[crate::ix::U((p) as usize)].rh() == (1i32).wrapping_add(s)) {
                            break 'l_found_f;
                        }
                    }
                    if (self.prim[crate::ix::U((p) as usize)].lh() == 0i32) {
                        {
                            if self.no_new_control_sequence {
                                p = undefined_primitive;
                            } else {
                                // §282
                                {
                                    if (self.prim[crate::ix::U((p) as usize)].rh() > 0i32) {
                                        {
                                            loop {
                                                if (self.prim_used == prim_base) {
                                                    self.overflow(579i32, prim_size);
                                                }
                                                self.prim_used = (self.prim_used).wrapping_sub(1i32);
                                                if (self.prim[crate::ix::U((self.prim_used) as usize)].rh() == 0i32) { break; }
                                            }
                                            { let __v149 = self.prim_used; self.prim[crate::ix::U((p) as usize)].set_lh(__v149); }
                                            p = self.prim_used;
                                        }
                                    }
                                    self.prim[crate::ix::U((p) as usize)].set_rh((s).wrapping_add(1i32));
                                }
                            }
                            // §281
                            break 'l_found_f;
                        }
                    }
                    p = self.prim[crate::ix::U((p) as usize)].lh();
                }
            }
        }
        prim_lookup = p;
        prim_lookup
    }

    /// We need to put \TeX's ``primitive'' control sequences into the hash
    /// table, together with their command code (which will be the `eq_type`)
    /// and an operand (which will be the `equiv`). The `primitive` procedure
    /// does this, in a way that no \TeX\ user can. The global value `cur_val`
    /// contains the new `eqtb` pointer after `primitive` has acted.
    /// Until pdf\TeX\ 1.40.19 (released in 2018), a bug in primitive handling
    /// caused, e.g., \.{\\pdfprimitive\\ \\q} to swallow the \.{\\q} instead of
    /// giving an undefined control sequence error. The original report was
    /// posted by Hironori Kitagawa
    /// (\.{tug.org/pipermail/tex-k/2017-October/002816.html}). Largely
    /// quoting from that message:
    /// The cause was `cur_tok` not being set in the ``Cases of `main_control`\dots''
    /// module, because `back_input` unscans the token, but only looks at
    /// `cur_tok`, which represents the internalized \.{\\pdfprimitive} at that
    /// ...
    // §286
    pub fn primitive(&mut self, mut s: str_number, mut c: quarterword, mut o: halfword) {
        let mut k: pool_pointer = 0; // §286
        let mut j: i32 = 0; // §286
        let mut l: small_number = 0; // §286
        let mut prim_val: i32 = 0; // §286
        if (s < 256i32) {
            {
                self.cur_val = (s).wrapping_add(257i32);
                prim_val = self.prim_lookup(s);
            }
        } else {
            {
                k = self.str_start[crate::ix::U((s) as usize)];
                l = (self.str_start[crate::ix::U(((s).wrapping_add(1i32)) as usize)]).wrapping_sub(k);
                if ((self.first).wrapping_add(l) > (buf_size).wrapping_add(1i32)) {
                    self.overflow(258i32, buf_size);
                }
                {
                    let __for_end_4 = (l).wrapping_sub(1i32);
                    j = 0i32;
                    while j <= __for_end_4 {
                        { let __ix150 = (self.first).wrapping_add(j); let __v151 = self.str_pool[crate::ix::U(((k).wrapping_add(j)) as usize)]; self.buffer[crate::ix::U((__ix150) as usize)] = __v151; }
                        j = j.wrapping_add(1);
                    }
                }
                self.cur_val = self.id_lookup(self.first, l);
                {
                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                }
                { let __ix152 = self.cur_val; self.hash[crate::ix::U(((__ix152) - 514) as usize)].set_rh(s); }
                prim_val = self.prim_lookup(s);
            }
        }
        { let __ix153 = self.cur_val; self.eqtb[crate::ix::U(((__ix153) - 1) as usize)].set_hh_b1(level_one); }
        { let __ix154 = self.cur_val; self.eqtb[crate::ix::U(((__ix154) - 1) as usize)].set_hh_b0(c); }
        { let __ix155 = self.cur_val; self.eqtb[crate::ix::U(((__ix155) - 1) as usize)].set_hh_rh(o); }
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)].set_hh_b1(level_one);
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)].set_hh_b0(c);
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)].set_hh_rh(o);
    }

    /// @<Declare \eTeX\ procedures for tr...
    // §306
    pub fn restore_trace(&mut self, mut p: halfword, mut s: str_number) {
        self.begin_diagnostic();
        self.print_char(123i32);
        self.print(s);
        self.print_char(32i32);
        self.show_eqtb(p);
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The `print_group` procedure prints the current level of grouping and
    /// the name corresponding to `cur_group`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1661
    pub fn print_group(&mut self, mut e: bool) {
        'l_exit_f: {
            match self.cur_group {
                bottom_level => {
                    {
                        self.print(1977i32);
                        break 'l_exit_f;
                    }
                }
                simple_group | semi_simple_group => {
                    {
                        if (self.cur_group == semi_simple_group) {
                            self.print(1978i32);
                        }
                        self.print(1979i32);
                    }
                }
                hbox_group | adjusted_hbox_group => {
                    {
                        if (self.cur_group == adjusted_hbox_group) {
                            self.print(1980i32);
                        }
                        self.print(1473i32);
                    }
                }
                vbox_group => {
                    self.print(1382i32);
                }
                vtop_group => {
                    self.print(1472i32);
                }
                align_group | no_align_group => {
                    {
                        if (self.cur_group == no_align_group) {
                            self.print(1981i32);
                        }
                        self.print(1982i32);
                    }
                }
                output_group => {
                    self.print(410i32);
                }
                disc_group => {
                    self.print(1983i32);
                }
                insert_group => {
                    self.print(340i32);
                }
                vcenter_group => {
                    self.print(618i32);
                }
                math_group | math_choice_group | math_shift_group | math_left_group => {
                    {
                        self.print(355i32);
                        if (self.cur_group == math_choice_group) {
                            self.print(1984i32);
                        } else {
                            if (self.cur_group == math_shift_group) {
                                self.print(1985i32);
                            } else {
                                if (self.cur_group == math_left_group) {
                                    self.print(1986i32);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            self.print(1987i32);
            self.print_int((((self.cur_level).wrapping_sub(0i32)) as i64));
            self.print_char(41i32);
            if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int() != 0i32) {
                {
                    if e {
                        self.print(377i32);
                    } else {
                        self.print(268i32);
                    }
                    self.print_int(((self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()) as i64));
                }
            }
        }
    }

    /// The `group_trace` procedure is called when a new level of grouping
    /// begins (`e=false`) or ends (`e=true`) with `saved(-1)` containing the
    /// line number.
    /// @<Declare \eTeX\ procedures for tr...
    // §1662
    pub fn group_trace(&mut self, mut e: bool) {
        self.begin_diagnostic();
        self.print_char(123i32);
        if e {
            self.print(1988i32);
        } else {
            self.print(1989i32);
        }
        self.print_group(e);
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

}
