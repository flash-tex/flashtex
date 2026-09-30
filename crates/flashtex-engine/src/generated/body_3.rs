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
    /// @<Declare \eTeX\ procedures for tok...
    // §1753
    pub fn pseudo_start(&mut self) {
        let mut old_setting: i32 = 0; // §1753
        let mut s: str_number = 0; // §1753
        let mut l: pool_pointer = 0; // §1753
        let mut m: pool_pointer = 0; // §1753
        let mut p: halfword = 0; // §1753
        let mut q: halfword = 0; // §1753
        let mut r: halfword = 0; // §1753
        let mut w: four_quarters = four_quarters::default(); // §1753
        let mut nl: i32 = 0; // §1753
        let mut sz: i32 = 0; // §1753
        self.scan_general_text();
        old_setting = self.selector;
        self.selector = new_string;
        self.token_show(temp_head);
        self.selector = old_setting;
        self.flush_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        // §1754
        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
        l = self.str_start[crate::ix::U((s) as usize)];
        nl = self.eqtb[crate::ix::U(((629067i32) - 1) as usize)].int();
        p = self.get_avail();
        q = p;
        while (l < self.pool_ptr) {
            {
                m = l;
                while ((l < self.pool_ptr) && (self.str_pool[crate::ix::U((l) as usize)] != nl)) {
                    l = (l).wrapping_add(1i32);
                }
                sz = (((l).wrapping_sub(m)).wrapping_add(7i32) / 4i32);
                if (sz == 1i32) {
                    sz = 2i32;
                }
                r = self.get_node(sz);
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                q = r;
                self.mem[crate::ix::U((q) as usize)].set_hh_lh((sz).wrapping_add(0i32));
                while (sz > 2i32) {
                    {
                        sz = (sz).wrapping_sub(1i32);
                        r = (r).wrapping_add(1i32);
                        { let __v350 = (self.str_pool[crate::ix::U((m) as usize)]).wrapping_add(0i32); w.set_b0(__v350); }
                        { let __v351 = (self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)]).wrapping_add(0i32); w.set_b1(__v351); }
                        { let __v352 = (self.str_pool[crate::ix::U(((m).wrapping_add(2i32)) as usize)]).wrapping_add(0i32); w.set_b2(__v352); }
                        { let __v353 = (self.str_pool[crate::ix::U(((m).wrapping_add(3i32)) as usize)]).wrapping_add(0i32); w.set_b3(__v353); }
                        self.mem[crate::ix::U((r) as usize)].set_qqqq(w);
                        m = (m).wrapping_add(4i32);
                    }
                }
                w.set_b0(32i32);
                w.set_b1(32i32);
                w.set_b2(32i32);
                w.set_b3(32i32);
                if (l > m) {
                    {
                        { let __v354 = (self.str_pool[crate::ix::U((m) as usize)]).wrapping_add(0i32); w.set_b0(__v354); }
                        if (l > (m).wrapping_add(1i32)) {
                            {
                                { let __v355 = (self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)]).wrapping_add(0i32); w.set_b1(__v355); }
                                if (l > (m).wrapping_add(2i32)) {
                                    {
                                        { let __v356 = (self.str_pool[crate::ix::U(((m).wrapping_add(2i32)) as usize)]).wrapping_add(0i32); w.set_b2(__v356); }
                                        if (l > (m).wrapping_add(3i32)) {
                                            { let __v357 = (self.str_pool[crate::ix::U(((m).wrapping_add(3i32)) as usize)]).wrapping_add(0i32); w.set_b3(__v357); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_qqqq(w);
                if (self.str_pool[crate::ix::U((l) as usize)] == nl) {
                    l = (l).wrapping_add(1i32);
                }
            }
        }
        { let __v358 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v358); }
        { let __v359 = self.pseudo_files; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v359); }
        self.pseudo_files = p;
        // §1753
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
        }
        // §1755
        self.begin_file_reading();
        self.line = 0i32;
        self.cur_input.limit_field = self.cur_input.start_field;
        self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
        if (self.eqtb[crate::ix::U(((629119i32) - 1) as usize)].int() > 0i32) {
            {
                if (self.term_offset > (self.max_print_line).wrapping_sub(3i32)) {
                    self.print_ln();
                } else {
                    if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                        self.print_char(32i32);
                    }
                }
                self.cur_input.name_field = 19i32;
                self.print(2029i32);
                self.open_parens = (self.open_parens).wrapping_add(1i32);
                crate::system::break_out(&mut self.term_out);
            }
        } else {
            self.cur_input.name_field = 18i32;
        }
    }

    /// \[27] Building token lists.
    /// The token lists for macros and for other things like \.{\\mark} and \.{\\output}
    /// and \.{\\write} are produced by a procedure called `scan_toks`.
    /// Before we get into the details of `scan_toks`, let's consider a much
    /// simpler task, that of converting the current string into a token list.
    /// The `str_toks` function does this; it classifies spaces as type `spacer`
    /// and everything else as type `other_char`.
    /// The token list created by `str_toks` begins at `link(temp_head)` and ends
    /// at the value `p` that is returned. (If `p=temp_head`, the list is empty.)
    // §490
    pub fn str_toks(&mut self, mut b: pool_pointer) -> halfword {
        let mut str_toks: halfword = 0;
        let mut p: halfword = 0; // §490
        let mut q: halfword = 0; // §490
        let mut t: halfword = 0; // §490
        let mut k: pool_pointer = 0; // §490
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        p = temp_head;
        self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
        k = b;
        while (k < self.pool_ptr) {
            {
                t = self.str_pool[crate::ix::U((k) as usize)];
                if (t == 32i32) {
                    t = space_token;
                } else {
                    t = (other_token).wrapping_add(t);
                }
                {
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
                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(t);
                    p = q;
                }
                k = (k).wrapping_add(1i32);
            }
        }
        self.pool_ptr = b;
        str_toks = p;
        str_toks
    }

    /// The main reason for wanting `str_toks` is the next function,
    /// `the_toks`, which has similar input/output characteristics.
    /// This procedure is supposed to scan something like `\.{\\skip\\count12}',
    /// i.e., whatever can follow `\.{\\the}', and it constructs a token list
    /// containing something like `\.{-3.0pt minus 0.5fill}'.
    // §491
    pub fn the_toks(&mut self) -> halfword {
        let mut the_toks: halfword = 0;
        let mut old_setting: i32 = 0; // §491
        let mut p: halfword = 0; // §491
        let mut q: halfword = 0; // §491
        let mut r: halfword = 0; // §491
        let mut b: pool_pointer = 0; // §491
        let mut c: small_number = 0; // §491
        'l_exit_f: {
            // §1688
            if (((self.cur_chr) % 2) != 0) {
                {
                    c = self.cur_chr;
                    self.scan_general_text();
                    if (c == 1i32) {
                        the_toks = self.cur_val;
                    } else {
                        {
                            old_setting = self.selector;
                            self.selector = new_string;
                            b = self.pool_ptr;
                            p = self.get_avail();
                            { let __v360 = self.mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v360); }
                            self.token_show(p);
                            self.flush_list(p);
                            self.selector = old_setting;
                            the_toks = self.str_toks(b);
                        }
                    }
                    break 'l_exit_f;
                }
            }
            // §491
            self.get_x_token();
            self.scan_something_internal(tok_val, false);
            if (self.cur_val_level >= ident_val) {
                // §492
                {
                    p = temp_head;
                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(null);
                    if (self.cur_val_level == ident_val) {
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            { let __v361 = (cs_token_flag).wrapping_add(self.cur_val); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v361); }
                            p = q;
                        }
                    } else {
                        if (self.cur_val != null) {
                            {
                                r = self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh();
                                while (r != null) {
                                    {
                                        {
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
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v362 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v362); }
                                            p = q;
                                        }
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                    the_toks = p;
                }
            } else {
                // §491
                {
                    old_setting = self.selector;
                    self.selector = new_string;
                    b = self.pool_ptr;
                    match self.cur_val_level {
                        int_val => {
                            self.print_int(((self.cur_val) as i64));
                        }
                        dimen_val => {
                            {
                                self.print_scaled(self.cur_val);
                                self.print(314i32);
                            }
                        }
                        glue_val => {
                            {
                                self.print_spec(self.cur_val, 314i32);
                                self.delete_glue_ref(self.cur_val);
                            }
                        }
                        mu_val => {
                            {
                                self.print_spec(self.cur_val, 347i32);
                                self.delete_glue_ref(self.cur_val);
                            }
                        }
                        _ => {}
                    }
                    self.selector = old_setting;
                    the_toks = self.str_toks(b);
                }
            }
        }
        the_toks
    }

    /// Here's part of the `expand` subroutine that we are now ready to complete:
    // §493
    pub fn ins_the_toks(&mut self) {
        { let __v363 = self.the_toks(); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v363); }
        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
    }

    /// The procedure `conv_toks` uses `str_toks` to insert the token list
    /// for `convert` functions into the scanner; `\.{\\outer}' control sequences
    /// are allowed to follow `\.{\\string}' and `\.{\\meaning}'.
    /// The extra temp string `u` is needed because `pdf_scan_ext_toks` incorporates
    /// any pending string in its output. In order to save such a pending string,
    /// we have to create a temporary string that is destroyed immediately after.
    // §496
    pub fn conv_toks(&mut self) {
        let mut old_setting: i32 = 0; // §496
        let mut p: halfword = 0; // §496
        let mut q: halfword = 0; // §496
        let mut c: i32 = 0; // §496
        let mut save_scanner_status: small_number = 0; // §496
        let mut save_def_ref: halfword = 0; // §496
        let mut save_warning_index: halfword = 0; // §496
        let mut booltemp: bool = false; // §496
        let mut i: i32 = 0; // §496
        let mut j: i32 = 0; // §496
        let mut b: pool_pointer = 0; // §496
        let mut s: str_number = 0; // §496
        let mut t: str_number = 0; // §496
        let mut u: str_number = 0; // §496
        'l_exit_f: {
            c = self.cur_chr;
            u = 0i32;
            // §497
            match c {
                number_code | roman_numeral_code => {
                    self.scan_int();
                }
                string_code | meaning_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        self.scanner_status = normal;
                        self.get_token();
                        self.scanner_status = save_scanner_status;
                    }
                }
                font_name_code => {
                    self.scan_font_ident();
                }
                eTeX_revision_code => {
                }
                expanded_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        self.begin_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), inserted);
                        {
                            { let __ix364 = self.def_ref; let __v365 = self.avail; self.mem[crate::ix::U((__ix364) as usize)].set_hh_rh(__v365); }
                            self.avail = self.def_ref;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                        self.def_ref = save_def_ref;
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdftex_revision_code => {
                }
                pdftex_banner_code => {
                }
                pdf_font_name_code | pdf_font_objnum_code | pdf_font_size_code => {
                    {
                        self.scan_font_ident();
                        if (self.cur_val == null_font) {
                            self.pdf_error(594i32, 873i32);
                        }
                        if (c != pdf_font_size_code) {
                            {
                                self.pdf_check_vf_cur_val();
                                if (!self.font_used[crate::ix::U((self.cur_val) as usize)]) {
                                    self.pdf_init_font_cur_val();
                                }
                            }
                        }
                    }
                }
                pdf_page_ref_code => {
                    {
                        self.scan_int();
                        if (self.cur_val <= 0i32) {
                            self.pdf_error(874i32, 875i32);
                        }
                    }
                }
                left_margin_kern_code | right_margin_kern_code => {
                    {
                        self.scan_register_num();
                        if (self.cur_val < 256i32) {
                            p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                        } else {
                            {
                                self.find_sa_element(box_val, self.cur_val, false);
                                if (self.cur_ptr == null) {
                                    p = null;
                                } else {
                                    p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                }
                            }
                        }
                        if ((p == null) || (self.mem[crate::ix::U((p) as usize)].hh().b0() != hlist_node)) {
                            self.pdf_error(876i32, 877i32);
                        }
                    }
                }
                pdf_xform_name_code => {
                    {
                        self.scan_int();
                        self.pdf_check_obj(obj_type_xform, self.cur_val);
                    }
                }
                pdf_escape_string_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.escapestring(self.str_start[crate::ix::U((s) as usize)]);
                        { let __v366 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v366); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_escape_name_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.escapename(self.str_start[crate::ix::U((s) as usize)]);
                        { let __v367 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v367); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_escape_hex_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.escapehex(self.str_start[crate::ix::U((s) as usize)]);
                        { let __v368 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v368); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_unescape_hex_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.unescapehex(self.str_start[crate::ix::U((s) as usize)]);
                        { let __v369 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v369); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_creation_date_code => {
                    {
                        b = self.pool_ptr;
                        self.getcreationdate();
                        { let __v370 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v370); }
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        break 'l_exit_f;
                    }
                }
                pdf_file_mod_date_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.getfilemoddate(s);
                        { let __v371 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v371); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_file_size_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.getfilesize(s);
                        { let __v372 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v372); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_mdfive_sum_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        booltemp = self.scan_keyword(878i32);
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.getmd5sum(s, booltemp);
                        { let __v373 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v373); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_file_dump_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.cur_val = 0i32;
                        if self.scan_keyword(879i32) {
                            {
                                self.scan_int();
                                if (self.cur_val < 0i32) {
                                    {
                                        {
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(264i32);
                                            }
                                            self.print(880i32);
                                        }
                                        {
                                            self.help_ptr = 2i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 881i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                                        }
                                        self.int_error(self.cur_val);
                                        self.cur_val = 0i32;
                                    }
                                }
                            }
                        }
                        i = self.cur_val;
                        self.cur_val = 0i32;
                        if self.scan_keyword(882i32) {
                            {
                                self.scan_int();
                                if (self.cur_val < 0i32) {
                                    {
                                        {
                                            if (self.interaction == error_stop_mode) {
                                            }
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(264i32);
                                            }
                                            self.print(883i32);
                                        }
                                        {
                                            self.help_ptr = 2i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 884i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                                        }
                                        self.int_error(self.cur_val);
                                        self.cur_val = 0i32;
                                    }
                                }
                            }
                        }
                        j = self.cur_val;
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.getfiledump(s, i, j);
                        { let __v374 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v374); }
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_match_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        booltemp = self.scan_keyword(885i32);
                        i = (1i32).wrapping_neg();
                        if self.scan_keyword(886i32) {
                            {
                                self.scan_int();
                                i = self.cur_val;
                            }
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.scan_pdf_ext_toks();
                        t = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        b = self.pool_ptr;
                        self.matchstrings(s, t, i, booltemp);
                        { let __v375 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v375); }
                        self.flush_str(t);
                        self.flush_str(s);
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                        break 'l_exit_f;
                    }
                }
                pdf_last_match_code => {
                    {
                        self.scan_int();
                        if (self.cur_val < 0i32) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(887i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 888i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 790i32;
                                }
                                self.int_error(self.cur_val);
                                self.cur_val = 0i32;
                            }
                        }
                        b = self.pool_ptr;
                        self.getmatch(self.cur_val);
                        { let __v376 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v376); }
                        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                        break 'l_exit_f;
                    }
                }
                pdf_strcmp_code => {
                    {
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.compare_strings();
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        if (u != 0i32) {
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                u = 0i32;
                            }
                        }
                    }
                }
                pdf_colorstack_init_code => {
                    {
                        booltemp = self.scan_keyword(889i32);
                        if self.scan_keyword(890i32) {
                            self.cur_val = direct_always;
                        } else {
                            if self.scan_keyword(889i32) {
                                self.cur_val = direct_page;
                            } else {
                                self.cur_val = set_origin;
                            }
                        }
                        save_scanner_status = self.scanner_status;
                        save_warning_index = self.warning_index;
                        save_def_ref = self.def_ref;
                        if (self.str_start[crate::ix::U((self.str_ptr) as usize)] < self.pool_ptr) {
                            u = self.make_string();
                        }
                        self.scan_pdf_ext_toks();
                        s = self.tokens_to_string(self.def_ref);
                        self.delete_token_ref(self.def_ref);
                        self.def_ref = save_def_ref;
                        self.warning_index = save_warning_index;
                        self.scanner_status = save_scanner_status;
                        self.cur_val = self.newcolorstack(s, self.cur_val, booltemp);
                        self.flush_str(s);
                        self.cur_val_level = int_val;
                        if (self.cur_val < 0i32) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(891i32);
                                }
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 892i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 893i32;
                                }
                                self.error();
                                self.cur_val = 0i32;
                                if (u != 0i32) {
                                    {
                                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                        u = 0i32;
                                    }
                                }
                            }
                        }
                    }
                }
                job_name_code => {
                    if (self.job_name == 0i32) {
                        self.open_log_file();
                    }
                }
                uniform_deviate_code => {
                    self.scan_int();
                }
                normal_deviate_code => {
                }
                pdf_insert_ht_code => {
                    self.scan_register_num();
                }
                pdf_ximage_bbox_code => {
                    {
                        self.scan_int();
                        self.pdf_check_obj(obj_type_ximage, self.cur_val);
                        i = self.pdf_mem[crate::ix::U(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int4).wrapping_add(4i32)) as usize)];
                        self.scan_int();
                        j = self.cur_val;
                        if ((j < 1i32) || (j > 4i32)) {
                            self.pdf_error(871i32, 894i32);
                        }
                    }
                }
                _ => {}
            }
            // §496
            old_setting = self.selector;
            self.selector = new_string;
            b = self.pool_ptr;
            // §498
            match c {
                number_code => {
                    self.print_int(((self.cur_val) as i64));
                }
                roman_numeral_code => {
                    self.print_roman_int(self.cur_val);
                }
                string_code => {
                    if (self.cur_cs != 0i32) {
                        self.sprint_cs(self.cur_cs);
                    } else {
                        self.print_char(self.cur_chr);
                    }
                }
                meaning_code => {
                    self.print_meaning();
                }
                font_name_code => {
                    {
                        self.print(self.font_name[crate::ix::U((self.cur_val) as usize)]);
                        if (self.font_size[crate::ix::U((self.cur_val) as usize)] != self.font_dsize[crate::ix::U((self.cur_val) as usize)]) {
                            {
                                self.print(895i32);
                                self.print_scaled(self.font_size[crate::ix::U((self.cur_val) as usize)]);
                                self.print(314i32);
                            }
                        }
                    }
                }
                eTeX_revision_code => {
                    self.print(eTeX_revision);
                }
                pdftex_revision_code => {
                    self.print(pdftex_revision);
                }
                pdftex_banner_code => {
                    self.print(self.pdftex_banner);
                }
                pdf_font_name_code | pdf_font_objnum_code => {
                    {
                        {
                            if (self.pdf_font_num[crate::ix::U((self.cur_val) as usize)] < 0i32) {
                                self.ff = (self.pdf_font_num[crate::ix::U((self.cur_val) as usize)]).wrapping_neg();
                            } else {
                                self.ff = self.cur_val;
                            }
                        }
                        if (c == pdf_font_name_code) {
                            self.print_int(((self.obj_tab[crate::ix::U((self.pdf_font_num[crate::ix::U((self.ff) as usize)]) as usize)].int0) as i64));
                        } else {
                            self.print_int(((self.pdf_font_num[crate::ix::U((self.ff) as usize)]) as i64));
                        }
                    }
                }
                pdf_font_size_code => {
                    {
                        self.print_scaled(self.font_size[crate::ix::U((self.cur_val) as usize)]);
                        self.print(314i32);
                    }
                }
                pdf_page_ref_code => {
                    { let __a377_0 = ((self.get_obj(obj_type_page, self.cur_val, false)) as i64); self.print_int(__a377_0) };
                }
                left_margin_kern_code => {
                    {
                        p = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                        while ((p != null) && (((!(p >= self.hi_mem_min)) && ((((((((((self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node)) || (((self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() != pdf_refximage_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() != pdf_refxform_node))) || ((((self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && (((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal)) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == auto_kern)))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null)))) || (((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 8i32)))) {
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        }
                        if ((((p != null) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == margin_kern_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == left_side)) {
                            self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                        } else {
                            self.print(48i32);
                        }
                        self.print(314i32);
                    }
                }
                right_margin_kern_code => {
                    {
                        q = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                        p = self.prev_rightmost(q, null);
                        while ((p != null) && (((!(p >= self.hi_mem_min)) && ((((((((((self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node)) || (((self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() != pdf_refximage_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() != pdf_refxform_node))) || ((((self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == null)) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == null)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && (((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal)) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == auto_kern)))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null)))) || (((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 9i32)))) {
                            p = self.prev_rightmost(q, p);
                        }
                        if ((((p != null) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == margin_kern_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == right_side)) {
                            self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                        } else {
                            self.print(48i32);
                        }
                        self.print(314i32);
                    }
                }
                pdf_xform_name_code => {
                    self.print_int(((self.obj_tab[crate::ix::U((self.cur_val) as usize)].int0) as i64));
                }
                pdf_strcmp_code => {
                    self.print_int(((self.cur_val) as i64));
                }
                pdf_colorstack_init_code => {
                    self.print_int(((self.cur_val) as i64));
                }
                uniform_deviate_code => {
                    { let __a378_0 = ((self.unif_rand(self.cur_val)) as i64); self.print_int(__a378_0) };
                }
                normal_deviate_code => {
                    { let __a379_0 = ((self.norm_rand()) as i64); self.print_int(__a379_0) };
                }
                pdf_insert_ht_code => {
                    {
                        i = (self.cur_val).wrapping_add(0i32);
                        p = page_ins_head;
                        while (i >= self.mem[crate::ix::U((self.mem[crate::ix::U((p) as usize)].hh().rh()) as usize)].hh().b1()) {
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        }
                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == i) {
                            self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                        } else {
                            self.print(48i32);
                        }
                        self.print(314i32);
                    }
                }
                pdf_ximage_bbox_code => {
                    {
                        if self.is_pdf_image(i) {
                            {
                                match j {
                                    1 => {
                                        { let __a380_0 = self.epdf_orig_x(i); self.print_scaled(__a380_0) };
                                    }
                                    2 => {
                                        { let __a381_0 = self.epdf_orig_y(i); self.print_scaled(__a381_0) };
                                    }
                                    3 => {
                                        { let __a382_0 = (self.epdf_orig_x(i)).wrapping_add(self.image_width(i)); self.print_scaled(__a382_0) };
                                    }
                                    4 => {
                                        { let __a383_0 = (self.epdf_orig_y(i)).wrapping_add(self.image_height(i)); self.print_scaled(__a383_0) };
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            self.print_scaled(0i32);
                        }
                        self.print(314i32);
                    }
                }
                job_name_code => {
                    self.print(self.job_name);
                }
                _ => {}
            }
            // §496
            self.selector = old_setting;
            { let __v384 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v384); }
            self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
        }
    }

    /// Now we can't postpone the difficulties any longer; we must bravely tackle
    /// `scan_toks`. This function returns a pointer to the tail of a new token
    /// list, and it also makes `def_ref` point to the reference count at the
    /// head of that list.
    /// There are two boolean parameters, `macro_def` and `xpand`. If `macro_def`
    /// is true, the goal is to create the token list for a macro definition;
    /// otherwise the goal is to create the token list for some other \TeX\
    /// primitive: \.{\\mark}, \.{\\output}, \.{\\everypar}, \.{\\lowercase},
    /// \.{\\uppercase}, \.{\\message}, \.{\\errmessage}, \.{\\write}, or
    /// \.{\\special}. In the latter cases a left brace must be scanned next; this
    /// left brace will not be part of the token list, nor will the matching right
    /// brace that comes at the end. If `xpand` is false, the token list will
    /// simply be copied from the input using `get_token`. Otherwise all expandable
    /// tokens will be expanded until unexpandable tokens are left, except that
    /// ...
    // §499
    pub fn scan_toks(&mut self, mut macro_def: bool, mut xpand: bool) -> halfword {
        let mut scan_toks: halfword = 0;
        let mut t: halfword = 0; // §499
        let mut s: halfword = 0; // §499
        let mut p: halfword = 0; // §499
        let mut q: halfword = 0; // §499
        let mut unbalance: halfword = 0; // §499
        let mut hash_brace: halfword = 0; // §499
        'l_found_f: {
            if macro_def {
                self.scanner_status = defining;
            } else {
                self.scanner_status = absorbing;
            }
            self.warning_index = self.cur_cs;
            self.def_ref = self.get_avail();
            { let __ix385 = self.def_ref; self.mem[crate::ix::U((__ix385) as usize)].set_hh_lh(null); }
            p = self.def_ref;
            hash_brace = 0i32;
            t = zero_token;
            if macro_def {
                // §500
                {
                    'l_done_f: {
                        'l_done1_f: {
                            while true {
                                {
                                    'l_continue_b: loop {
                                        self.intr_weak = true;
                                        self.get_token();
                                        self.intr_weak = false;
                                        if (self.cur_tok < right_brace_limit) {
                                            break 'l_done1_f;
                                        }
                                        if (self.cur_cmd == mac_param) {
                                            // §502
                                            {
                                                s = (match_token).wrapping_add(self.cur_chr);
                                                self.get_token();
                                                if (self.cur_tok < left_brace_limit) {
                                                    {
                                                        hash_brace = self.cur_tok;
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            { let __v386 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v386); }
                                                            p = q;
                                                        }
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
                                                            p = q;
                                                        }
                                                        break 'l_done_f;
                                                    }
                                                }
                                                if (t == 3129i32) {
                                                    {
                                                        {
                                                            if (self.interaction == error_stop_mode) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(264i32);
                                                            }
                                                            self.print(898i32);
                                                        }
                                                        {
                                                            self.help_ptr = 2i32;
                                                            self.help_line[crate::ix::U((1i32) as usize)] = 899i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 900i32;
                                                        }
                                                        self.error();
                                                        continue 'l_continue_b;
                                                    }
                                                } else {
                                                    {
                                                        t = (t).wrapping_add(1i32);
                                                        if (self.cur_tok != t) {
                                                            {
                                                                {
                                                                    if (self.interaction == error_stop_mode) {
                                                                    }
                                                                    if self.file_line_error_style_p {
                                                                        self.print_file_line();
                                                                    } else {
                                                                        self.print_nl(264i32);
                                                                    }
                                                                    self.print(901i32);
                                                                }
                                                                {
                                                                    self.help_ptr = 2i32;
                                                                    self.help_line[crate::ix::U((1i32) as usize)] = 902i32;
                                                                    self.help_line[crate::ix::U((0i32) as usize)] = 903i32;
                                                                }
                                                                self.back_error();
                                                            }
                                                        }
                                                        self.cur_tok = s;
                                                    }
                                                }
                                            }
                                        }
                                        // §500
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v387 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v387); }
                                            p = q;
                                        }
                                        break 'l_continue_b;
                                    }
                                }
                            }
                        }
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
                            p = q;
                        }
                        if (self.cur_cmd == right_brace) {
                            // §501
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(744i32);
                                }
                                self.align_state = (self.align_state).wrapping_add(1i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 896i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 897i32;
                                }
                                self.error();
                                break 'l_found_f;
                            }
                        }
                    }
                    // §500
                }
            } else {
                // §499
                self.scan_left_brace();
            }
            // §503
            unbalance = 1i32;
            while true {
                {
                    if xpand {
                        // §504
                        {
                            'l_done2_f: {
                                while true {
                                    {
                                        self.get_next();
                                        if (self.cur_cmd >= call) {
                                            if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
                                                {
                                                    self.cur_cmd = relax;
                                                    self.cur_chr = no_expand_flag;
                                                }
                                            }
                                        }
                                        if (self.cur_cmd <= max_command) {
                                            break 'l_done2_f;
                                        }
                                        if (self.cur_cmd != the) {
                                            self.expand();
                                        } else {
                                            {
                                                q = self.the_toks();
                                                if (self.mem[crate::ix::U((temp_head) as usize)].hh().rh() != null) {
                                                    {
                                                        { let __v388 = self.mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v388); }
                                                        p = q;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            self.x_token();
                        }
                    } else {
                        // §503
                        {
                            self.intr_weak = true;
                            self.get_token();
                            self.intr_weak = false;
                        }
                    }
                    if (self.cur_tok < right_brace_limit) {
                        if (self.cur_cmd < right_brace) {
                            unbalance = (unbalance).wrapping_add(1i32);
                        } else {
                            {
                                unbalance = (unbalance).wrapping_sub(1i32);
                                if (unbalance == 0i32) {
                                    break 'l_found_f;
                                }
                            }
                        }
                    } else {
                        if (self.cur_cmd == mac_param) {
                            if macro_def {
                                // §505
                                {
                                    s = self.cur_tok;
                                    if xpand {
                                        self.get_x_token();
                                    } else {
                                        self.get_token();
                                    }
                                    if (self.cur_cmd != mac_param) {
                                        if ((self.cur_tok <= zero_token) || (self.cur_tok > t)) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(264i32);
                                                    }
                                                    self.print(904i32);
                                                }
                                                self.sprint_cs(self.warning_index);
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 905i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 906i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 907i32;
                                                }
                                                self.back_error();
                                                self.cur_tok = s;
                                            }
                                        } else {
                                            self.cur_tok = (1232i32).wrapping_add(self.cur_chr);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §503
                    {
                        q = self.get_avail();
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                        { let __v389 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v389); }
                        p = q;
                    }
                }
            }
        }
        // §499
        self.scanner_status = normal;
        if (hash_brace != 0i32) {
            {
                q = self.get_avail();
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(hash_brace);
                p = q;
            }
        }
        scan_toks = p;
        scan_toks
    }

    /// The `read_toks` procedure constructs a token list like that for any
    /// macro definition, and makes `cur_val` point to it. Parameter `r` points
    /// to the control sequence that will receive this token list.
    // §508
    pub fn read_toks(&mut self, mut n: i32, mut r: halfword, mut j: halfword) {
        let mut p: halfword = 0; // §508
        let mut q: halfword = 0; // §508
        let mut s: i32 = 0; // §508
        let mut m: small_number = 0; // §508
        self.scanner_status = defining;
        self.warning_index = r;
        self.def_ref = self.get_avail();
        { let __ix390 = self.def_ref; self.mem[crate::ix::U((__ix390) as usize)].set_hh_lh(null); }
        p = self.def_ref;
        {
            q = self.get_avail();
            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
            p = q;
        }
        if ((n < 0i32) || (n > 15i32)) {
            m = 16i32;
        } else {
            m = n;
        }
        s = self.align_state;
        self.align_state = 1000000i32;
        loop {
            'l_done_f: {
                // §509
                self.begin_file_reading();
                self.cur_input.name_field = (m).wrapping_add(1i32);
                if (self.read_open[crate::ix::U((m) as usize)] == closed) {
                    // §510
                    if (self.interaction > nonstop_mode) {
                        if (n < 0i32) {
                            {
                                self.print(348i32);
                                self.term_input();
                            }
                        } else {
                            {
                                self.print_ln();
                                self.sprint_cs(r);
                                {
                                    self.print(61i32);
                                    self.term_input();
                                }
                                n = (1i32).wrapping_neg();
                            }
                        }
                    } else {
                        {
                            self.cur_input.limit_field = 0i32;
                            self.fatal_error(908i32);
                        }
                    }
                } else {
                    // §509
                    if (self.read_open[crate::ix::U((m) as usize)] == just_open) {
                        // §511
                        if { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.input_ln(&mut __f0, false); self.read_file[crate::ix::U((m) as usize)] = __f0; __r } {
                            self.read_open[crate::ix::U((m) as usize)] = normal;
                        } else {
                            {
                                { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.a_close(&mut __f0); self.read_file[crate::ix::U((m) as usize)] = __f0; __r };
                                self.read_open[crate::ix::U((m) as usize)] = closed;
                            }
                        }
                    } else {
                        // §512
                        {
                            if (!{ let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.input_ln(&mut __f0, true); self.read_file[crate::ix::U((m) as usize)] = __f0; __r }) {
                                {
                                    { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.a_close(&mut __f0); self.read_file[crate::ix::U((m) as usize)] = __f0; __r };
                                    self.read_open[crate::ix::U((m) as usize)] = closed;
                                    if (self.align_state != 1000000i32) {
                                        {
                                            self.runaway();
                                            {
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(264i32);
                                                }
                                                self.print(909i32);
                                            }
                                            self.print_esc(613i32);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 910i32;
                                            }
                                            self.align_state = 1000000i32;
                                            self.cur_input.limit_field = 0i32;
                                            self.error();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // §509
                self.cur_input.limit_field = self.last;
                if ((self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix391 = self.cur_input.limit_field; let __v392 = self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix391) as usize)] = __v392; }
                }
                self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                self.cur_input.loc_field = self.cur_input.start_field;
                self.cur_input.state_field = new_line;
                // §1761
                if (j == 1i32) {
                    {
                        while (self.cur_input.loc_field <= self.cur_input.limit_field) {
                            {
                                self.cur_chr = self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)];
                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                if (self.cur_chr == 32i32) {
                                    self.cur_tok = space_token;
                                } else {
                                    self.cur_tok = (self.cur_chr).wrapping_add(3072i32);
                                }
                                {
                                    q = self.get_avail();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                    { let __v393 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v393); }
                                    p = q;
                                }
                            }
                        }
                        break 'l_done_f;
                    }
                }
                // §509
                while true {
                    {
                        self.get_token();
                        if (self.cur_tok == 0i32) {
                            break 'l_done_f;
                        }
                        if (self.align_state < 1000000i32) {
                            {
                                loop {
                                    self.get_token();
                                    if (self.cur_tok == 0i32) { break; }
                                }
                                self.align_state = 1000000i32;
                                break 'l_done_f;
                            }
                        }
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            { let __v394 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v394); }
                            p = q;
                        }
                    }
                }
            }
            self.end_file_reading();
            if (self.align_state == 1000000i32) { break; }
        }
        // §508
        self.cur_val = self.def_ref;
        self.scanner_status = normal;
        self.align_state = s;
    }

    /// Here is a procedure that ignores text until coming to an \.{\\or},
    /// \.{\\else}, or \.{\\fi} at the current level of $\.{\\if}\ldots\.{\\fi}$
    /// nesting. After it has acted, `cur_chr` will indicate the token that
    /// was found, but `cur_tok` will not be set (because this makes the
    /// procedure run faster).
    // §520
    pub fn pass_text(&mut self) {
        let mut l: i32 = 0; // §520
        let mut save_scanner_status: small_number = 0; // §520
        'l_done_f: {
            save_scanner_status = self.scanner_status;
            self.scanner_status = skipping;
            l = 0i32;
            self.skip_line = self.line;
            while true {
                {
                    self.get_next();
                    if (self.cur_cmd == fi_or_else) {
                        {
                            if (l == 0i32) {
                                break 'l_done_f;
                            }
                            if (self.cur_chr == fi_code) {
                                l = (l).wrapping_sub(1i32);
                            }
                        }
                    } else {
                        if (self.cur_cmd == if_test) {
                            l = (l).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.scanner_status = save_scanner_status;
        if (self.eqtb[crate::ix::U(((629118i32) - 1) as usize)].int() > 0i32) {
            self.show_cur_cmd_chr();
        }
    }

    /// Here's a procedure that changes the `if_limit` code corresponding to
    /// a given value of `cond_ptr`.
    // §523
    pub fn change_if_limit(&mut self, mut l: small_number, mut p: halfword) {
        let mut q: halfword = 0; // §523
        'l_exit_f: {
            if (p == self.cond_ptr) {
                self.if_limit = l;
            } else {
                {
                    q = self.cond_ptr;
                    while true {
                        {
                            if (q == null) {
                                self.confusion(911i32);
                            }
                            if (self.mem[crate::ix::U((q) as usize)].hh().rh() == p) {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(l);
                                    break 'l_exit_f;
                                }
                            }
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                    }
                }
            }
        }
    }

    /// A condition is started when the `expand` procedure encounters
    /// an `if_test` command; in that case `expand` reduces to `conditional`,
    /// which is a recursive procedure.
    // §524
    pub fn conditional(&mut self) {
        let mut b: bool = false; // §524
        let mut e: bool = false; // §524
        let mut r: i32 = 0; // §524
        let mut m: i32 = 0; // §524
        let mut n: i32 = 0; // §524
        let mut p: halfword = 0; // §524
        let mut q: halfword = 0; // §524
        let mut save_scanner_status: small_number = 0; // §524
        let mut save_cond_ptr: halfword = 0; // §524
        let mut this_if: small_number = 0; // §524
        let mut is_unless: bool = false; // §524
        'l_exit_f: {
            'l_common_ending_f: {
                if (self.eqtb[crate::ix::U(((629118i32) - 1) as usize)].int() > 0i32) {
                    if (self.eqtb[crate::ix::U(((629054i32) - 1) as usize)].int() <= 1i32) {
                        self.show_cur_cmd_chr();
                    }
                }
                // §521
                {
                    p = self.get_node(if_node_size);
                    { let __v395 = self.cond_ptr; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v395); }
                    { let __v396 = self.if_limit; self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v396); }
                    { let __v397 = self.cur_if; self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v397); }
                    { let __v398 = self.if_line; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v398); }
                    self.cond_ptr = p;
                    self.cur_if = self.cur_chr;
                    self.if_limit = if_code;
                    self.if_line = self.line;
                }
                // §524
                save_cond_ptr = self.cond_ptr;
                is_unless = (self.cur_chr >= unless_code);
                this_if = (self.cur_chr % unless_code);
                // §527
                match this_if {
                    if_char_code | if_cat_code => {
                        // §532
                        {
                            {
                                self.get_x_token();
                                if (self.cur_cmd == relax) {
                                    if (self.cur_chr == no_expand_flag) {
                                        {
                                            self.cur_cmd = active_char;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(4096i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > active_char) || (self.cur_chr > 255i32)) {
                                {
                                    m = relax;
                                    n = 256i32;
                                }
                            } else {
                                {
                                    m = self.cur_cmd;
                                    n = self.cur_chr;
                                }
                            }
                            {
                                self.get_x_token();
                                if (self.cur_cmd == relax) {
                                    if (self.cur_chr == no_expand_flag) {
                                        {
                                            self.cur_cmd = active_char;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(4096i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > active_char) || (self.cur_chr > 255i32)) {
                                {
                                    self.cur_cmd = relax;
                                    self.cur_chr = 256i32;
                                }
                            }
                            if (this_if == if_char_code) {
                                b = (n == self.cur_chr);
                            } else {
                                b = (m == self.cur_cmd);
                            }
                        }
                    }
                    if_int_code | if_dim_code => {
                        // §529
                        {
                            if (this_if == if_int_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            n = self.cur_val;
                            // §432
                            loop {
                                self.get_x_token();
                                if (self.cur_cmd != spacer) { break; }
                            }
                            // §529
                            if ((self.cur_tok >= 3132i32) && (self.cur_tok <= 3134i32)) {
                                r = (self.cur_tok).wrapping_sub(3072i32);
                            } else {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(937i32);
                                    }
                                    self.print_cmd_chr(if_test, this_if);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 938i32;
                                    }
                                    self.back_error();
                                    r = 61i32;
                                }
                            }
                            if (this_if == if_int_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            match r {
                                60 => {
                                    b = (n < self.cur_val);
                                }
                                61 => {
                                    b = (n == self.cur_val);
                                }
                                62 => {
                                    b = (n > self.cur_val);
                                }
                                _ => {}
                            }
                        }
                    }
                    if_odd_code => {
                        // §530
                        {
                            self.scan_int();
                            b = (((self.cur_val) % 2) != 0);
                        }
                    }
                    if_vmode_code => {
                        // §527
                        b = ((self.cur_list.mode_field).wrapping_abs() == vmode);
                    }
                    if_hmode_code => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == hmode);
                    }
                    if_mmode_code => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == mmode);
                    }
                    if_inner_code => {
                        b = (self.cur_list.mode_field < 0i32);
                    }
                    if_void_code | if_hbox_code | if_vbox_code => {
                        // §531
                        {
                            self.scan_register_num();
                            if (self.cur_val < 256i32) {
                                p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(box_val, self.cur_val, false);
                                    if (self.cur_ptr == null) {
                                        p = null;
                                    } else {
                                        p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                            if (this_if == if_void_code) {
                                b = (p == null);
                            } else {
                                if (p == null) {
                                    b = false;
                                } else {
                                    if (this_if == if_hbox_code) {
                                        b = (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node);
                                    } else {
                                        b = (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node);
                                    }
                                }
                            }
                        }
                    }
                    ifx_code => {
                        // §533
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            n = self.cur_cs;
                            p = self.cur_cmd;
                            q = self.cur_chr;
                            self.get_next();
                            if (self.cur_cmd != p) {
                                b = false;
                            } else {
                                if (self.cur_cmd < call) {
                                    b = (self.cur_chr == q);
                                } else {
                                    // §534
                                    {
                                        p = self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh();
                                        q = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as usize)].hh().rh();
                                        if (p == q) {
                                            b = true;
                                        } else {
                                            {
                                                while ((p != null) && (q != null)) {
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().lh() != self.mem[crate::ix::U((q) as usize)].hh().lh()) {
                                                        p = null;
                                                    } else {
                                                        {
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                                b = ((p == null) && (q == null));
                                            }
                                        }
                                    }
                                }
                            }
                            // §533
                            self.scanner_status = save_scanner_status;
                        }
                    }
                    if_eof_code => {
                        // §527
                        {
                            self.scan_four_bit_int_or_18();
                            if (self.cur_val == 18i32) {
                                b = (!self.shellenabledp);
                            } else {
                                b = (self.read_open[crate::ix::U((self.cur_val) as usize)] == closed);
                            }
                        }
                    }
                    if_true_code => {
                        b = true;
                    }
                    if_false_code => {
                        b = false;
                    }
                    if_def_code => {
                        // §1766
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            b = (self.cur_cmd != undefined_cs);
                            self.scanner_status = save_scanner_status;
                        }
                    }
                    if_cs_code => {
                        // §1767
                        {
                            n = self.get_avail();
                            p = n;
                            e = self.is_in_csname;
                            self.is_in_csname = true;
                            loop {
                                self.get_x_token();
                                if (self.cur_cs == 0i32) {
                                    {
                                        q = self.get_avail();
                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                        { let __v399 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v399); }
                                        p = q;
                                    }
                                }
                                if (self.cur_cs != 0i32) { break; }
                            }
                            if (self.cur_cmd != end_cs_name) {
                                // §399
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(712i32);
                                    }
                                    self.print_esc(581i32);
                                    self.print(713i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 714i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 715i32;
                                    }
                                    self.back_error();
                                }
                            }
                            // §1768
                            m = self.first;
                            p = self.mem[crate::ix::U((n) as usize)].hh().rh();
                            while (p != null) {
                                {
                                    if (m >= self.max_buf_stack) {
                                        {
                                            self.max_buf_stack = (m).wrapping_add(1i32);
                                            if (self.max_buf_stack == buf_size) {
                                                self.overflow(258i32, buf_size);
                                            }
                                        }
                                    }
                                    { let __v400 = (self.mem[crate::ix::U((p) as usize)].hh().lh() % 256i32); self.buffer[crate::ix::U((m) as usize)] = __v400; }
                                    m = (m).wrapping_add(1i32);
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            if (m > (self.first).wrapping_add(1i32)) {
                                self.cur_cs = self.id_lookup(self.first, (m).wrapping_sub(self.first));
                            } else {
                                if (m == self.first) {
                                    self.cur_cs = null_cs;
                                } else {
                                    self.cur_cs = (single_base).wrapping_add(self.buffer[crate::ix::U((self.first) as usize)]);
                                }
                            }
                            // §1767
                            self.flush_list(n);
                            if self.intr_rec_on {
                                self.flashtex_intr_read(self.cur_cs);
                            }
                            b = (self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0() != undefined_cs);
                            self.is_in_csname = e;
                        }
                    }
                    if_in_csname_code => {
                        // §1769
                        b = self.is_in_csname;
                    }
                    if_pdfabs_dim_code | if_pdfabs_num_code => {
                        {
                            if (this_if == if_pdfabs_num_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            n = self.cur_val;
                            if (n < 0i32) {
                                n = (n).wrapping_neg();
                            }
                            // §432
                            loop {
                                self.get_x_token();
                                if (self.cur_cmd != spacer) { break; }
                            }
                            // §1769
                            if ((self.cur_tok >= 3132i32) && (self.cur_tok <= 3134i32)) {
                                r = (self.cur_tok).wrapping_sub(3072i32);
                            } else {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(264i32);
                                        }
                                        self.print(937i32);
                                    }
                                    self.print_cmd_chr(if_test, this_if);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 938i32;
                                    }
                                    self.back_error();
                                    r = 61i32;
                                }
                            }
                            if (this_if == if_pdfabs_num_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            if (self.cur_val < 0i32) {
                                self.cur_val = (self.cur_val).wrapping_neg();
                            }
                            match r {
                                60 => {
                                    b = (n < self.cur_val);
                                }
                                61 => {
                                    b = (n == self.cur_val);
                                }
                                62 => {
                                    b = (n > self.cur_val);
                                }
                                _ => {}
                            }
                        }
                    }
                    if_font_char_code => {
                        {
                            self.scan_font_ident();
                            n = self.cur_val;
                            self.scan_char_num();
                            if ((self.font_bc[crate::ix::U((n) as usize)] <= self.cur_val) && (self.font_ec[crate::ix::U((n) as usize)] >= self.cur_val)) {
                                b = (self.font_info[crate::ix::U((((self.char_base[crate::ix::U((n) as usize)]).wrapping_add(self.cur_val)).wrapping_add(0i32)) as usize)].qqqq().b0() > min_quarterword);
                            } else {
                                b = false;
                            }
                        }
                    }
                    if_case_code => {
                        // §535
                        {
                            self.scan_int();
                            n = self.cur_val;
                            if (self.eqtb[crate::ix::U(((629054i32) - 1) as usize)].int() > 1i32) {
                                {
                                    self.begin_diagnostic();
                                    self.print(939i32);
                                    self.print_int(((n) as i64));
                                    self.print_char(125i32);
                                    self.end_diagnostic(false);
                                }
                            }
                            while (n != 0i32) {
                                {
                                    self.pass_text();
                                    if (self.cond_ptr == save_cond_ptr) {
                                        if (self.cur_chr == or_code) {
                                            n = (n).wrapping_sub(1i32);
                                        } else {
                                            break 'l_common_ending_f;
                                        }
                                    } else {
                                        if (self.cur_chr == fi_code) {
                                            // §522
                                            {
                                                if self.intr_rec_on {
                                                    self.flashtex_intr_pop_cond();
                                                }
                                                if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                                                    self.if_warning();
                                                }
                                                p = self.cond_ptr;
                                                self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                                self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                                self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                self.free_node(p, if_node_size);
                                            }
                                        }
                                    }
                                }
                            }
                            // §535
                            self.change_if_limit(or_code, save_cond_ptr);
                            break 'l_exit_f;
                        }
                    }
                    if_pdfprimitive_code => {
                        // §527
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            self.scanner_status = save_scanner_status;
                            if (self.cur_cs < hash_base) {
                                m = self.prim_lookup((self.cur_cs).wrapping_sub(257i32));
                            } else {
                                m = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 514) as usize)].rh());
                            }
                            b = ((((self.cur_cmd != undefined_cs) && (m != undefined_primitive)) && (self.cur_cmd == self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(m)) - 1) as usize)].hh().b0())) && (self.cur_chr == self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(m)) - 1) as usize)].hh().rh()));
                        }
                    }
                    _ => {}
                }
                // §524
                if is_unless {
                    b = (!b);
                }
                if (self.eqtb[crate::ix::U(((629054i32) - 1) as usize)].int() > 1i32) {
                    // §528
                    {
                        self.begin_diagnostic();
                        if b {
                            self.print(935i32);
                        } else {
                            self.print(936i32);
                        }
                        self.end_diagnostic(false);
                    }
                }
                // §524
                if b {
                    {
                        self.change_if_limit(else_code, save_cond_ptr);
                        break 'l_exit_f;
                    }
                }
                // §526
                while true {
                    {
                        self.pass_text();
                        if (self.cond_ptr == save_cond_ptr) {
                            {
                                if (self.cur_chr != or_code) {
                                    break 'l_common_ending_f;
                                }
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(264i32);
                                    }
                                    self.print(933i32);
                                }
                                self.print_esc(931i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 934i32;
                                }
                                self.error();
                            }
                        } else {
                            if (self.cur_chr == fi_code) {
                                // §522
                                {
                                    if self.intr_rec_on {
                                        self.flashtex_intr_pop_cond();
                                    }
                                    if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                                        self.if_warning();
                                    }
                                    p = self.cond_ptr;
                                    self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                    self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    self.free_node(p, if_node_size);
                                }
                            }
                        }
                    }
                }
            }
            // §524
            if (self.cur_chr == fi_code) {
                // §522
                {
                    if self.intr_rec_on {
                        self.flashtex_intr_pop_cond();
                    }
                    if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                        self.if_warning();
                    }
                    p = self.cond_ptr;
                    self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                    self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                    self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                    self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.free_node(p, if_node_size);
                }
            } else {
                // §524
                self.if_limit = fi_code;
            }
        }
    }

    /// Here now is the first of the system-dependent routines for file name scanning.
    // §541
    pub fn begin_name(&mut self) {
        self.area_delimiter = 0i32;
        self.ext_delimiter = 0i32;
        self.quoted_filename = false;
    }

    /// And here's the second. The string pool might change as the file name is
    /// being scanned, since a new \.{\\csname} might be entered; therefore we keep
    /// `area_delimiter` and `ext_delimiter` relative to the beginning of the current
    /// string, instead of assigning an absolute address like `pool_ptr` to them.
    // §542
    pub fn more_name(&mut self, mut c: ASCII_code) -> bool {
        let mut more_name: bool = false;
        if (((c == 32i32) && self.stop_at_space) && (!self.quoted_filename)) {
            more_name = false;
        } else {
            if (c == 34i32) {
                {
                    self.quoted_filename = (!self.quoted_filename);
                    more_name = true;
                }
            } else {
                {
                    {
                        if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                            self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                        }
                    }
                    {
                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = c;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    if (c == 47i32) {
                        {
                            self.area_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]);
                            self.ext_delimiter = 0i32;
                        }
                    } else {
                        if (c == 46i32) {
                            self.ext_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]);
                        }
                    }
                    more_name = true;
                }
            }
        }
        more_name
    }

    /// The third.
    // §543
    pub fn end_name(&mut self) {
        let mut j: pool_pointer = 0; // §543
        let mut s: pool_pointer = 0; // §543
        let mut t: pool_pointer = 0; // §543
        let mut must_quote: bool = false; // §543
        if ((self.str_ptr).wrapping_add(3i32) > max_strings) {
            self.overflow(260i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        {
            if ((self.pool_ptr).wrapping_add(6i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        if (self.area_delimiter != 0i32) {
            {
                must_quote = false;
                s = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                t = (self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.area_delimiter);
                j = s;
                while ((!must_quote) && (j < t)) {
                    {
                        must_quote = (self.str_pool[crate::ix::U((j) as usize)] == 32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
                if must_quote {
                    {
                        {
                            let __for_end_6 = t;
                            j = (self.pool_ptr).wrapping_sub(1i32);
                            while j >= __for_end_6 {
                                { let __v401 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U(((j).wrapping_add(2i32)) as usize)] = __v401; }
                                j = j.wrapping_sub(1);
                            }
                        }
                        self.str_pool[crate::ix::U(((t).wrapping_add(1i32)) as usize)] = 34i32;
                        {
                            let __for_end_6 = s;
                            j = (t).wrapping_sub(1i32);
                            while j >= __for_end_6 {
                                { let __v402 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] = __v402; }
                                j = j.wrapping_sub(1);
                            }
                        }
                        self.str_pool[crate::ix::U((s) as usize)] = 34i32;
                        if (self.ext_delimiter != 0i32) {
                            self.ext_delimiter = (self.ext_delimiter).wrapping_add(2i32);
                        }
                        self.area_delimiter = (self.area_delimiter).wrapping_add(2i32);
                        self.pool_ptr = (self.pool_ptr).wrapping_add(2i32);
                    }
                }
            }
        }
        s = (self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.area_delimiter);
        if (self.ext_delimiter == 0i32) {
            t = self.pool_ptr;
        } else {
            t = ((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.ext_delimiter)).wrapping_sub(1i32);
        }
        must_quote = false;
        j = s;
        while ((!must_quote) && (j < t)) {
            {
                must_quote = (self.str_pool[crate::ix::U((j) as usize)] == 32i32);
                j = (j).wrapping_add(1i32);
            }
        }
        if must_quote {
            {
                {
                    let __for_end_4 = t;
                    j = (self.pool_ptr).wrapping_sub(1i32);
                    while j >= __for_end_4 {
                        { let __v403 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U(((j).wrapping_add(2i32)) as usize)] = __v403; }
                        j = j.wrapping_sub(1);
                    }
                }
                self.str_pool[crate::ix::U(((t).wrapping_add(1i32)) as usize)] = 34i32;
                {
                    let __for_end_4 = s;
                    j = (t).wrapping_sub(1i32);
                    while j >= __for_end_4 {
                        { let __v404 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] = __v404; }
                        j = j.wrapping_sub(1);
                    }
                }
                self.str_pool[crate::ix::U((s) as usize)] = 34i32;
                if (self.ext_delimiter != 0i32) {
                    self.ext_delimiter = (self.ext_delimiter).wrapping_add(2i32);
                }
                self.pool_ptr = (self.pool_ptr).wrapping_add(2i32);
            }
        }
        if (self.ext_delimiter != 0i32) {
            {
                s = ((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.ext_delimiter)).wrapping_sub(1i32);
                t = self.pool_ptr;
                must_quote = false;
                j = s;
                while ((!must_quote) && (j < t)) {
                    {
                        must_quote = (self.str_pool[crate::ix::U((j) as usize)] == 32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
                if must_quote {
                    {
                        self.str_pool[crate::ix::U(((t).wrapping_add(1i32)) as usize)] = 34i32;
                        {
                            let __for_end_6 = s;
                            j = (t).wrapping_sub(1i32);
                            while j >= __for_end_6 {
                                { let __v405 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] = __v405; }
                                j = j.wrapping_sub(1);
                            }
                        }
                        self.str_pool[crate::ix::U((s) as usize)] = 34i32;
                        self.pool_ptr = (self.pool_ptr).wrapping_add(2i32);
                    }
                }
            }
        }
        if (self.area_delimiter == 0i32) {
            self.cur_area = 348i32;
        } else {
            {
                self.cur_area = self.str_ptr;
                { let __ix406 = (self.str_ptr).wrapping_add(1i32); let __v407 = (self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.area_delimiter); self.str_start[crate::ix::U((__ix406) as usize)] = __v407; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
            }
        }
        if (self.ext_delimiter == 0i32) {
            {
                self.cur_ext = 348i32;
                self.cur_name = self.make_string();
            }
        } else {
            {
                self.cur_name = self.str_ptr;
                { let __ix408 = (self.str_ptr).wrapping_add(1i32); let __v409 = (((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(self.ext_delimiter)).wrapping_sub(self.area_delimiter)).wrapping_sub(1i32); self.str_start[crate::ix::U((__ix408) as usize)] = __v409; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                self.cur_ext = self.make_string();
            }
        }
    }

    /// Another system-dependent routine is needed to convert three internal
    /// \TeX\ strings
    /// into the `name_of_file` value that is used to open files. The present code
    /// allows both lowercase and uppercase letters in the file name.
    // §545
    pub fn pack_file_name(&mut self, mut n: str_number, mut a: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §545
        let mut c: ASCII_code = 0; // §545
        let mut j: pool_pointer = 0; // §545
        k = 0i32;
        {
            let __for_end_2 = (self.str_start[crate::ix::U(((a).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U((a) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v410 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v410; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[crate::ix::U(((n).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U((n) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v411 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v411; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[crate::ix::U(((e).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U((e) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v412 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v412; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        if (k <= file_name_size) {
            self.name_length = k;
        } else {
            self.name_length = file_name_size;
        }
        {
            let __for_end_2 = file_name_size;
            k = (self.name_length).wrapping_add(1i32);
            while k <= __for_end_2 {
                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = b' ';
                k = k.wrapping_add(1);
            }
        }
    }

    /// Here is the messy routine that was just mentioned. It sets `name_of_file`
    /// from the first `n` characters of `TEX_format_default`, followed by
    /// `buffer[a..b]`, followed by the last `format_ext_length` characters of
    /// `TEX_format_default`.
    /// We dare not give error messages here, since \TeX\ calls this routine before
    /// the `error` routine is ready to roll. Instead, we simply drop excess characters,
    /// since the error will be detected in another way when a strange file name
    /// isn't found.
    // §549
    pub fn pack_buffered_name(&mut self, mut n: small_number, mut a: i32, mut b: i32) {
        let mut k: i32 = 0; // §549
        let mut c: ASCII_code = 0; // §549
        let mut j: i32 = 0; // §549
        if ((((n).wrapping_add(b)).wrapping_sub(a)).wrapping_add(5i32) > file_name_size) {
            b = (((a).wrapping_add(file_name_size)).wrapping_sub(n)).wrapping_sub(5i32);
        }
        k = 0i32;
        {
            let __for_end_2 = n;
            j = 1i32;
            while j <= __for_end_2 {
                {
                    c = self.xord[crate::ix::U((self.TEX_format_default[crate::ix::U(((j) - 1) as usize)]) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v413 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v413; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = b;
            j = a;
            while j <= __for_end_2 {
                {
                    c = self.buffer[crate::ix::U((j) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v414 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v414; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = format_default_length;
            j = 17i32;
            while j <= __for_end_2 {
                {
                    c = self.xord[crate::ix::U((self.TEX_format_default[crate::ix::U(((j) - 1) as usize)]) as usize)];
                    if (!(c == 34i32)) {
                        {
                            k = (k).wrapping_add(1i32);
                            if (k <= file_name_size) {
                                { let __v415 = self.xchr[crate::ix::U((c) as usize)]; self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v415; }
                            }
                        }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        if (k <= file_name_size) {
            self.name_length = k;
        } else {
            self.name_length = file_name_size;
        }
        {
            let __for_end_2 = file_name_size;
            k = (self.name_length).wrapping_add(1i32);
            while k <= __for_end_2 {
                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = b' ';
                k = k.wrapping_add(1);
            }
        }
    }

    /// Operating systems often make it possible to determine the exact name (and
    /// possible version number) of a file that has been opened. The following routine,
    /// which simply makes a \TeX\ string from the value of `name_of_file`, should
    /// ideally be changed to deduce the full name of file~`f`, which is the file
    /// most recently opened, if it is possible to do this in a \PASCAL\ program.
    /// This routine might be called after string memory has overflowed, hence
    /// we dare not use ``str_room`'.
    // §551
    pub fn make_name_string(&mut self) -> str_number {
        let mut make_name_string: str_number = 0;
        let mut k: i32 = 0; // §551
        let mut save_area_delimiter: pool_pointer = 0; // §551
        let mut save_ext_delimiter: pool_pointer = 0; // §551
        let mut save_name_in_progress: bool = false; // §551
        let mut save_stop_at_space: bool = false; // §551
        if ((((self.pool_ptr).wrapping_add(self.name_length) > pool_size) || (self.str_ptr == max_strings)) || ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) > 0i32)) {
            make_name_string = 63i32;
        } else {
            {
                {
                    let __for_end_4 = self.name_length;
                    k = 1i32;
                    while k <= __for_end_4 {
                        {
                            { let __ix416 = self.pool_ptr; let __v417 = self.xord[crate::ix::U((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as usize)]; self.str_pool[crate::ix::U((__ix416) as usize)] = __v417; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                        k = k.wrapping_add(1);
                    }
                }
                make_name_string = self.make_string();
                save_area_delimiter = self.area_delimiter;
                save_ext_delimiter = self.ext_delimiter;
                save_name_in_progress = self.name_in_progress;
                save_stop_at_space = self.stop_at_space;
                self.name_in_progress = true;
                self.begin_name();
                self.stop_at_space = false;
                k = 1i32;
                while ((k <= self.name_length) && self.more_name(((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32))) {
                    k = (k).wrapping_add(1i32);
                }
                self.stop_at_space = save_stop_at_space;
                self.end_name();
                self.name_in_progress = save_name_in_progress;
                self.area_delimiter = save_area_delimiter;
                self.ext_delimiter = save_ext_delimiter;
            }
        }
        make_name_string
    }

    /// Now let's consider the ``driver''
    /// routines by which \TeX\ deals with file names
    /// in a system-independent manner.  First comes a procedure that looks for a
    /// file name in the input by calling `get_x_token` for the information.
    // §552
    pub fn scan_file_name(&mut self) {
        let mut save_warning_index: halfword = 0; // §552
        'l_done_f: {
            save_warning_index = self.warning_index;
            self.warning_index = self.cur_cs;
            // §430
            loop {
                self.get_x_token();
                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
            }
            // §552
            self.back_input();
            if (self.cur_cmd == left_brace) {
                self.scan_file_name_braced();
            } else {
                {
                    self.name_in_progress = true;
                    self.begin_name();
                    // §432
                    loop {
                        self.get_x_token();
                        if (self.cur_cmd != spacer) { break; }
                    }
                    // §552
                    while true {
                        {
                            if ((self.cur_cmd > other_char) || (self.cur_chr > 255i32)) {
                                {
                                    self.back_input();
                                    break 'l_done_f;
                                }
                            }
                            if (((self.cur_chr == 32i32) && (self.cur_input.state_field != token_list)) && (self.cur_input.loc_field > self.cur_input.limit_field)) {
                                break 'l_done_f;
                            }
                            if (!self.more_name(self.cur_chr)) {
                                break 'l_done_f;
                            }
                            self.get_x_token();
                        }
                    }
                }
            }
        }
        self.end_name();
        self.name_in_progress = false;
        self.warning_index = save_warning_index;
    }

    /// Here is a routine that manufactures the output file names, assuming that
    /// `job_name<>0`. It ignores and changes the current settings of `cur_area`
    /// and `cur_ext`.
    // §555
    pub fn pack_job_name(&mut self, mut s: str_number) {
        self.cur_area = 348i32;
        self.cur_ext = s;
        self.cur_name = self.job_name;
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// If some trouble arises when \TeX\ tries to open a file, the following
    /// routine calls upon the user to supply another file name. Parameter~`s`
    /// is used in the error message to identify the type of file; parameter~`e`
    /// is the default extension if none is given. Upon exit from the routine,
    /// variables `cur_name`, `cur_area`, `cur_ext`, and `name_of_file` are
    /// ready for another attempt at file opening.
    // §556
    pub fn prompt_file_name(&mut self, mut s: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §556
        if (self.interaction == scroll_mode) {
        }
        if (s == 943i32) {
            {
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(264i32);
                }
                self.print(944i32);
            }
        } else {
            {
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(264i32);
                }
                self.print(945i32);
            }
        }
        self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
        self.print(946i32);
        if (e == 947i32) {
            self.show_context();
        }
        self.print_nl(948i32);
        self.print(s);
        if (self.interaction < scroll_mode) {
            self.fatal_error(949i32);
        }
        crate::system::break_in(&mut self.term_in, true);
        {
            self.print(650i32);
            self.term_input();
        }
        // §557
        {
            'l_done_f: {
                self.begin_name();
                k = self.first;
                while ((self.buffer[crate::ix::U((k) as usize)] == 32i32) && (k < self.last)) {
                    k = (k).wrapping_add(1i32);
                }
                while true {
                    {
                        if (k == self.last) {
                            break 'l_done_f;
                        }
                        if (!self.more_name(self.buffer[crate::ix::U((k) as usize)])) {
                            break 'l_done_f;
                        }
                        k = (k).wrapping_add(1i32);
                    }
                }
            }
            self.end_name();
        }
        // §556
        if (self.cur_ext == 348i32) {
            self.cur_ext = e;
        }
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// The `open_log_file` routine is used to open the transcript file and to help
    /// it catch up to what has previously been printed on the terminal.
    // §560
    pub fn open_log_file(&mut self) {
        let mut old_setting: i32 = 0; // §560
        let mut k: i32 = 0; // §560
        let mut l: i32 = 0; // §560
        let mut months: [u8; 36] = [0u8; 36]; // §560
        old_setting = self.selector;
        if (self.job_name == 0i32) {
            self.job_name = self.get_job_name(952i32);
        }
        self.pack_job_name(953i32);
        self.recorder_change_filename();
        self.pack_job_name(954i32);
        while (!{ let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_open_out(&mut __f0); self.log_file = __f0; __r }) {
            // §561
            {
                self.selector = term_only;
                self.prompt_file_name(956i32, 954i32);
            }
        }
        // §560
        self.log_name = { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_make_name_string(&mut __f0); self.log_file = __f0; __r };
        self.selector = log_only;
        self.log_opened = true;
        // §562
        {
            {
                crate::system::wr_str(&mut self.log_file, "This is pdfTeX, Version 3.141592653");
                crate::system::wr_str(&mut self.log_file, "-2.6");
                crate::system::wr_str(&mut self.log_file, "-1.40.29");
            }
            self.wlog_version_string();
            self.slow_print(self.format_ident);
            self.print(957i32);
            self.print_int(((self.sys_day) as i64));
            self.print_char(32i32);
            crate::system::copy_str(&mut months, "JANFEBMARAPRMAYJUNJULAUGSEPOCTNOVDEC");
            {
                let __for_end_3 = (3i32).wrapping_mul(self.sys_month);
                k = ((3i32).wrapping_mul(self.sys_month)).wrapping_sub(2i32);
                while k <= __for_end_3 {
                    {
                        let __w0 = months[crate::ix::U(((k) - 1) as usize)];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_char(32i32);
            self.print_int(((self.sys_year) as i64));
            self.print_char(32i32);
            self.print_two((self.sys_time / 60i32));
            self.print_char(58i32);
            self.print_two((self.sys_time % 60i32));
            if (self.eTeX_mode == 1i32) {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, "entering extended mode");
                    }
                }
            }
            if self.shellenabledp {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    if self.restrictedshell {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "restricted ");
                            }
                        }
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, "\\write18 enabled.");
                    }
                }
            }
            if self.file_line_error_style_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " file:line:error style messages enabled.");
                    }
                }
            }
            if self.parse_first_line_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " %&-line parsing enabled.");
                    }
                }
            }
            if self.translate_filename_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " (");
                    }
                    self.wlog_translate_filename();
                    {
                        let __w0 = b')';
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                }
            }
        }
        // §560
        { let __ix418 = self.input_ptr; let __v419 = self.cur_input; self.input_stack[crate::ix::U((__ix418) as usize)] = __v419; }
        self.print_nl(955i32);
        l = self.input_stack[crate::ix::U((0i32) as usize)].limit_field;
        if (self.buffer[crate::ix::U((l) as usize)] == self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int()) {
            l = (l).wrapping_sub(1i32);
        }
        {
            let __for_end_2 = l;
            k = 1i32;
            while k <= __for_end_2 {
                self.print(self.buffer[crate::ix::U((k) as usize)]);
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.selector = (old_setting).wrapping_add(2i32);
    }

    /// Let's turn now to the procedure that is used to initiate file reading
    /// when an `\.{\\input}' command is being processed.
    /// Beware: For historic reasons, this code foolishly conserves a tiny bit
    /// of string pool space; but that can confuse the interactive `\.E' option.
    // §563
    pub fn start_input(&mut self) {
        let mut temp_str: str_number = 0; // §563
        let mut v: halfword = 0; // §563
        'l_done_f: {
            self.scan_file_name();
            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
            while true {
                {
                    self.begin_file_reading();
                    self.set_tex_input_type(true);
                    if (self.kpse_in_name_ok() && { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)]); let __r = self.a_open_in(&mut __f0); self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)] = __f0; __r }) {
                        break 'l_done_f;
                    }
                    self.end_file_reading();
                    self.prompt_file_name(943i32, 348i32);
                }
            }
        }
        self.cur_input.name_field = { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)]); let __r = self.a_make_name_string(&mut __f0); self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)] = __f0; __r };
        { let __ix420 = self.in_open; let __v421 = self.cur_input.name_field; self.full_source_filename_stack[crate::ix::U((__ix420) as usize)] = __v421; }
        if (self.job_name == 0i32) {
            {
                self.job_name = self.get_job_name(self.cur_name);
                self.open_log_file();
            }
        }
        if ((self.term_offset).wrapping_add((self.str_start[crate::ix::U(((self.full_source_filename_stack[crate::ix::U((self.in_open) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.full_source_filename_stack[crate::ix::U((self.in_open) as usize)]) as usize)])) > (self.max_print_line).wrapping_sub(2i32)) {
            self.print_ln();
        } else {
            if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                self.print_char(32i32);
            }
        }
        self.print_char(40i32);
        self.open_parens = (self.open_parens).wrapping_add(1i32);
        self.slow_print(self.full_source_filename_stack[crate::ix::U((self.in_open) as usize)]);
        crate::system::break_out(&mut self.term_out);
        if (self.eqtb[crate::ix::U(((629076i32) - 1) as usize)].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_ln();
                self.print_char(126i32);
                v = (self.input_ptr).wrapping_sub(1i32);
                if (v < self.eqtb[crate::ix::U(((629076i32) - 1) as usize)].int()) {
                    while (v > 0i32) {
                        {
                            self.print_char(46i32);
                            v = (v).wrapping_sub(1i32);
                        }
                    }
                } else {
                    self.print_char(126i32);
                }
                self.slow_print(958i32);
                self.slow_print(self.cur_name);
                self.slow_print(self.cur_ext);
                self.print_ln();
                self.end_diagnostic(false);
            }
        }
        self.cur_input.state_field = new_line;
        // §564
        {
            self.line = 1i32;
            if { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)]); let __r = self.input_ln(&mut __f0, false); self.input_file[crate::ix::U(((self.cur_input.index_field) - 1) as usize)] = __f0; __r } {
            }
            self.firm_up_the_line();
            if ((self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int() > 255i32)) {
                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
            } else {
                { let __ix422 = self.cur_input.limit_field; let __v423 = self.eqtb[crate::ix::U(((629066i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix422) as usize)] = __v423; }
            }
            self.first = (self.cur_input.limit_field).wrapping_add(1i32);
            self.cur_input.loc_field = self.cur_input.start_field;
        }
    }

    /// \TeX\ checks the information of a \.{TFM} file for validity as the
    /// file is being read in, so that no further checks will be needed when
    /// typesetting is going on. The somewhat tedious subroutine that does this
    /// is called `read_font_info`. It has four parameters: the user font
    /// identifier~`u`, the file name and area strings `nom` and `aire`, and the
    /// ``at'' size~`s`. If `s`~is negative, it's the negative of a scale factor
    /// to be applied to the design size; `s=-1000` is the normal case.
    /// Otherwise `s` will be substituted for the design size; in this
    /// case, `s` must be positive and less than $2048\rm\,pt$
    /// (i.e., it must be less than $2^{27}$ when considered as an integer).
    /// The subroutine opens and closes a global file variable called `tfm_file`.
    /// It returns the value of the internal font number that was just loaded.
    /// If an error is detected, an error message is issued and no font
    /// information is stored; `null_font` is returned in this case.
    /// ...
    // §586
    pub fn read_font_info(&mut self, mut u: halfword, mut nom: str_number, mut aire: str_number, mut s: scaled) -> internal_font_number {
        let mut read_font_info: internal_font_number = 0;
        let mut k: font_index = 0; // §586
        let mut file_opened: bool = false; // §586
        let mut lf: halfword = 0; // §586
        let mut lh: halfword = 0; // §586
        let mut bc: halfword = 0; // §586
        let mut ec: halfword = 0; // §586
        let mut nw: halfword = 0; // §586
        let mut nh: halfword = 0; // §586
        let mut nd: halfword = 0; // §586
        let mut ni: halfword = 0; // §586
        let mut nl: halfword = 0; // §586
        let mut nk: halfword = 0; // §586
        let mut ne: halfword = 0; // §586
        let mut np: halfword = 0; // §586
        let mut f: internal_font_number = 0; // §586
        let mut g: internal_font_number = 0; // §586
        let mut a: eight_bits = 0; // §586
        let mut b: eight_bits = 0; // §586
        let mut c: eight_bits = 0; // §586
        let mut d: eight_bits = 0; // §586
        let mut qw: four_quarters = four_quarters::default(); // §586
        let mut sw: scaled = 0; // §586
        let mut bch_label: i32 = 0; // §586
        let mut bchar: i32 = 0; // §586
        let mut z: scaled = 0; // §586
        let mut alpha: i32 = 0; // §586
        let mut beta: i32 = 0; // §586
        'l_done_f: {
            'l_L11_f: {
                g = null_font;
                // §589
                file_opened = false;
                if (aire == 348i32) {
                    self.pack_file_name(nom, TEX_font_area, 969i32);
                } else {
                    self.pack_file_name(nom, aire, 969i32);
                }
                if (!{ let mut __f0 = ::core::mem::take(&mut self.tfm_file); let __r = self.b_open_in(&mut __f0); self.tfm_file = __f0; __r }) {
                    break 'l_L11_f;
                }
                file_opened = true;
                // §591
                {
                    {
                        lf = self.tfm_file.buf;
                        if (lf > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        lf = ((lf).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        lh = self.tfm_file.buf;
                        if (lh > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        lh = ((lh).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        bc = self.tfm_file.buf;
                        if (bc > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        bc = ((bc).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        ec = self.tfm_file.buf;
                        if (ec > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        ec = ((ec).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    if ((bc > (ec).wrapping_add(1i32)) || (ec > 255i32)) {
                        break 'l_L11_f;
                    }
                    if (bc > 255i32) {
                        {
                            bc = 1i32;
                            ec = 0i32;
                        }
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        nw = self.tfm_file.buf;
                        if (nw > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        nw = ((nw).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        nh = self.tfm_file.buf;
                        if (nh > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        nh = ((nh).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        nd = self.tfm_file.buf;
                        if (nd > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        nd = ((nd).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        ni = self.tfm_file.buf;
                        if (ni > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        ni = ((ni).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        nl = self.tfm_file.buf;
                        if (nl > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        nl = ((nl).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        nk = self.tfm_file.buf;
                        if (nk > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        nk = ((nk).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        ne = self.tfm_file.buf;
                        if (ne > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        ne = ((ne).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        np = self.tfm_file.buf;
                        if (np > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        np = ((np).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    if (lf != ((((((((((6i32).wrapping_add(lh)).wrapping_add(((ec).wrapping_sub(bc)).wrapping_add(1i32))).wrapping_add(nw)).wrapping_add(nh)).wrapping_add(nd)).wrapping_add(ni)).wrapping_add(nl)).wrapping_add(nk)).wrapping_add(ne)).wrapping_add(np)) {
                        break 'l_L11_f;
                    }
                    if ((((nw == 0i32) || (nh == 0i32)) || (nd == 0i32)) || (ni == 0i32)) {
                        break 'l_L11_f;
                    }
                }
                // §592
                lf = ((lf).wrapping_sub(6i32)).wrapping_sub(lh);
                if (np < 7i32) {
                    lf = ((lf).wrapping_add(7i32)).wrapping_sub(np);
                }
                if ((self.font_ptr == font_max) || ((self.fmem_ptr).wrapping_add(lf) > font_mem_size)) {
                    // §593
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(960i32);
                        }
                        self.sprint_cs(u);
                        self.print_char(61i32);
                        self.print_file_name(nom, aire, 348i32);
                        if (s >= 0i32) {
                            {
                                self.print(895i32);
                                self.print_scaled(s);
                                self.print(314i32);
                            }
                        } else {
                            if (s != (1000i32).wrapping_neg()) {
                                {
                                    self.print(961i32);
                                    self.print_int((((s).wrapping_neg()) as i64));
                                }
                            }
                        }
                        self.print(970i32);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 971i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 972i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 973i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 974i32;
                        }
                        self.error();
                        break 'l_done_f;
                    }
                }
                // §592
                f = (self.font_ptr).wrapping_add(1i32);
                { let __v424 = (self.fmem_ptr).wrapping_sub(bc); self.char_base[crate::ix::U((f) as usize)] = __v424; }
                { let __v425 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(ec)).wrapping_add(1i32); self.width_base[crate::ix::U((f) as usize)] = __v425; }
                { let __v426 = (self.width_base[crate::ix::U((f) as usize)]).wrapping_add(nw); self.height_base[crate::ix::U((f) as usize)] = __v426; }
                { let __v427 = (self.height_base[crate::ix::U((f) as usize)]).wrapping_add(nh); self.depth_base[crate::ix::U((f) as usize)] = __v427; }
                { let __v428 = (self.depth_base[crate::ix::U((f) as usize)]).wrapping_add(nd); self.italic_base[crate::ix::U((f) as usize)] = __v428; }
                { let __v429 = (self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(ni); self.lig_kern_base[crate::ix::U((f) as usize)] = __v429; }
                { let __v430 = ((self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add(nl)).wrapping_sub((256i32).wrapping_mul(128i32)); self.kern_base[crate::ix::U((f) as usize)] = __v430; }
                { let __v431 = ((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(nk); self.exten_base[crate::ix::U((f) as usize)] = __v431; }
                { let __v432 = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_add(ne); self.param_base[crate::ix::U((f) as usize)] = __v432; }
                // §594
                {
                    if (lh < 2i32) {
                        break 'l_L11_f;
                    }
                    {
                        crate::system::get_byte(&mut self.tfm_file);
                        a = self.tfm_file.buf;
                        qw.set_b0((a).wrapping_add(0i32));
                        crate::system::get_byte(&mut self.tfm_file);
                        b = self.tfm_file.buf;
                        qw.set_b1((b).wrapping_add(0i32));
                        crate::system::get_byte(&mut self.tfm_file);
                        c = self.tfm_file.buf;
                        qw.set_b2((c).wrapping_add(0i32));
                        crate::system::get_byte(&mut self.tfm_file);
                        d = self.tfm_file.buf;
                        qw.set_b3((d).wrapping_add(0i32));
                        self.font_check[crate::ix::U((f) as usize)] = qw;
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    {
                        z = self.tfm_file.buf;
                        if (z > 127i32) {
                            break 'l_L11_f;
                        }
                        crate::system::get_byte(&mut self.tfm_file);
                        z = ((z).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    }
                    crate::system::get_byte(&mut self.tfm_file);
                    z = ((z).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                    crate::system::get_byte(&mut self.tfm_file);
                    z = ((z).wrapping_mul(16i32)).wrapping_add((self.tfm_file.buf / 16i32));
                    if (z < unity) {
                        break 'l_L11_f;
                    }
                    while (lh > 2i32) {
                        {
                            crate::system::get_byte(&mut self.tfm_file);
                            crate::system::get_byte(&mut self.tfm_file);
                            crate::system::get_byte(&mut self.tfm_file);
                            crate::system::get_byte(&mut self.tfm_file);
                            lh = (lh).wrapping_sub(1i32);
                        }
                    }
                    self.font_dsize[crate::ix::U((f) as usize)] = z;
                    if (s != (1000i32).wrapping_neg()) {
                        if (s >= 0i32) {
                            z = s;
                        } else {
                            z = self.xn_over_d(z, (s).wrapping_neg(), 1000i32);
                        }
                    }
                    self.font_size[crate::ix::U((f) as usize)] = z;
                }
                // §595
                {
                    let __for_end_4 = (self.width_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                    k = self.fmem_ptr;
                    while k <= __for_end_4 {
                        {
                            {
                                crate::system::get_byte(&mut self.tfm_file);
                                a = self.tfm_file.buf;
                                qw.set_b0((a).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                b = self.tfm_file.buf;
                                qw.set_b1((b).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                c = self.tfm_file.buf;
                                qw.set_b2((c).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                d = self.tfm_file.buf;
                                qw.set_b3((d).wrapping_add(0i32));
                                self.font_info[crate::ix::U((k) as usize)].set_qqqq(qw);
                            }
                            if ((((a >= nw) || ((b / 16i32) >= nh)) || ((b % 16i32) >= nd)) || ((c / 4i32) >= ni)) {
                                break 'l_L11_f;
                            }
                            match (c % 4i32) {
                                lig_tag => {
                                    if (d >= nl) {
                                        break 'l_L11_f;
                                    }
                                }
                                ext_tag => {
                                    if (d >= ne) {
                                        break 'l_L11_f;
                                    }
                                }
                                list_tag => {
                                    // §596
                                    {
                                        'l_not_found_f: {
                                            {
                                                if ((d < bc) || (d > ec)) {
                                                    break 'l_L11_f;
                                                }
                                            }
                                            while (d < ((k).wrapping_add(bc)).wrapping_sub(self.fmem_ptr)) {
                                                {
                                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(d)) as usize)].qqqq();
                                                    if (((qw.b2()).wrapping_sub(0i32) % 4i32) != list_tag) {
                                                        break 'l_not_found_f;
                                                    }
                                                    d = (qw.b3()).wrapping_sub(0i32);
                                                }
                                            }
                                            if (d == ((k).wrapping_add(bc)).wrapping_sub(self.fmem_ptr)) {
                                                break 'l_L11_f;
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // §595
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §598
                {
                    // §599
                    {
                        alpha = 16i32;
                        if (z >= 134217728i32) {
                            self.pdf_error(594i32, 975i32);
                        }
                        while (z >= 8388608i32) {
                            {
                                z = (z / 2i32);
                                alpha = (alpha).wrapping_add(alpha);
                            }
                        }
                        beta = (256i32 / alpha);
                        alpha = (alpha).wrapping_mul(z);
                    }
                    // §598
                    {
                        let __for_end_5 = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                        k = self.width_base[crate::ix::U((f) as usize)];
                        while k <= __for_end_5 {
                            {
                                crate::system::get_byte(&mut self.tfm_file);
                                a = self.tfm_file.buf;
                                crate::system::get_byte(&mut self.tfm_file);
                                b = self.tfm_file.buf;
                                crate::system::get_byte(&mut self.tfm_file);
                                c = self.tfm_file.buf;
                                crate::system::get_byte(&mut self.tfm_file);
                                d = self.tfm_file.buf;
                                sw = ((((((d).wrapping_mul(z) / 256i32)).wrapping_add((c).wrapping_mul(z)) / 256i32)).wrapping_add((b).wrapping_mul(z)) / beta);
                                if (a == 0i32) {
                                    self.font_info[crate::ix::U((k) as usize)].set_int(sw);
                                } else {
                                    if (a == 255i32) {
                                        self.font_info[crate::ix::U((k) as usize)].set_int((sw).wrapping_sub(alpha));
                                    } else {
                                        break 'l_L11_f;
                                    }
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    if (self.font_info[crate::ix::U((self.width_base[crate::ix::U((f) as usize)]) as usize)].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[crate::ix::U((self.height_base[crate::ix::U((f) as usize)]) as usize)].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[crate::ix::U((self.depth_base[crate::ix::U((f) as usize)]) as usize)].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[crate::ix::U((self.italic_base[crate::ix::U((f) as usize)]) as usize)].int() != 0i32) {
                        break 'l_L11_f;
                    }
                }
                // §600
                bch_label = 32767i32;
                bchar = 256i32;
                if (nl > 0i32) {
                    {
                        {
                            let __for_end_6 = ((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_sub(1i32);
                            k = self.lig_kern_base[crate::ix::U((f) as usize)];
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_byte(&mut self.tfm_file);
                                        a = self.tfm_file.buf;
                                        qw.set_b0((a).wrapping_add(0i32));
                                        crate::system::get_byte(&mut self.tfm_file);
                                        b = self.tfm_file.buf;
                                        qw.set_b1((b).wrapping_add(0i32));
                                        crate::system::get_byte(&mut self.tfm_file);
                                        c = self.tfm_file.buf;
                                        qw.set_b2((c).wrapping_add(0i32));
                                        crate::system::get_byte(&mut self.tfm_file);
                                        d = self.tfm_file.buf;
                                        qw.set_b3((d).wrapping_add(0i32));
                                        self.font_info[crate::ix::U((k) as usize)].set_qqqq(qw);
                                    }
                                    if (a > 128i32) {
                                        {
                                            if (((256i32).wrapping_mul(c)).wrapping_add(d) >= nl) {
                                                break 'l_L11_f;
                                            }
                                            if (a == 255i32) {
                                                if (k == self.lig_kern_base[crate::ix::U((f) as usize)]) {
                                                    bchar = b;
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            if (b != bchar) {
                                                {
                                                    {
                                                        if ((b < bc) || (b > ec)) {
                                                            break 'l_L11_f;
                                                        }
                                                    }
                                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(b)) as usize)].qqqq();
                                                    if (!(qw.b0() > min_quarterword)) {
                                                        break 'l_L11_f;
                                                    }
                                                }
                                            }
                                            if (c < 128i32) {
                                                {
                                                    {
                                                        if ((d < bc) || (d > ec)) {
                                                            break 'l_L11_f;
                                                        }
                                                    }
                                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(d)) as usize)].qqqq();
                                                    if (!(qw.b0() > min_quarterword)) {
                                                        break 'l_L11_f;
                                                    }
                                                }
                                            } else {
                                                if (((256i32).wrapping_mul((c).wrapping_sub(128i32))).wrapping_add(d) >= nk) {
                                                    break 'l_L11_f;
                                                }
                                            }
                                            if (a < 128i32) {
                                                if ((((k).wrapping_sub(self.lig_kern_base[crate::ix::U((f) as usize)])).wrapping_add(a)).wrapping_add(1i32) >= nl) {
                                                    break 'l_L11_f;
                                                }
                                            }
                                        }
                                    }
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        if (a == 255i32) {
                            bch_label = ((256i32).wrapping_mul(c)).wrapping_add(d);
                        }
                    }
                }
                {
                    let __for_end_4 = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                    k = (self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32));
                    while k <= __for_end_4 {
                        {
                            crate::system::get_byte(&mut self.tfm_file);
                            a = self.tfm_file.buf;
                            crate::system::get_byte(&mut self.tfm_file);
                            b = self.tfm_file.buf;
                            crate::system::get_byte(&mut self.tfm_file);
                            c = self.tfm_file.buf;
                            crate::system::get_byte(&mut self.tfm_file);
                            d = self.tfm_file.buf;
                            sw = ((((((d).wrapping_mul(z) / 256i32)).wrapping_add((c).wrapping_mul(z)) / 256i32)).wrapping_add((b).wrapping_mul(z)) / beta);
                            if (a == 0i32) {
                                self.font_info[crate::ix::U((k) as usize)].set_int(sw);
                            } else {
                                if (a == 255i32) {
                                    self.font_info[crate::ix::U((k) as usize)].set_int((sw).wrapping_sub(alpha));
                                } else {
                                    break 'l_L11_f;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §601
                {
                    let __for_end_4 = (self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                    k = self.exten_base[crate::ix::U((f) as usize)];
                    while k <= __for_end_4 {
                        {
                            {
                                crate::system::get_byte(&mut self.tfm_file);
                                a = self.tfm_file.buf;
                                qw.set_b0((a).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                b = self.tfm_file.buf;
                                qw.set_b1((b).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                c = self.tfm_file.buf;
                                qw.set_b2((c).wrapping_add(0i32));
                                crate::system::get_byte(&mut self.tfm_file);
                                d = self.tfm_file.buf;
                                qw.set_b3((d).wrapping_add(0i32));
                                self.font_info[crate::ix::U((k) as usize)].set_qqqq(qw);
                            }
                            if (a != 0i32) {
                                {
                                    {
                                        if ((a < bc) || (a > ec)) {
                                            break 'l_L11_f;
                                        }
                                    }
                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(a)) as usize)].qqqq();
                                    if (!(qw.b0() > min_quarterword)) {
                                        break 'l_L11_f;
                                    }
                                }
                            }
                            if (b != 0i32) {
                                {
                                    {
                                        if ((b < bc) || (b > ec)) {
                                            break 'l_L11_f;
                                        }
                                    }
                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(b)) as usize)].qqqq();
                                    if (!(qw.b0() > min_quarterword)) {
                                        break 'l_L11_f;
                                    }
                                }
                            }
                            if (c != 0i32) {
                                {
                                    {
                                        if ((c < bc) || (c > ec)) {
                                            break 'l_L11_f;
                                        }
                                    }
                                    qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq();
                                    if (!(qw.b0() > min_quarterword)) {
                                        break 'l_L11_f;
                                    }
                                }
                            }
                            {
                                {
                                    if ((d < bc) || (d > ec)) {
                                        break 'l_L11_f;
                                    }
                                }
                                qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(d)) as usize)].qqqq();
                                if (!(qw.b0() > min_quarterword)) {
                                    break 'l_L11_f;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §602
                {
                    {
                        let __for_end_5 = np;
                        k = 1i32;
                        while k <= __for_end_5 {
                            if (k == 1i32) {
                                {
                                    crate::system::get_byte(&mut self.tfm_file);
                                    sw = self.tfm_file.buf;
                                    if (sw > 127i32) {
                                        sw = (sw).wrapping_sub(256i32);
                                    }
                                    crate::system::get_byte(&mut self.tfm_file);
                                    sw = ((sw).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                                    crate::system::get_byte(&mut self.tfm_file);
                                    sw = ((sw).wrapping_mul(256i32)).wrapping_add(self.tfm_file.buf);
                                    crate::system::get_byte(&mut self.tfm_file);
                                    { let __ix433 = self.param_base[crate::ix::U((f) as usize)]; let __v434 = ((sw).wrapping_mul(16i32)).wrapping_add((self.tfm_file.buf / 16i32)); self.font_info[crate::ix::U((__ix433) as usize)].set_int(__v434); }
                                }
                            } else {
                                {
                                    crate::system::get_byte(&mut self.tfm_file);
                                    a = self.tfm_file.buf;
                                    crate::system::get_byte(&mut self.tfm_file);
                                    b = self.tfm_file.buf;
                                    crate::system::get_byte(&mut self.tfm_file);
                                    c = self.tfm_file.buf;
                                    crate::system::get_byte(&mut self.tfm_file);
                                    d = self.tfm_file.buf;
                                    sw = ((((((d).wrapping_mul(z) / 256i32)).wrapping_add((c).wrapping_mul(z)) / 256i32)).wrapping_add((b).wrapping_mul(z)) / beta);
                                    if (a == 0i32) {
                                        { let __ix435 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix435) as usize)].set_int(sw); }
                                    } else {
                                        if (a == 255i32) {
                                            { let __ix436 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix436) as usize)].set_int((sw).wrapping_sub(alpha)); }
                                        } else {
                                            break 'l_L11_f;
                                        }
                                    }
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    if crate::system::eof(&self.tfm_file) {
                        break 'l_L11_f;
                    }
                    {
                        let __for_end_5 = 7i32;
                        k = (np).wrapping_add(1i32);
                        while k <= __for_end_5 {
                            { let __ix437 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix437) as usize)].set_int(0i32); }
                            k = k.wrapping_add(1);
                        }
                    }
                }
                // §603
                if (np >= 7i32) {
                    self.font_params[crate::ix::U((f) as usize)] = np;
                } else {
                    self.font_params[crate::ix::U((f) as usize)] = 7i32;
                }
                { let __v438 = self.eqtb[crate::ix::U(((629064i32) - 1) as usize)].int(); self.hyphen_char[crate::ix::U((f) as usize)] = __v438; }
                { let __v439 = self.eqtb[crate::ix::U(((629065i32) - 1) as usize)].int(); self.skew_char[crate::ix::U((f) as usize)] = __v439; }
                if (bch_label < nl) {
                    { let __v440 = (bch_label).wrapping_add(self.lig_kern_base[crate::ix::U((f) as usize)]); self.bchar_label[crate::ix::U((f) as usize)] = __v440; }
                } else {
                    self.bchar_label[crate::ix::U((f) as usize)] = non_address;
                }
                self.font_bchar[crate::ix::U((f) as usize)] = (bchar).wrapping_add(0i32);
                self.font_false_bchar[crate::ix::U((f) as usize)] = (bchar).wrapping_add(0i32);
                if (bchar <= ec) {
                    if (bchar >= bc) {
                        {
                            qw = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(bchar)) as usize)].qqqq();
                            if (qw.b0() > min_quarterword) {
                                self.font_false_bchar[crate::ix::U((f) as usize)] = non_char;
                            }
                        }
                    }
                }
                self.font_name[crate::ix::U((f) as usize)] = nom;
                self.font_area[crate::ix::U((f) as usize)] = aire;
                self.font_bc[crate::ix::U((f) as usize)] = bc;
                self.font_ec[crate::ix::U((f) as usize)] = ec;
                self.font_glue[crate::ix::U((f) as usize)] = null;
                { let __v441 = (self.char_base[crate::ix::U((f) as usize)]).wrapping_sub(0i32); self.char_base[crate::ix::U((f) as usize)] = __v441; }
                { let __v442 = (self.width_base[crate::ix::U((f) as usize)]).wrapping_sub(0i32); self.width_base[crate::ix::U((f) as usize)] = __v442; }
                { let __v443 = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_sub(0i32); self.lig_kern_base[crate::ix::U((f) as usize)] = __v443; }
                { let __v444 = (self.kern_base[crate::ix::U((f) as usize)]).wrapping_sub(0i32); self.kern_base[crate::ix::U((f) as usize)] = __v444; }
                { let __v445 = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_sub(0i32); self.exten_base[crate::ix::U((f) as usize)] = __v445; }
                { let __v446 = (self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32); self.param_base[crate::ix::U((f) as usize)] = __v446; }
                self.fmem_ptr = (self.fmem_ptr).wrapping_add(lf);
                self.font_ptr = f;
                g = f;
                break 'l_done_f;
            }
            // §586
            {
                // §587
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(264i32);
                }
                self.print(960i32);
            }
            self.sprint_cs(u);
            self.print_char(61i32);
            self.print_file_name(nom, aire, 348i32);
            if (s >= 0i32) {
                {
                    self.print(895i32);
                    self.print_scaled(s);
                    self.print(314i32);
                }
            } else {
                if (s != (1000i32).wrapping_neg()) {
                    {
                        self.print(961i32);
                        self.print_int((((s).wrapping_neg()) as i64));
                    }
                }
            }
            if file_opened {
                self.print(962i32);
            } else {
                self.print(963i32);
            }
            {
                self.help_ptr = 5i32;
                self.help_line[crate::ix::U((4i32) as usize)] = 964i32;
                self.help_line[crate::ix::U((3i32) as usize)] = 965i32;
                self.help_line[crate::ix::U((2i32) as usize)] = 966i32;
                self.help_line[crate::ix::U((1i32) as usize)] = 967i32;
                self.help_line[crate::ix::U((0i32) as usize)] = 968i32;
            }
            self.error();
        }
        // §586
        if file_opened {
            { let mut __f0 = ::core::mem::take(&mut self.tfm_file); let __r = self.b_close(&mut __f0); self.tfm_file = __f0; __r };
        }
        read_font_info = g;
        read_font_info
    }

    /// A `fix_word` whose four bytes are $(a,b,c,d)$ from left to right represents
    /// the number
    /// $$x=\left\{\vcenter{\halign{$#$,\hfil\qquad&if $#$\hfil\cr
    /// b\cdot2^{-4}+c\cdot2^{-12}+d\cdot2^{-20}&a=0;\cr
    /// -16+b\cdot2^{-4}+c\cdot2^{-12}+d\cdot2^{-20}&a=255.\cr}}\right.$$
    /// (No other choices of `a` are allowed, since the magnitude of a number in
    /// design-size units must be less than 16.)  We want to multiply this
    /// quantity by the integer~`z`, which is known to be less than $2^{27}$.
    /// If $`z`<2^{23}$, the individual multiplications $b\cdot z$,
    /// $c\cdot z$, $d\cdot z$ cannot overflow; otherwise we will divide `z` by 2,
    /// 4, 8, or 16, to obtain a multiplier less than $2^{23}$, and we can
    /// compensate for this later. If `z` has thereby been replaced by
    /// $`z`^\prime=`z`/2^e$, let $\beta=2^{4-e}$; we shall compute
    /// $$\lfloor(b+c\cdot2^{-8}+d\cdot2^{-16})\,z^\prime/\beta\rfloor$$
    /// ...
    // §597
    pub fn store_scaled_f(&mut self, mut sq: scaled, mut z: scaled) -> scaled {
        let mut store_scaled_f: scaled = 0;
        let mut a: eight_bits = 0; // §597
        let mut b: eight_bits = 0; // §597
        let mut c: eight_bits = 0; // §597
        let mut d: eight_bits = 0; // §597
        let mut sw: scaled = 0; // §597
        let mut alpha: i32 = 0; // §597
        let mut beta: i32 = 0; // §597
        alpha = 16i32;
        if (z >= 134217728i32) {
            self.pdf_error(594i32, 975i32);
        }
        while (z >= 8388608i32) {
            {
                z = (z / 2i32);
                alpha = (alpha).wrapping_add(alpha);
            }
        }
        beta = (256i32 / alpha);
        alpha = (alpha).wrapping_mul(z);
        if (sq >= 0i32) {
            {
                d = (sq % 256i32);
                sq = (sq / 256i32);
                c = (sq % 256i32);
                sq = (sq / 256i32);
                b = (sq % 256i32);
                sq = (sq / 256i32);
                a = (sq % 256i32);
            }
        } else {
            {
                sq = ((sq).wrapping_add(1073741824i32)).wrapping_add(1073741824i32);
                d = (sq % 256i32);
                sq = (sq / 256i32);
                c = (sq % 256i32);
                sq = (sq / 256i32);
                b = (sq % 256i32);
                sq = (sq / 256i32);
                a = ((sq).wrapping_add(128i32) % 256i32);
            }
        }
        sw = ((((((d).wrapping_mul(z) / 256i32)).wrapping_add((c).wrapping_mul(z)) / 256i32)).wrapping_add((b).wrapping_mul(z)) / beta);
        if (a == 0i32) {
            store_scaled_f = sw;
        } else {
            if (a == 255i32) {
                store_scaled_f = (sw).wrapping_sub(alpha);
            } else {
                self.pdf_error(976i32, 977i32);
            }
        }
        store_scaled_f
    }

    /// When \TeX\ wants to typeset a character that doesn't exist, the
    /// character node is not created; thus the output routine can assume
    /// that characters exist when it sees them. The following procedure
    /// prints a warning message unless the user has suppressed it.
    // §608
    pub fn char_warning(&mut self, mut f: internal_font_number, mut c: eight_bits) {
        let mut old_setting: i32 = 0; // §608
        if (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() > 0i32) {
            {
                old_setting = self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].int();
                if ((self.eTeX_mode == 1i32) && (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() > 1i32)) {
                    self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].set_int(1i32);
                }
                if (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() > 2i32) {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(264i32);
                        }
                        self.print(986i32);
                    }
                } else {
                    {
                        self.begin_diagnostic();
                        self.print_nl(986i32);
                    }
                }
                self.print(c);
                if (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() > 2i32) {
                    {
                        self.print(288i32);
                        self.print_hex(c);
                        self.print(41i32);
                    }
                }
                self.print(987i32);
                self.slow_print(self.font_name[crate::ix::U((f) as usize)]);
                if (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() < 3i32) {
                    self.print_char(33i32);
                }
                self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].set_int(old_setting);
                if (self.eqtb[crate::ix::U(((629053i32) - 1) as usize)].int() > 2i32) {
                    {
                        self.help_ptr = 0i32;
                        self.error();
                    }
                } else {
                    self.end_diagnostic(false);
                }
            }
        }
    }

    /// Here is a function that returns a pointer to a character node for a
    /// given character in a given font. If that character doesn't exist,
    /// `null` is returned instead.
    // §609
    pub fn new_character(&mut self, mut f: internal_font_number, mut c: eight_bits) -> halfword {
        let mut new_character: halfword = 0;
        let mut p: halfword = 0; // §609
        'l_exit_f: {
            if (self.font_bc[crate::ix::U((f) as usize)] <= c) {
                if (self.font_ec[crate::ix::U((f) as usize)] >= c) {
                    if (self.font_info[crate::ix::U((((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)).wrapping_add(0i32)) as usize)].qqqq().b0() > min_quarterword) {
                        {
                            p = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_b0(f);
                            self.mem[crate::ix::U((p) as usize)].set_hh_b1((c).wrapping_add(0i32));
                            new_character = p;
                            break 'l_exit_f;
                        }
                    }
                }
            }
            self.char_warning(f, c);
            new_character = null;
        }
        new_character
    }

    /// The actual output of `dvi_buf[a..b]` to `dvi_file` is performed by calling
    /// `write_dvi(a,b)`. For best results, this procedure should be optimized to
    /// run as fast as possible on each particular system, since it is part of
    /// \TeX's inner loop. It is safe to assume that `a` and `b+1` will both be
    /// multiples of 4 when `write_dvi(a,b)` is called; therefore it is possible on
    /// many machines to use efficient methods to pack four bytes per word and to
    /// output an array of words with one system call.
    // §624
    pub fn write_dvi(&mut self, mut a: dvi_index, mut b: dvi_index) {
        let mut k: dvi_index = 0; // §624
        {
            let __for_end_2 = b;
            k = a;
            while k <= __for_end_2 {
                {
                    let __w = self.dvi_buf[crate::ix::U((k) as usize)];
                    crate::system::write_byte(&mut self.dvi_file, __w);
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// To put a byte in the buffer without paying the cost of invoking a procedure
    /// each time, we use the macro `dvi_out`.
    // §625
    pub fn dvi_swap(&mut self) {
        if (self.dvi_limit == dvi_buf_size) {
            {
                self.write_dvi(0i32, (self.half_buf).wrapping_sub(1i32));
                self.dvi_limit = self.half_buf;
                self.dvi_offset = (self.dvi_offset).wrapping_add(dvi_buf_size);
                self.dvi_ptr = 0i32;
            }
        } else {
            {
                self.write_dvi(self.half_buf, (dvi_buf_size).wrapping_sub(1i32));
                self.dvi_limit = dvi_buf_size;
            }
        }
        self.dvi_gone = (self.dvi_gone).wrapping_add(self.half_buf);
    }

    /// The `dvi_four` procedure outputs four bytes in two's complement notation,
    /// without risking arithmetic overflow.
    // §627
    pub fn dvi_four(&mut self, mut x: i32) {
        if (x >= 0i32) {
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (x / 16777216i32);
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        } else {
            {
                x = (x).wrapping_add(1073741824i32);
                x = (x).wrapping_add(1073741824i32);
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = ((x / 16777216i32)).wrapping_add(128i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        }
        x = (x % 16777216i32);
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (x / 65536i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        x = (x % 65536i32);
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (x / 256i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (x % 256i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
    }

    /// A mild optimization of the output is performed by the `dvi_pop`
    /// routine, which issues a `pop` unless it is possible to cancel a
    /// ``push` `pop`' pair. The parameter to `dvi_pop` is the byte address
    /// following the old `push` that matches the new `pop`.
    // §628
    pub fn dvi_pop(&mut self, mut l: i32) {
        if ((l == (self.dvi_offset).wrapping_add(self.dvi_ptr)) && (self.dvi_ptr > 0i32)) {
            self.dvi_ptr = (self.dvi_ptr).wrapping_sub(1i32);
        } else {
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = pop;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
    }

    /// Here's a procedure that outputs a font definition. Since \TeX82 uses at
    /// most 256 different fonts per job, `fnt_def1` is always used as the command code.
    // §629
    pub fn dvi_font_def(&mut self, mut f: internal_font_number) {
        let mut k: pool_pointer = 0; // §629
        if (f <= 256i32) {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = fnt_def1;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (f).wrapping_sub(1i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        } else {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = 244i32;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = ((f).wrapping_sub(1i32) / 256i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = ((f).wrapping_sub(1i32) % 256i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        }
        {
            { let __ix447 = self.dvi_ptr; let __v448 = (self.font_check[crate::ix::U((f) as usize)].b0()).wrapping_sub(0i32); self.dvi_buf[crate::ix::U((__ix447) as usize)] = __v448; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix449 = self.dvi_ptr; let __v450 = (self.font_check[crate::ix::U((f) as usize)].b1()).wrapping_sub(0i32); self.dvi_buf[crate::ix::U((__ix449) as usize)] = __v450; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix451 = self.dvi_ptr; let __v452 = (self.font_check[crate::ix::U((f) as usize)].b2()).wrapping_sub(0i32); self.dvi_buf[crate::ix::U((__ix451) as usize)] = __v452; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix453 = self.dvi_ptr; let __v454 = (self.font_check[crate::ix::U((f) as usize)].b3()).wrapping_sub(0i32); self.dvi_buf[crate::ix::U((__ix453) as usize)] = __v454; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        self.dvi_four(self.font_size[crate::ix::U((f) as usize)]);
        self.dvi_four(self.font_dsize[crate::ix::U((f) as usize)]);
        {
            { let __ix455 = self.dvi_ptr; let __v456 = (self.str_start[crate::ix::U(((self.font_area[crate::ix::U((f) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.font_area[crate::ix::U((f) as usize)]) as usize)]); self.dvi_buf[crate::ix::U((__ix455) as usize)] = __v456; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            { let __ix457 = self.dvi_ptr; let __v458 = (self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U((self.font_name[crate::ix::U((f) as usize)]) as usize)]); self.dvi_buf[crate::ix::U((__ix457) as usize)] = __v458; }
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        // §630
        {
            let __for_end_2 = (self.str_start[crate::ix::U(((self.font_area[crate::ix::U((f) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            k = self.str_start[crate::ix::U((self.font_area[crate::ix::U((f) as usize)]) as usize)];
            while k <= __for_end_2 {
                {
                    { let __ix459 = self.dvi_ptr; let __v460 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix459) as usize)] = __v460; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_add(1i32)) as usize)]).wrapping_sub(1i32);
            k = self.str_start[crate::ix::U((self.font_name[crate::ix::U((f) as usize)]) as usize)];
            while k <= __for_end_2 {
                {
                    { let __ix461 = self.dvi_ptr; let __v462 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix461) as usize)] = __v462; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Here is a subroutine that produces a \.{DVI} command for some specified
    /// downward or rightward motion. It has two parameters: `w` is the amount
    /// of motion, and `o` is either `down1` or `right1`. We use the fact that
    /// the command codes have convenient arithmetic properties: `y1-down1=w1-right1`
    /// and `z1-down1=x1-right1`.
    // §634
    pub fn movement(&mut self, mut w: scaled, mut o: eight_bits) {
        let mut mstate: small_number = 0; // §634
        let mut p: halfword = 0; // §634
        let mut q: halfword = 0; // §634
        let mut k: i32 = 0; // §634
        'l_exit_f: {
            'l_found_f: {
                'l_start_of_TEX_f: {
                    'l_L2_f: {
                        'l_not_found_f: {
                            q = self.get_node(movement_node_size);
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(w);
                            { let __v463 = (self.dvi_offset).wrapping_add(self.dvi_ptr); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v463); }
                            if (o == down1) {
                                {
                                    { let __v464 = self.down_ptr; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v464); }
                                    self.down_ptr = q;
                                }
                            } else {
                                {
                                    { let __v465 = self.right_ptr; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v465); }
                                    self.right_ptr = q;
                                }
                            }
                            // §638
                            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            mstate = none_seen;
                            while (p != null) {
                                {
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == w) {
                                        // §639
                                        match (mstate).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().lh()) {
                                            3 | 4 | 15 | 16 => {
                                                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §640
                                                    {
                                                        k = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v466 = (self.dvi_buf[crate::ix::U((k) as usize)]).wrapping_add(5i32); self.dvi_buf[crate::ix::U((k) as usize)] = __v466; }
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(y_here);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            5 | 9 | 11 => {
                                                // §639
                                                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §641
                                                    {
                                                        k = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v467 = (self.dvi_buf[crate::ix::U((k) as usize)]).wrapping_add(10i32); self.dvi_buf[crate::ix::U((k) as usize)] = __v467; }
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(z_here);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            1 | 2 | 8 | 13 => {
                                                // §639
                                                break 'l_found_f;
                                            }
                                            _ => {
                                            }
                                        }
                                    } else {
                                        // §638
                                        match (mstate).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().lh()) {
                                            1 => {
                                                mstate = y_seen;
                                            }
                                            2 => {
                                                mstate = z_seen;
                                            }
                                            8 | 13 => {
                                                break 'l_not_found_f;
                                            }
                                            _ => {
                                            }
                                        }
                                    }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                        }
                        // §637
                        self.mem[crate::ix::U((q) as usize)].set_hh_lh(yz_OK);
                        if ((w).wrapping_abs() >= 8388608i32) {
                            {
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (o).wrapping_add(3i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                self.dvi_four(w);
                                break 'l_exit_f;
                            }
                        }
                        if ((w).wrapping_abs() >= 32768i32) {
                            {
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (o).wrapping_add(2i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                if (w < 0i32) {
                                    w = (w).wrapping_add(16777216i32);
                                }
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (w / 65536i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                w = (w % 65536i32);
                                break 'l_L2_f;
                            }
                        }
                        if ((w).wrapping_abs() >= 128i32) {
                            {
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (o).wrapping_add(1i32);
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                                if (w < 0i32) {
                                    w = (w).wrapping_add(65536i32);
                                }
                                break 'l_L2_f;
                            }
                        }
                        {
                            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = o;
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        if (w < 0i32) {
                            w = (w).wrapping_add(256i32);
                        }
                        break 'l_start_of_TEX_f;
                    }
                    {
                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (w / 256i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                }
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (w % 256i32);
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                break 'l_exit_f;
            }
            // §634
            { let __v468 = self.mem[crate::ix::U((p) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v468); }
            // §636
            if (self.mem[crate::ix::U((q) as usize)].hh().lh() == y_here) {
                {
                    {
                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (o).wrapping_add(4i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                        {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            match self.mem[crate::ix::U((q) as usize)].hh().lh() {
                                yz_OK => {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(z_OK);
                                }
                                y_OK => {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(d_fixed);
                                }
                                _ => {
                                }
                            }
                        }
                    }
                }
            } else {
                {
                    {
                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (o).wrapping_add(9i32);
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                        {
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            match self.mem[crate::ix::U((q) as usize)].hh().lh() {
                                yz_OK => {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(y_OK);
                                }
                                z_OK => {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(d_fixed);
                                }
                                _ => {
                                }
                            }
                        }
                    }
                }
            }
        }
        // §634
    }

    /// In case you are wondering when all the movement nodes are removed from
    /// \TeX's memory, the answer is that they are recycled just before
    /// `hlist_out` and `vlist_out` finish outputting a box. This restores the
    /// down and right stacks to the state they were in before the box was output,
    /// except that some `info`'s may have become more restrictive.
    // §642
    pub fn prune_movements(&mut self, mut l: i32) {
        let mut p: halfword = 0; // §642
        'l_exit_f: {
            'l_done_f: {
                while (self.down_ptr != null) {
                    {
                        if (self.mem[crate::ix::U(((self.down_ptr).wrapping_add(2i32)) as usize)].int() < l) {
                            break 'l_done_f;
                        }
                        p = self.down_ptr;
                        self.down_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        self.free_node(p, movement_node_size);
                    }
                }
            }
            while (self.right_ptr != null) {
                {
                    if (self.mem[crate::ix::U(((self.right_ptr).wrapping_add(2i32)) as usize)].int() < l) {
                        break 'l_exit_f;
                    }
                    p = self.right_ptr;
                    self.right_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.free_node(p, movement_node_size);
                }
            }
        }
    }

    /// After all this preliminary shuffling, we come finally to the routines
    /// that actually send out the requested data. Let's do \.{\\special} first
    /// (it's easier).
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1615
    pub fn special_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1615
        let mut k: pool_pointer = 0; // §1615
        let mut h: halfword = 0; // §1615
        let mut q: halfword = 0; // §1615
        let mut r: halfword = 0; // §1615
        let mut old_mode: i32 = 0; // §1615
        if (self.cur_h != self.dvi_h) {
            {
                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                self.dvi_h = self.cur_h;
            }
        }
        if (self.cur_v != self.dvi_v) {
            {
                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                self.dvi_v = self.cur_v;
            }
        }
        old_setting = self.selector;
        self.selector = new_string;
        self.selector = old_setting;
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
            {
                // §1618
                q = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(637i32);
                r = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                self.mem[crate::ix::U((r) as usize)].set_hh_lh(end_write_token);
                self.begin_token_list(q, inserted);
                self.begin_token_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), write_text);
                q = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(379i32);
                self.begin_token_list(q, inserted);
                old_mode = self.cur_list.mode_field;
                self.cur_list.mode_field = 0i32;
                self.cur_cs = self.write_loc;
                q = self.scan_toks(false, true);
                self.cur_list.mode_field = old_mode;
                self.get_token();
                if (self.cur_tok != end_write_token) {
                    // §1619
                    {
                        {
                            if (self.interaction == error_stop_mode) {
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
                            self.help_line[crate::ix::U((1i32) as usize)] = 1912i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1425i32;
                        }
                        self.error();
                        loop {
                            self.get_token();
                            if (self.cur_tok == end_write_token) { break; }
                        }
                    }
                }
                // §1618
                self.end_token_list();
                // §1615
                h = self.def_ref;
            }
        } else {
            h = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
        }
        self.selector = new_string;
        self.show_token_list(self.mem[crate::ix::U((h) as usize)].hh().rh(), null, (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]) < 256i32) {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = xxx1;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix469 = self.dvi_ptr; let __v470 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]); self.dvi_buf[crate::ix::U((__ix469) as usize)] = __v470; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
            }
        } else {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = xxx4;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                self.dvi_four((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]));
            }
        }
        {
            let __for_end_2 = (self.pool_ptr).wrapping_sub(1i32);
            k = self.str_start[crate::ix::U((self.str_ptr) as usize)];
            while k <= __for_end_2 {
                {
                    { let __ix471 = self.dvi_ptr; let __v472 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix471) as usize)] = __v472; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
            self.flush_list(self.def_ref);
        }
    }

    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1617
    pub fn write_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1617
        let mut old_mode: i32 = 0; // §1617
        let mut j: small_number = 0; // §1617
        let mut q: halfword = 0; // §1617
        let mut r: halfword = 0; // §1617
        let mut d: i32 = 0; // §1617
        let mut clobbered: bool = false; // §1617
        let mut runsystem_ret: i32 = 0; // §1617
        // §1618
        q = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_lh(637i32);
        r = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
        self.mem[crate::ix::U((r) as usize)].set_hh_lh(end_write_token);
        self.begin_token_list(q, inserted);
        self.begin_token_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), write_text);
        q = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_lh(379i32);
        self.begin_token_list(q, inserted);
        old_mode = self.cur_list.mode_field;
        self.cur_list.mode_field = 0i32;
        self.cur_cs = self.write_loc;
        q = self.scan_toks(false, true);
        self.cur_list.mode_field = old_mode;
        self.get_token();
        if (self.cur_tok != end_write_token) {
            // §1619
            {
                {
                    if (self.interaction == error_stop_mode) {
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
                    self.help_line[crate::ix::U((1i32) as usize)] = 1912i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 1425i32;
                }
                self.error();
                loop {
                    self.get_token();
                    if (self.cur_tok == end_write_token) { break; }
                }
            }
        }
        // §1618
        self.end_token_list();
        // §1617
        old_setting = self.selector;
        j = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
        if (j == 18i32) {
            self.selector = new_string;
        } else {
            if self.write_open[crate::ix::U((j) as usize)] {
                self.selector = j;
            } else {
                {
                    if ((j == 17i32) && (self.selector == term_and_log)) {
                        self.selector = log_only;
                    }
                    self.print_nl(348i32);
                }
            }
        }
        self.token_show(self.def_ref);
        self.print_ln();
        self.flush_list(self.def_ref);
        if (j == 18i32) {
            {
                if (self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].int() <= 0i32) {
                    self.selector = log_only;
                } else {
                    self.selector = term_and_log;
                }
                if (!self.log_opened) {
                    self.selector = term_only;
                }
                self.print_nl(1904i32);
                {
                    let __for_end_4 = ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)])).wrapping_sub(1i32);
                    d = 0i32;
                    while d <= __for_end_4 {
                        {
                            self.print(self.str_pool[crate::ix::U(((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(d)) as usize)]);
                        }
                        d = d.wrapping_add(1);
                    }
                }
                self.print(1905i32);
                if self.shellenabledp {
                    {
                        {
                            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                self.overflow(259i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                            }
                        }
                        {
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 0i32;
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                        clobbered = false;
                        {
                            let __for_end_6 = ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)])).wrapping_sub(1i32);
                            d = 0i32;
                            while d <= __for_end_6 {
                                {
                                    { let __ix473 = (self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(d); let __v474 = ((self.xchr[crate::ix::U((self.str_pool[crate::ix::U(((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(d)) as usize)]) as usize)]) as i32); self.str_pool[crate::ix::U((__ix473) as usize)] = __v474; }
                                    if ((self.str_pool[crate::ix::U(((self.str_start[crate::ix::U((self.str_ptr) as usize)]).wrapping_add(d)) as usize)] == null_code) && (d < ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)])).wrapping_sub(1i32))) {
                                        clobbered = true;
                                    }
                                }
                                d = d.wrapping_add(1);
                            }
                        }
                        if clobbered {
                            self.print(1906i32);
                        } else {
                            {
                                runsystem_ret = self.runsystem(self.str_start[crate::ix::U((self.str_ptr) as usize)], ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)])).wrapping_sub(1i32));
                                if (runsystem_ret == (1i32).wrapping_neg()) {
                                    self.print(1907i32);
                                } else {
                                    if (runsystem_ret == 0i32) {
                                        self.print(1908i32);
                                    } else {
                                        if (runsystem_ret == 1i32) {
                                            self.print(1909i32);
                                        } else {
                                            if (runsystem_ret == 2i32) {
                                                self.print(1910i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    {
                        self.print(1911i32);
                    }
                }
                self.print_char(46i32);
                self.print_nl(348i32);
                self.print_ln();
                self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
            }
        }
        self.selector = old_setting;
    }

    /// The `out_what` procedure takes care of outputting whatsit nodes for
    /// `vlist_out` and `hlist_out`\kern-.3pt.
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1620
    pub fn out_what(&mut self, mut p: halfword) {
        let mut j: small_number = 0; // §1620
        let mut old_setting: i32 = 0; // §1620
        match self.mem[crate::ix::U((p) as usize)].hh().b1() {
            open_node | write_node | close_node => {
                // §1622
                if (!self.doing_leaders) {
                    {
                        j = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == write_node) {
                            self.write_out(p);
                        } else {
                            {
                                if self.write_open[crate::ix::U((j) as usize)] {
                                    {
                                        { let mut __f0 = ::core::mem::take(&mut self.write_file[crate::ix::U((j) as usize)]); let __r = self.a_close(&mut __f0); self.write_file[crate::ix::U((j) as usize)] = __f0; __r };
                                        { let __v475 = false; self.write_open[crate::ix::U((j) as usize)] = __v475; }
                                    }
                                }
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == close_node) {
                                } else {
                                    if (j < 16i32) {
                                        {
                                            self.cur_name = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                            self.cur_area = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh();
                                            self.cur_ext = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh();
                                            if (self.cur_ext == 348i32) {
                                                self.cur_ext = 947i32;
                                            }
                                            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                                            while ((!self.kpse_out_name_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.write_file[crate::ix::U((j) as usize)]); let __r = self.a_open_out(&mut __f0); self.write_file[crate::ix::U((j) as usize)] = __f0; __r })) {
                                                self.prompt_file_name(1914i32, 947i32);
                                            }
                                            { let __v476 = true; self.write_open[crate::ix::U((j) as usize)] = __v476; }
                                            if (self.log_opened && self.texmf_yesno_log_openout()) {
                                                {
                                                    old_setting = self.selector;
                                                    if (self.eqtb[crate::ix::U(((629047i32) - 1) as usize)].int() <= 0i32) {
                                                        self.selector = log_only;
                                                    } else {
                                                        self.selector = term_and_log;
                                                    }
                                                    self.print_nl(1915i32);
                                                    self.print_int(((j) as i64));
                                                    self.print(1916i32);
                                                    self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
                                                    self.print(946i32);
                                                    self.print_nl(348i32);
                                                    self.print_ln();
                                                    self.selector = old_setting;
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
            special_node | latespecial_node => {
                // §1620
                self.special_out(p);
            }
            language_node => {
            }
            pdf_save_pos_node => {
                // §1621
                {
                    self.pdf_last_x_pos = (self.cur_h).wrapping_add(4736286i32);
                    self.pdf_last_y_pos = ((self.cur_page_height).wrapping_sub(self.cur_v)).wrapping_sub(4736286i32);
                }
            }
            _ => {
                // §1620
                {
                    if ((pdftex_first_extension_code <= self.mem[crate::ix::U((p) as usize)].hh().b1()) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= pdftex_last_extension_code)) {
                        self.pdf_error(1845i32, 1913i32);
                    } else {
                        self.confusion(1845i32);
                    }
                }
            }
        }
    }

    // §1719
    pub fn new_edge(&mut self, mut s: small_number, mut w: scaled) -> halfword {
        let mut new_edge: halfword = 0;
        let mut p: halfword = 0; // §1719
        p = self.get_node(edge_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(edge_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(w);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        new_edge = p;
        new_edge
    }

    /// The `reverse` function defined here is responsible to reverse the
    /// nodes of an hlist (segment). The first parameter `this_box` is the enclosing
    /// hlist node, the second parameter `t` is to become the tail of the reversed
    /// list, and the global variable `temp_ptr` is the head of the list to be
    /// reversed. Finally `cur_g` and `cur_glue` are the current glue rounding state
    /// variables, to be updated by this function. We remove nodes from the original
    /// list and add them to the head of the new one.
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1723
    pub fn reverse(&mut self, mut this_box: halfword, mut t: halfword, cur_g: &mut scaled, cur_glue: &mut f64) -> halfword {
        let mut reverse: halfword = 0;
        let mut l: halfword = 0; // §1723
        let mut p: halfword = 0; // §1723
        let mut q: halfword = 0; // §1723
        let mut g_order: glue_ord = 0; // §1723
        let mut g_sign: i32 = 0; // §1723
        let mut glue_temp: f64 = 0.0; // §1723
        let mut m: halfword = 0; // §1723
        let mut n: halfword = 0; // §1723
        'l_done_f: {
            g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
            g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
            l = t;
            p = self.temp_ptr;
            m = min_halfword;
            n = min_halfword;
            while true {
                {
                    while (p != null) {
                        'l_reswitch_b: loop {
                            // §1724
                            if (p >= self.hi_mem_min) {
                                loop {
                                    self.f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    self.c = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    self.cur_h = (self.cur_h).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0())) as usize)].int());
                                    q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                                    l = p;
                                    p = q;
                                    if (!(p >= self.hi_mem_min)) { break; }
                                }
                            } else {
                                // §1725
                                {
                                    'l_L15_f: {
                                        q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                            hlist_node | vlist_node | rule_node | kern_node => {
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                            }
                                            glue_node => {
                                                // §1726
                                                {
                                                    self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                    self.rule_wd = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub((*cur_g));
                                                    if (g_sign != normal) {
                                                        {
                                                            if (g_sign == stretching) {
                                                                {
                                                                    if (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                        {
                                                                            (*cur_glue) = ((*cur_glue) + ((self.mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64));
                                                                            glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * (*cur_glue));
                                                                            if (glue_temp > 1000000000.0f64) {
                                                                                glue_temp = 1000000000.0f64;
                                                                            } else {
                                                                                if (glue_temp < (-1000000000.0f64)) {
                                                                                    glue_temp = (-1000000000.0f64);
                                                                                }
                                                                            }
                                                                            (*cur_g) = crate::system::pas_round(glue_temp);
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                    {
                                                                        (*cur_glue) = ((*cur_glue) - ((self.mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64));
                                                                        glue_temp = (self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * (*cur_glue));
                                                                        if (glue_temp > 1000000000.0f64) {
                                                                            glue_temp = 1000000000.0f64;
                                                                        } else {
                                                                            if (glue_temp < (-1000000000.0f64)) {
                                                                                glue_temp = (-1000000000.0f64);
                                                                            }
                                                                        }
                                                                        (*cur_g) = crate::system::pas_round(glue_temp);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.rule_wd = (self.rule_wd).wrapping_add((*cur_g));
                                                    // §1699
                                                    if (((g_sign == stretching) && (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == shrinking) && (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().rh() == null) {
                                                                    self.free_node(self.g, glue_spec_size);
                                                                } else {
                                                                    { let __ix477 = self.g; let __v478 = (self.mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix477) as usize)].set_hh_rh(__v478); }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() < a_leaders) {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    { let __v479 = self.rule_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v479); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(glue_spec_size);
                                                                    { let __ix480 = self.g; self.mem[crate::ix::U((__ix480) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix481 = self.g; self.mem[crate::ix::U((__ix481) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix482 = (self.g).wrapping_add(1i32); let __v483 = self.rule_wd; self.mem[crate::ix::U((__ix482) as usize)].set_int(__v483); }
                                                                    { let __ix484 = (self.g).wrapping_add(2i32); self.mem[crate::ix::U((__ix484) as usize)].set_int(0i32); }
                                                                    { let __ix485 = (self.g).wrapping_add(3i32); self.mem[crate::ix::U((__ix485) as usize)].set_int(0i32); }
                                                                    { let __v486 = self.g; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v486); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            ligature_node => {
                                                // §1727
                                                {
                                                    self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                                    self.temp_ptr = p;
                                                    p = self.get_avail();
                                                    { let __v487 = self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((p) as usize)] = __v487; }
                                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                    self.free_node(self.temp_ptr, small_node_size);
                                                    continue 'l_reswitch_b;
                                                }
                                            }
                                            math_node => {
                                                // §1728
                                                {
                                                    self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                    if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                        if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                            {
                                                                self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    self.temp_ptr = self.LR_ptr;
                                                                    self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix488 = self.temp_ptr; let __v489 = self.avail; self.mem[crate::ix::U((__ix488) as usize)].set_hh_rh(__v489); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                                if (n > min_halfword) {
                                                                    {
                                                                        n = (n).wrapping_sub(1i32);
                                                                        { let __v490 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v490); }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                        if (m > min_halfword) {
                                                                            m = (m).wrapping_sub(1i32);
                                                                        } else {
                                                                            // §1729
                                                                            {
                                                                                self.free_node(p, small_node_size);
                                                                                self.mem[crate::ix::U((t) as usize)].set_hh_rh(q);
                                                                                { let __v491 = self.rule_wd; self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v491); }
                                                                                { let __v492 = ((self.cur_h).wrapping_neg()).wrapping_sub(self.rule_wd); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v492); }
                                                                                break 'l_done_f;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        // §1728
                                                        {
                                                            {
                                                                self.temp_ptr = self.get_avail();
                                                                { let __ix493 = self.temp_ptr; let __v494 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix493) as usize)].set_hh_lh(__v494); }
                                                                { let __ix495 = self.temp_ptr; let __v496 = self.LR_ptr; self.mem[crate::ix::U((__ix495) as usize)].set_hh_rh(__v496); }
                                                                self.LR_ptr = self.temp_ptr;
                                                            }
                                                            if ((n > min_halfword) || ((self.mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir)) {
                                                                {
                                                                    n = (n).wrapping_add(1i32);
                                                                    { let __v497 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v497); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    m = (m).wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            edge_node => {
                                                // §1725
                                                self.confusion(2025i32);
                                            }
                                            _ => {
                                                break 'l_L15_f;
                                            }
                                        }
                                        self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                                    }
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                                        if ((self.rule_wd == 0i32) || (l == null)) {
                                            {
                                                self.free_node(p, small_node_size);
                                                p = l;
                                            }
                                        }
                                    }
                                    l = p;
                                    p = q;
                                }
                            }
                            break 'l_reswitch_b;
                        }
                    }
                    // §1723
                    if (((t == null) && (m == min_halfword)) && (n == min_halfword)) {
                        break 'l_done_f;
                    }
                    p = self.new_math(0i32, self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh());
                    self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                }
            }
        }
        reverse = l;
        reverse
    }

    /// The recursive procedures `hlist_out` and `vlist_out` each have local variables
    /// `save_h` and `save_v` to hold the values of `dvi_h` and `dvi_v` just before
    /// entering a new level of recursion.  In effect, the values of `save_h` and
    /// `save_v` on \TeX's run-time stack correspond to the values of `h` and `v`
    /// that a \.{DVI}-reading program will push onto its coordinate stack.
    // §647
    pub fn hlist_out(&mut self) {
        let mut base_line: scaled = 0; // §647
        let mut left_edge: scaled = 0; // §647
        let mut save_h: scaled = 0; // §647
        let mut save_v: scaled = 0; // §647
        let mut this_box: halfword = 0; // §647
        let mut g_order: glue_ord = 0; // §647
        let mut g_sign: i32 = 0; // §647
        let mut p: halfword = 0; // §647
        let mut save_loc: i32 = 0; // §647
        let mut leader_box: halfword = 0; // §647
        let mut leader_wd: scaled = 0; // §647
        let mut lx: scaled = 0; // §647
        let mut outer_doing_leaders: bool = false; // §647
        let mut edge: scaled = 0; // §647
        let mut prev_p: halfword = 0; // §647
        let mut glue_temp: f64 = 0.0; // §647
        let mut cur_glue: f64 = 0.0; // §647
        let mut cur_g: scaled = 0; // §647
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        if (self.cur_s > 0i32) {
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = push;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
        if (self.cur_s > self.max_push) {
            self.max_push = self.cur_s;
        }
        save_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
        base_line = self.cur_v;
        prev_p = (this_box).wrapping_add(5i32);
        // §1714
        if (self.eTeX_mode == 1i32) {
            {
                // §1710
                {
                    self.temp_ptr = self.get_avail();
                    { let __ix498 = self.temp_ptr; self.mem[crate::ix::U((__ix498) as usize)].set_hh_lh(before); }
                    { let __ix499 = self.temp_ptr; let __v500 = self.LR_ptr; self.mem[crate::ix::U((__ix499) as usize)].set_hh_rh(__v500); }
                    self.LR_ptr = self.temp_ptr;
                }
                // §1714
                if ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) == dlist) {
                    if (self.cur_dir == right_to_left) {
                        {
                            self.cur_dir = left_to_right;
                            self.cur_h = (self.cur_h).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(0i32);
                    }
                }
                if ((self.cur_dir == right_to_left) && ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) != reversed)) {
                    // §1721
                    {
                        save_h = self.cur_h;
                        self.temp_ptr = p;
                        p = self.new_kern(0i32);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        self.cur_h = 0i32;
                        { let __v501 = { let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, null, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v501); }
                        { let __v502 = (self.cur_h).wrapping_neg(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v502); }
                        self.cur_h = save_h;
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(1i32);
                    }
                }
            }
        }
        // §647
        left_edge = self.cur_h;
        while (p != null) {
            'l_reswitch_b: loop {
                // §648
                if (p >= self.hi_mem_min) {
                    {
                        if (self.cur_h != self.dvi_h) {
                            {
                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                                self.dvi_h = self.cur_h;
                            }
                        }
                        if (self.cur_v != self.dvi_v) {
                            {
                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                self.dvi_v = self.cur_v;
                            }
                        }
                        loop {
                            self.f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                            self.c = self.mem[crate::ix::U((p) as usize)].hh().b1();
                            if (self.f != self.dvi_f) {
                                // §649
                                {
                                    if (!self.font_used[crate::ix::U((self.f) as usize)]) {
                                        {
                                            self.dvi_font_def(self.f);
                                            { let __ix503 = self.f; let __v504 = true; self.font_used[crate::ix::U((__ix503) as usize)] = __v504; }
                                        }
                                    }
                                    if (self.f <= 64i32) {
                                        {
                                            { let __ix505 = self.dvi_ptr; let __v506 = (self.f).wrapping_add(170i32); self.dvi_buf[crate::ix::U((__ix505) as usize)] = __v506; }
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                    } else {
                                        if (self.f <= 256i32) {
                                            {
                                                {
                                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = fnt1;
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                                {
                                                    { let __ix507 = self.dvi_ptr; let __v508 = (self.f).wrapping_sub(1i32); self.dvi_buf[crate::ix::U((__ix507) as usize)] = __v508; }
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                            }
                                        } else {
                                            {
                                                {
                                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = 236i32;
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                                {
                                                    { let __ix509 = self.dvi_ptr; let __v510 = ((self.f).wrapping_sub(1i32) / 256i32); self.dvi_buf[crate::ix::U((__ix509) as usize)] = __v510; }
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                                {
                                                    { let __ix511 = self.dvi_ptr; let __v512 = ((self.f).wrapping_sub(1i32) % 256i32); self.dvi_buf[crate::ix::U((__ix511) as usize)] = __v512; }
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    self.dvi_f = self.f;
                                }
                            }
                            // §648
                            if (self.c >= 128i32) {
                                {
                                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set1;
                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                    if (self.dvi_ptr == self.dvi_limit) {
                                        self.dvi_swap();
                                    }
                                }
                            }
                            {
                                { let __ix513 = self.dvi_ptr; let __v514 = (self.c).wrapping_sub(0i32); self.dvi_buf[crate::ix::U((__ix513) as usize)] = __v514; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            self.cur_h = (self.cur_h).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0())) as usize)].int());
                            prev_p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            if (!(p >= self.hi_mem_min)) { break; }
                        }
                        self.dvi_h = self.cur_h;
                    }
                } else {
                    // §650
                    {
                        'l_L15_f: {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §651
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null) {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            } else {
                                                {
                                                    save_h = self.dvi_h;
                                                    save_v = self.dvi_v;
                                                    self.cur_v = (base_line).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    self.temp_ptr = p;
                                                    edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                    if (self.cur_dir == right_to_left) {
                                                        self.cur_h = edge;
                                                    }
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                                        self.vlist_out();
                                                    } else {
                                                        self.hlist_out();
                                                    }
                                                    self.dvi_h = save_h;
                                                    self.dvi_v = save_v;
                                                    self.cur_h = edge;
                                                    self.cur_v = base_line;
                                                }
                                            }
                                        }
                                        rule_node => {
                                            // §650
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1614
                                            self.out_what(p);
                                        }
                                        glue_node => {
                                            // §653
                                            {
                                                self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_wd = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != normal) {
                                                    {
                                                        if (g_sign == stretching) {
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
                                                    if (((g_sign == stretching) && (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == shrinking) && (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().rh() == null) {
                                                                    self.free_node(self.g, glue_spec_size);
                                                                } else {
                                                                    { let __ix515 = self.g; let __v516 = (self.mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix515) as usize)].set_hh_rh(__v516); }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() < a_leaders) {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    { let __v517 = self.rule_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v517); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(glue_spec_size);
                                                                    { let __ix518 = self.g; self.mem[crate::ix::U((__ix518) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix519 = self.g; self.mem[crate::ix::U((__ix519) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix520 = (self.g).wrapping_add(1i32); let __v521 = self.rule_wd; self.mem[crate::ix::U((__ix520) as usize)].set_int(__v521); }
                                                                    { let __ix522 = (self.g).wrapping_add(2i32); self.mem[crate::ix::U((__ix522) as usize)].set_int(0i32); }
                                                                    { let __ix523 = (self.g).wrapping_add(3i32); self.mem[crate::ix::U((__ix523) as usize)].set_int(0i32); }
                                                                    { let __v524 = self.g; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v524); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // §653
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    // §654
                                                    {
                                                        leader_box = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == rule_node) {
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
                                                                if (self.cur_dir == right_to_left) {
                                                                    self.cur_h = (self.cur_h).wrapping_sub(10i32);
                                                                }
                                                                edge = (self.cur_h).wrapping_add(self.rule_wd);
                                                                lx = 0i32;
                                                                // §655
                                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == a_leaders) {
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
                                                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                                                            self.cur_h = (self.cur_h).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_h = (self.cur_h).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §654
                                                                while ((self.cur_h).wrapping_add(leader_wd) <= edge) {
                                                                    // §656
                                                                    {
                                                                        self.cur_v = (base_line).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        if (self.cur_v != self.dvi_v) {
                                                                            {
                                                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                                                                self.dvi_v = self.cur_v;
                                                                            }
                                                                        }
                                                                        save_v = self.dvi_v;
                                                                        if (self.cur_h != self.dvi_h) {
                                                                            {
                                                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                                                                                self.dvi_h = self.cur_h;
                                                                            }
                                                                        }
                                                                        save_h = self.dvi_h;
                                                                        self.temp_ptr = leader_box;
                                                                        if (self.cur_dir == right_to_left) {
                                                                            self.cur_h = (self.cur_h).wrapping_add(leader_wd);
                                                                        }
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == vlist_node) {
                                                                            self.vlist_out();
                                                                        } else {
                                                                            self.hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.dvi_v = save_v;
                                                                        self.dvi_h = save_h;
                                                                        self.cur_v = base_line;
                                                                        self.cur_h = ((save_h).wrapping_add(leader_wd)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §654
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
                                                // §653
                                                break 'l_L13_f;
                                            }
                                        }
                                        margin_kern_node | kern_node => {
                                            // §650
                                            self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        math_node => {
                                            // §1716
                                            {
                                                if (self.eTeX_mode == 1i32) {
                                                    // §1717
                                                    {
                                                        if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                            if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = self.LR_ptr;
                                                                    self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix525 = self.temp_ptr; let __v526 = self.avail; self.mem[crate::ix::U((__ix525) as usize)].set_hh_rh(__v526); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() > L_code) {
                                                                        self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    self.temp_ptr = self.get_avail();
                                                                    { let __ix527 = self.temp_ptr; let __v528 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix527) as usize)].set_hh_lh(__v528); }
                                                                    { let __ix529 = self.temp_ptr; let __v530 = self.LR_ptr; self.mem[crate::ix::U((__ix529) as usize)].set_hh_rh(__v530); }
                                                                    self.LR_ptr = self.temp_ptr;
                                                                }
                                                                if ((self.mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir) {
                                                                    // §1722
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                        self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                                        self.free_node(p, small_node_size);
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        p = self.new_edge(self.cur_dir, self.rule_wd);
                                                                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                                                                        self.cur_h = ((self.cur_h).wrapping_sub(left_edge)).wrapping_add(self.rule_wd);
                                                                        { let __v531 = { let __a1 = self.new_edge((1i32).wrapping_sub(self.cur_dir), 0i32); let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, __a1, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v531); }
                                                                        { let __v532 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v532); }
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        self.cur_h = save_h;
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1717
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                    }
                                                }
                                                // §1716
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        ligature_node => {
                                            // §826
                                            {
                                                { let __v533 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((lig_trick) as usize)] = __v533; }
                                                { let __v534 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v534); }
                                                p = lig_trick;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        edge_node => {
                                            // §1720
                                            {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                left_edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                self.cur_dir = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                            }
                                        }
                                        _ => {
                                            // §650
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_ht == (1073741824i32).wrapping_neg()) {
                                    // §652
                                    self.rule_ht = self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int();
                                }
                                if (self.rule_dp == (1073741824i32).wrapping_neg()) {
                                    self.rule_dp = self.mem[crate::ix::U(((this_box).wrapping_add(2i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_h != self.dvi_h) {
                                            {
                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                                                self.dvi_h = self.cur_h;
                                            }
                                        }
                                        self.cur_v = (base_line).wrapping_add(self.rule_dp);
                                        if (self.cur_v != self.dvi_v) {
                                            {
                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                                self.dvi_v = self.cur_v;
                                            }
                                        }
                                        {
                                            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set_rule;
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                        self.dvi_four(self.rule_ht);
                                        self.dvi_four(self.rule_wd);
                                        self.cur_v = base_line;
                                        self.dvi_h = (self.dvi_h).wrapping_add(self.rule_wd);
                                    }
                                }
                            }
                            // §650
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
                    while (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != before) {
                        {
                            if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() > L_code) {
                                self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                            }
                            {
                                self.temp_ptr = self.LR_ptr;
                                self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                {
                                    { let __ix535 = self.temp_ptr; let __v536 = self.avail; self.mem[crate::ix::U((__ix535) as usize)].set_hh_rh(__v536); }
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
                            { let __ix537 = self.temp_ptr; let __v538 = self.avail; self.mem[crate::ix::U((__ix537) as usize)].set_hh_rh(__v538); }
                            self.avail = self.temp_ptr;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                    }
                }
                // §1715
                if ((self.mem[crate::ix::U((this_box) as usize)].hh().b1()).wrapping_sub(0i32) == dlist) {
                    self.cur_dir = right_to_left;
                }
            }
        }
        // §647
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `vlist_out` routine is similar to `hlist_out`, but a bit simpler.
    // §657
    pub fn vlist_out(&mut self) {
        let mut left_edge: scaled = 0; // §657
        let mut top_edge: scaled = 0; // §657
        let mut save_h: scaled = 0; // §657
        let mut save_v: scaled = 0; // §657
        let mut this_box: halfword = 0; // §657
        let mut g_order: glue_ord = 0; // §657
        let mut g_sign: i32 = 0; // §657
        let mut p: halfword = 0; // §657
        let mut save_loc: i32 = 0; // §657
        let mut leader_box: halfword = 0; // §657
        let mut leader_ht: scaled = 0; // §657
        let mut lx: scaled = 0; // §657
        let mut outer_doing_leaders: bool = false; // §657
        let mut edge: scaled = 0; // §657
        let mut glue_temp: f64 = 0.0; // §657
        let mut cur_glue: f64 = 0.0; // §657
        let mut cur_g: scaled = 0; // §657
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        self.cur_s = (self.cur_s).wrapping_add(1i32);
        if (self.cur_s > 0i32) {
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = push;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
        }
        if (self.cur_s > self.max_push) {
            self.max_push = self.cur_s;
        }
        save_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
        left_edge = self.cur_h;
        self.cur_v = (self.cur_v).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int());
        top_edge = self.cur_v;
        while (p != null) {
            // §658
            {
                'l_L15_f: {
                    if (p >= self.hi_mem_min) {
                        self.confusion(989i32);
                    } else {
                        // §659
                        {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §660
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == null) {
                                                self.cur_v = ((self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                            } else {
                                                {
                                                    self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    if (self.cur_v != self.dvi_v) {
                                                        {
                                                            self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                                            self.dvi_v = self.cur_v;
                                                        }
                                                    }
                                                    save_h = self.dvi_h;
                                                    save_v = self.dvi_v;
                                                    if (self.cur_dir == right_to_left) {
                                                        self.cur_h = (left_edge).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    } else {
                                                        self.cur_h = (left_edge).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int());
                                                    }
                                                    self.temp_ptr = p;
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                                                        self.vlist_out();
                                                    } else {
                                                        self.hlist_out();
                                                    }
                                                    self.dvi_h = save_h;
                                                    self.dvi_v = save_v;
                                                    self.cur_v = (save_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    self.cur_h = left_edge;
                                                }
                                            }
                                        }
                                        rule_node => {
                                            // §659
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1613
                                            self.out_what(p);
                                        }
                                        glue_node => {
                                            // §662
                                            {
                                                self.g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                self.rule_ht = (self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int()).wrapping_sub(cur_g);
                                                if (g_sign != normal) {
                                                    {
                                                        if (g_sign == stretching) {
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
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    // §663
                                                    {
                                                        leader_box = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == rule_node) {
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
                                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == a_leaders) {
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
                                                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == c_leaders) {
                                                                            self.cur_v = (self.cur_v).wrapping_add((self.lr / 2i32));
                                                                        } else {
                                                                            {
                                                                                lx = (self.lr / (self.lq).wrapping_add(1i32));
                                                                                self.cur_v = (self.cur_v).wrapping_add(((self.lr).wrapping_sub(((self.lq).wrapping_sub(1i32)).wrapping_mul(lx)) / 2i32));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §663
                                                                while ((self.cur_v).wrapping_add(leader_ht) <= edge) {
                                                                    // §665
                                                                    {
                                                                        if (self.cur_dir == right_to_left) {
                                                                            self.cur_h = (left_edge).wrapping_sub(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        } else {
                                                                            self.cur_h = (left_edge).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(4i32)) as usize)].int());
                                                                        }
                                                                        if (self.cur_h != self.dvi_h) {
                                                                            {
                                                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                                                                                self.dvi_h = self.cur_h;
                                                                            }
                                                                        }
                                                                        save_h = self.dvi_h;
                                                                        self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int());
                                                                        if (self.cur_v != self.dvi_v) {
                                                                            {
                                                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                                                                self.dvi_v = self.cur_v;
                                                                            }
                                                                        }
                                                                        save_v = self.dvi_v;
                                                                        self.temp_ptr = leader_box;
                                                                        outer_doing_leaders = self.doing_leaders;
                                                                        self.doing_leaders = true;
                                                                        if (self.mem[crate::ix::U((leader_box) as usize)].hh().b0() == vlist_node) {
                                                                            self.vlist_out();
                                                                        } else {
                                                                            self.hlist_out();
                                                                        }
                                                                        self.doing_leaders = outer_doing_leaders;
                                                                        self.dvi_v = save_v;
                                                                        self.dvi_h = save_h;
                                                                        self.cur_h = left_edge;
                                                                        self.cur_v = (((save_v).wrapping_sub(self.mem[crate::ix::U(((leader_box).wrapping_add(3i32)) as usize)].int())).wrapping_add(leader_ht)).wrapping_add(lx);
                                                                    }
                                                                }
                                                                // §663
                                                                self.cur_v = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §662
                                                break 'l_L13_f;
                                            }
                                        }
                                        kern_node => {
                                            // §659
                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        _ => {
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_wd == (1073741824i32).wrapping_neg()) {
                                    // §661
                                    self.rule_wd = self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                                if ((self.rule_ht > 0i32) && (self.rule_wd > 0i32)) {
                                    {
                                        if (self.cur_dir == right_to_left) {
                                            self.cur_h = (self.cur_h).wrapping_sub(self.rule_wd);
                                        }
                                        if (self.cur_h != self.dvi_h) {
                                            {
                                                self.movement((self.cur_h).wrapping_sub(self.dvi_h), right1);
                                                self.dvi_h = self.cur_h;
                                            }
                                        }
                                        if (self.cur_v != self.dvi_v) {
                                            {
                                                self.movement((self.cur_v).wrapping_sub(self.dvi_v), down1);
                                                self.dvi_v = self.cur_v;
                                            }
                                        }
                                        {
                                            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = put_rule;
                                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                            if (self.dvi_ptr == self.dvi_limit) {
                                                self.dvi_swap();
                                            }
                                        }
                                        self.dvi_four(self.rule_ht);
                                        self.dvi_four(self.rule_wd);
                                        self.cur_h = left_edge;
                                    }
                                }
                                break 'l_L15_f;
                            }
                            // §659
                            self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                        }
                    }
                }
                // §658
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        // §657
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `hlist_out` and `vlist_out` procedures are now complete, so we are
    /// ready for the `dvi_ship_out` routine that gets them started in the first place.
    // §666
    pub fn dvi_ship_out(&mut self, mut p: halfword) {
        let mut page_loc: i32 = 0; // §666
        let mut j: i32 = 0; // §666
        let mut k: i32 = 0; // §666
        let mut s: pool_pointer = 0; // §666
        let mut old_setting: i32 = 0; // §666
        'l_done_f: {
            if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_nl(348i32);
                    self.print_ln();
                    self.print(990i32);
                }
            }
            if (self.term_offset > (self.max_print_line).wrapping_sub(9i32)) {
                self.print_ln();
            } else {
                if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                    self.print_char(32i32);
                }
            }
            self.print_char(91i32);
            j = 9i32;
            while ((self.eqtb[crate::ix::U((((count_base).wrapping_add(j)) - 1) as usize)].int() == 0i32) && (j > 0i32)) {
                j = (j).wrapping_sub(1i32);
            }
            {
                let __for_end_3 = j;
                k = 0i32;
                while k <= __for_end_3 {
                    {
                        self.print_int(((self.eqtb[crate::ix::U((((count_base).wrapping_add(k)) - 1) as usize)].int()) as i64));
                        if (k < j) {
                            self.print_char(46i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            crate::system::break_out(&mut self.term_out);
            if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_char(93i32);
                    self.begin_diagnostic();
                    self.show_box(p);
                    self.end_diagnostic(true);
                }
            }
            // §669
            if ((((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > max_dimen) || (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > max_dimen)) || (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int()) > max_dimen)) || ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int()) > max_dimen)) {
                {
                    {
                        if (self.interaction == error_stop_mode) {
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
            // §645
            self.dvi_h = 0i32;
            self.dvi_v = 0i32;
            self.cur_h = self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int();
            self.dvi_f = null_font;
            // §644
            self.cur_h_offset = self.eqtb[crate::ix::U(((629658i32) - 1) as usize)].int();
            self.cur_v_offset = self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int();
            if (self.eqtb[crate::ix::U(((629663i32) - 1) as usize)].int() != 0i32) {
                self.cur_page_width = self.eqtb[crate::ix::U(((629663i32) - 1) as usize)].int();
            } else {
                self.cur_page_width = ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add((2i32).wrapping_mul(self.cur_h_offset))).wrapping_add((2i32).wrapping_mul(4736286i32));
            }
            if (self.eqtb[crate::ix::U(((629664i32) - 1) as usize)].int() != 0i32) {
                self.cur_page_height = self.eqtb[crate::ix::U(((629664i32) - 1) as usize)].int();
            } else {
                self.cur_page_height = (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add((2i32).wrapping_mul(self.cur_v_offset))).wrapping_add((2i32).wrapping_mul(4736286i32));
            }
            // §645
            if (self.output_file_name == 0i32) {
                {
                    if (self.job_name == 0i32) {
                        self.open_log_file();
                    }
                    self.pack_job_name(950i32);
                    while (!{ let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.b_open_out(&mut __f0); self.dvi_file = __f0; __r }) {
                        self.prompt_file_name(951i32, 950i32);
                    }
                    self.output_file_name = { let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.b_make_name_string(&mut __f0); self.dvi_file = __f0; __r };
                }
            }
            if (self.total_pages == 0i32) {
                {
                    {
                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = pre;
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = id_byte;
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    self.dvi_four(25400000i32);
                    self.dvi_four(473628672i32);
                    self.prepare_mag();
                    self.dvi_four(self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int());
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.print(988i32);
                    self.print_int(((self.eqtb[crate::ix::U(((629041i32) - 1) as usize)].int()) as i64));
                    self.print_char(46i32);
                    self.print_two(self.eqtb[crate::ix::U(((629040i32) - 1) as usize)].int());
                    self.print_char(46i32);
                    self.print_two(self.eqtb[crate::ix::U(((629039i32) - 1) as usize)].int());
                    self.print_char(58i32);
                    self.print_two((self.eqtb[crate::ix::U(((629038i32) - 1) as usize)].int() / 60i32));
                    self.print_two((self.eqtb[crate::ix::U(((629038i32) - 1) as usize)].int() % 60i32));
                    self.selector = old_setting;
                    {
                        { let __ix539 = self.dvi_ptr; let __v540 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U((self.str_ptr) as usize)]); self.dvi_buf[crate::ix::U((__ix539) as usize)] = __v540; }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    {
                        let __for_end_5 = (self.pool_ptr).wrapping_sub(1i32);
                        s = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                        while s <= __for_end_5 {
                            {
                                { let __ix541 = self.dvi_ptr; let __v542 = self.str_pool[crate::ix::U((s) as usize)]; self.dvi_buf[crate::ix::U((__ix541) as usize)] = __v542; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            s = s.wrapping_add(1);
                        }
                    }
                    self.pool_ptr = self.str_start[crate::ix::U((self.str_ptr) as usize)];
                }
            }
            // §668
            page_loc = (self.dvi_offset).wrapping_add(self.dvi_ptr);
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = bop;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            {
                let __for_end_3 = 9i32;
                k = 0i32;
                while k <= __for_end_3 {
                    self.dvi_four(self.eqtb[crate::ix::U((((count_base).wrapping_add(k)) - 1) as usize)].int());
                    k = k.wrapping_add(1);
                }
            }
            self.dvi_four(self.last_bop);
            self.last_bop = page_loc;
            self.cur_v = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((629659i32) - 1) as usize)].int());
            self.temp_ptr = p;
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node) {
                self.vlist_out();
            } else {
                self.hlist_out();
            }
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = eop;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            self.total_pages = (self.total_pages).wrapping_add(1i32);
            self.cur_s = (1i32).wrapping_neg();
        }
        // §666
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
                if ((self.LR_ptr != null) || (self.cur_dir != left_to_right)) {
                    self.confusion(2026i32);
                }
            }
        }
        // §666
        if (self.eqtb[crate::ix::U(((629052i32) - 1) as usize)].int() <= 0i32) {
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

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_pdf_compress_level(&mut self) -> i32 {
        let mut get_pdf_compress_level: i32 = 0;
        get_pdf_compress_level = self.eqtb[crate::ix::U(((629080i32) - 1) as usize)].int();
        get_pdf_compress_level
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_pdf_suppress_warning_dup_map(&mut self) -> i32 {
        let mut get_pdf_suppress_warning_dup_map: i32 = 0;
        get_pdf_suppress_warning_dup_map = self.eqtb[crate::ix::U(((629108i32) - 1) as usize)].int();
        get_pdf_suppress_warning_dup_map
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_pdf_suppress_warning_page_group(&mut self) -> i32 {
        let mut get_pdf_suppress_warning_page_group: i32 = 0;
        get_pdf_suppress_warning_page_group = self.eqtb[crate::ix::U(((629109i32) - 1) as usize)].int();
        get_pdf_suppress_warning_page_group
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_pdf_suppress_ptex_info(&mut self) -> i32 {
        let mut get_pdf_suppress_ptex_info: i32 = 0;
        get_pdf_suppress_ptex_info = self.eqtb[crate::ix::U(((629111i32) - 1) as usize)].int();
        get_pdf_suppress_ptex_info
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_pdf_omit_charset(&mut self) -> i32 {
        let mut get_pdf_omit_charset: i32 = 0;
        get_pdf_omit_charset = self.eqtb[crate::ix::U(((629112i32) - 1) as usize)].int();
        get_pdf_omit_charset
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_ptex_use_underscore(&mut self) -> bool {
        let mut get_ptex_use_underscore: bool = false;
        get_ptex_use_underscore = ((self.eqtb[crate::ix::U(((629115i32) - 1) as usize)].int() > 0i32) || (self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int() >= 2i32));
        get_ptex_use_underscore
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_nullfont(&mut self) -> internal_font_number {
        let mut get_nullfont: internal_font_number = 0;
        get_nullfont = null_font;
        get_nullfont
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_fontbase(&mut self) -> internal_font_number {
        let mut get_fontbase: internal_font_number = 0;
        get_fontbase = font_base;
        get_fontbase
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_nullcs(&mut self) -> halfword {
        let mut get_nullcs: halfword = 0;
        get_nullcs = null_cs;
        get_nullcs
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_nullptr(&mut self) -> halfword {
        let mut get_nullptr: halfword = 0;
        get_nullptr = null;
        get_nullptr
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_tex_int(&mut self, mut code: i32) -> i32 {
        let mut get_tex_int: i32 = 0;
        get_tex_int = self.eqtb[crate::ix::U((((int_base).wrapping_add(code)) - 1) as usize)].int();
        get_tex_int
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_tex_dimen(&mut self, mut code: i32) -> scaled {
        let mut get_tex_dimen: scaled = 0;
        get_tex_dimen = self.eqtb[crate::ix::U((((dimen_base).wrapping_add(code)) - 1) as usize)].int();
        get_tex_dimen
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_x_height(&mut self, mut f: internal_font_number) -> scaled {
        let mut get_x_height: scaled = 0;
        get_x_height = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        get_x_height
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_charwidth(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut get_charwidth: scaled = 0;
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
            get_charwidth = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int();
        } else {
            get_charwidth = 0i32;
        }
        get_charwidth
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_charheight(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut get_charheight: scaled = 0;
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
            get_charheight = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((f) as usize)]).wrapping_add(((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b1()).wrapping_sub(0i32) / 16i32))) as usize)].int();
        } else {
            get_charheight = 0i32;
        }
        get_charheight
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_chardepth(&mut self, mut f: internal_font_number, mut c: eight_bits) -> scaled {
        let mut get_chardepth: scaled = 0;
        if (((self.font_bc[crate::ix::U((f) as usize)] <= c) && (c <= self.font_ec[crate::ix::U((f) as usize)])) && (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0() > min_quarterword)) {
            get_chardepth = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((f) as usize)]).wrapping_add(((self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b1()).wrapping_sub(0i32) % 16i32))) as usize)].int();
        } else {
            get_chardepth = 0i32;
        }
        get_chardepth
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_quad(&mut self, mut f: internal_font_number) -> scaled {
        let mut get_quad: scaled = 0;
        get_quad = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        get_quad
    }

    /// The subroutines define the corresponding macros so we can use them
    /// in C.
    // §673
    pub fn get_slant(&mut self, mut f: internal_font_number) -> scaled {
        let mut get_slant: scaled = 0;
        get_slant = self.font_info[crate::ix::U(((slant_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        get_slant
    }

    /// Helper for debugging purposes:
    // §674
    pub fn short_display_n(&mut self, mut p: i32, mut m: i32) {
        let mut n: i32 = 0; // §674
        let mut i: i32 = 0; // §674
        i = 0i32;
        self.font_in_short_display = null_font;
        if (p == null) {
            return;
        }
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
                    {
                        if ((((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node)) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == explicit))) {
                            i = (i).wrapping_add(1i32);
                        }
                        if (i >= m) {
                            return;
                        }
                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node) {
                            {
                                self.print(124i32);
                                self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
                                self.print(124i32);
                                self.short_display(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                self.print(124i32);
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
                    }
                }
                // §674
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                if (p == null) {
                    return;
                }
            }
        }
        crate::system::break_out(&mut self.term_out);
    }

    /// We use `pdf_get_mem` to allocate memory in `pdf_mem`.
    // §678
    pub fn pdf_get_mem(&mut self, mut s: i32) -> i32 {
        let mut pdf_get_mem: i32 = 0;
        let mut a: i32 = 0; // §678
        if (s > (sup_pdf_mem_size).wrapping_sub(self.pdf_mem_ptr)) {
            self.overflow(1003i32, self.pdf_mem_size);
        }
        if ((self.pdf_mem_ptr).wrapping_add(s) > self.pdf_mem_size) {
            {
                a = (((0.2f64 * ((self.pdf_mem_size) as f64))) as i32);
                if ((self.pdf_mem_ptr).wrapping_add(s) > (self.pdf_mem_size).wrapping_add(a)) {
                    self.pdf_mem_size = (self.pdf_mem_ptr).wrapping_add(s);
                } else {
                    if (self.pdf_mem_size < (sup_pdf_mem_size).wrapping_sub(a)) {
                        self.pdf_mem_size = (self.pdf_mem_size).wrapping_add(a);
                    } else {
                        self.pdf_mem_size = sup_pdf_mem_size;
                    }
                }
                self.pdf_mem.resize_len(((self.pdf_mem_size) as usize) + 1);
            }
        }
        pdf_get_mem = self.pdf_mem_ptr;
        self.pdf_mem_ptr = (self.pdf_mem_ptr).wrapping_add(s);
        pdf_get_mem
    }

    // §682
    pub fn fix_int(&mut self, mut val: i32, mut min: i32, mut max: i32) -> i32 {
        let mut fix_int: i32 = 0;
        if (val < min) {
            fix_int = min;
        } else {
            if (val > max) {
                fix_int = max;
            } else {
                fix_int = val;
            }
        }
        fix_int
    }

    /// This ensures that `pdf_major_version` and `pdf_minor_version` are set
    /// to reasonable values before any bytes have been written to the generated
    /// PDF file. We also save their current values in case the user tries to
    /// change them later, along with `pdf_objcompresslevel`,
    /// `pdf_image_hicolor`, and various other parameters that must be fixed
    /// before any PDF output happens.
    /// Here also the PDF file is opened by `ensure_pdf_open` and the PDF header
    /// is written.
    // §683
    pub fn check_pdfversion(&mut self) {
        if (!self.pdf_version_written) {
            {
                self.pdf_version_written = true;
                if (self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int() < 1i32) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1005i32);
                        }
                        self.print_ln();
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1006i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1007i32;
                        }
                        self.int_error(self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int());
                        self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].set_int(1i32);
                    }
                }
                if ((self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].int() > 9i32)) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(264i32);
                            }
                            self.print(1008i32);
                        }
                        self.print_ln();
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 1009i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 1010i32;
                        }
                        self.int_error(self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].int());
                        self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].set_int(4i32);
                    }
                }
                self.fixed_pdf_major_version = self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int();
                self.fixed_pdf_minor_version = self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].int();
                self.fixed_gamma = self.fix_int(self.eqtb[crate::ix::U(((629093i32) - 1) as usize)].int(), 0i32, 1000000i32);
                self.fixed_image_gamma = self.fix_int(self.eqtb[crate::ix::U(((629094i32) - 1) as usize)].int(), 0i32, 1000000i32);
                self.fixed_image_hicolor = ((self.fix_int(self.eqtb[crate::ix::U(((629095i32) - 1) as usize)].int(), 0i32, 1i32)) != 0);
                self.fixed_image_apply_gamma = self.fix_int(self.eqtb[crate::ix::U(((629096i32) - 1) as usize)].int(), 0i32, 1i32);
                self.fixed_pdf_objcompresslevel = self.fix_int(self.eqtb[crate::ix::U(((629100i32) - 1) as usize)].int(), 0i32, 3i32);
                self.fixed_pdf_draftmode = self.fix_int(self.eqtb[crate::ix::U(((629105i32) - 1) as usize)].int(), 0i32, 1i32);
                self.fixed_inclusion_copy_font = self.fix_int(self.eqtb[crate::ix::U(((629106i32) - 1) as usize)].int(), 0i32, 1i32);
                if (((self.fixed_pdf_major_version > 1i32) || (self.fixed_pdf_minor_version >= 5i32)) && (self.fixed_pdf_objcompresslevel > 0i32)) {
                    self.pdf_os_enable = true;
                } else {
                    {
                        if (self.fixed_pdf_objcompresslevel > 0i32) {
                            {
                                self.pdf_warning(1011i32, 1012i32, true, true);
                                self.fixed_pdf_objcompresslevel = 0i32;
                            }
                        }
                        self.pdf_os_enable = false;
                    }
                }
                self.ensure_pdf_open();
                self.fix_pdfoutput();
                self.pdf_print(1013i32);
                self.pdf_print_int(((self.fixed_pdf_major_version) as i64));
                self.pdf_print(46i32);
                {
                    self.pdf_print_int(((self.fixed_pdf_minor_version) as i64));
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
                self.pdf_print(37i32);
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
                        self.pdf_buf_set(self.pdf_ptr, 208i32);
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
                        self.pdf_buf_set(self.pdf_ptr, 212i32);
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
                        self.pdf_buf_set(self.pdf_ptr, 197i32);
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
                        self.pdf_buf_set(self.pdf_ptr, 216i32);
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
                        self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
            }
        } else {
            {
                if ((self.fixed_pdf_minor_version != self.eqtb[crate::ix::U(((629089i32) - 1) as usize)].int()) || (self.fixed_pdf_major_version != self.eqtb[crate::ix::U(((629088i32) - 1) as usize)].int())) {
                    self.pdf_error(1014i32, 1015i32);
                }
            }
        }
    }

    /// Checks that we have a name for the generated PDF file and that it's open.
    // §684
    pub fn ensure_pdf_open(&mut self) {
        if (self.output_file_name != 0i32) {
            return;
        }
        if (self.job_name == 0i32) {
            self.open_log_file();
        }
        self.pack_job_name(1016i32);
        if (self.fixed_pdf_draftmode == 0i32) {
            while (!{ let mut __f0 = ::core::mem::take(&mut self.pdf_file); let __r = self.b_open_out(&mut __f0); self.pdf_file = __f0; __r }) {
                self.prompt_file_name(951i32, 1016i32);
            }
        }
        self.output_file_name = { let mut __f0 = ::core::mem::take(&mut self.pdf_file); let __r = self.b_make_name_string(&mut __f0); self.pdf_file = __f0; __r };
    }

    /// The PDF buffer is flushed by calling `pdf_flush`, which checks the
    /// variable `zip_write_state` and will compress the buffer before flushing if
    /// necessary. We call `pdf_begin_stream` to begin a stream  and `pdf_end_stream`
    /// to finish it. The stream contents will be compressed if compression is turn on.
    // §685
    pub fn pdf_flush(&mut self) {
        let mut saved_pdf_gone: longinteger = 0; // §685
        if (!self.pdf_os_mode) {
            {
                saved_pdf_gone = self.pdf_gone;
                match self.zip_write_state {
                    no_zip => {
                        if (self.pdf_ptr > 0i32) {
                            {
                                if (self.fixed_pdf_draftmode == 0i32) {
                                    self.write_pdf(0i32, (self.pdf_ptr).wrapping_sub(1i32));
                                }
                                self.pdf_gone = (self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64));
                                self.pdf_last_byte = self.pdf_buf_get((self.pdf_ptr).wrapping_sub(1i32));
                            }
                        }
                    }
                    zip_writing => {
                        if (self.fixed_pdf_draftmode == 0i32) {
                            self.write_zip(false);
                        }
                    }
                    zip_finish => {
                        {
                            if (self.fixed_pdf_draftmode == 0i32) {
                                self.write_zip(true);
                            }
                            self.zip_write_state = no_zip;
                        }
                    }
                    _ => {}
                }
                self.pdf_ptr = 0i32;
                if (saved_pdf_gone > self.pdf_gone) {
                    self.pdf_error(1017i32, 1018i32);
                }
            }
        }
    }

    /// The PDF buffer is flushed by calling `pdf_flush`, which checks the
    /// variable `zip_write_state` and will compress the buffer before flushing if
    /// necessary. We call `pdf_begin_stream` to begin a stream  and `pdf_end_stream`
    /// to finish it. The stream contents will be compressed if compression is turn on.
    // §685
    pub fn pdf_begin_stream(&mut self) {
        {
            self.pdf_print(1019i32);
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
        self.pdf_seek_write_length = true;
        self.pdf_stream_length_offset = ((self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64))).wrapping_sub(((11i32) as i64));
        self.pdf_stream_length = ((0i32) as i64);
        self.pdf_last_byte = 0i32;
        if (self.eqtb[crate::ix::U(((629080i32) - 1) as usize)].int() > 0i32) {
            {
                {
                    self.pdf_print(1020i32);
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
                    self.pdf_print(1022i32);
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
                self.pdf_flush();
                self.zip_write_state = zip_writing;
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
                    self.pdf_print(1022i32);
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
                self.pdf_save_offset = (self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64));
            }
        }
    }

    /// The PDF buffer is flushed by calling `pdf_flush`, which checks the
    /// variable `zip_write_state` and will compress the buffer before flushing if
    /// necessary. We call `pdf_begin_stream` to begin a stream  and `pdf_end_stream`
    /// to finish it. The stream contents will be compressed if compression is turn on.
    // §685
    pub fn pdf_end_stream(&mut self) {
        if (self.zip_write_state == zip_writing) {
            self.zip_write_state = zip_finish;
        } else {
            self.pdf_stream_length = ((self.pdf_gone).wrapping_add(((self.pdf_ptr) as i64))).wrapping_sub(self.pdf_save_offset);
        }
        self.pdf_flush();
        if self.pdf_seek_write_length {
            self.write_stream_length(self.pdf_stream_length, self.pdf_stream_length_offset);
        }
        self.pdf_seek_write_length = false;
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
        {
            self.pdf_print(1023i32);
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
        self.pdf_end_obj();
    }

    /// Next subroutines are needed for controlling spacing in PDF page description.
    /// For a given character `c` from a font `f`,
    /// the procedure `adv_char_width` advances `pdf_h`
    /// by {\it about\/} the amount `w`, which is the character width.
    /// But we cannot simply add `w` to `pdf_h`.
    /// Instead we have to bring the required shift into the same raster,
    /// on which also the \.{/Widths} array values,
    /// as they appear in the PDF file, are based.
    /// The `scaled_out` value is the `w` value moved into this raster.
    /// The \.{/Widths} values are used by the PDF reader independently
    /// to update its positions.
    /// So one has to be sure, that calculations are properly synchronized.
    /// Currently the \.{/Widths} array values are output
    /// with one digit after the decimal point,
    /// ...
    // §690
    pub fn adv_char_width(&mut self, mut f: internal_font_number, mut c: eight_bits) {
        let mut w: scaled = 0; // §690
        let mut s_out: scaled = 0; // §690
        let mut s: i32 = 0; // §690
        w = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq().b0())) as usize)].int();
        if self.isscalable(f) {
            {
                if (self.pdf_cur_Tm_a == 0i32) {
                    {
                        s = self.divide_scaled(w, self.pdf_font_size[crate::ix::U((f) as usize)], 4i32);
                        s_out = self.scaled_out;
                        self.pdf_delta_h = (self.pdf_delta_h).wrapping_add(s_out);
                    }
                } else {
                    {
                        s = { let __a543_0 = self.round_xn_over_d(w, 1000i32, (1000i32).wrapping_add(self.pdf_cur_Tm_a)); let __a543_1 = self.pdf_font_size[crate::ix::U((f) as usize)]; let __a543_2 = 4i32; self.divide_scaled(__a543_0, __a543_1, __a543_2) };
                        s_out = { let __a544_0 = self.round_xn_over_d(self.pdf_font_size[crate::ix::U((f) as usize)], (s).wrapping_abs(), 10000i32); let __a544_1 = (1000i32).wrapping_add(self.pdf_cur_Tm_a); let __a544_2 = 1000i32; self.round_xn_over_d(__a544_0, __a544_1, __a544_2) };
                        if (s < 0i32) {
                            s_out = (s_out).wrapping_neg();
                        }
                        self.pdf_delta_h = (self.pdf_delta_h).wrapping_add(s_out);
                    }
                }
                self.adv_char_width_s = crate::system::pas_round((((s) as f64) / ((10i32) as f64)));
                self.adv_char_width_s_out = s_out;
            }
        } else {
            self.pdf_delta_h = (self.pdf_delta_h).wrapping_add(self.get_pk_char_width(f, w));
        }
    }

    /// Next subroutines are needed for controlling spacing in PDF page description.
    /// For a given character `c` from a font `f`,
    /// the procedure `adv_char_width` advances `pdf_h`
    /// by {\it about\/} the amount `w`, which is the character width.
    /// But we cannot simply add `w` to `pdf_h`.
    /// Instead we have to bring the required shift into the same raster,
    /// on which also the \.{/Widths} array values,
    /// as they appear in the PDF file, are based.
    /// The `scaled_out` value is the `w` value moved into this raster.
    /// The \.{/Widths} values are used by the PDF reader independently
    /// to update its positions.
    /// So one has to be sure, that calculations are properly synchronized.
    /// Currently the \.{/Widths} array values are output
    /// with one digit after the decimal point,
    /// ...
    // §690
    pub fn pdf_print_real(&mut self, mut m: i32, mut d: i32) {
        if (m < 0i32) {
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
                m = (m).wrapping_neg();
            }
        }
        self.pdf_print_int((((m / self.ten_pow[crate::ix::U((d) as usize)])) as i64));
        m = (m % self.ten_pow[crate::ix::U((d) as usize)]);
        if (m > 0i32) {
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
                        self.pdf_buf_set(self.pdf_ptr, 46i32);
                        self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                    }
                }
                d = (d).wrapping_sub(1i32);
                while (m < self.ten_pow[crate::ix::U((d) as usize)]) {
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
                        d = (d).wrapping_sub(1i32);
                    }
                }
                while ((m % 10i32) == 0i32) {
                    m = (m / 10i32);
                }
                self.pdf_print_int(((m) as i64));
            }
        }
    }

    /// Next subroutines are needed for controlling spacing in PDF page description.
    /// For a given character `c` from a font `f`,
    /// the procedure `adv_char_width` advances `pdf_h`
    /// by {\it about\/} the amount `w`, which is the character width.
    /// But we cannot simply add `w` to `pdf_h`.
    /// Instead we have to bring the required shift into the same raster,
    /// on which also the \.{/Widths} array values,
    /// as they appear in the PDF file, are based.
    /// The `scaled_out` value is the `w` value moved into this raster.
    /// The \.{/Widths} values are used by the PDF reader independently
    /// to update its positions.
    /// So one has to be sure, that calculations are properly synchronized.
    /// Currently the \.{/Widths} array values are output
    /// with one digit after the decimal point,
    /// ...
    // §690
    pub fn pdf_print_bp(&mut self, mut s: scaled) {
        { let __a545_0 = self.divide_scaled(s, self.one_hundred_bp, (self.fixed_decimal_digits).wrapping_add(2i32)); let __a545_1 = self.fixed_decimal_digits; self.pdf_print_real(__a545_0, __a545_1) };
    }

    /// Next subroutines are needed for controlling spacing in PDF page description.
    /// For a given character `c` from a font `f`,
    /// the procedure `adv_char_width` advances `pdf_h`
    /// by {\it about\/} the amount `w`, which is the character width.
    /// But we cannot simply add `w` to `pdf_h`.
    /// Instead we have to bring the required shift into the same raster,
    /// on which also the \.{/Widths} array values,
    /// as they appear in the PDF file, are based.
    /// The `scaled_out` value is the `w` value moved into this raster.
    /// The \.{/Widths} values are used by the PDF reader independently
    /// to update its positions.
    /// So one has to be sure, that calculations are properly synchronized.
    /// Currently the \.{/Widths} array values are output
    /// with one digit after the decimal point,
    /// ...
    // §690
    pub fn pdf_print_mag_bp(&mut self, mut s: scaled) {
        self.prepare_mag();
        if (self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int() != 1000i32) {
            s = self.round_xn_over_d(s, self.eqtb[crate::ix::U(((629035i32) - 1) as usize)].int(), 1000i32);
        }
        self.pdf_print_bp(s);
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn pdf_set_origin(&mut self, mut h: scaled, mut v: scaled) {
        if ((((h).wrapping_sub(self.pdf_origin_h)).wrapping_abs() >= self.min_bp_val) || (((v).wrapping_sub(self.pdf_origin_v)).wrapping_abs() >= self.min_bp_val)) {
            {
                self.pdf_print(1032i32);
                self.pdf_print_bp((h).wrapping_sub(self.pdf_origin_h));
                self.pdf_origin_h = (self.pdf_origin_h).wrapping_add(self.scaled_out);
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
                self.pdf_print_bp((self.pdf_origin_v).wrapping_sub(v));
                self.pdf_origin_v = (self.pdf_origin_v).wrapping_sub(self.scaled_out);
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
                            self.pdf_buf_set(self.pdf_ptr, pdf_new_line_char);
                            self.pdf_ptr = (self.pdf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.pdf_h = self.pdf_origin_h;
        self.pdf_tj_start_h = self.pdf_h;
        self.pdf_v = self.pdf_origin_v;
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn pdf_set_origin_temp(&mut self, mut h: scaled, mut v: scaled) {
        if ((((h).wrapping_sub(self.pdf_origin_h)).wrapping_abs() >= self.min_bp_val) || (((v).wrapping_sub(self.pdf_origin_v)).wrapping_abs() >= self.min_bp_val)) {
            {
                self.pdf_print(1032i32);
                self.pdf_print_bp((h).wrapping_sub(self.pdf_origin_h));
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
                self.pdf_print_bp((self.pdf_origin_v).wrapping_sub(v));
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
    // §692
    pub fn pdf_end_string(&mut self) {
        if self.pdf_doing_string {
            {
                self.pdf_print(1034i32);
                self.pdf_doing_string = false;
            }
        }
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn pdf_end_string_nl(&mut self) {
        if self.pdf_doing_string {
            {
                {
                    self.pdf_print(1034i32);
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
                self.pdf_doing_string = false;
            }
        }
    }

    /// Following procedures implement low-level subroutines to convert \TeX{}
    /// internal structures to PDF page description.
    // §692
    pub fn get_font_auto_expand_ratio(&mut self, mut f: internal_font_number) -> i32 {
        let mut get_font_auto_expand_ratio: i32 = 0;
        if self.pdf_font_auto_expand[crate::ix::U((f) as usize)] {
            get_font_auto_expand_ratio = self.pdf_font_expand_ratio[crate::ix::U((f) as usize)];
        } else {
            get_font_auto_expand_ratio = 0i32;
        }
        get_font_auto_expand_ratio
    }

}
