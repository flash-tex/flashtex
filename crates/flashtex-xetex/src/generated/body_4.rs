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
    /// `effective_char_info` is `char_info` of the effective character, without
    /// a second mapping.
    /// @<Declare additional functions for ML\TeX
    // §1716
    pub fn effective_char_info(&mut self, mut f: internal_font_number, mut c: quarterword) -> four_quarters {
        let mut effective_char_info: four_quarters = four_quarters::default();
        if ((!self.xtx_ligature_present) && (self.font_mapping[crate::ix::U((f) as usize)] != nil)) {
            c = self.apply_tfm_font_mapping(self.font_mapping[crate::ix::U((f) as usize)], c);
        }
        self.xtx_ligature_present = false;
        effective_char_info = self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(c)) as usize)].qqqq();
        effective_char_info
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
    // §595
    pub fn read_font_info(&mut self, mut u: halfword, mut nom: str_number, mut aire: str_number, mut s: scaled) -> internal_font_number {
        let mut read_font_info: internal_font_number = 0;
        let mut k: font_index = 0; // §595
        let mut name_too_long: bool = false; // §595
        let mut file_opened: bool = false; // §595
        let mut lf: halfword = 0; // §595
        let mut lh: halfword = 0; // §595
        let mut bc: halfword = 0; // §595
        let mut ec: halfword = 0; // §595
        let mut nw: halfword = 0; // §595
        let mut nh: halfword = 0; // §595
        let mut nd: halfword = 0; // §595
        let mut ni: halfword = 0; // §595
        let mut nl: halfword = 0; // §595
        let mut nk: halfword = 0; // §595
        let mut ne: halfword = 0; // §595
        let mut np: halfword = 0; // §595
        let mut f: internal_font_number = 0; // §595
        let mut g: internal_font_number = 0; // §595
        let mut a: eight_bits = 0; // §595
        let mut b: eight_bits = 0; // §595
        let mut c: eight_bits = 0; // §595
        let mut d: eight_bits = 0; // §595
        let mut qw: four_quarters = four_quarters::default(); // §595
        let mut sw: scaled = 0; // §595
        let mut bch_label: i32 = 0; // §595
        let mut bchar: i32 = 0; // §595
        let mut z: scaled = 0; // §595
        let mut alpha: i32 = 0; // §595
        let mut beta: i32 = 0; // §595
        'l_done_f: {
            'l_L11_f: {
                g = null_font;
                file_opened = false;
                self.pack_file_name(nom, aire, self.cur_ext);
                if (self.eqtb[crate::ix::U(((7892347i32) - 1) as usize)].int() > 0i32) {
                    {
                        self.begin_diagnostic();
                        self.print_nl(66188i32);
                        self.print_name_of_file_c();
                        self.print(((b'"') as i32));
                        if (s < 0i32) {
                            {
                                self.print(66189i32);
                                self.print_int((s).wrapping_neg());
                            }
                        } else {
                            {
                                self.print(66121i32);
                                self.print_scaled(s);
                                self.print(65689i32);
                            }
                        }
                        self.end_diagnostic(false);
                    }
                }
                if self.quoted_filename {
                    {
                        g = self.load_native_font(u, nom, aire, s);
                        if (g != null_font) {
                            break 'l_done_f;
                        }
                    }
                }
                // §598
                name_too_long = ((self.length(nom) > 255i32) || (self.length(aire) > 255i32));
                if name_too_long {
                    break 'l_L11_f;
                }
                self.pack_file_name(nom, aire, 65626i32);
                self.check_for_tfm_font_mapping();
                if { let mut __f0 = ::core::mem::take(&mut self.tfm_file); let __r = self.b_open_in(&mut __f0); self.tfm_file = __f0; __r } {
                    {
                        file_opened = true;
                        // §600
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
                        // §601
                        lf = ((lf).wrapping_sub(6i32)).wrapping_sub(lh);
                        if (np < 7i32) {
                            lf = ((lf).wrapping_add(7i32)).wrapping_sub(np);
                        }
                        if ((self.font_ptr == font_max) || ((self.fmem_ptr).wrapping_add(lf) > font_mem_size)) {
                            // §602
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66192i32);
                                }
                                self.sprint_cs(u);
                                self.print_char(61i32);
                                if (self.file_name_quote_char != 0i32) {
                                    self.print_char(self.file_name_quote_char);
                                }
                                self.print_file_name(nom, aire, self.cur_ext);
                                if (self.file_name_quote_char != 0i32) {
                                    self.print_char(self.file_name_quote_char);
                                }
                                if (s >= 0i32) {
                                    {
                                        self.print(66121i32);
                                        self.print_scaled(s);
                                        self.print(65689i32);
                                    }
                                } else {
                                    if (s != (1000i32).wrapping_neg()) {
                                        {
                                            self.print(66189i32);
                                            self.print_int((s).wrapping_neg());
                                        }
                                    }
                                }
                                self.print(66201i32);
                                {
                                    self.help_ptr = 4i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 66202i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 66203i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66204i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66205i32;
                                }
                                self.error();
                                break 'l_done_f;
                            }
                        }
                        // §601
                        f = (self.font_ptr).wrapping_add(1i32);
                        { let __v411 = (self.fmem_ptr).wrapping_sub(bc); self.char_base[crate::ix::U((f) as usize)] = __v411; }
                        { let __v412 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(ec)).wrapping_add(1i32); self.width_base[crate::ix::U((f) as usize)] = __v412; }
                        { let __v413 = (self.width_base[crate::ix::U((f) as usize)]).wrapping_add(nw); self.height_base[crate::ix::U((f) as usize)] = __v413; }
                        { let __v414 = (self.height_base[crate::ix::U((f) as usize)]).wrapping_add(nh); self.depth_base[crate::ix::U((f) as usize)] = __v414; }
                        { let __v415 = (self.depth_base[crate::ix::U((f) as usize)]).wrapping_add(nd); self.italic_base[crate::ix::U((f) as usize)] = __v415; }
                        { let __v416 = (self.italic_base[crate::ix::U((f) as usize)]).wrapping_add(ni); self.lig_kern_base[crate::ix::U((f) as usize)] = __v416; }
                        { let __v417 = ((self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_add(nl)).wrapping_sub((256i32).wrapping_mul(128i32)); self.kern_base[crate::ix::U((f) as usize)] = __v417; }
                        { let __v418 = ((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(nk); self.exten_base[crate::ix::U((f) as usize)] = __v418; }
                        { let __v419 = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_add(ne); self.param_base[crate::ix::U((f) as usize)] = __v419; }
                        // §603
                        {
                            if (lh < 2i32) {
                                break 'l_L11_f;
                            }
                            {
                                crate::system::get_byte(&mut self.tfm_file);
                                a = self.tfm_file.buf;
                                qw.set_b0(a);
                                crate::system::get_byte(&mut self.tfm_file);
                                b = self.tfm_file.buf;
                                qw.set_b1(b);
                                crate::system::get_byte(&mut self.tfm_file);
                                c = self.tfm_file.buf;
                                qw.set_b2(c);
                                crate::system::get_byte(&mut self.tfm_file);
                                d = self.tfm_file.buf;
                                qw.set_b3(d);
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
                                    {
                                        self.save_arith_error = self.arith_error;
                                        sw = z;
                                        z = self.xn_over_d(z, (s).wrapping_neg(), 1000i32);
                                        if (self.arith_error || (z >= 134217728i32)) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66192i32);
                                                }
                                                self.sprint_cs(u);
                                                self.print_char(61i32);
                                                if (self.file_name_quote_char != 0i32) {
                                                    self.print_char(self.file_name_quote_char);
                                                }
                                                self.print_file_name(nom, aire, self.cur_ext);
                                                if (self.file_name_quote_char != 0i32) {
                                                    self.print_char(self.file_name_quote_char);
                                                }
                                                if (s >= 0i32) {
                                                    {
                                                        self.print(66121i32);
                                                        self.print_scaled(s);
                                                        self.print(65689i32);
                                                    }
                                                } else {
                                                    if (s != (1000i32).wrapping_neg()) {
                                                        {
                                                            self.print(66189i32);
                                                            self.print_int((s).wrapping_neg());
                                                        }
                                                    }
                                                }
                                                self.print(66206i32);
                                                {
                                                    self.help_ptr = 1i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66207i32;
                                                }
                                                self.error();
                                                z = sw;
                                            }
                                        }
                                        self.arith_error = self.save_arith_error;
                                    }
                                }
                            }
                            self.font_size[crate::ix::U((f) as usize)] = z;
                        }
                        // §604
                        {
                            let __for_end_6 = (self.width_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                            k = self.fmem_ptr;
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_byte(&mut self.tfm_file);
                                        a = self.tfm_file.buf;
                                        qw.set_b0(a);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        b = self.tfm_file.buf;
                                        qw.set_b1(b);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        c = self.tfm_file.buf;
                                        qw.set_b2(c);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        d = self.tfm_file.buf;
                                        qw.set_b3(d);
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
                                            // §605
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
                                                            if ((qw.b2() % 4i32) != list_tag) {
                                                                break 'l_not_found_f;
                                                            }
                                                            d = qw.b3();
                                                        }
                                                    }
                                                    if (d == ((k).wrapping_add(bc)).wrapping_sub(self.fmem_ptr)) {
                                                        break 'l_L11_f;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            // §604
                                        }
                                    }
                                }
                                k = k.wrapping_add(1);
                            }
                        }
                        // §606
                        {
                            // §607
                            {
                                alpha = 16i32;
                                while (z >= 8388608i32) {
                                    {
                                        z = (z / 2i32);
                                        alpha = (alpha).wrapping_add(alpha);
                                    }
                                }
                                beta = (256i32 / alpha);
                                alpha = (alpha).wrapping_mul(z);
                            }
                            // §606
                            {
                                let __for_end_7 = (self.lig_kern_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                                k = self.width_base[crate::ix::U((f) as usize)];
                                while k <= __for_end_7 {
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
                        // §608
                        bch_label = 32767i32;
                        bchar = 256i32;
                        if (nl > 0i32) {
                            {
                                {
                                    let __for_end_8 = ((self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_sub(1i32);
                                    k = self.lig_kern_base[crate::ix::U((f) as usize)];
                                    while k <= __for_end_8 {
                                        {
                                            {
                                                crate::system::get_byte(&mut self.tfm_file);
                                                a = self.tfm_file.buf;
                                                qw.set_b0(a);
                                                crate::system::get_byte(&mut self.tfm_file);
                                                b = self.tfm_file.buf;
                                                qw.set_b1(b);
                                                crate::system::get_byte(&mut self.tfm_file);
                                                c = self.tfm_file.buf;
                                                qw.set_b2(c);
                                                crate::system::get_byte(&mut self.tfm_file);
                                                d = self.tfm_file.buf;
                                                qw.set_b3(d);
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
                            let __for_end_6 = (self.exten_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                            k = (self.kern_base[crate::ix::U((f) as usize)]).wrapping_add((256i32).wrapping_mul(128i32));
                            while k <= __for_end_6 {
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
                        // §609
                        {
                            let __for_end_6 = (self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32);
                            k = self.exten_base[crate::ix::U((f) as usize)];
                            while k <= __for_end_6 {
                                {
                                    {
                                        crate::system::get_byte(&mut self.tfm_file);
                                        a = self.tfm_file.buf;
                                        qw.set_b0(a);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        b = self.tfm_file.buf;
                                        qw.set_b1(b);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        c = self.tfm_file.buf;
                                        qw.set_b2(c);
                                        crate::system::get_byte(&mut self.tfm_file);
                                        d = self.tfm_file.buf;
                                        qw.set_b3(d);
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
                        // §610
                        {
                            {
                                let __for_end_7 = np;
                                k = 1i32;
                                while k <= __for_end_7 {
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
                                            { let __ix420 = self.param_base[crate::ix::U((f) as usize)]; let __v421 = ((sw).wrapping_mul(16i32)).wrapping_add((self.tfm_file.buf / 16i32)); self.font_info[crate::ix::U((__ix420) as usize)].set_int(__v421); }
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
                                                { let __ix422 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix422) as usize)].set_int(sw); }
                                            } else {
                                                if (a == 255i32) {
                                                    { let __ix423 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix423) as usize)].set_int((sw).wrapping_sub(alpha)); }
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
                                let __for_end_7 = 7i32;
                                k = (np).wrapping_add(1i32);
                                while k <= __for_end_7 {
                                    { let __ix424 = ((self.param_base[crate::ix::U((f) as usize)]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[crate::ix::U((__ix424) as usize)].set_int(0i32); }
                                    k = k.wrapping_add(1);
                                }
                            }
                        }
                        // §611
                        if (np >= 7i32) {
                            self.font_params[crate::ix::U((f) as usize)] = np;
                        } else {
                            self.font_params[crate::ix::U((f) as usize)] = 7i32;
                        }
                        { let __v425 = self.eqtb[crate::ix::U(((7892310i32) - 1) as usize)].int(); self.hyphen_char[crate::ix::U((f) as usize)] = __v425; }
                        { let __v426 = self.eqtb[crate::ix::U(((7892311i32) - 1) as usize)].int(); self.skew_char[crate::ix::U((f) as usize)] = __v426; }
                        if (bch_label < nl) {
                            { let __v427 = (bch_label).wrapping_add(self.lig_kern_base[crate::ix::U((f) as usize)]); self.bchar_label[crate::ix::U((f) as usize)] = __v427; }
                        } else {
                            self.bchar_label[crate::ix::U((f) as usize)] = non_address;
                        }
                        self.font_bchar[crate::ix::U((f) as usize)] = bchar;
                        self.font_false_bchar[crate::ix::U((f) as usize)] = bchar;
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
                        self.font_glue[crate::ix::U((f) as usize)] = (268435455i32).wrapping_neg();
                        { let __v428 = self.char_base[crate::ix::U((f) as usize)]; self.char_base[crate::ix::U((f) as usize)] = __v428; }
                        { let __v429 = self.width_base[crate::ix::U((f) as usize)]; self.width_base[crate::ix::U((f) as usize)] = __v429; }
                        { let __v430 = self.lig_kern_base[crate::ix::U((f) as usize)]; self.lig_kern_base[crate::ix::U((f) as usize)] = __v430; }
                        { let __v431 = self.kern_base[crate::ix::U((f) as usize)]; self.kern_base[crate::ix::U((f) as usize)] = __v431; }
                        { let __v432 = self.exten_base[crate::ix::U((f) as usize)]; self.exten_base[crate::ix::U((f) as usize)] = __v432; }
                        { let __v433 = (self.param_base[crate::ix::U((f) as usize)]).wrapping_sub(1i32); self.param_base[crate::ix::U((f) as usize)] = __v433; }
                        self.fmem_ptr = (self.fmem_ptr).wrapping_add(lf);
                        self.font_ptr = f;
                        g = f;
                        { let __v434 = self.load_tfm_font_mapping(); self.font_mapping[crate::ix::U((f) as usize)] = __v434; }
                        break 'l_done_f;
                    }
                }
                // §595
                if (g != null_font) {
                    break 'l_done_f;
                }
                if (!self.quoted_filename) {
                    {
                        g = self.load_native_font(u, nom, aire, s);
                        if (g != null_font) {
                            break 'l_done_f;
                        }
                    }
                }
            }
            if (self.eqtb[crate::ix::U(((7892334i32) - 1) as usize)].int() == 0i32) {
                {
                    // §596
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66192i32);
                    }
                    self.sprint_cs(u);
                    self.print_char(61i32);
                    if (self.file_name_quote_char != 0i32) {
                        self.print_char(self.file_name_quote_char);
                    }
                    self.print_file_name(nom, aire, self.cur_ext);
                    if (self.file_name_quote_char != 0i32) {
                        self.print_char(self.file_name_quote_char);
                    }
                    if (s >= 0i32) {
                        {
                            self.print(66121i32);
                            self.print_scaled(s);
                            self.print(65689i32);
                        }
                    } else {
                        if (s != (1000i32).wrapping_neg()) {
                            {
                                self.print(66189i32);
                                self.print_int((s).wrapping_neg());
                            }
                        }
                    }
                    if file_opened {
                        self.print(66193i32);
                    } else {
                        if name_too_long {
                            self.print(66194i32);
                        } else {
                            self.print(66195i32);
                        }
                    }
                    {
                        self.help_ptr = 5i32;
                        self.help_line[crate::ix::U((4i32) as usize)] = 66196i32;
                        self.help_line[crate::ix::U((3i32) as usize)] = 66197i32;
                        self.help_line[crate::ix::U((2i32) as usize)] = 66198i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66199i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66200i32;
                    }
                    self.error();
                }
            }
        }
        // §595
        if file_opened {
            { let mut __f0 = ::core::mem::take(&mut self.tfm_file); let __r = self.b_close(&mut __f0); self.tfm_file = __f0; __r };
        }
        if (self.eqtb[crate::ix::U(((7892347i32) - 1) as usize)].int() > 0i32) {
            {
                if (g == null_font) {
                    {
                        self.begin_diagnostic();
                        self.print_nl(66190i32);
                        self.end_diagnostic(false);
                    }
                } else {
                    if file_opened {
                        {
                            self.begin_diagnostic();
                            self.print_nl(66191i32);
                            self.print_name_of_file_c();
                            self.end_diagnostic(false);
                        }
                    }
                }
            }
        }
        read_font_info = g;
        read_font_info
    }

    /// When \TeX\ wants to typeset a character that doesn't exist, the
    /// character node is not created; thus the output routine can assume
    /// that characters exist when it sees them. The following procedure
    /// prints a warning message unless the user has suppressed it.
    /// @<Declare subroutines for `new_character`
    // §616
    pub fn print_ucs_code(&mut self, mut n: UnicodeScalar) {
        let mut k: i32 = 0; // §616
        k = 0i32;
        self.print(66216i32);
        loop {
            self.dig[crate::ix::U((k) as usize)] = (n % 16i32);
            n = (n / 16i32);
            k = (k).wrapping_add(1i32);
            if (n == 0i32) { break; }
        }
        while (k < 4i32) {
            {
                self.dig[crate::ix::U((k) as usize)] = 0i32;
                k = (k).wrapping_add(1i32);
            }
        }
        self.print_the_digs(k);
    }

    /// When \TeX\ wants to typeset a character that doesn't exist, the
    /// character node is not created; thus the output routine can assume
    /// that characters exist when it sees them. The following procedure
    /// prints a warning message unless the user has suppressed it.
    /// @<Declare subroutines for `new_character`
    // §616
    pub fn char_warning(&mut self, mut f: internal_font_number, mut c: i32) {
        let mut old_setting: i32 = 0; // §616
        if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() > 0i32) {
            {
                old_setting = self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int();
                if ((self.eTeX_mode == 1i32) && (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() > 1i32)) {
                    self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].set_int(1i32);
                }
                if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() > 2i32) {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66217i32);
                    }
                } else {
                    {
                        self.begin_diagnostic();
                        self.print_nl(66217i32);
                    }
                }
                if (c < 65536i32) {
                    self.print(c);
                } else {
                    self.print_char(c);
                }
                self.print(65566i32);
                if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
                    self.print_ucs_code(c);
                } else {
                    self.print_hex(c);
                }
                self.print(41i32);
                self.print(66218i32);
                self.print(self.font_name[crate::ix::U((f) as usize)]);
                if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() < 3i32) {
                    self.print_char(33i32);
                }
                self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].set_int(old_setting);
                if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() > 2i32) {
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

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn new_native_word_node(&mut self, mut f: internal_font_number, mut n: i32) -> halfword {
        let mut new_native_word_node: halfword = 0;
        let mut l: i32 = 0; // §744
        let mut q: halfword = 0; // §744
        l = (native_node_size).wrapping_add((((n).wrapping_mul(2i32)).wrapping_add(7i32) / 8i32));
        q = self.get_node(l);
        self.mem[crate::ix::U((q) as usize)].set_hh_b0(whatsit_node);
        if (self.eqtb[crate::ix::U(((7892349i32) - 1) as usize)].int() > 0i32) {
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(native_word_node_AT);
        } else {
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(native_word_node);
        }
        self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_qqqq_b0(l);
        self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_qqqq_b1(f);
        self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_qqqq_b2(n);
        self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].set_qqqq_b3(0i32);
        self.mem[crate::ix::U(((q).wrapping_add(5i32)) as usize)].set_int(null_ptr);
        new_native_word_node = q;
        new_native_word_node
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn new_native_character(&mut self, mut f: internal_font_number, mut c: UnicodeScalar) -> halfword {
        let mut new_native_character: halfword = 0;
        let mut p: halfword = 0; // §744
        let mut i: i32 = 0; // §744
        let mut len: i32 = 0; // §744
        if (self.font_mapping[crate::ix::U((f) as usize)] != 0i32) {
            {
                if (c > 65535i32) {
                    {
                        if ((self.pool_ptr).wrapping_add(2i32) > pool_size) {
                            self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                        }
                    }
                } else {
                    {
                        if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                            self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                        }
                    }
                }
                {
                    if (c > 65535i32) {
                        {
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((c).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32);
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((c % 1024i32)).wrapping_add(56320i32);
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = c;
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    }
                }
                len = self.apply_mapping_pool(self.font_mapping[crate::ix::U((f) as usize)], self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)], (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]));
                self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                i = 0i32;
                while (i < len) {
                    {
                        if ((self.mapped_text[crate::ix::U((i) as usize)] >= 55296i32) && (self.mapped_text[crate::ix::U((i) as usize)] < 56320i32)) {
                            {
                                c = ((((self.mapped_text[crate::ix::U((i) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32)).wrapping_add(self.mapped_text[crate::ix::U(((i).wrapping_add(1i32)) as usize)])).wrapping_add(9216i32);
                                if (self.map_char_to_glyph(f, c) == 0i32) {
                                    {
                                        self.char_warning(f, c);
                                    }
                                }
                                i = (i).wrapping_add(2i32);
                            }
                        } else {
                            {
                                if (self.map_char_to_glyph(f, self.mapped_text[crate::ix::U((i) as usize)]) == 0i32) {
                                    {
                                        self.char_warning(f, self.mapped_text[crate::ix::U((i) as usize)]);
                                    }
                                }
                                i = (i).wrapping_add(1i32);
                            }
                        }
                    }
                }
                p = self.new_native_word_node(f, len);
                {
                    let __for_end_4 = (len).wrapping_sub(1i32);
                    i = 0i32;
                    while i <= __for_end_4 {
                        {
                            self.set_native_char(p, i, self.mapped_text[crate::ix::U((i) as usize)]);
                        }
                        i = i.wrapping_add(1);
                    }
                }
            }
        } else {
            {
                if (self.eqtb[crate::ix::U(((7892299i32) - 1) as usize)].int() > 0i32) {
                    if (self.map_char_to_glyph(f, c) == 0i32) {
                        {
                            self.char_warning(f, c);
                        }
                    }
                }
                p = self.get_node(7i32);
                self.mem[crate::ix::U((p) as usize)].set_hh_b0(whatsit_node);
                self.mem[crate::ix::U((p) as usize)].set_hh_b1(native_word_node);
                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b0(7i32);
                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b3(0i32);
                self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].set_int(null_ptr);
                self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b1(f);
                if (c > 65535i32) {
                    {
                        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(2i32);
                        self.set_native_char(p, 0i32, (((c).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32));
                        self.set_native_char(p, 1i32, (((c).wrapping_sub(65536i32) % 1024i32)).wrapping_add(56320i32));
                    }
                } else {
                    {
                        self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].set_qqqq_b2(1i32);
                        self.set_native_char(p, 0i32, c);
                    }
                }
            }
        }
        self.set_native_metrics(p, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
        new_native_character = p;
        new_native_character
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn font_feature_warning(&mut self, mut featureNameP: void_pointer, mut featLen: i32, mut settingNameP: void_pointer, mut setLen: i32) {
        let mut i: i32 = 0; // §744
        self.begin_diagnostic();
        self.print_nl(66288i32);
        if (setLen > 0i32) {
            {
                self.print(66289i32);
                self.print_utf8_str(settingNameP, setLen);
                self.print(66290i32);
            }
        }
        self.print(66291i32);
        self.print_utf8_str(featureNameP, featLen);
        self.print(66292i32);
        i = 1i32;
        while (((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32) != 0i32) {
            {
                self.print_raw_char(((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32), true);
                i = (i).wrapping_add(1i32);
            }
        }
        self.print(66172i32);
        self.end_diagnostic(false);
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn font_mapping_warning(&mut self, mut mappingNameP: void_pointer, mut mappingNameLen: i32, mut warningType: i32) {
        let mut i: i32 = 0; // §744
        self.begin_diagnostic();
        if (warningType == 0i32) {
            self.print_nl(66293i32);
        } else {
            self.print_nl(66294i32);
        }
        self.print_utf8_str(mappingNameP, mappingNameLen);
        self.print(66295i32);
        i = 1i32;
        while (((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32) != 0i32) {
            {
                self.print_raw_char(((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32), true);
                i = (i).wrapping_add(1i32);
            }
        }
        match warningType {
            1 => {
                self.print(66296i32);
            }
            2 => {
                {
                    self.print(66297i32);
                    self.print_nl(66298i32);
                }
            }
            _ => {
                self.print(66172i32);
            }
        }
        self.end_diagnostic(false);
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn graphite_warning(&mut self) {
        let mut i: i32 = 0; // §744
        self.begin_diagnostic();
        self.print_nl(66299i32);
        i = 1i32;
        while (((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32) != 0i32) {
            {
                self.print_raw_char(((self.name_of_file[crate::ix::U(((i) - 1) as usize)]) as i32), true);
                i = (i).wrapping_add(1i32);
            }
        }
        self.print(66300i32);
        self.end_diagnostic(false);
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn load_native_font(&mut self, mut u: halfword, mut nom: str_number, mut aire: str_number, mut s: scaled) -> internal_font_number {
        let mut load_native_font: internal_font_number = 0;
        let mut k: i32 = 0; // §744
        let mut num_font_dimens: i32 = 0; // §744
        let mut font_engine: void_pointer = 0; // §744
        let mut actual_size: scaled = 0; // §744
        let mut p: halfword = 0; // §744
        let mut ascent: scaled = 0; // §744
        let mut descent: scaled = 0; // §744
        let mut font_slant: scaled = 0; // §744
        let mut x_ht: scaled = 0; // §744
        let mut cap_ht: scaled = 0; // §744
        let mut f: internal_font_number = 0; // §744
        let mut full_name: str_number = 0; // §744
        'l_done_f: {
            load_native_font = null_font;
            font_engine = self.find_native_font(s);
            if (font_engine == 0i32) {
                break 'l_done_f;
            }
            if (s >= 0i32) {
                actual_size = s;
            } else {
                {
                    if (s != (1000i32).wrapping_neg()) {
                        actual_size = self.xn_over_d(self.loaded_font_design_size, (s).wrapping_neg(), 1000i32);
                    } else {
                        actual_size = self.loaded_font_design_size;
                    }
                }
            }
            {
                if ((self.pool_ptr).wrapping_add(self.name_length) > pool_size) {
                    self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                }
            }
            {
                let __for_end_3 = self.name_length;
                k = 1i32;
                while k <= __for_end_3 {
                    {
                        if (((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32) > 65535i32) {
                            {
                                { let __ix435 = self.pool_ptr; let __v436 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.str_pool[crate::ix::U((__ix435) as usize)] = __v436; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                { let __ix437 = self.pool_ptr; let __v438 = ((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32) % 1024i32)).wrapping_add(56320i32); self.str_pool[crate::ix::U((__ix437) as usize)] = __v438; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        } else {
                            {
                                { let __ix439 = self.pool_ptr; let __v440 = ((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32); self.str_pool[crate::ix::U((__ix439) as usize)] = __v440; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            full_name = self.make_string();
            {
                let __for_end_3 = self.font_ptr;
                f = 1i32;
                while f <= __for_end_3 {
                    if (((self.font_area[crate::ix::U((f) as usize)] == self.native_font_type_flag) && self.str_eq_str(self.font_name[crate::ix::U((f) as usize)], full_name)) && (self.font_size[crate::ix::U((f) as usize)] == actual_size)) {
                        {
                            self.release_font_engine(font_engine, self.native_font_type_flag);
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                            }
                            load_native_font = f;
                            break 'l_done_f;
                        }
                    }
                    f = f.wrapping_add(1);
                }
            }
            if ((self.native_font_type_flag == otgr_font_flag) && self.isOpenTypeMathFont(font_engine)) {
                num_font_dimens = (10i32).wrapping_add(55i32);
            } else {
                num_font_dimens = 8i32;
            }
            if ((self.font_ptr == font_max) || ((self.fmem_ptr).wrapping_add(num_font_dimens) > font_mem_size)) {
                {
                    // §602
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66192i32);
                        }
                        self.sprint_cs(u);
                        self.print_char(61i32);
                        if (self.file_name_quote_char != 0i32) {
                            self.print_char(self.file_name_quote_char);
                        }
                        self.print_file_name(nom, aire, self.cur_ext);
                        if (self.file_name_quote_char != 0i32) {
                            self.print_char(self.file_name_quote_char);
                        }
                        if (s >= 0i32) {
                            {
                                self.print(66121i32);
                                self.print_scaled(s);
                                self.print(65689i32);
                            }
                        } else {
                            if (s != (1000i32).wrapping_neg()) {
                                {
                                    self.print(66189i32);
                                    self.print_int((s).wrapping_neg());
                                }
                            }
                        }
                        self.print(66201i32);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 66202i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 66203i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66204i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66205i32;
                        }
                        self.error();
                        break 'l_done_f;
                    }
                }
            }
            // §744
            self.font_ptr = (self.font_ptr).wrapping_add(1i32);
            { let __ix441 = self.font_ptr; let __v442 = self.native_font_type_flag; self.font_area[crate::ix::U((__ix441) as usize)] = __v442; }
            self.font_name[crate::ix::U((self.font_ptr) as usize)] = full_name;
            { let __ix443 = self.font_ptr; self.font_check[crate::ix::U((__ix443) as usize)].set_b0(0i32); }
            { let __ix444 = self.font_ptr; self.font_check[crate::ix::U((__ix444) as usize)].set_b1(0i32); }
            { let __ix445 = self.font_ptr; self.font_check[crate::ix::U((__ix445) as usize)].set_b2(0i32); }
            { let __ix446 = self.font_ptr; self.font_check[crate::ix::U((__ix446) as usize)].set_b3(0i32); }
            self.font_glue[crate::ix::U((self.font_ptr) as usize)] = (268435455i32).wrapping_neg();
            { let __ix447 = self.font_ptr; let __v448 = self.loaded_font_design_size; self.font_dsize[crate::ix::U((__ix447) as usize)] = __v448; }
            self.font_size[crate::ix::U((self.font_ptr) as usize)] = actual_size;
            if (self.native_font_type_flag == aat_font_flag) {
                {
                    { let mut __f1 = ::core::mem::take(&mut ascent); let mut __f2 = ::core::mem::take(&mut descent); let mut __f3 = ::core::mem::take(&mut x_ht); let mut __f4 = ::core::mem::take(&mut cap_ht); let mut __f5 = ::core::mem::take(&mut font_slant); let __r = self.aat_get_font_metrics(font_engine, &mut __f1, &mut __f2, &mut __f3, &mut __f4, &mut __f5); ascent = __f1; descent = __f2; x_ht = __f3; cap_ht = __f4; font_slant = __f5; __r };
                }
            } else {
                {
                    { let mut __f1 = ::core::mem::take(&mut ascent); let mut __f2 = ::core::mem::take(&mut descent); let mut __f3 = ::core::mem::take(&mut x_ht); let mut __f4 = ::core::mem::take(&mut cap_ht); let mut __f5 = ::core::mem::take(&mut font_slant); let __r = self.ot_get_font_metrics(font_engine, &mut __f1, &mut __f2, &mut __f3, &mut __f4, &mut __f5); ascent = __f1; descent = __f2; x_ht = __f3; cap_ht = __f4; font_slant = __f5; __r };
                }
            }
            self.height_base[crate::ix::U((self.font_ptr) as usize)] = ascent;
            self.depth_base[crate::ix::U((self.font_ptr) as usize)] = (descent).wrapping_neg();
            self.font_params[crate::ix::U((self.font_ptr) as usize)] = num_font_dimens;
            self.font_bc[crate::ix::U((self.font_ptr) as usize)] = 0i32;
            self.font_ec[crate::ix::U((self.font_ptr) as usize)] = 65535i32;
            { let __ix449 = self.font_ptr; let __v450 = false; self.font_used[crate::ix::U((__ix449) as usize)] = __v450; }
            { let __ix451 = self.font_ptr; let __v452 = self.eqtb[crate::ix::U(((7892310i32) - 1) as usize)].int(); self.hyphen_char[crate::ix::U((__ix451) as usize)] = __v452; }
            { let __ix453 = self.font_ptr; let __v454 = self.eqtb[crate::ix::U(((7892311i32) - 1) as usize)].int(); self.skew_char[crate::ix::U((__ix453) as usize)] = __v454; }
            { let __ix455 = self.font_ptr; let __v456 = (self.fmem_ptr).wrapping_sub(1i32); self.param_base[crate::ix::U((__ix455) as usize)] = __v456; }
            self.font_layout_engine[crate::ix::U((self.font_ptr) as usize)] = font_engine;
            self.font_mapping[crate::ix::U((self.font_ptr) as usize)] = 0i32;
            { let __ix457 = self.font_ptr; let __v458 = self.loaded_font_letter_space; self.font_letter_space[crate::ix::U((__ix457) as usize)] = __v458; }
            p = self.new_native_character(self.font_ptr, 32i32);
            s = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.loaded_font_letter_space);
            self.free_node(p, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b0());
            { let __ix459 = self.fmem_ptr; self.font_info[crate::ix::U((__ix459) as usize)].set_int(font_slant); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix460 = self.fmem_ptr; self.font_info[crate::ix::U((__ix460) as usize)].set_int(s); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix461 = self.fmem_ptr; self.font_info[crate::ix::U((__ix461) as usize)].set_int((s / 2i32)); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix462 = self.fmem_ptr; self.font_info[crate::ix::U((__ix462) as usize)].set_int((s / 3i32)); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix463 = self.fmem_ptr; self.font_info[crate::ix::U((__ix463) as usize)].set_int(x_ht); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix464 = self.fmem_ptr; let __v465 = self.font_size[crate::ix::U((self.font_ptr) as usize)]; self.font_info[crate::ix::U((__ix464) as usize)].set_int(__v465); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix466 = self.fmem_ptr; self.font_info[crate::ix::U((__ix466) as usize)].set_int((s / 3i32)); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            { let __ix467 = self.fmem_ptr; self.font_info[crate::ix::U((__ix467) as usize)].set_int(cap_ht); }
            self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
            if (num_font_dimens == (10i32).wrapping_add(55i32)) {
                {
                    { let __ix468 = self.fmem_ptr; self.font_info[crate::ix::U((__ix468) as usize)].set_int(num_font_dimens); }
                    self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                    {
                        let __for_end_5 = lastMathConstant;
                        k = 0i32;
                        while k <= __for_end_5 {
                            {
                                { let __ix469 = self.fmem_ptr; let __v470 = self.get_ot_math_constant(self.font_ptr, k); self.font_info[crate::ix::U((__ix469) as usize)].set_int(__v470); }
                                self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                }
            }
            { let __ix471 = self.font_ptr; let __v472 = self.loaded_font_mapping; self.font_mapping[crate::ix::U((__ix471) as usize)] = __v472; }
            { let __ix473 = self.font_ptr; let __v474 = self.loaded_font_flags; self.font_flags[crate::ix::U((__ix473) as usize)] = __v474; }
            load_native_font = self.font_ptr;
        }
        load_native_font
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn do_locale_linebreaks(&mut self, mut s: i32, mut len: i32) {
        let mut offs: i32 = 0; // §744
        let mut prevOffs: i32 = 0; // §744
        let mut i: i32 = 0; // §744
        let mut use_penalty: bool = false; // §744
        let mut use_skip: bool = false; // §744
        if ((self.eqtb[crate::ix::U(((7892336i32) - 1) as usize)].int() == 0i32) || (len == 1i32)) {
            {
                { let __ix475 = self.cur_list.tail_field; let __v476 = self.new_native_word_node(self.main_f, len); self.mem[crate::ix::U((__ix475) as usize)].set_hh_rh(__v476); }
                self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                {
                    let __for_end_4 = (len).wrapping_sub(1i32);
                    i = 0i32;
                    while i <= __for_end_4 {
                        self.set_native_char(self.cur_list.tail_field, i, self.native_text[crate::ix::U(((s).wrapping_add(i)) as usize)]);
                        i = i.wrapping_add(1);
                    }
                }
                self.set_native_metrics(self.cur_list.tail_field, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
            }
        } else {
            {
                use_skip = (self.eqtb[crate::ix::U(((1205779i32) - 1) as usize)].hh().rh() != zero_glue);
                use_penalty = ((self.eqtb[crate::ix::U(((7892337i32) - 1) as usize)].int() != 0i32) || (!use_skip));
                self.linebreak_start(self.main_f, self.eqtb[crate::ix::U(((7892336i32) - 1) as usize)].int(), s, len);
                offs = 0i32;
                loop {
                    prevOffs = offs;
                    offs = self.linebreak_next();
                    if (offs > 0i32) {
                        {
                            if (prevOffs != 0i32) {
                                {
                                    if use_penalty {
                                        {
                                            { let __ix477 = self.cur_list.tail_field; let __v478 = self.new_penalty(self.eqtb[crate::ix::U(((7892337i32) - 1) as usize)].int()); self.mem[crate::ix::U((__ix477) as usize)].set_hh_rh(__v478); }
                                            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                        }
                                    }
                                    if use_skip {
                                        {
                                            { let __ix479 = self.cur_list.tail_field; let __v480 = self.new_param_glue(XeTeX_linebreak_skip_code); self.mem[crate::ix::U((__ix479) as usize)].set_hh_rh(__v480); }
                                            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                                        }
                                    }
                                }
                            }
                            { let __ix481 = self.cur_list.tail_field; let __v482 = self.new_native_word_node(self.main_f, (offs).wrapping_sub(prevOffs)); self.mem[crate::ix::U((__ix481) as usize)].set_hh_rh(__v482); }
                            self.cur_list.tail_field = self.mem[crate::ix::U((self.cur_list.tail_field) as usize)].hh().rh();
                            {
                                let __for_end_7 = (offs).wrapping_sub(1i32);
                                i = prevOffs;
                                while i <= __for_end_7 {
                                    self.set_native_char(self.cur_list.tail_field, (i).wrapping_sub(prevOffs), self.native_text[crate::ix::U(((s).wrapping_add(i)) as usize)]);
                                    i = i.wrapping_add(1);
                                }
                            }
                            self.set_native_metrics(self.cur_list.tail_field, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                        }
                    }
                    if (offs < 0i32) { break; }
                }
            }
        }
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn bad_utf8_warning(&mut self) {
        self.begin_diagnostic();
        self.print_nl(66301i32);
        if (self.cur_input.name_field == 0i32) {
            self.print(66302i32);
        } else {
            {
                self.print(66303i32);
                self.print_int(self.line);
            }
        }
        self.print(66304i32);
        self.end_diagnostic(false);
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn get_input_normalization_state(&mut self) -> i32 {
        let mut get_input_normalization_state: i32 = 0;
        get_input_normalization_state = self.eqtb[crate::ix::U(((7892344i32) - 1) as usize)].int();
        get_input_normalization_state
    }

    /// Native font support requires these additional subroutines.
    /// `new_native_word_node` creates the node, but does not actually set its metrics;
    /// call `set_native_metrics(node)` if that is required.
    /// @<Declare subroutines for `new_character`
    // §744
    pub fn get_tracing_fonts_state(&mut self) -> i32 {
        let mut get_tracing_fonts_state: i32 = 0;
        get_tracing_fonts_state = self.eqtb[crate::ix::U(((7892347i32) - 1) as usize)].int();
        get_tracing_fonts_state
    }

    /// Here is a function that returns a pointer to a character node for a
    /// given character in a given font. If that character doesn't exist,
    /// `null` is returned instead.
    // §618
    pub fn new_character(&mut self, mut f: internal_font_number, mut c: UTF16_code) -> halfword {
        let mut new_character: halfword = 0;
        let mut p: halfword = 0; // §618
        let mut ec: quarterword = 0; // §618
        'l_exit_f: {
            if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
                {
                    new_character = self.new_native_character(f, c);
                    break 'l_exit_f;
                }
            }
            ec = self.effective_char(false, f, c);
            if (self.font_bc[crate::ix::U((f) as usize)] <= ec) {
                if (self.font_ec[crate::ix::U((f) as usize)] >= ec) {
                    if (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(ec)) as usize)].qqqq().b0() > min_quarterword) {
                        {
                            p = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_b0(f);
                            self.mem[crate::ix::U((p) as usize)].set_hh_b1(c);
                            new_character = p;
                            break 'l_exit_f;
                        }
                    }
                }
            }
            self.char_warning(f, c);
            new_character = (268435455i32).wrapping_neg();
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
    // §633
    pub fn write_dvi(&mut self, mut a: dvi_index, mut b: dvi_index) {
        let mut k: dvi_index = 0; // §633
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
    // §634
    pub fn dvi_swap(&mut self) {
        if (self.dvi_ptr > (2147483647i32).wrapping_sub(self.dvi_offset)) {
            {
                self.cur_s = (2i32).wrapping_neg();
                self.fatal_error(66219i32);
            }
        }
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
    // §636
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

    /// The `dvi_four` procedure outputs four bytes in two's complement notation,
    /// without risking arithmetic overflow.
    // §636
    pub fn dvi_two(&mut self, mut s: UTF16_code) {
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (s / 256i32);
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = (s % 256i32);
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
    // §637
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
    // §638
    pub fn dvi_native_font_def(&mut self, mut f: internal_font_number) {
        let mut font_def_length: i32 = 0; // §638
        let mut i: i32 = 0; // §638
        {
            self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = define_native_font;
            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
            if (self.dvi_ptr == self.dvi_limit) {
                self.dvi_swap();
            }
        }
        self.dvi_four((f).wrapping_sub(1i32));
        font_def_length = self.make_font_def(f);
        {
            let __for_end_2 = (font_def_length).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                {
                    { let __ix483 = self.dvi_ptr; let __v484 = self.xdv_buffer[crate::ix::U((i) as usize)]; self.dvi_buf[crate::ix::U((__ix483) as usize)] = __v484; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                i = i.wrapping_add(1);
            }
        }
    }

    /// Here's a procedure that outputs a font definition. Since \TeX82 uses at
    /// most 256 different fonts per job, `fnt_def1` is always used as the command code.
    // §638
    pub fn dvi_font_def(&mut self, mut f: internal_font_number) {
        let mut k: pool_pointer = 0; // §638
        let mut l: i32 = 0; // §638
        if ((self.font_area[crate::ix::U((f) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag)) {
            self.dvi_native_font_def(f);
        } else {
            {
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
                    { let __ix485 = self.dvi_ptr; let __v486 = self.font_check[crate::ix::U((f) as usize)].b0(); self.dvi_buf[crate::ix::U((__ix485) as usize)] = __v486; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix487 = self.dvi_ptr; let __v488 = self.font_check[crate::ix::U((f) as usize)].b1(); self.dvi_buf[crate::ix::U((__ix487) as usize)] = __v488; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix489 = self.dvi_ptr; let __v490 = self.font_check[crate::ix::U((f) as usize)].b2(); self.dvi_buf[crate::ix::U((__ix489) as usize)] = __v490; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix491 = self.dvi_ptr; let __v492 = self.font_check[crate::ix::U((f) as usize)].b3(); self.dvi_buf[crate::ix::U((__ix491) as usize)] = __v492; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                self.dvi_four(self.font_size[crate::ix::U((f) as usize)]);
                self.dvi_four(self.font_dsize[crate::ix::U((f) as usize)]);
                {
                    { let __ix493 = self.dvi_ptr; let __v494 = self.length(self.font_area[crate::ix::U((f) as usize)]); self.dvi_buf[crate::ix::U((__ix493) as usize)] = __v494; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                // §639
                l = 0i32;
                k = self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_sub(65536i32)) as usize)];
                while ((l == 0i32) && (k < self.str_start[crate::ix::U((((self.font_name[crate::ix::U((f) as usize)]).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) {
                    {
                        if (self.str_pool[crate::ix::U((k) as usize)] == 58i32) {
                            l = (k).wrapping_sub(self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_sub(65536i32)) as usize)]);
                        }
                        k = (k).wrapping_add(1i32);
                    }
                }
                if (l == 0i32) {
                    l = self.length(self.font_name[crate::ix::U((f) as usize)]);
                }
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = l;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    let __for_end_4 = (self.str_start[crate::ix::U((((self.font_area[crate::ix::U((f) as usize)]).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
                    k = self.str_start[crate::ix::U(((self.font_area[crate::ix::U((f) as usize)]).wrapping_sub(65536i32)) as usize)];
                    while k <= __for_end_4 {
                        {
                            { let __ix495 = self.dvi_ptr; let __v496 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix495) as usize)] = __v496; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                {
                    let __for_end_4 = ((self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_sub(65536i32)) as usize)]).wrapping_add(l)).wrapping_sub(1i32);
                    k = self.str_start[crate::ix::U(((self.font_name[crate::ix::U((f) as usize)]).wrapping_sub(65536i32)) as usize)];
                    while k <= __for_end_4 {
                        {
                            { let __ix497 = self.dvi_ptr; let __v498 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix497) as usize)] = __v498; }
                            self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                            if (self.dvi_ptr == self.dvi_limit) {
                                self.dvi_swap();
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
            }
        }
    }

    /// Here is a subroutine that produces a \.{DVI} command for some specified
    /// downward or rightward motion. It has two parameters: `w` is the amount
    /// of motion, and `o` is either `down1` or `right1`. We use the fact that
    /// the command codes have convenient arithmetic properties: `y1-down1=w1-right1`
    /// and `z1-down1=x1-right1`.
    // §643
    pub fn movement(&mut self, mut w: scaled, mut o: eight_bits) {
        let mut mstate: small_number = 0; // §643
        let mut p: halfword = 0; // §643
        let mut q: halfword = 0; // §643
        let mut k: i32 = 0; // §643
        'l_exit_f: {
            'l_found_f: {
                'l_start_of_TEX_f: {
                    'l_L2_f: {
                        'l_not_found_f: {
                            q = self.get_node(movement_node_size);
                            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(w);
                            { let __v499 = (self.dvi_offset).wrapping_add(self.dvi_ptr); self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v499); }
                            if (o == down1) {
                                {
                                    { let __v500 = self.down_ptr; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v500); }
                                    self.down_ptr = q;
                                }
                            } else {
                                {
                                    { let __v501 = self.right_ptr; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v501); }
                                    self.right_ptr = q;
                                }
                            }
                            // §647
                            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            mstate = none_seen;
                            while (p != (268435455i32).wrapping_neg()) {
                                {
                                    if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == w) {
                                        // §648
                                        match (mstate).wrapping_add(self.mem[crate::ix::U((p) as usize)].hh().lh()) {
                                            3 | 4 | 15 | 16 => {
                                                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §649
                                                    {
                                                        k = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v502 = (self.dvi_buf[crate::ix::U((k) as usize)]).wrapping_add(5i32); self.dvi_buf[crate::ix::U((k) as usize)] = __v502; }
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(y_here);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            5 | 9 | 11 => {
                                                // §648
                                                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() < self.dvi_gone) {
                                                    break 'l_not_found_f;
                                                } else {
                                                    // §650
                                                    {
                                                        k = (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int()).wrapping_sub(self.dvi_offset);
                                                        if (k < 0i32) {
                                                            k = (k).wrapping_add(dvi_buf_size);
                                                        }
                                                        { let __v503 = (self.dvi_buf[crate::ix::U((k) as usize)]).wrapping_add(10i32); self.dvi_buf[crate::ix::U((k) as usize)] = __v503; }
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_lh(z_here);
                                                        break 'l_found_f;
                                                    }
                                                }
                                            }
                                            1 | 2 | 8 | 13 => {
                                                // §648
                                                break 'l_found_f;
                                            }
                                            _ => {
                                            }
                                        }
                                    } else {
                                        // §647
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
                        // §646
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
            // §643
            { let __v504 = self.mem[crate::ix::U((p) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v504); }
            // §645
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
        // §643
    }

    /// In case you are wondering when all the movement nodes are removed from
    /// \TeX's memory, the answer is that they are recycled just before
    /// `hlist_out` and `vlist_out` finish outputting a box. This restores the
    /// down and right stacks to the state they were in before the box was output,
    /// except that some `info`'s may have become more restrictive.
    // §651
    pub fn prune_movements(&mut self, mut l: i32) {
        let mut p: halfword = 0; // §651
        'l_exit_f: {
            'l_done_f: {
                while (self.down_ptr != (268435455i32).wrapping_neg()) {
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
            while (self.right_ptr != (268435455i32).wrapping_neg()) {
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
    // §1431
    pub fn special_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1431
        let mut k: pool_pointer = 0; // §1431
        let mut h: halfword = 0; // §1431
        let mut q: halfword = 0; // §1431
        let mut r: halfword = 0; // §1431
        let mut old_mode: i32 = 0; // §1431
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
        self.doing_special = true;
        old_setting = self.selector;
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
            {
                // §1434
                q = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(4194429i32);
                r = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                self.mem[crate::ix::U((r) as usize)].set_hh_lh(end_write_token);
                self.begin_token_list(q, inserted);
                self.begin_token_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), write_text);
                q = self.get_avail();
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(2097275i32);
                self.begin_token_list(q, inserted);
                old_mode = self.cur_list.mode_field;
                self.cur_list.mode_field = 0i32;
                self.cur_cs = self.write_loc;
                q = self.scan_toks(false, true);
                self.cur_list.mode_field = old_mode;
                self.get_token();
                if (self.cur_tok != end_write_token) {
                    // §1435
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(65938i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66769i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66440i32;
                        }
                        self.error();
                        loop {
                            self.get_token();
                            if (self.cur_tok == end_write_token) { break; }
                        }
                    }
                }
                // §1434
                self.end_token_list();
                // §1431
                h = self.def_ref;
            }
        } else {
            h = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
        }
        self.selector = new_string;
        self.show_token_list(self.mem[crate::ix::U((h) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = old_setting;
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]) < 256i32) {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = xxx1;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix505 = self.dvi_ptr; let __v506 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]); self.dvi_buf[crate::ix::U((__ix505) as usize)] = __v506; }
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
                self.dvi_four((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]));
            }
        }
        {
            let __for_end_2 = (self.pool_ptr).wrapping_sub(1i32);
            k = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
            while k <= __for_end_2 {
                {
                    { let __ix507 = self.dvi_ptr; let __v508 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix507) as usize)] = __v508; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
        if (self.mem[crate::ix::U((p) as usize)].hh().b1() == latespecial_node) {
            self.flush_list(self.def_ref);
        }
        self.doing_special = false;
    }

    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1433
    pub fn write_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1433
        let mut old_mode: i32 = 0; // §1433
        let mut j: small_number = 0; // §1433
        let mut k: i32 = 0; // §1433
        let mut q: halfword = 0; // §1433
        let mut r: halfword = 0; // §1433
        let mut d: i32 = 0; // §1433
        let mut clobbered: bool = false; // §1433
        let mut runsystem_ret: i32 = 0; // §1433
        // §1434
        q = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_lh(4194429i32);
        r = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
        self.mem[crate::ix::U((r) as usize)].set_hh_lh(end_write_token);
        self.begin_token_list(q, inserted);
        self.begin_token_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh(), write_text);
        q = self.get_avail();
        self.mem[crate::ix::U((q) as usize)].set_hh_lh(2097275i32);
        self.begin_token_list(q, inserted);
        old_mode = self.cur_list.mode_field;
        self.cur_list.mode_field = 0i32;
        self.cur_cs = self.write_loc;
        q = self.scan_toks(false, true);
        self.cur_list.mode_field = old_mode;
        self.get_token();
        if (self.cur_tok != end_write_token) {
            // §1435
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65938i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66769i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66440i32;
                }
                self.error();
                loop {
                    self.get_token();
                    if (self.cur_tok == end_write_token) { break; }
                }
            }
        }
        // §1434
        self.end_token_list();
        // §1433
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
                    self.print_nl(65626i32);
                }
            }
        }
        self.token_show(self.def_ref);
        self.print_ln();
        self.flush_list(self.def_ref);
        if (j == 18i32) {
            {
                if (self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int() <= 0i32) {
                    self.selector = log_only;
                } else {
                    self.selector = term_and_log;
                }
                if (!self.log_opened) {
                    self.selector = term_only;
                }
                self.print_nl(66761i32);
                {
                    let __for_end_4 = ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)])).wrapping_sub(1i32);
                    d = 0i32;
                    while d <= __for_end_4 {
                        {
                            self.print(self.str_pool[crate::ix::U(((self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]).wrapping_add(d)) as usize)]);
                        }
                        d = d.wrapping_add(1);
                    }
                }
                self.print(66762i32);
                if self.shellenabledp {
                    {
                        {
                            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                            }
                        }
                        {
                            if (0i32 > 65535i32) {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65536i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((0i32 % 1024i32)).wrapping_add(56320i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            } else {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 0i32;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                        clobbered = false;
                        {
                            let __for_end_6 = ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)])).wrapping_sub(1i32);
                            d = 0i32;
                            while d <= __for_end_6 {
                                {
                                    if ((self.str_pool[crate::ix::U(((self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]).wrapping_add(d)) as usize)] == null_code) && (d < ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)])).wrapping_sub(1i32))) {
                                        clobbered = true;
                                    }
                                }
                                d = d.wrapping_add(1);
                            }
                        }
                        if clobbered {
                            self.print(66763i32);
                        } else {
                            {
                                runsystem_ret = self.runsystem(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)], ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)])).wrapping_sub(1i32));
                                if (runsystem_ret == (1i32).wrapping_neg()) {
                                    self.print(66764i32);
                                } else {
                                    if (runsystem_ret == 0i32) {
                                        self.print(66765i32);
                                    } else {
                                        if (runsystem_ret == 1i32) {
                                            self.print(66766i32);
                                        } else {
                                            if (runsystem_ret == 2i32) {
                                                self.print(66767i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    {
                        self.print(66768i32);
                    }
                }
                self.print_char(46i32);
                self.print_nl(65626i32);
                self.print_ln();
                self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
            }
        }
        self.selector = old_setting;
    }

    /// The `out_what` procedure takes care of outputting whatsit nodes for
    /// `vlist_out` and `hlist_out`\kern-.3pt.
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1436
    pub fn pic_out(&mut self, mut p: halfword) {
        let mut old_setting: i32 = 0; // §1436
        let mut i: i32 = 0; // §1436
        let mut k: pool_pointer = 0; // §1436
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
        self.print(66770i32);
        self.print(66771i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().lh());
        self.print(32i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh());
        self.print(32i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().lh());
        self.print(32i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(6i32)) as usize)].hh().rh());
        self.print(32i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(7i32)) as usize)].hh().lh());
        self.print(32i32);
        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(7i32)) as usize)].hh().rh());
        self.print(32i32);
        self.print(66772i32);
        if (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b1() > 32767i32) {
            self.print_int((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b1()).wrapping_sub(65536i32));
        } else {
            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b1());
        }
        self.print(32i32);
        match self.mem[crate::ix::U(((p).wrapping_add(8i32)) as usize)].hh().b0() {
            pdfbox_crop => {
                self.print(66773i32);
            }
            pdfbox_media => {
                self.print(66774i32);
            }
            pdfbox_bleed => {
                self.print(66775i32);
            }
            pdfbox_art => {
                self.print(66776i32);
            }
            pdfbox_trim => {
                self.print(66777i32);
            }
            _ => {
            }
        }
        self.print(40i32);
        {
            let __for_end_2 = (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].hh().b0()).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                { let __a509_0 = self.pic_path_byte(p, i); let __a509_1 = true; self.print_raw_char(__a509_0, __a509_1) };
                i = i.wrapping_add(1);
            }
        }
        self.print(41i32);
        self.selector = old_setting;
        if ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]) < 256i32) {
            {
                {
                    self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = xxx1;
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                {
                    { let __ix510 = self.dvi_ptr; let __v511 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]); self.dvi_buf[crate::ix::U((__ix510) as usize)] = __v511; }
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
                self.dvi_four((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]));
            }
        }
        {
            let __for_end_2 = (self.pool_ptr).wrapping_sub(1i32);
            k = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
            while k <= __for_end_2 {
                {
                    { let __ix512 = self.dvi_ptr; let __v513 = self.str_pool[crate::ix::U((k) as usize)]; self.dvi_buf[crate::ix::U((__ix512) as usize)] = __v513; }
                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                    if (self.dvi_ptr == self.dvi_limit) {
                        self.dvi_swap();
                    }
                }
                k = k.wrapping_add(1);
            }
        }
        self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
    }

    /// The `out_what` procedure takes care of outputting whatsit nodes for
    /// `vlist_out` and `hlist_out`\kern-.3pt.
    /// @<Declare procedures needed in `hlist_out`, `vlist_out`
    // §1436
    pub fn out_what(&mut self, mut p: halfword) {
        let mut j: small_number = 0; // §1436
        let mut old_setting: i32 = 0; // §1436
        match self.mem[crate::ix::U((p) as usize)].hh().b1() {
            open_node | write_node | close_node => {
                // §1437
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
                                        { let __v514 = false; self.write_open[crate::ix::U((j) as usize)] = __v514; }
                                    }
                                }
                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == close_node) {
                                } else {
                                    if (j < 16i32) {
                                        {
                                            self.cur_name = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                            self.cur_area = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().lh();
                                            self.cur_ext = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].hh().rh();
                                            if (self.cur_ext == 65626i32) {
                                                self.cur_ext = 66173i32;
                                            }
                                            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
                                            while ((!self.kpse_out_name_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.write_file[crate::ix::U((j) as usize)]); let __r = self.a_open_out(&mut __f0); self.write_file[crate::ix::U((j) as usize)] = __f0; __r })) {
                                                self.prompt_file_name(66779i32, 66173i32);
                                            }
                                            { let __v515 = true; self.write_open[crate::ix::U((j) as usize)] = __v515; }
                                            if (self.log_opened && self.texmf_yesno_log_openout()) {
                                                {
                                                    old_setting = self.selector;
                                                    if (self.eqtb[crate::ix::U(((7892293i32) - 1) as usize)].int() <= 0i32) {
                                                        self.selector = log_only;
                                                    } else {
                                                        self.selector = term_and_log;
                                                    }
                                                    self.print_nl(66780i32);
                                                    self.print_int(j);
                                                    self.print(66781i32);
                                                    self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
                                                    self.print(66172i32);
                                                    self.print_nl(65626i32);
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
                // §1436
                self.special_out(p);
            }
            language_node => {
            }
            _ => {
                self.confusion(66778i32);
            }
        }
    }

    // §1529
    pub fn new_edge(&mut self, mut s: small_number, mut w: scaled) -> halfword {
        let mut new_edge: halfword = 0;
        let mut p: halfword = 0; // §1529
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
    // §1533
    pub fn reverse(&mut self, mut this_box: halfword, mut t: halfword, cur_g: &mut scaled, cur_glue: &mut f64) -> halfword {
        let mut reverse: halfword = 0;
        let mut l: halfword = 0; // §1533
        let mut p: halfword = 0; // §1533
        let mut q: halfword = 0; // §1533
        let mut g_order: glue_ord = 0; // §1533
        let mut g_sign: i32 = 0; // §1533
        let mut glue_temp: f64 = 0.0; // §1533
        let mut m: halfword = 0; // §1533
        let mut n: halfword = 0; // §1533
        'l_done_f: {
            g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
            g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
            l = t;
            p = self.temp_ptr;
            m = (268435455i32).wrapping_neg();
            n = (268435455i32).wrapping_neg();
            while true {
                {
                    while (p != (268435455i32).wrapping_neg()) {
                        'l_reswitch_b: loop {
                            // §1534
                            if (p >= self.hi_mem_min) {
                                loop {
                                    self.f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    self.c = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    self.cur_h = (self.cur_h).wrapping_add({ let __s517 = ((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add({ let __s516 = ((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.effective_char(true, self.f, self.c))) as usize; self.font_info[crate::ix::U(__s516)] }.qqqq().b0())) as usize; self.font_info[crate::ix::U(__s517)] }.int());
                                    q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                                    l = p;
                                    p = q;
                                    if (!(p >= self.hi_mem_min)) { break; }
                                }
                            } else {
                                // §1535
                                {
                                    'l_L15_f: {
                                        q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                        match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                            hlist_node | vlist_node | rule_node | kern_node => {
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                            }
                                            whatsit_node => {
                                                // §1536
                                                if (((((self.mem[crate::ix::U((p) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= native_word_node_AT)) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pic_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_node)) {
                                                    self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                } else {
                                                    break 'l_L15_f;
                                                }
                                            }
                                            glue_node => {
                                                // §1537
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
                                                    // §1509
                                                    if (((g_sign == stretching) && (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == shrinking) && (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                                    self.free_node(self.g, glue_spec_size);
                                                                } else {
                                                                    { let __ix518 = self.g; let __v519 = (self.mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix518) as usize)].set_hh_rh(__v519); }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() < a_leaders) {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    { let __v520 = self.rule_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v520); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(glue_spec_size);
                                                                    { let __ix521 = self.g; self.mem[crate::ix::U((__ix521) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix522 = self.g; self.mem[crate::ix::U((__ix522) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix523 = (self.g).wrapping_add(1i32); let __v524 = self.rule_wd; self.mem[crate::ix::U((__ix523) as usize)].set_int(__v524); }
                                                                    { let __ix525 = (self.g).wrapping_add(2i32); self.mem[crate::ix::U((__ix525) as usize)].set_int(0i32); }
                                                                    { let __ix526 = (self.g).wrapping_add(3i32); self.mem[crate::ix::U((__ix526) as usize)].set_int(0i32); }
                                                                    { let __v527 = self.g; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v527); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            ligature_node => {
                                                // §1538
                                                {
                                                    self.flush_node_list(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh());
                                                    self.temp_ptr = p;
                                                    p = self.get_avail();
                                                    { let __v528 = self.mem[crate::ix::U(((self.temp_ptr).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((p) as usize)] = __v528; }
                                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                    self.free_node(self.temp_ptr, small_node_size);
                                                    continue 'l_reswitch_b;
                                                }
                                            }
                                            math_node => {
                                                // §1539
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
                                                                        { let __ix529 = self.temp_ptr; let __v530 = self.avail; self.mem[crate::ix::U((__ix529) as usize)].set_hh_rh(__v530); }
                                                                        self.avail = self.temp_ptr;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                                if (n > (268435455i32).wrapping_neg()) {
                                                                    {
                                                                        n = (n).wrapping_sub(1i32);
                                                                        { let __v531 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_sub(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v531); }
                                                                    }
                                                                } else {
                                                                    {
                                                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                        if (m > (268435455i32).wrapping_neg()) {
                                                                            m = (m).wrapping_sub(1i32);
                                                                        } else {
                                                                            // §1540
                                                                            {
                                                                                self.free_node(p, medium_node_size);
                                                                                self.mem[crate::ix::U((t) as usize)].set_hh_rh(q);
                                                                                { let __v532 = self.rule_wd; self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v532); }
                                                                                { let __v533 = ((self.cur_h).wrapping_neg()).wrapping_sub(self.rule_wd); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v533); }
                                                                                break 'l_done_f;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        // §1539
                                                        {
                                                            {
                                                                self.temp_ptr = self.get_avail();
                                                                { let __ix534 = self.temp_ptr; let __v535 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix534) as usize)].set_hh_lh(__v535); }
                                                                { let __ix536 = self.temp_ptr; let __v537 = self.LR_ptr; self.mem[crate::ix::U((__ix536) as usize)].set_hh_rh(__v537); }
                                                                self.LR_ptr = self.temp_ptr;
                                                            }
                                                            if ((n > (268435455i32).wrapping_neg()) || ((self.mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir)) {
                                                                {
                                                                    n = (n).wrapping_add(1i32);
                                                                    { let __v538 = (self.mem[crate::ix::U((p) as usize)].hh().b1()).wrapping_add(1i32); self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v538); }
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
                                                // §1535
                                                self.confusion(66918i32);
                                            }
                                            _ => {
                                                break 'l_L15_f;
                                            }
                                        }
                                        self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                                    }
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(l);
                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) {
                                        if ((self.rule_wd == 0i32) || (l == (268435455i32).wrapping_neg())) {
                                            {
                                                self.free_node(p, medium_node_size);
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
                    // §1533
                    if (((t == (268435455i32).wrapping_neg()) && (m == (268435455i32).wrapping_neg())) && (n == (268435455i32).wrapping_neg())) {
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
    // §655
    pub fn hlist_out(&mut self) {
        let mut base_line: scaled = 0; // §655
        let mut left_edge: scaled = 0; // §655
        let mut save_h: scaled = 0; // §655
        let mut save_v: scaled = 0; // §655
        let mut this_box: halfword = 0; // §655
        let mut g_order: glue_ord = 0; // §655
        let mut g_sign: i32 = 0; // §655
        let mut p: halfword = 0; // §655
        let mut save_loc: i32 = 0; // §655
        let mut leader_box: halfword = 0; // §655
        let mut leader_wd: scaled = 0; // §655
        let mut lx: scaled = 0; // §655
        let mut outer_doing_leaders: bool = false; // §655
        let mut edge: scaled = 0; // §655
        let mut prev_p: halfword = 0; // §655
        let mut len: i32 = 0; // §655
        let mut q: halfword = 0; // §655
        let mut r: halfword = 0; // §655
        let mut k: i32 = 0; // §655
        let mut j: i32 = 0; // §655
        let mut glue_temp: f64 = 0.0; // §655
        let mut cur_glue: f64 = 0.0; // §655
        let mut cur_g: scaled = 0; // §655
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        if (self.eqtb[crate::ix::U(((7892348i32) - 1) as usize)].int() > 1i32) {
            {
                // §656
                p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
                prev_p = (this_box).wrapping_add(5i32);
                while (p != (268435455i32).wrapping_neg()) {
                    {
                        if (self.mem[crate::ix::U((p) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                            {
                                if (((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((p) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= native_word_node_AT))) && (self.font_letter_space[crate::ix::U((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1()) as usize)] == 0i32)) {
                                    {
                                        // goto labels: L1236, L1237
                                        let mut __goto_1: i32 = 0;
                                        'l_dispatch_1: loop {
                                            if __goto_1 <= 0 {
                                                r = p;
                                                k = self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b2();
                                                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                            }
                                            if __goto_1 <= 1 { // L1236
                                                while (((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (((((self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == ins_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == adjust_node)) || ((self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= 4i32)))) {
                                                    // §657
                                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                }
                                                // §656
                                                if ((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) {
                                                    {
                                                        if ((self.mem[crate::ix::U((q) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == normal)) {
                                                            {
                                                                if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh() == self.font_glue[crate::ix::U((self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b1()) as usize)]) {
                                                                    {
                                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        // §657
                                                                        while (((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (((((self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == ins_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == adjust_node)) || ((self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= 4i32)))) {
                                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        }
                                                                        // §656
                                                                        if (((((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((q) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b1() == self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b1())) {
                                                                            {
                                                                                p = q;
                                                                                k = ((k).wrapping_add(1i32)).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b2());
                                                                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                }
                                                                if ((((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == kern_node)) && (self.mem[crate::ix::U((q) as usize)].hh().b1() == space_adjustment)) {
                                                                    {
                                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        // §657
                                                                        while (((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (((((self.mem[crate::ix::U((q) as usize)].hh().b0() == penalty_node) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == ins_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((q) as usize)].hh().b0() == adjust_node)) || ((self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= 4i32)))) {
                                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        }
                                                                        // §656
                                                                        if (((((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((q) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b1() == self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b1())) {
                                                                            {
                                                                                p = q;
                                                                                k = ((k).wrapping_add(1i32)).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b2());
                                                                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                { __goto_1 = 2; continue 'l_dispatch_1; }
                                                            }
                                                        }
                                                        if (((((q != (268435455i32).wrapping_neg()) && (!(q >= self.hi_mem_min))) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((q) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b1() == self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b1())) {
                                                            {
                                                                p = q;
                                                                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if __goto_1 <= 2 { // L1237
                                                if (p != r) {
                                                    {
                                                        {
                                                            if ((self.pool_ptr).wrapping_add(k) > pool_size) {
                                                                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                                            }
                                                        }
                                                        k = 0i32;
                                                        q = r;
                                                        while true {
                                                            {
                                                                if (self.mem[crate::ix::U((q) as usize)].hh().b0() == whatsit_node) {
                                                                    {
                                                                        if ((self.mem[crate::ix::U((q) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((q) as usize)].hh().b1() <= native_word_node_AT)) {
                                                                            {
                                                                                {
                                                                                    let __for_end_20 = (self.mem[crate::ix::U(((q).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                    j = 0i32;
                                                                                    while j <= __for_end_20 {
                                                                                        {
                                                                                            if (self.get_native_char(q, j) > 65535i32) {
                                                                                                {
                                                                                                    { let __ix539 = self.pool_ptr; let __v540 = (((self.get_native_char(q, j)).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.str_pool[crate::ix::U((__ix539) as usize)] = __v540; }
                                                                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                                    { let __ix541 = self.pool_ptr; let __v542 = ((self.get_native_char(q, j) % 1024i32)).wrapping_add(56320i32); self.str_pool[crate::ix::U((__ix541) as usize)] = __v542; }
                                                                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                                }
                                                                                            } else {
                                                                                                {
                                                                                                    { let __ix543 = self.pool_ptr; let __v544 = self.get_native_char(q, j); self.str_pool[crate::ix::U((__ix543) as usize)] = __v544; }
                                                                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        j = j.wrapping_add(1);
                                                                                    }
                                                                                }
                                                                                k = (k).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == glue_node) {
                                                                        {
                                                                            {
                                                                                if (32i32 > 65535i32) {
                                                                                    {
                                                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((65504i32).wrapping_neg() / 1024i32)).wrapping_add(55296i32);
                                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((32i32 % 1024i32)).wrapping_add(56320i32);
                                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                } else {
                                                                                    {
                                                                                        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
                                                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                            self.g = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().lh();
                                                                            k = (k).wrapping_add(self.mem[crate::ix::U(((self.g).wrapping_add(1i32)) as usize)].int());
                                                                            if (g_sign != normal) {
                                                                                {
                                                                                    if (g_sign == stretching) {
                                                                                        {
                                                                                            if (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order) {
                                                                                                {
                                                                                                    k = (k).wrapping_add(crate::system::pas_round((self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * ((self.mem[crate::ix::U(((self.g).wrapping_add(2i32)) as usize)].int()) as f64))));
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        {
                                                                                            if (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order) {
                                                                                                {
                                                                                                    k = (k).wrapping_sub(crate::system::pas_round((self.mem[crate::ix::U(((this_box).wrapping_add(6i32)) as usize)].gr() * ((self.mem[crate::ix::U(((self.g).wrapping_add(3i32)) as usize)].int()) as f64))));
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        if (self.mem[crate::ix::U((q) as usize)].hh().b0() == kern_node) {
                                                                            {
                                                                                k = (k).wrapping_add(self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int());
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                if (q == p) {
                                                                    break;
                                                                } else {
                                                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                }
                                                            }
                                                        }
                                                        q = self.new_native_word_node(self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].qqqq().b1(), (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]));
                                                        { let __v545 = self.mem[crate::ix::U((r) as usize)].hh().b1(); self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v545); }
                                                        {
                                                            let __for_end_14 = ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)])).wrapping_sub(1i32);
                                                            j = 0i32;
                                                            while j <= __for_end_14 {
                                                                self.set_native_char(q, j, self.str_pool[crate::ix::U(((self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]).wrapping_add(j)) as usize)]);
                                                                j = j.wrapping_add(1);
                                                            }
                                                        }
                                                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(k);
                                                        self.set_justified_native_glyphs(q);
                                                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(q);
                                                        { let __v546 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v546); }
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                        prev_p = r;
                                                        p = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                                        while (p != (268435455i32).wrapping_neg()) {
                                                            {
                                                                if ((!(p >= self.hi_mem_min)) && (((((self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node)) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= 4i32)))) {
                                                                    {
                                                                        { let __v547 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(__v547); }
                                                                        { let __v548 = self.mem[crate::ix::U((q) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v548); }
                                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                                        q = p;
                                                                    }
                                                                }
                                                                prev_p = p;
                                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            }
                                                        }
                                                        self.flush_node_list(r);
                                                        self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                                                        p = q;
                                                    }
                                                }
                                            }
                                            break 'l_dispatch_1;
                                        }
                                    }
                                }
                                prev_p = p;
                            }
                        }
                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
            }
        }
        // §655
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
        // §1524
        if (self.eTeX_mode == 1i32) {
            {
                // §1520
                {
                    self.temp_ptr = self.get_avail();
                    { let __ix549 = self.temp_ptr; self.mem[crate::ix::U((__ix549) as usize)].set_hh_lh(before); }
                    { let __ix550 = self.temp_ptr; let __v551 = self.LR_ptr; self.mem[crate::ix::U((__ix550) as usize)].set_hh_rh(__v551); }
                    self.LR_ptr = self.temp_ptr;
                }
                // §1524
                if (self.mem[crate::ix::U((this_box) as usize)].hh().b1() == dlist) {
                    if (self.cur_dir == right_to_left) {
                        {
                            self.cur_dir = left_to_right;
                            self.cur_h = (self.cur_h).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int());
                        }
                    } else {
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(0i32);
                    }
                }
                if ((self.cur_dir == right_to_left) && (self.mem[crate::ix::U((this_box) as usize)].hh().b1() != reversed)) {
                    // §1531
                    {
                        save_h = self.cur_h;
                        self.temp_ptr = p;
                        p = self.new_kern(0i32);
                        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh(0i32);
                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                        self.cur_h = 0i32;
                        { let __v552 = { let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, (268435455i32).wrapping_neg(), &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v552); }
                        { let __v553 = (self.cur_h).wrapping_neg(); self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v553); }
                        self.cur_h = save_h;
                        self.mem[crate::ix::U((this_box) as usize)].set_hh_b1(reversed);
                    }
                }
            }
        }
        // §655
        left_edge = self.cur_h;
        while (p != (268435455i32).wrapping_neg()) {
            'l_reswitch_b: loop {
                // §658
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
                            if ((p != lig_trick) && (self.font_mapping[crate::ix::U((self.f) as usize)] != nil)) {
                                self.c = self.apply_tfm_font_mapping(self.font_mapping[crate::ix::U((self.f) as usize)], self.c);
                            }
                            if (self.f != self.dvi_f) {
                                // §659
                                {
                                    if (!self.font_used[crate::ix::U((self.f) as usize)]) {
                                        {
                                            self.dvi_font_def(self.f);
                                            { let __ix554 = self.f; let __v555 = true; self.font_used[crate::ix::U((__ix554) as usize)] = __v555; }
                                        }
                                    }
                                    if (self.f <= 64i32) {
                                        {
                                            { let __ix556 = self.dvi_ptr; let __v557 = (self.f).wrapping_add(170i32); self.dvi_buf[crate::ix::U((__ix556) as usize)] = __v557; }
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
                                                    { let __ix558 = self.dvi_ptr; let __v559 = (self.f).wrapping_sub(1i32); self.dvi_buf[crate::ix::U((__ix558) as usize)] = __v559; }
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
                                                    { let __ix560 = self.dvi_ptr; let __v561 = ((self.f).wrapping_sub(1i32) / 256i32); self.dvi_buf[crate::ix::U((__ix560) as usize)] = __v561; }
                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                        self.dvi_swap();
                                                    }
                                                }
                                                {
                                                    { let __ix562 = self.dvi_ptr; let __v563 = ((self.f).wrapping_sub(1i32) % 256i32); self.dvi_buf[crate::ix::U((__ix562) as usize)] = __v563; }
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
                            // §658
                            if (self.font_ec[crate::ix::U((self.f) as usize)] >= self.c) {
                                if (self.font_bc[crate::ix::U((self.f) as usize)] <= self.c) {
                                    if (self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0() > min_quarterword) {
                                        {
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
                                                { let __ix564 = self.dvi_ptr; let __v565 = self.c; self.dvi_buf[crate::ix::U((__ix564) as usize)] = __v565; }
                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                if (self.dvi_ptr == self.dvi_limit) {
                                                    self.dvi_swap();
                                                }
                                            }
                                            self.cur_h = (self.cur_h).wrapping_add(self.font_info[crate::ix::U(((self.width_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.font_info[crate::ix::U(((self.char_base[crate::ix::U((self.f) as usize)]).wrapping_add(self.c)) as usize)].qqqq().b0())) as usize)].int());
                                        }
                                    }
                                }
                            }
                            prev_p = self.mem[crate::ix::U((prev_p) as usize)].hh().rh();
                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            if (!(p >= self.hi_mem_min)) { break; }
                        }
                        self.dvi_h = self.cur_h;
                    }
                } else {
                    // §660
                    {
                        'l_L15_f: {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §661
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
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
                                            // §660
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1430
                                            {
                                                match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                                    native_word_node | native_word_node_AT | glyph_node => {
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
                                                            self.f = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1();
                                                            if (self.f != self.dvi_f) {
                                                                // §659
                                                                {
                                                                    if (!self.font_used[crate::ix::U((self.f) as usize)]) {
                                                                        {
                                                                            self.dvi_font_def(self.f);
                                                                            { let __ix566 = self.f; let __v567 = true; self.font_used[crate::ix::U((__ix566) as usize)] = __v567; }
                                                                        }
                                                                    }
                                                                    if (self.f <= 64i32) {
                                                                        {
                                                                            { let __ix568 = self.dvi_ptr; let __v569 = (self.f).wrapping_add(170i32); self.dvi_buf[crate::ix::U((__ix568) as usize)] = __v569; }
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
                                                                                    { let __ix570 = self.dvi_ptr; let __v571 = (self.f).wrapping_sub(1i32); self.dvi_buf[crate::ix::U((__ix570) as usize)] = __v571; }
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
                                                                                    { let __ix572 = self.dvi_ptr; let __v573 = ((self.f).wrapping_sub(1i32) / 256i32); self.dvi_buf[crate::ix::U((__ix572) as usize)] = __v573; }
                                                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                                                        self.dvi_swap();
                                                                                    }
                                                                                }
                                                                                {
                                                                                    { let __ix574 = self.dvi_ptr; let __v575 = ((self.f).wrapping_sub(1i32) % 256i32); self.dvi_buf[crate::ix::U((__ix574) as usize)] = __v575; }
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
                                                            // §1430
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node) {
                                                                {
                                                                    {
                                                                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set_glyphs;
                                                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                        if (self.dvi_ptr == self.dvi_limit) {
                                                                            self.dvi_swap();
                                                                        }
                                                                    }
                                                                    self.dvi_four(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                                    self.dvi_two(1i32);
                                                                    self.dvi_four(0i32);
                                                                    self.dvi_four(0i32);
                                                                    self.dvi_two(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                                                    self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b1() == native_word_node_AT) {
                                                                        {
                                                                            if ((self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2() > 0i32) || (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].int() != null_ptr)) {
                                                                                {
                                                                                    {
                                                                                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set_text_and_glyphs;
                                                                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                        if (self.dvi_ptr == self.dvi_limit) {
                                                                                            self.dvi_swap();
                                                                                        }
                                                                                    }
                                                                                    len = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2();
                                                                                    self.dvi_two(len);
                                                                                    {
                                                                                        let __for_end_21 = (len).wrapping_sub(1i32);
                                                                                        k = 0i32;
                                                                                        while k <= __for_end_21 {
                                                                                            {
                                                                                                { let __a576_0 = self.get_native_char(p, k); self.dvi_two(__a576_0) };
                                                                                            }
                                                                                            k = k.wrapping_add(1);
                                                                                        }
                                                                                    }
                                                                                    len = self.make_xdv_glyph_array_data(p);
                                                                                    {
                                                                                        let __for_end_21 = (len).wrapping_sub(1i32);
                                                                                        k = 0i32;
                                                                                        while k <= __for_end_21 {
                                                                                            {
                                                                                                { let __ix577 = self.dvi_ptr; let __v578 = self.xdv_buffer_byte(k); self.dvi_buf[crate::ix::U((__ix577) as usize)] = __v578; }
                                                                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                                if (self.dvi_ptr == self.dvi_limit) {
                                                                                                    self.dvi_swap();
                                                                                                }
                                                                                            }
                                                                                            k = k.wrapping_add(1);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        {
                                                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].int() != null_ptr) {
                                                                                {
                                                                                    {
                                                                                        self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set_glyphs;
                                                                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                        if (self.dvi_ptr == self.dvi_limit) {
                                                                                            self.dvi_swap();
                                                                                        }
                                                                                    }
                                                                                    len = self.make_xdv_glyph_array_data(p);
                                                                                    {
                                                                                        let __for_end_21 = (len).wrapping_sub(1i32);
                                                                                        k = 0i32;
                                                                                        while k <= __for_end_21 {
                                                                                            {
                                                                                                { let __ix579 = self.dvi_ptr; let __v580 = self.xdv_buffer_byte(k); self.dvi_buf[crate::ix::U((__ix579) as usize)] = __v580; }
                                                                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                                if (self.dvi_ptr == self.dvi_limit) {
                                                                                                    self.dvi_swap();
                                                                                                }
                                                                                            }
                                                                                            k = k.wrapping_add(1);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                                }
                                                            }
                                                            self.dvi_h = self.cur_h;
                                                        }
                                                    }
                                                    pic_node | pdf_node => {
                                                        {
                                                            save_h = self.dvi_h;
                                                            save_v = self.dvi_v;
                                                            self.cur_v = base_line;
                                                            edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                            self.pic_out(p);
                                                            self.dvi_h = save_h;
                                                            self.dvi_v = save_v;
                                                            self.cur_h = edge;
                                                            self.cur_v = base_line;
                                                        }
                                                    }
                                                    pdf_save_pos_node => {
                                                        // §1427
                                                        {
                                                            self.pdf_last_x_pos = (self.cur_h).wrapping_add(4736286i32);
                                                            self.pdf_last_y_pos = ((self.cur_page_height).wrapping_sub(self.cur_v)).wrapping_sub(4736286i32);
                                                        }
                                                    }
                                                    _ => {
                                                        // §1430
                                                        self.out_what(p);
                                                    }
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §663
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
                                                    // §1509
                                                    if (((g_sign == stretching) && (self.mem[crate::ix::U((self.g) as usize)].hh().b0() == g_order)) || ((g_sign == shrinking) && (self.mem[crate::ix::U((self.g) as usize)].hh().b1() == g_order))) {
                                                        {
                                                            {
                                                                if (self.mem[crate::ix::U((self.g) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                                    self.free_node(self.g, glue_spec_size);
                                                                } else {
                                                                    { let __ix581 = self.g; let __v582 = (self.mem[crate::ix::U((self.g) as usize)].hh().rh()).wrapping_sub(1i32); self.mem[crate::ix::U((__ix581) as usize)].set_hh_rh(__v582); }
                                                                }
                                                            }
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() < a_leaders) {
                                                                {
                                                                    self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                    { let __v583 = self.rule_wd; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v583); }
                                                                }
                                                            } else {
                                                                {
                                                                    self.g = self.get_node(glue_spec_size);
                                                                    { let __ix584 = self.g; self.mem[crate::ix::U((__ix584) as usize)].set_hh_b0(4i32); }
                                                                    { let __ix585 = self.g; self.mem[crate::ix::U((__ix585) as usize)].set_hh_b1(4i32); }
                                                                    { let __ix586 = (self.g).wrapping_add(1i32); let __v587 = self.rule_wd; self.mem[crate::ix::U((__ix586) as usize)].set_int(__v587); }
                                                                    { let __ix588 = (self.g).wrapping_add(2i32); self.mem[crate::ix::U((__ix588) as usize)].set_int(0i32); }
                                                                    { let __ix589 = (self.g).wrapping_add(3i32); self.mem[crate::ix::U((__ix589) as usize)].set_int(0i32); }
                                                                    { let __v590 = self.g; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v590); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // §663
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                                    // §664
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
                                                                // §665
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
                                                                // §664
                                                                while ((self.cur_h).wrapping_add(leader_wd) <= edge) {
                                                                    // §666
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
                                                                // §664
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
                                                // §663
                                                break 'l_L13_f;
                                            }
                                        }
                                        margin_kern_node => {
                                            // §660
                                            {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        kern_node => {
                                            self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        math_node => {
                                            // §1526
                                            {
                                                if (self.eTeX_mode == 1i32) {
                                                    // §1527
                                                    {
                                                        if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                            if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                                {
                                                                    self.temp_ptr = self.LR_ptr;
                                                                    self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                    {
                                                                        { let __ix591 = self.temp_ptr; let __v592 = self.avail; self.mem[crate::ix::U((__ix591) as usize)].set_hh_rh(__v592); }
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
                                                                    { let __ix593 = self.temp_ptr; let __v594 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix593) as usize)].set_hh_lh(__v594); }
                                                                    { let __ix595 = self.temp_ptr; let __v596 = self.LR_ptr; self.mem[crate::ix::U((__ix595) as usize)].set_hh_rh(__v596); }
                                                                    self.LR_ptr = self.temp_ptr;
                                                                }
                                                                if ((self.mem[crate::ix::U((p) as usize)].hh().b1() / R_code) != self.cur_dir) {
                                                                    // §1532
                                                                    {
                                                                        save_h = self.cur_h;
                                                                        self.temp_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                        self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                                        self.free_node(p, medium_node_size);
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        p = self.new_edge(self.cur_dir, self.rule_wd);
                                                                        self.mem[crate::ix::U((prev_p) as usize)].set_hh_rh(p);
                                                                        self.cur_h = ((self.cur_h).wrapping_sub(left_edge)).wrapping_add(self.rule_wd);
                                                                        { let __v597 = { let __a1 = self.new_edge((1i32).wrapping_sub(self.cur_dir), 0i32); let mut __f2 = ::core::mem::take(&mut cur_g); let mut __f3 = ::core::mem::take(&mut cur_glue); let __r = self.reverse(this_box, __a1, &mut __f2, &mut __f3); cur_g = __f2; cur_glue = __f3; __r }; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v597); }
                                                                        { let __v598 = self.cur_h; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(__v598); }
                                                                        self.cur_dir = (1i32).wrapping_sub(self.cur_dir);
                                                                        self.cur_h = save_h;
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §1527
                                                        self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                    }
                                                }
                                                // §1526
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        ligature_node => {
                                            // §692
                                            {
                                                { let __v599 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((lig_trick) as usize)] = __v599; }
                                                { let __v600 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v600); }
                                                p = lig_trick;
                                                self.xtx_ligature_present = true;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        edge_node => {
                                            // §1530
                                            {
                                                self.cur_h = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                left_edge = (self.cur_h).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                self.cur_dir = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                            }
                                        }
                                        _ => {
                                            // §660
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_ht == (1073741824i32).wrapping_neg()) {
                                    // §662
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
                            // §660
                            self.cur_h = (self.cur_h).wrapping_add(self.rule_wd);
                        }
                        prev_p = p;
                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                }
                break 'l_reswitch_b;
            }
        }
        // §1525
        if (self.eTeX_mode == 1i32) {
            {
                // §1528
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
                                    { let __ix601 = self.temp_ptr; let __v602 = self.avail; self.mem[crate::ix::U((__ix601) as usize)].set_hh_rh(__v602); }
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
                            { let __ix603 = self.temp_ptr; let __v604 = self.avail; self.mem[crate::ix::U((__ix603) as usize)].set_hh_rh(__v604); }
                            self.avail = self.temp_ptr;
                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                        }
                    }
                }
                // §1525
                if (self.mem[crate::ix::U((this_box) as usize)].hh().b1() == dlist) {
                    self.cur_dir = right_to_left;
                }
            }
        }
        // §655
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `vlist_out` routine is similar to `hlist_out`, but a bit simpler.
    // §667
    pub fn vlist_out(&mut self) {
        let mut left_edge: scaled = 0; // §667
        let mut top_edge: scaled = 0; // §667
        let mut save_h: scaled = 0; // §667
        let mut save_v: scaled = 0; // §667
        let mut this_box: halfword = 0; // §667
        let mut g_order: glue_ord = 0; // §667
        let mut g_sign: i32 = 0; // §667
        let mut p: halfword = 0; // §667
        let mut save_loc: i32 = 0; // §667
        let mut leader_box: halfword = 0; // §667
        let mut leader_ht: scaled = 0; // §667
        let mut lx: scaled = 0; // §667
        let mut outer_doing_leaders: bool = false; // §667
        let mut edge: scaled = 0; // §667
        let mut glue_temp: f64 = 0.0; // §667
        let mut cur_glue: f64 = 0.0; // §667
        let mut cur_g: scaled = 0; // §667
        let mut upwards: bool = false; // §667
        cur_g = 0i32;
        cur_glue = 0.0f64;
        this_box = self.temp_ptr;
        g_order = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b1();
        g_sign = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().b0();
        p = self.mem[crate::ix::U(((this_box).wrapping_add(5i32)) as usize)].hh().rh();
        upwards = (self.mem[crate::ix::U((this_box) as usize)].hh().b1() == 1i32);
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
        if upwards {
            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((this_box).wrapping_add(2i32)) as usize)].int());
        } else {
            self.cur_v = (self.cur_v).wrapping_sub(self.mem[crate::ix::U(((this_box).wrapping_add(3i32)) as usize)].int());
        }
        top_edge = self.cur_v;
        while (p != (268435455i32).wrapping_neg()) {
            // §668
            {
                'l_L15_f: {
                    if (p >= self.hi_mem_min) {
                        self.confusion(66221i32);
                    } else {
                        // §669
                        {
                            'l_L13_f: {
                                'l_L14_f: {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node => {
                                            // §670
                                            if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                                {
                                                    if upwards {
                                                        self.cur_v = ((self.cur_v).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    } else {
                                                        self.cur_v = ((self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int())).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    }
                                                }
                                            } else {
                                                {
                                                    if upwards {
                                                        self.cur_v = (self.cur_v).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    } else {
                                                        self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    }
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
                                                    if upwards {
                                                        self.cur_v = (save_v).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                    } else {
                                                        self.cur_v = (save_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                    }
                                                    self.cur_h = left_edge;
                                                }
                                            }
                                        }
                                        rule_node => {
                                            // §669
                                            {
                                                self.rule_ht = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                self.rule_dp = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                self.rule_wd = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                break 'l_L14_f;
                                            }
                                        }
                                        whatsit_node => {
                                            // §1426
                                            {
                                                match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                                    glyph_node => {
                                                        {
                                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                            self.cur_h = left_edge;
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
                                                            self.f = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1();
                                                            if (self.f != self.dvi_f) {
                                                                // §659
                                                                {
                                                                    if (!self.font_used[crate::ix::U((self.f) as usize)]) {
                                                                        {
                                                                            self.dvi_font_def(self.f);
                                                                            { let __ix605 = self.f; let __v606 = true; self.font_used[crate::ix::U((__ix605) as usize)] = __v606; }
                                                                        }
                                                                    }
                                                                    if (self.f <= 64i32) {
                                                                        {
                                                                            { let __ix607 = self.dvi_ptr; let __v608 = (self.f).wrapping_add(170i32); self.dvi_buf[crate::ix::U((__ix607) as usize)] = __v608; }
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
                                                                                    { let __ix609 = self.dvi_ptr; let __v610 = (self.f).wrapping_sub(1i32); self.dvi_buf[crate::ix::U((__ix609) as usize)] = __v610; }
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
                                                                                    { let __ix611 = self.dvi_ptr; let __v612 = ((self.f).wrapping_sub(1i32) / 256i32); self.dvi_buf[crate::ix::U((__ix611) as usize)] = __v612; }
                                                                                    self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                                    if (self.dvi_ptr == self.dvi_limit) {
                                                                                        self.dvi_swap();
                                                                                    }
                                                                                }
                                                                                {
                                                                                    { let __ix613 = self.dvi_ptr; let __v614 = ((self.f).wrapping_sub(1i32) % 256i32); self.dvi_buf[crate::ix::U((__ix613) as usize)] = __v614; }
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
                                                            // §1426
                                                            {
                                                                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = set_glyphs;
                                                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                                                if (self.dvi_ptr == self.dvi_limit) {
                                                                    self.dvi_swap();
                                                                }
                                                            }
                                                            self.dvi_four(0i32);
                                                            self.dvi_two(1i32);
                                                            self.dvi_four(0i32);
                                                            self.dvi_four(0i32);
                                                            self.dvi_two(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                            self.cur_h = left_edge;
                                                        }
                                                    }
                                                    pic_node | pdf_node => {
                                                        {
                                                            save_h = self.dvi_h;
                                                            save_v = self.dvi_v;
                                                            self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                            self.pic_out(p);
                                                            self.dvi_h = save_h;
                                                            self.dvi_v = save_v;
                                                            self.cur_v = (save_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                                                            self.cur_h = left_edge;
                                                        }
                                                    }
                                                    pdf_save_pos_node => {
                                                        // §1427
                                                        {
                                                            self.pdf_last_x_pos = (self.cur_h).wrapping_add(4736286i32);
                                                            self.pdf_last_y_pos = ((self.cur_page_height).wrapping_sub(self.cur_v)).wrapping_sub(4736286i32);
                                                        }
                                                    }
                                                    _ => {
                                                        // §1426
                                                        self.out_what(p);
                                                    }
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §672
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
                                                    // §673
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
                                                                // §674
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
                                                                // §673
                                                                while ((self.cur_v).wrapping_add(leader_ht) <= edge) {
                                                                    // §675
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
                                                                // §673
                                                                self.cur_v = (edge).wrapping_sub(10i32);
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                }
                                                // §672
                                                break 'l_L13_f;
                                            }
                                        }
                                        kern_node => {
                                            // §669
                                            if upwards {
                                                self.cur_v = (self.cur_v).wrapping_sub(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            } else {
                                                self.cur_v = (self.cur_v).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                            }
                                        }
                                        _ => {
                                        }
                                    }
                                    break 'l_L15_f;
                                }
                                if (self.rule_wd == (1073741824i32).wrapping_neg()) {
                                    // §671
                                    self.rule_wd = self.mem[crate::ix::U(((this_box).wrapping_add(1i32)) as usize)].int();
                                }
                                self.rule_ht = (self.rule_ht).wrapping_add(self.rule_dp);
                                if upwards {
                                    self.cur_v = (self.cur_v).wrapping_sub(self.rule_ht);
                                } else {
                                    self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                                }
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
                            // §669
                            if upwards {
                                self.cur_v = (self.cur_v).wrapping_sub(self.rule_ht);
                            } else {
                                self.cur_v = (self.cur_v).wrapping_add(self.rule_ht);
                            }
                        }
                    }
                }
                // §668
                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
            }
        }
        // §667
        self.prune_movements(save_loc);
        if (self.cur_s > 0i32) {
            self.dvi_pop(save_loc);
        }
        self.cur_s = (self.cur_s).wrapping_sub(1i32);
    }

    /// The `hlist_out` and `vlist_out` procedures are now complete, so we are
    /// ready for the `ship_out` routine that gets them started in the first place.
    // §676
    pub fn ship_out(&mut self, mut p: halfword) {
        let mut page_loc: i32 = 0; // §676
        let mut j: i32 = 0; // §676
        let mut k: i32 = 0; // §676
        let mut s: pool_pointer = 0; // §676
        let mut old_setting: i32 = 0; // §676
        'l_done_f: {
            if (self.job_name == 0i32) {
                self.open_log_file();
            }
            if (self.eqtb[crate::ix::U(((7892298i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_nl(65626i32);
                    self.print_ln();
                    self.print(66222i32);
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
                        self.print_int(self.eqtb[crate::ix::U((((count_base).wrapping_add(k)) - 1) as usize)].int());
                        if (k < j) {
                            self.print_char(46i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            crate::system::break_out(&mut self.term_out);
            if (self.eqtb[crate::ix::U(((7892298i32) - 1) as usize)].int() > 0i32) {
                {
                    self.print_char(93i32);
                    self.begin_diagnostic();
                    self.show_box(p);
                    self.end_diagnostic(true);
                }
            }
            // §679
            if ((((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > max_dimen) || (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > max_dimen)) || (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((9006739i32) - 1) as usize)].int()) > max_dimen)) || ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006738i32) - 1) as usize)].int()) > max_dimen)) {
                {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66228i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66229i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 66230i32;
                    }
                    self.error();
                    if (self.eqtb[crate::ix::U(((7892298i32) - 1) as usize)].int() <= 0i32) {
                        {
                            self.begin_diagnostic();
                            self.print_nl(66231i32);
                            self.show_box(p);
                            self.end_diagnostic(true);
                        }
                    }
                    break 'l_done_f;
                }
            }
            if (((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((9006739i32) - 1) as usize)].int()) > self.max_v) {
                self.max_v = ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add(self.eqtb[crate::ix::U(((9006739i32) - 1) as usize)].int());
            }
            if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006738i32) - 1) as usize)].int()) > self.max_h) {
                self.max_h = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006738i32) - 1) as usize)].int());
            }
            // §653
            self.dvi_h = 0i32;
            self.dvi_v = 0i32;
            self.cur_h = self.eqtb[crate::ix::U(((9006738i32) - 1) as usize)].int();
            self.dvi_f = null_font;
            // §1428
            self.cur_h_offset = (((((self.eqtb[crate::ix::U(((9006738i32) - 1) as usize)].int()) as f64) + ((((unity).wrapping_mul(7227i32)) as f64) / ((100i32) as f64)))) as i32);
            self.cur_v_offset = (((((self.eqtb[crate::ix::U(((9006739i32) - 1) as usize)].int()) as f64) + ((((unity).wrapping_mul(7227i32)) as f64) / ((100i32) as f64)))) as i32);
            if (self.eqtb[crate::ix::U(((9006741i32) - 1) as usize)].int() != 0i32) {
                self.cur_page_width = self.eqtb[crate::ix::U(((9006741i32) - 1) as usize)].int();
            } else {
                self.cur_page_width = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add((2i32).wrapping_mul(self.cur_h_offset));
            }
            if (self.eqtb[crate::ix::U(((9006742i32) - 1) as usize)].int() != 0i32) {
                self.cur_page_height = self.eqtb[crate::ix::U(((9006742i32) - 1) as usize)].int();
            } else {
                self.cur_page_height = ((self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int())).wrapping_add((2i32).wrapping_mul(self.cur_v_offset));
            }
            // §653
            if (self.output_file_name == 0i32) {
                {
                    if (self.job_name == 0i32) {
                        self.open_log_file();
                    }
                    self.pack_job_name(self.output_file_extension);
                    while (!{ let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.dvi_open_out(&mut __f0); self.dvi_file = __f0; __r }) {
                        self.prompt_file_name(66177i32, self.output_file_extension);
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
                    self.dvi_four(self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                    if (self.output_comment_length() >= 0i32) {
                        {
                            {
                                { let __ix615 = self.dvi_ptr; let __v616 = self.output_comment_length(); self.dvi_buf[crate::ix::U((__ix615) as usize)] = __v616; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            {
                                let __for_end_7 = (self.output_comment_length()).wrapping_sub(1i32);
                                s = 0i32;
                                while s <= __for_end_7 {
                                    {
                                        { let __ix617 = self.dvi_ptr; let __v618 = self.output_comment_byte(s); self.dvi_buf[crate::ix::U((__ix617) as usize)] = __v618; }
                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                        if (self.dvi_ptr == self.dvi_limit) {
                                            self.dvi_swap();
                                        }
                                    }
                                    s = s.wrapping_add(1);
                                }
                            }
                        }
                    } else {
                        {
                            old_setting = self.selector;
                            self.selector = new_string;
                            self.print(66220i32);
                            self.print_int(self.eqtb[crate::ix::U(((7892287i32) - 1) as usize)].int());
                            self.print_char(46i32);
                            self.print_two(self.eqtb[crate::ix::U(((7892286i32) - 1) as usize)].int());
                            self.print_char(46i32);
                            self.print_two(self.eqtb[crate::ix::U(((7892285i32) - 1) as usize)].int());
                            self.print_char(58i32);
                            self.print_two((self.eqtb[crate::ix::U(((7892284i32) - 1) as usize)].int() / 60i32));
                            self.print_two((self.eqtb[crate::ix::U(((7892284i32) - 1) as usize)].int() % 60i32));
                            self.selector = old_setting;
                            {
                                { let __ix619 = self.dvi_ptr; let __v620 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]); self.dvi_buf[crate::ix::U((__ix619) as usize)] = __v620; }
                                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                if (self.dvi_ptr == self.dvi_limit) {
                                    self.dvi_swap();
                                }
                            }
                            {
                                let __for_end_7 = (self.pool_ptr).wrapping_sub(1i32);
                                s = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                                while s <= __for_end_7 {
                                    {
                                        { let __ix621 = self.dvi_ptr; let __v622 = self.str_pool[crate::ix::U((s) as usize)]; self.dvi_buf[crate::ix::U((__ix621) as usize)] = __v622; }
                                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                                        if (self.dvi_ptr == self.dvi_limit) {
                                            self.dvi_swap();
                                        }
                                    }
                                    s = s.wrapping_add(1);
                                }
                            }
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                }
            }
            // §678
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
            old_setting = self.selector;
            self.selector = new_string;
            self.print(66226i32);
            if ((self.eqtb[crate::ix::U(((9006741i32) - 1) as usize)].int() > 0i32) && (self.eqtb[crate::ix::U(((9006742i32) - 1) as usize)].int() > 0i32)) {
                {
                    self.print(66084i32);
                    self.print(32i32);
                    self.print_scaled(self.eqtb[crate::ix::U(((9006741i32) - 1) as usize)].int());
                    self.print(65689i32);
                    self.print(32i32);
                    self.print(66085i32);
                    self.print(32i32);
                    self.print_scaled(self.eqtb[crate::ix::U(((9006742i32) - 1) as usize)].int());
                    self.print(65689i32);
                }
            } else {
                self.print(66227i32);
            }
            self.selector = old_setting;
            {
                self.dvi_buf[crate::ix::U((self.dvi_ptr) as usize)] = xxx1;
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            {
                { let __ix623 = self.dvi_ptr; let __v624 = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]); self.dvi_buf[crate::ix::U((__ix623) as usize)] = __v624; }
                self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                if (self.dvi_ptr == self.dvi_limit) {
                    self.dvi_swap();
                }
            }
            {
                let __for_end_3 = (self.pool_ptr).wrapping_sub(1i32);
                s = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                while s <= __for_end_3 {
                    {
                        { let __ix625 = self.dvi_ptr; let __v626 = self.str_pool[crate::ix::U((s) as usize)]; self.dvi_buf[crate::ix::U((__ix625) as usize)] = __v626; }
                        self.dvi_ptr = (self.dvi_ptr).wrapping_add(1i32);
                        if (self.dvi_ptr == self.dvi_limit) {
                            self.dvi_swap();
                        }
                    }
                    s = s.wrapping_add(1);
                }
            }
            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
            self.cur_v = (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int()).wrapping_add(self.eqtb[crate::ix::U(((9006739i32) - 1) as usize)].int());
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
            if (!self.no_pdf_output) {
                { let mut __f0 = ::core::mem::take(&mut self.dvi_file); let __r = self.fflush(&mut __f0); self.dvi_file = __f0; __r };
            }
        }
        // §676
        if (self.eTeX_mode == 1i32) {
            // §1541
            {
                if (self.LR_problems > 0i32) {
                    {
                        // §1523
                        {
                            self.print_ln();
                            self.print_nl(66915i32);
                            self.print_int((self.LR_problems / 10000i32));
                            self.print(66916i32);
                            self.print_int((self.LR_problems % 10000i32));
                            self.print(66917i32);
                            self.LR_problems = 0i32;
                        }
                        // §1541
                        self.print_char(41i32);
                        self.print_ln();
                    }
                }
                if ((self.LR_ptr != (268435455i32).wrapping_neg()) || (self.cur_dir != left_to_right)) {
                    self.confusion(66919i32);
                }
            }
        }
        // §676
        if (self.eqtb[crate::ix::U(((7892298i32) - 1) as usize)].int() <= 0i32) {
            self.print_char(93i32);
        }
        self.dead_cycles = 0i32;
        crate::system::break_out(&mut self.term_out);
        // §677
        if (self.eqtb[crate::ix::U(((7892295i32) - 1) as usize)].int() > 1i32) {
            {
                self.print_nl(66223i32);
                self.print_int(self.var_used);
                self.print_char(38i32);
                self.print_int(self.dyn_used);
                self.print_char(59i32);
            }
        }
        self.flush_node_list(p);
        if (self.eqtb[crate::ix::U(((7892295i32) - 1) as usize)].int() > 1i32) {
            {
                self.print(66224i32);
                self.print_int(self.var_used);
                self.print_char(38i32);
                self.print_int(self.dyn_used);
                self.print(66225i32);
                self.print_int(((self.hi_mem_min).wrapping_sub(self.lo_mem_max)).wrapping_sub(1i32));
                self.print_ln();
            }
        }
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
    // §684
    pub fn scan_spec(&mut self, mut c: group_code, mut three_codes: bool) {
        let mut s: i32 = 0; // §684
        let mut spec_code: i32 = 0; // §684
        'l_found_f: {
            if three_codes {
                s = self.save_stack[crate::ix::U(((self.save_ptr).wrapping_add(0i32)) as usize)].int();
            }
            if self.scan_keyword(66244i32) {
                spec_code = exactly;
            } else {
                if self.scan_keyword(66245i32) {
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
                { let __ix627 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix627) as usize)].set_int(s); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        { let __ix628 = (self.save_ptr).wrapping_add(0i32); self.save_stack[crate::ix::U((__ix628) as usize)].set_int(spec_code); }
        { let __ix629 = (self.save_ptr).wrapping_add(1i32); let __v630 = self.cur_val; self.save_stack[crate::ix::U((__ix629) as usize)].set_int(__v630); }
        self.save_ptr = (self.save_ptr).wrapping_add(2i32);
        self.new_save_level(c);
        self.scan_left_brace();
    }

    /// Some stuff for character protrusion.
    // §688
    pub fn char_pw(&mut self, mut p: halfword, mut side: small_number) -> scaled {
        let mut char_pw: scaled = 0;
        let mut f: internal_font_number = 0; // §688
        let mut c: i32 = 0; // §688
        char_pw = 0i32;
        if (side == left_side) {
            self.last_leftmost_char = (268435455i32).wrapping_neg();
        } else {
            self.last_rightmost_char = (268435455i32).wrapping_neg();
        }
        if (p == (268435455i32).wrapping_neg()) {
            return char_pw;
        }
        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((p) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((p) as usize)].hh().b1() <= native_word_node_AT))) {
            {
                if (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].int() != null_ptr) {
                    {
                        f = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1();
                        char_pw = { let __a631_0 = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(); let __a631_1 = self.get_native_word_cp(p, side); let __a631_2 = 1000i32; self.round_xn_over_d(__a631_0, __a631_1, __a631_2) };
                    }
                }
                return char_pw;
            }
        }
        if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == glyph_node)) {
            {
                f = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1();
                char_pw = { let __a632_0 = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(); let __a632_1 = self.get_cp_code(f, self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2(), side); let __a632_2 = 1000i32; self.round_xn_over_d(__a632_0, __a632_1, __a632_2) };
                return char_pw;
            }
        }
        if (!(p >= self.hi_mem_min)) {
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == ligature_node) {
                    p = (p).wrapping_add(1i32);
                } else {
                    return char_pw;
                }
            }
        }
        f = self.mem[crate::ix::U((p) as usize)].hh().b0();
        c = self.get_cp_code(f, self.mem[crate::ix::U((p) as usize)].hh().b1(), side);
        match side {
            left_side => {
                self.last_leftmost_char = p;
            }
            right_side => {
                self.last_rightmost_char = p;
            }
            _ => {}
        }
        if (c == 0i32) {
            return char_pw;
        }
        char_pw = self.round_xn_over_d(self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int(), c, 1000i32);
        char_pw
    }

    /// Some stuff for character protrusion.
    // §688
    pub fn new_margin_kern(&mut self, mut w: scaled, mut p: halfword, mut side: small_number) -> halfword {
        let mut new_margin_kern: halfword = 0;
        let mut k: halfword = 0; // §688
        k = self.get_node(margin_kern_node_size);
        self.mem[crate::ix::U((k) as usize)].set_hh_b0(margin_kern_node);
        self.mem[crate::ix::U((k) as usize)].set_hh_b1(side);
        self.mem[crate::ix::U(((k).wrapping_add(1i32)) as usize)].set_int(w);
        new_margin_kern = k;
        new_margin_kern
    }

    /// Here now is `hpack`, which contains few if any surprises.
    // §689
    pub fn hpack(&mut self, mut p: halfword, mut w: scaled, mut m: small_number) -> halfword {
        let mut hpack: halfword = 0;
        let mut r: halfword = 0; // §689
        let mut q: halfword = 0; // §689
        let mut h: scaled = 0; // §689
        let mut d: scaled = 0; // §689
        let mut x: scaled = 0; // §689
        let mut s: scaled = 0; // §689
        let mut g: halfword = 0; // §689
        let mut o: glue_ord = 0; // §689
        let mut f: internal_font_number = 0; // §689
        let mut i: four_quarters = four_quarters::default(); // §689
        let mut hd: eight_bits = 0; // §689
        let mut pp: halfword = 0; // §689
        let mut ppp: halfword = 0; // §689
        let mut total_chars: i32 = 0; // §689
        let mut k: i32 = 0; // §689
        // goto labels: common_ending, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.last_badness = 0i32;
                r = self.get_node(box_node_size);
                self.mem[crate::ix::U((r) as usize)].set_hh_b0(hlist_node);
                self.mem[crate::ix::U((r) as usize)].set_hh_b1(min_quarterword);
                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                q = (r).wrapping_add(5i32);
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                h = 0i32;
                // §690
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
                // §689
                if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                    // §1520
                    {
                        self.temp_ptr = self.get_avail();
                        { let __ix633 = self.temp_ptr; self.mem[crate::ix::U((__ix633) as usize)].set_hh_lh(before); }
                        { let __ix634 = self.temp_ptr; let __v635 = self.LR_ptr; self.mem[crate::ix::U((__ix634) as usize)].set_hh_rh(__v635); }
                        self.LR_ptr = self.temp_ptr;
                    }
                }
                // §689
                while (p != (268435455i32).wrapping_neg()) {
                    // §691
                    {
                        'l_reswitch_b: loop {
                            while (p >= self.hi_mem_min) {
                                // §694
                                {
                                    f = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    i = { let __s636 = ((self.char_base[crate::ix::U((f) as usize)]).wrapping_add(self.effective_char(true, f, self.mem[crate::ix::U((p) as usize)].hh().b1()))) as usize; self.font_info[crate::ix::U(__s636)] }.qqqq();
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
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            // §691
                            if (p != (268435455i32).wrapping_neg()) {
                                {
                                    match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                        hlist_node | vlist_node | rule_node | unset_node => {
                                            // §693
                                            {
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= rule_node) {
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
                                        ins_node | mark_node | adjust_node => {
                                            // §691
                                            if ((self.adjust_tail != (268435455i32).wrapping_neg()) || (self.pre_adjust_tail != (268435455i32).wrapping_neg())) {
                                                // §697
                                                {
                                                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node) {
                                                        {
                                                            if (self.mem[crate::ix::U((p) as usize)].hh().b1() != 0i32) {
                                                                {
                                                                    if (self.pre_adjust_tail == (268435455i32).wrapping_neg()) {
                                                                        self.confusion(66246i32);
                                                                    }
                                                                    { let __ix637 = self.pre_adjust_tail; let __v638 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix637) as usize)].set_hh_rh(__v638); }
                                                                    while (self.mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                                        self.pre_adjust_tail = self.mem[crate::ix::U((self.pre_adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            } else {
                                                                {
                                                                    if (self.adjust_tail == (268435455i32).wrapping_neg()) {
                                                                        self.confusion(66246i32);
                                                                    }
                                                                    { let __ix639 = self.adjust_tail; let __v640 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int(); self.mem[crate::ix::U((__ix639) as usize)].set_hh_rh(__v640); }
                                                                    while (self.mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                                        self.adjust_tail = self.mem[crate::ix::U((self.adjust_tail) as usize)].hh().rh();
                                                                    }
                                                                }
                                                            }
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            self.free_node(self.mem[crate::ix::U((q) as usize)].hh().rh(), small_node_size);
                                                        }
                                                    } else {
                                                        {
                                                            { let __ix641 = self.adjust_tail; self.mem[crate::ix::U((__ix641) as usize)].set_hh_rh(p); }
                                                            self.adjust_tail = p;
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                        }
                                                    }
                                                    self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                    p = q;
                                                }
                                            }
                                        }
                                        whatsit_node => {
                                            // §1420
                                            {
                                                match self.mem[crate::ix::U((p) as usize)].hh().b1() {
                                                    native_word_node | native_word_node_AT => {
                                                        {
                                                            if ((q != (r).wrapping_add(5i32)) && (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node)) {
                                                                k = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                            } else {
                                                                k = 0i32;
                                                            }
                                                            while (self.mem[crate::ix::U((q) as usize)].hh().rh() != p) {
                                                                {
                                                                    k = (k).wrapping_sub(1i32);
                                                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() == disc_node) {
                                                                        k = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                                    }
                                                                }
                                                            }
                                                            pp = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            'l_restart_b: loop {
                                                                if (((k <= 0i32) && (pp != (268435455i32).wrapping_neg())) && (!(pp >= self.hi_mem_min))) {
                                                                    {
                                                                        if (((self.mem[crate::ix::U((pp) as usize)].hh().b0() == whatsit_node) && ((self.mem[crate::ix::U((pp) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((pp) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U(((pp).wrapping_add(4i32)) as usize)].qqqq().b1() == self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1())) {
                                                                            {
                                                                                pp = self.mem[crate::ix::U((pp) as usize)].hh().rh();
                                                                                continue 'l_restart_b;
                                                                            }
                                                                        } else {
                                                                            if (self.mem[crate::ix::U((pp) as usize)].hh().b0() == disc_node) {
                                                                                {
                                                                                    ppp = self.mem[crate::ix::U((pp) as usize)].hh().rh();
                                                                                    if (((((ppp != (268435455i32).wrapping_neg()) && (!(ppp >= self.hi_mem_min))) && (self.mem[crate::ix::U((ppp) as usize)].hh().b0() == whatsit_node)) && ((self.mem[crate::ix::U((ppp) as usize)].hh().b1() >= native_word_node) && (self.mem[crate::ix::U((ppp) as usize)].hh().b1() <= native_word_node_AT))) && (self.mem[crate::ix::U(((ppp).wrapping_add(4i32)) as usize)].qqqq().b1() == self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1())) {
                                                                                        {
                                                                                            pp = self.mem[crate::ix::U((ppp) as usize)].hh().rh();
                                                                                            continue 'l_restart_b;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                if (pp != self.mem[crate::ix::U((p) as usize)].hh().rh()) {
                                                                    {
                                                                        total_chars = 0i32;
                                                                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        while (p != pp) {
                                                                            {
                                                                                if (self.mem[crate::ix::U((p) as usize)].hh().b0() == whatsit_node) {
                                                                                    total_chars = (total_chars).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2());
                                                                                }
                                                                                ppp = p;
                                                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                                            }
                                                                        }
                                                                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        pp = self.new_native_word_node(self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b1(), total_chars);
                                                                        { let __v642 = self.mem[crate::ix::U((p) as usize)].hh().b1(); self.mem[crate::ix::U((pp) as usize)].set_hh_b1(__v642); }
                                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(pp);
                                                                        { let __v643 = self.mem[crate::ix::U((ppp) as usize)].hh().rh(); self.mem[crate::ix::U((pp) as usize)].set_hh_rh(__v643); }
                                                                        self.mem[crate::ix::U((ppp) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                                        total_chars = 0i32;
                                                                        ppp = p;
                                                                        loop {
                                                                            if (self.mem[crate::ix::U((ppp) as usize)].hh().b0() == whatsit_node) {
                                                                                {
                                                                                    let __for_end_20 = (self.mem[crate::ix::U(((ppp).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
                                                                                    k = 0i32;
                                                                                    while k <= __for_end_20 {
                                                                                        {
                                                                                            { let __a644_0 = pp; let __a644_1 = total_chars; let __a644_2 = self.get_native_char(ppp, k); self.set_native_char(__a644_0, __a644_1, __a644_2) };
                                                                                            total_chars = (total_chars).wrapping_add(1i32);
                                                                                        }
                                                                                        k = k.wrapping_add(1);
                                                                                    }
                                                                                }
                                                                            }
                                                                            ppp = self.mem[crate::ix::U((ppp) as usize)].hh().rh();
                                                                            if (ppp == (268435455i32).wrapping_neg()) { break; }
                                                                        }
                                                                        self.flush_node_list(p);
                                                                        p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                                        self.set_native_metrics(p, (self.eqtb[crate::ix::U(((7892342i32) - 1) as usize)].int() > 0i32));
                                                                    }
                                                                }
                                                                if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > h) {
                                                                    h = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                                }
                                                                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > d) {
                                                                    d = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                                }
                                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                                break 'l_restart_b;
                                                            }
                                                        }
                                                    }
                                                    glyph_node | pic_node | pdf_node => {
                                                        {
                                                            if (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() > h) {
                                                                h = self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int();
                                                            }
                                                            if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() > d) {
                                                                d = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                            }
                                                            x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                        }
                                                    }
                                                    _ => {
                                                    }
                                                }
                                            }
                                        }
                                        glue_node => {
                                            // §698
                                            {
                                                g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                                o = self.mem[crate::ix::U((g) as usize)].hh().b0();
                                                { let __v645 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v645; }
                                                o = self.mem[crate::ix::U((g) as usize)].hh().b1();
                                                { let __v646 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v646; }
                                                if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
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
                                        kern_node => {
                                            // §691
                                            x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        margin_kern_node => {
                                            x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                        }
                                        math_node => {
                                            {
                                                x = (x).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                                                if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                                                    // §1521
                                                    if (((self.mem[crate::ix::U((p) as usize)].hh().b1()) % 2) != 0) {
                                                        if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32)) {
                                                            {
                                                                self.temp_ptr = self.LR_ptr;
                                                                self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                                                {
                                                                    { let __ix647 = self.temp_ptr; let __v648 = self.avail; self.mem[crate::ix::U((__ix647) as usize)].set_hh_rh(__v648); }
                                                                    self.avail = self.temp_ptr;
                                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                }
                                                            }
                                                        } else {
                                                            {
                                                                self.LR_problems = (self.LR_problems).wrapping_add(1i32);
                                                                self.mem[crate::ix::U((p) as usize)].set_hh_b0(kern_node);
                                                                self.mem[crate::ix::U((p) as usize)].set_hh_b1(explicit);
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            self.temp_ptr = self.get_avail();
                                                            { let __ix649 = self.temp_ptr; let __v650 = ((L_code).wrapping_mul((self.mem[crate::ix::U((p) as usize)].hh().b1() / L_code))).wrapping_add(3i32); self.mem[crate::ix::U((__ix649) as usize)].set_hh_lh(__v650); }
                                                            { let __ix651 = self.temp_ptr; let __v652 = self.LR_ptr; self.mem[crate::ix::U((__ix651) as usize)].set_hh_rh(__v652); }
                                                            self.LR_ptr = self.temp_ptr;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        ligature_node => {
                                            // §692
                                            {
                                                { let __v653 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]; self.mem[crate::ix::U((lig_trick) as usize)] = __v653; }
                                                { let __v654 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((lig_trick) as usize)].set_hh_rh(__v654); }
                                                p = lig_trick;
                                                self.xtx_ligature_present = true;
                                                continue 'l_reswitch_b;
                                            }
                                        }
                                        _ => {
                                            // §691
                                        }
                                    }
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            break 'l_reswitch_b;
                        }
                    }
                }
                // §689
                if (self.adjust_tail != (268435455i32).wrapping_neg()) {
                    { let __ix655 = self.adjust_tail; self.mem[crate::ix::U((__ix655) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                }
                if (self.pre_adjust_tail != (268435455i32).wrapping_neg()) {
                    { let __ix656 = self.pre_adjust_tail; self.mem[crate::ix::U((__ix656) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
                }
                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                // §699
                if (m == additional) {
                    w = (x).wrapping_add(w);
                }
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                x = (w).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        { __goto_1 = 2; continue 'l_dispatch_1; }
                    }
                } else {
                    if (x > 0i32) {
                        // §700
                        {
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
                            // §700
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v657 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v657); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == normal) {
                                if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                    // §702
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((normal) as usize)]);
                                        if (self.last_badness > self.eqtb[crate::ix::U(((7892290i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(66247i32);
                                                } else {
                                                    self.print_nl(66248i32);
                                                }
                                                self.print(66249i32);
                                                self.print_int(self.last_badness);
                                                { __goto_1 = 1; continue 'l_dispatch_1; }
                                            }
                                        }
                                    }
                                }
                            }
                            // §700
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    } else {
                        // §706
                        {
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
                            // §706
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v658 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v658); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == normal)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §708
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > self.eqtb[crate::ix::U(((9006728i32) - 1) as usize)].int()) || (self.eqtb[crate::ix::U(((7892290i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            if ((self.eqtb[crate::ix::U(((9006736i32) - 1) as usize)].int() > 0i32) && (((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > self.eqtb[crate::ix::U(((9006728i32) - 1) as usize)].int())) {
                                                {
                                                    while (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                        q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                    }
                                                    { let __v659 = self.new_rule(); self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v659); }
                                                    { let __ix660 = (self.mem[crate::ix::U((q) as usize)].hh().rh()).wrapping_add(1i32); let __v661 = self.eqtb[crate::ix::U(((9006736i32) - 1) as usize)].int(); self.mem[crate::ix::U((__ix660) as usize)].set_int(__v661); }
                                                }
                                            }
                                            self.print_ln();
                                            self.print_nl(66255i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]));
                                            self.print(66256i32);
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                            } else {
                                // §706
                                if (o == normal) {
                                    if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                        // §709
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((normal) as usize)]);
                                            if (self.last_badness > self.eqtb[crate::ix::U(((7892290i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(66257i32);
                                                    self.print_int(self.last_badness);
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §706
                            { __goto_1 = 2; continue 'l_dispatch_1; }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // common_ending
                // §689
                if self.output_active {
                    // §705
                    self.print(66250i32);
                } else {
                    {
                        if (self.pack_begin_line != 0i32) {
                            {
                                if (self.pack_begin_line > 0i32) {
                                    self.print(66251i32);
                                } else {
                                    self.print(66252i32);
                                }
                                self.print_int((self.pack_begin_line).wrapping_abs());
                                self.print(66253i32);
                            }
                        } else {
                            self.print(66254i32);
                        }
                        self.print_int(self.line);
                    }
                }
                self.print_ln();
                self.font_in_short_display = null_font;
                self.short_display(self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh());
                self.print_ln();
                self.begin_diagnostic();
                self.show_box(r);
                self.end_diagnostic(true);
            }
            if __goto_1 <= 2 { // exit
                // §689
                if (self.eqtb[crate::ix::U(((7892339i32) - 1) as usize)].int() > 0i32) {
                    // §1522
                    {
                        if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() != before) {
                            {
                                while (self.mem[crate::ix::U((q) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                    q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                }
                                loop {
                                    self.temp_ptr = q;
                                    q = self.new_math(0i32, self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh());
                                    { let __ix662 = self.temp_ptr; self.mem[crate::ix::U((__ix662) as usize)].set_hh_rh(q); }
                                    self.LR_problems = (self.LR_problems).wrapping_add(10000i32);
                                    {
                                        self.temp_ptr = self.LR_ptr;
                                        self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                                        {
                                            { let __ix663 = self.temp_ptr; let __v664 = self.avail; self.mem[crate::ix::U((__ix663) as usize)].set_hh_rh(__v664); }
                                            self.avail = self.temp_ptr;
                                            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                        }
                                    }
                                    if (self.mem[crate::ix::U((self.LR_ptr) as usize)].hh().lh() == before) { break; }
                                }
                            }
                        }
                        if (self.LR_problems > 0i32) {
                            {
                                // §1523
                                {
                                    self.print_ln();
                                    self.print_nl(66915i32);
                                    self.print_int((self.LR_problems / 10000i32));
                                    self.print(66916i32);
                                    self.print_int((self.LR_problems % 10000i32));
                                    self.print(66917i32);
                                    self.LR_problems = 0i32;
                                }
                                // §1522
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                        {
                            self.temp_ptr = self.LR_ptr;
                            self.LR_ptr = self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().rh();
                            {
                                { let __ix665 = self.temp_ptr; let __v666 = self.avail; self.mem[crate::ix::U((__ix665) as usize)].set_hh_rh(__v666); }
                                self.avail = self.temp_ptr;
                                self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                            }
                        }
                        if (self.LR_ptr != (268435455i32).wrapping_neg()) {
                            self.confusion(66914i32);
                        }
                    }
                }
                // §689
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
    // §710
    pub fn vpackage(&mut self, mut p: halfword, mut h: scaled, mut m: small_number, mut l: scaled) -> halfword {
        let mut vpackage: halfword = 0;
        let mut r: halfword = 0; // §710
        let mut w: scaled = 0; // §710
        let mut d: scaled = 0; // §710
        let mut x: scaled = 0; // §710
        let mut s: scaled = 0; // §710
        let mut g: halfword = 0; // §710
        let mut o: glue_ord = 0; // §710
        'l_exit_f: {
            'l_common_ending_f: {
                self.last_badness = 0i32;
                r = self.get_node(box_node_size);
                self.mem[crate::ix::U((r) as usize)].set_hh_b0(vlist_node);
                if (self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int() > 0i32) {
                    self.mem[crate::ix::U((r) as usize)].set_hh_b1(1i32);
                } else {
                    self.mem[crate::ix::U((r) as usize)].set_hh_b1(min_quarterword);
                }
                self.mem[crate::ix::U(((r).wrapping_add(4i32)) as usize)].set_int(0i32);
                self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_rh(p);
                w = 0i32;
                // §690
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
                // §710
                while (p != (268435455i32).wrapping_neg()) {
                    // §711
                    {
                        if (p >= self.hi_mem_min) {
                            self.confusion(66258i32);
                        } else {
                            match self.mem[crate::ix::U((p) as usize)].hh().b0() {
                                hlist_node | vlist_node | rule_node | unset_node => {
                                    // §712
                                    {
                                        x = ((x).wrapping_add(d)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                        d = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= rule_node) {
                                            s = 0i32;
                                        } else {
                                            s = self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].int();
                                        }
                                        if ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s) > w) {
                                            w = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int()).wrapping_add(s);
                                        }
                                    }
                                }
                                whatsit_node => {
                                    // §1419
                                    {
                                        if ((self.mem[crate::ix::U((p) as usize)].hh().b1() == pic_node) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == pdf_node)) {
                                            {
                                                x = ((x).wrapping_add(d)).wrapping_add(self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int());
                                                d = self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                                                if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() > w) {
                                                    w = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                }
                                            }
                                        }
                                    }
                                }
                                glue_node => {
                                    // §713
                                    {
                                        x = (x).wrapping_add(d);
                                        d = 0i32;
                                        g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh();
                                        x = (x).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int());
                                        o = self.mem[crate::ix::U((g) as usize)].hh().b0();
                                        { let __v667 = (self.total_stretch[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(2i32)) as usize)].int()); self.total_stretch[crate::ix::U((o) as usize)] = __v667; }
                                        o = self.mem[crate::ix::U((g) as usize)].hh().b1();
                                        { let __v668 = (self.total_shrink[crate::ix::U((o) as usize)]).wrapping_add(self.mem[crate::ix::U(((g).wrapping_add(3i32)) as usize)].int()); self.total_shrink[crate::ix::U((o) as usize)] = __v668; }
                                        if (self.mem[crate::ix::U((p) as usize)].hh().b1() >= a_leaders) {
                                            {
                                                g = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh();
                                                if (self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int() > w) {
                                                    w = self.mem[crate::ix::U(((g).wrapping_add(1i32)) as usize)].int();
                                                }
                                            }
                                        }
                                    }
                                }
                                kern_node => {
                                    // §711
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
                // §710
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_int(w);
                if (d > l) {
                    {
                        x = ((x).wrapping_add(d)).wrapping_sub(l);
                        self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(l);
                    }
                } else {
                    self.mem[crate::ix::U(((r).wrapping_add(2i32)) as usize)].set_int(d);
                }
                // §714
                if (m == additional) {
                    h = (x).wrapping_add(h);
                }
                self.mem[crate::ix::U(((r).wrapping_add(3i32)) as usize)].set_int(h);
                x = (h).wrapping_sub(x);
                if (x == 0i32) {
                    {
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                        self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(normal);
                        self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                        break 'l_exit_f;
                    }
                } else {
                    if (x > 0i32) {
                        // §715
                        {
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
                            // §715
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(stretching);
                            if (self.total_stretch[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v669 = (((x) as f64) / ((self.total_stretch[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v669); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (o == normal) {
                                if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                    // §716
                                    {
                                        self.last_badness = self.badness(x, self.total_stretch[crate::ix::U((normal) as usize)]);
                                        if (self.last_badness > self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].int()) {
                                            {
                                                self.print_ln();
                                                if (self.last_badness > 100i32) {
                                                    self.print_nl(66247i32);
                                                } else {
                                                    self.print_nl(66248i32);
                                                }
                                                self.print(66259i32);
                                                self.print_int(self.last_badness);
                                                break 'l_common_ending_f;
                                            }
                                        }
                                    }
                                }
                            }
                            // §715
                            break 'l_exit_f;
                        }
                    } else {
                        // §718
                        {
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
                            // §718
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b1(o);
                            self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(shrinking);
                            if (self.total_shrink[crate::ix::U((o) as usize)] != 0i32) {
                                { let __v670 = ((((x).wrapping_neg()) as f64) / ((self.total_shrink[crate::ix::U((o) as usize)]) as f64)); self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(__v670); }
                            } else {
                                {
                                    self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].set_hh_b0(normal);
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(0.0f64);
                                }
                            }
                            if (((self.total_shrink[crate::ix::U((o) as usize)] < (x).wrapping_neg()) && (o == normal)) && (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg())) {
                                {
                                    self.last_badness = 1000000i32;
                                    self.mem[crate::ix::U(((r).wrapping_add(6i32)) as usize)].set_gr(1.0f64);
                                    // §719
                                    if ((((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]) > self.eqtb[crate::ix::U(((9006729i32) - 1) as usize)].int()) || (self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].int() < 100i32)) {
                                        {
                                            self.print_ln();
                                            self.print_nl(66260i32);
                                            self.print_scaled(((x).wrapping_neg()).wrapping_sub(self.total_shrink[crate::ix::U((normal) as usize)]));
                                            self.print(66261i32);
                                            break 'l_common_ending_f;
                                        }
                                    }
                                }
                            } else {
                                // §718
                                if (o == normal) {
                                    if (self.mem[crate::ix::U(((r).wrapping_add(5i32)) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                        // §720
                                        {
                                            self.last_badness = self.badness((x).wrapping_neg(), self.total_shrink[crate::ix::U((normal) as usize)]);
                                            if (self.last_badness > self.eqtb[crate::ix::U(((7892291i32) - 1) as usize)].int()) {
                                                {
                                                    self.print_ln();
                                                    self.print_nl(66262i32);
                                                    self.print_int(self.last_badness);
                                                    break 'l_common_ending_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §718
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §710
            if self.output_active {
                // §717
                self.print(66250i32);
            } else {
                {
                    if (self.pack_begin_line != 0i32) {
                        {
                            self.print(66252i32);
                            self.print_int((self.pack_begin_line).wrapping_abs());
                            self.print(66253i32);
                        }
                    } else {
                        self.print(66254i32);
                    }
                    self.print_int(self.line);
                    self.print_ln();
                }
            }
            self.begin_diagnostic();
            self.show_box(r);
            self.end_diagnostic(true);
        }
        // §710
        vpackage = r;
        vpackage
    }

    /// When a box is being appended to the current vertical list, the
    /// baselineskip calculation is handled by the `append_to_vlist` routine.
    // §721
    pub fn append_to_vlist(&mut self, mut b: halfword) {
        let mut d: scaled = 0; // §721
        let mut p: halfword = 0; // §721
        let mut upwards: bool = false; // §721
        upwards = (self.eqtb[crate::ix::U(((7892341i32) - 1) as usize)].int() > 0i32);
        if (self.cur_list.aux_field.int() > (65536000i32).wrapping_neg()) {
            {
                if upwards {
                    d = ((self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((1205765i32) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_list.aux_field.int())).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int());
                } else {
                    d = ((self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((1205765i32) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()).wrapping_sub(self.cur_list.aux_field.int())).wrapping_sub(self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int());
                }
                if (d < self.eqtb[crate::ix::U(((9006722i32) - 1) as usize)].int()) {
                    p = self.new_param_glue(line_skip_code);
                } else {
                    {
                        p = self.new_skip_param(baseline_skip_code);
                        { let __ix671 = (self.temp_ptr).wrapping_add(1i32); self.mem[crate::ix::U((__ix671) as usize)].set_int(d); }
                    }
                }
                { let __ix672 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix672) as usize)].set_hh_rh(p); }
                self.cur_list.tail_field = p;
            }
        }
        { let __ix673 = self.cur_list.tail_field; self.mem[crate::ix::U((__ix673) as usize)].set_hh_rh(b); }
        self.cur_list.tail_field = b;
        if upwards {
            { let __v674 = self.mem[crate::ix::U(((b).wrapping_add(3i32)) as usize)].int(); self.cur_list.aux_field.set_int(__v674); }
        } else {
            { let __v675 = self.mem[crate::ix::U(((b).wrapping_add(2i32)) as usize)].int(); self.cur_list.aux_field.set_int(__v675); }
        }
    }

    /// The `new_noad` function creates an `ord_noad` that is completely null.
    // §728
    pub fn new_noad(&mut self) -> halfword {
        let mut new_noad: halfword = 0;
        let mut p: halfword = 0; // §728
        p = self.get_node(noad_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(ord_noad);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(normal);
        { let __v676 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh(__v676); }
        { let __v677 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].set_hh(__v677); }
        { let __v678 = self.empty_field; self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh(__v678); }
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
    // §730
    pub fn new_style(&mut self, mut s: small_number) -> halfword {
        let mut new_style: halfword = 0;
        let mut p: halfword = 0; // §730
        p = self.get_node(style_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(style_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(s);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
        new_style = p;
        new_style
    }

    /// Finally, the \.{\\mathchoice} primitive creates a `choice_node`, which
    /// has special subfields `display_mlist`, `text_mlist`, `script_mlist`,
    /// and `script_script_mlist` pointing to the mlists for each style.
    // §731
    pub fn new_choice(&mut self) -> halfword {
        let mut new_choice: halfword = 0;
        let mut p: halfword = 0; // §731
        p = self.get_node(style_node_size);
        self.mem[crate::ix::U((p) as usize)].set_hh_b0(choice_node);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(0i32);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_lh((268435455i32).wrapping_neg());
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        new_choice = p;
        new_choice
    }

    /// The inelegant introduction of `show_info` in the code above seems better
    /// than the alternative of using \PASCAL's strange `forward` declaration for a
    /// procedure with parameters. The \PASCAL\ convention about dropping parameters
    /// from a post-`forward` procedure is, frankly, so intolerable to the author
    /// of \TeX\ that he would rather stoop to communication via a global temporary
    /// variable. (A similar stoopidity occurred with respect to `hlist_out` and
    /// `vlist_out` above, and it will occur with respect to `mlist_to_hlist` below.)
    // §735
    pub fn show_info(&mut self) {
        self.show_node_list(self.mem[crate::ix::U((self.temp_ptr) as usize)].hh().lh());
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
    pub fn math_x_height(&mut self, mut size_code: i32) -> scaled {
        let mut math_x_height: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 5i32);
        } else {
            rval = self.font_info[crate::ix::U(((5i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        math_x_height = rval;
        math_x_height
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
    pub fn math_quad(&mut self, mut size_code: i32) -> scaled {
        let mut math_quad: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 6i32);
        } else {
            rval = self.font_info[crate::ix::U(((6i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        math_quad = rval;
        math_quad
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
    pub fn num1(&mut self, mut size_code: i32) -> scaled {
        let mut num1: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 8i32);
        } else {
            rval = self.font_info[crate::ix::U(((8i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        num1 = rval;
        num1
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
    pub fn num2(&mut self, mut size_code: i32) -> scaled {
        let mut num2: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 9i32);
        } else {
            rval = self.font_info[crate::ix::U(((9i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        num2 = rval;
        num2
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
    pub fn num3(&mut self, mut size_code: i32) -> scaled {
        let mut num3: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 10i32);
        } else {
            rval = self.font_info[crate::ix::U(((10i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        num3 = rval;
        num3
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
    pub fn denom1(&mut self, mut size_code: i32) -> scaled {
        let mut denom1: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 11i32);
        } else {
            rval = self.font_info[crate::ix::U(((11i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        denom1 = rval;
        denom1
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
    pub fn denom2(&mut self, mut size_code: i32) -> scaled {
        let mut denom2: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 12i32);
        } else {
            rval = self.font_info[crate::ix::U(((12i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        denom2 = rval;
        denom2
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
    pub fn sup1(&mut self, mut size_code: i32) -> scaled {
        let mut sup1: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 13i32);
        } else {
            rval = self.font_info[crate::ix::U(((13i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sup1 = rval;
        sup1
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
    pub fn sup2(&mut self, mut size_code: i32) -> scaled {
        let mut sup2: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 14i32);
        } else {
            rval = self.font_info[crate::ix::U(((14i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sup2 = rval;
        sup2
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
    pub fn sup3(&mut self, mut size_code: i32) -> scaled {
        let mut sup3: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 15i32);
        } else {
            rval = self.font_info[crate::ix::U(((15i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sup3 = rval;
        sup3
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
    pub fn sub1(&mut self, mut size_code: i32) -> scaled {
        let mut sub1: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 16i32);
        } else {
            rval = self.font_info[crate::ix::U(((16i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sub1 = rval;
        sub1
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
    pub fn sub2(&mut self, mut size_code: i32) -> scaled {
        let mut sub2: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 17i32);
        } else {
            rval = self.font_info[crate::ix::U(((17i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sub2 = rval;
        sub2
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
    pub fn sup_drop(&mut self, mut size_code: i32) -> scaled {
        let mut sup_drop: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 18i32);
        } else {
            rval = self.font_info[crate::ix::U(((18i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sup_drop = rval;
        sup_drop
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
    pub fn sub_drop(&mut self, mut size_code: i32) -> scaled {
        let mut sub_drop: scaled = 0;
        let mut f: i32 = 0; // §742
        let mut rval: scaled = 0; // §742
        f = self.eqtb[crate::ix::U((((1206826i32).wrapping_add(size_code)) - 1) as usize)].hh().rh();
        if ((self.font_area[crate::ix::U((f) as usize)] == otgr_font_flag) && self.isOpenTypeMathFont(self.font_layout_engine[crate::ix::U((f) as usize)])) {
            rval = self.get_native_mathsy_param(f, 19i32);
        } else {
            rval = self.font_info[crate::ix::U(((19i32).wrapping_add(self.param_base[crate::ix::U((f) as usize)])) as usize)].int();
        }
        sub_drop = rval;
        sub_drop
    }

}
