// GENERATED FILE -- DO NOT EDIT.
// Translated WEB procedures and functions.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(dead_code, unreachable_code, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// The `scan_optional_equals` routine looks for an optional `\.=' sign preceded
    /// by optional spaces; `\.{\\relax}' is not ignored here.
    // §405
    pub fn scan_optional_equals(&mut self) {
        // §406
        loop {
            self.get_x_token();
            if (self.cur_cmd != 10i32) { break; }
        }
        // §405
        if (self.cur_tok != 3133i32) {
            self.back_input();
        }
    }

    /// In case you are getting bored, here is a slightly less trivial routine:
    /// Given a string of lowercase letters, like `\.{pt}' or `\.{plus}' or
    /// `\.{width}', the `scan_keyword` routine checks to see whether the next
    /// tokens of input match this string. The match must be exact, except that
    /// uppercase letters will match their lowercase counterparts; uppercase
    /// equivalents are determined by subtracting `"a"-"A"`, rather than using the
    /// `uc_code` table, since \TeX\ uses this routine only for its own limited
    /// set of keywords.
    /// If a match is found, the characters are effectively removed from the input
    /// and `true` is returned. Otherwise `false` is returned, and the input
    /// is left essentially unchanged (except for the fact that some macros
    /// may have been expanded, etc.).
    // §407
    pub fn scan_keyword(&mut self, mut s: str_number) -> bool {
        let mut scan_keyword: bool = false;
        let mut p: halfword = 0; // §407
        let mut q: halfword = 0; // §407
        let mut k: pool_pointer = 0; // §407
        'l_exit_f: {
            p = 29987i32;
            self.mem[(p) as usize].set_hh_rh(0i32);
            k = self.str_start[(s) as usize];
            while (k < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                {
                    self.get_x_token();
                    if ((self.cur_cs == 0i32) && ((self.cur_chr == self.str_pool[(k) as usize]) || (self.cur_chr == (self.str_pool[(k) as usize]).wrapping_sub(32i32)))) {
                        {
                            {
                                q = self.get_avail();
                                self.mem[(p) as usize].set_hh_rh(q);
                                { let __v157 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v157); }
                                p = q;
                            }
                            k = (k).wrapping_add(1i32);
                        }
                    } else {
                        if ((self.cur_cmd != 10i32) || (p != 29987i32)) {
                            {
                                self.back_input();
                                if (p != 29987i32) {
                                    self.begin_token_list(self.mem[(29987i32) as usize].hh().rh(), 3i32);
                                }
                                scan_keyword = false;
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            self.flush_list(self.mem[(29987i32) as usize].hh().rh());
            scan_keyword = true;
        }
        scan_keyword
    }

    /// Here is a procedure that sounds an alarm when mu and non-mu units
    /// are being switched.
    // §408
    pub fn mu_error(&mut self) {
        {
            if (self.interaction == 3i32) {
            }
            self.print_nl(262i32);
            self.print(662i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[(0i32) as usize] = 663i32;
        }
        self.error();
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §433
    pub fn scan_eight_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(687i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 688i32;
                    self.help_line[(0i32) as usize] = 689i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §434
    pub fn scan_char_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(690i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 691i32;
                    self.help_line[(0i32) as usize] = 689i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §435
    pub fn scan_four_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 15i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(692i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 693i32;
                    self.help_line[(0i32) as usize] = 689i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §436
    pub fn scan_fifteen_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 32767i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(694i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 695i32;
                    self.help_line[(0i32) as usize] = 689i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §437
    pub fn scan_twenty_seven_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 134217727i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(696i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 697i32;
                    self.help_line[(0i32) as usize] = 689i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// Before we forget about the format of these tables, let's deal with two
    /// of \TeX's basic scanning routines related to font information.
    /// @<Declare procedures that scan font-related stuff
    // §577
    pub fn scan_font_ident(&mut self) {
        let mut f: internal_font_number = 0; // §577
        let mut m: halfword = 0; // §577
        // §406
        loop {
            self.get_x_token();
            if (self.cur_cmd != 10i32) { break; }
        }
        // §577
        if (self.cur_cmd == 88i32) {
            f = self.eqtb[((3934i32) - 1) as usize].hh().rh();
        } else {
            if (self.cur_cmd == 87i32) {
                f = self.cur_chr;
            } else {
                if (self.cur_cmd == 86i32) {
                    {
                        m = self.cur_chr;
                        self.scan_four_bit_int();
                        f = self.eqtb[(((m).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                    }
                } else {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(817i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 818i32;
                            self.help_line[(0i32) as usize] = 819i32;
                        }
                        self.back_error();
                        f = 0i32;
                    }
                }
            }
        }
        self.cur_val = f;
    }

    /// The following routine is used to implement `\.{\\fontdimen} `n` `f`'.
    /// The boolean parameter `writing` is set `true` if the calling program
    /// intends to change the parameter value.
    /// @<Declare procedures that scan font-related stuff
    // §578
    pub fn find_font_dimen(&mut self, mut writing: bool) {
        let mut f: internal_font_number = 0; // §578
        let mut n: i32 = 0; // §578
        self.scan_int();
        n = self.cur_val;
        self.scan_font_ident();
        f = self.cur_val;
        if (n <= 0i32) {
            self.cur_val = self.fmem_ptr;
        } else {
            {
                if (((writing && (n <= 4i32)) && (n >= 2i32)) && (self.font_glue[(f) as usize] != 0i32)) {
                    {
                        self.delete_glue_ref(self.font_glue[(f) as usize]);
                        self.font_glue[(f) as usize] = 0i32;
                    }
                }
                if (n > self.font_params[(f) as usize]) {
                    if (f < self.font_ptr) {
                        self.cur_val = self.fmem_ptr;
                    } else {
                        // §580
                        {
                            loop {
                                if (self.fmem_ptr == font_mem_size) {
                                    self.overflow(824i32, font_mem_size);
                                }
                                { let __ix158 = self.fmem_ptr; self.font_info[(__ix158) as usize].set_int(0i32); }
                                self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                                { let __v159 = (self.font_params[(f) as usize]).wrapping_add(1i32); self.font_params[(f) as usize] = __v159; }
                                if (n == self.font_params[(f) as usize]) { break; }
                            }
                            self.cur_val = (self.fmem_ptr).wrapping_sub(1i32);
                        }
                    }
                } else {
                    // §578
                    self.cur_val = (n).wrapping_add(self.param_base[(f) as usize]);
                }
            }
        }
        // §579
        if (self.cur_val == self.fmem_ptr) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(802i32);
                }
                self.print_esc(self.hash[(((2624i32).wrapping_add(f)) - 514) as usize].rh());
                self.print(820i32);
                self.print_int(self.font_params[(f) as usize]);
                self.print(821i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 822i32;
                    self.help_line[(0i32) as usize] = 823i32;
                }
                self.error();
            }
        }
    }

    /// OK, we're ready for `scan_something_internal` itself. A second parameter,
    /// `negative`, is set `true` if the value that is found should be negated.
    /// It is assumed that `cur_cmd` and `cur_chr` represent the first token of
    /// the internal quantity to be scanned; an error will be signalled if
    /// `cur_cmd<min_internal` or `cur_cmd>max_internal`.
    // §413
    pub fn scan_something_internal(&mut self, mut level: small_number, mut negative: bool) {
        let mut m: halfword = 0; // §413
        let mut p: i32 = 0; // §413
        m = self.cur_chr;
        match self.cur_cmd {
            85 => {
                // §414
                {
                    self.scan_char_num();
                    if (m == 5007i32) {
                        {
                            self.cur_val = (self.eqtb[(((5007i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh()).wrapping_sub(0i32);
                            self.cur_val_level = 0i32;
                        }
                    } else {
                        if (m < 5007i32) {
                            {
                                self.cur_val = self.eqtb[(((m).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                                self.cur_val_level = 0i32;
                            }
                        } else {
                            {
                                self.cur_val = self.eqtb[(((m).wrapping_add(self.cur_val)) - 1) as usize].int();
                                self.cur_val_level = 0i32;
                            }
                        }
                    }
                }
            }
            71 | 72 | 86 | 87 | 88 => {
                // §415
                if (level != 5i32) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(664i32);
                        }
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 665i32;
                            self.help_line[(1i32) as usize] = 666i32;
                            self.help_line[(0i32) as usize] = 667i32;
                        }
                        self.back_error();
                        {
                            self.cur_val = 0i32;
                            self.cur_val_level = 1i32;
                        }
                    }
                } else {
                    if (self.cur_cmd <= 72i32) {
                        {
                            if (self.cur_cmd < 72i32) {
                                {
                                    self.scan_eight_bit_int();
                                    m = (3422i32).wrapping_add(self.cur_val);
                                }
                            }
                            {
                                self.cur_val = self.eqtb[((m) - 1) as usize].hh().rh();
                                self.cur_val_level = 5i32;
                            }
                        }
                    } else {
                        {
                            self.back_input();
                            self.scan_font_ident();
                            {
                                self.cur_val = (2624i32).wrapping_add(self.cur_val);
                                self.cur_val_level = 4i32;
                            }
                        }
                    }
                }
            }
            73 => {
                // §413
                {
                    self.cur_val = self.eqtb[((m) - 1) as usize].int();
                    self.cur_val_level = 0i32;
                }
            }
            74 => {
                {
                    self.cur_val = self.eqtb[((m) - 1) as usize].int();
                    self.cur_val_level = 1i32;
                }
            }
            75 => {
                {
                    self.cur_val = self.eqtb[((m) - 1) as usize].hh().rh();
                    self.cur_val_level = 2i32;
                }
            }
            76 => {
                {
                    self.cur_val = self.eqtb[((m) - 1) as usize].hh().rh();
                    self.cur_val_level = 3i32;
                }
            }
            79 => {
                // §418
                if ((self.cur_list.mode_field).wrapping_abs() != m) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(680i32);
                        }
                        self.print_cmd_chr(79i32, m);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[(3i32) as usize] = 681i32;
                            self.help_line[(2i32) as usize] = 682i32;
                            self.help_line[(1i32) as usize] = 683i32;
                            self.help_line[(0i32) as usize] = 684i32;
                        }
                        self.error();
                        if (level != 5i32) {
                            {
                                self.cur_val = 0i32;
                                self.cur_val_level = 1i32;
                            }
                        } else {
                            {
                                self.cur_val = 0i32;
                                self.cur_val_level = 0i32;
                            }
                        }
                    }
                } else {
                    if (m == 1i32) {
                        {
                            self.cur_val = self.cur_list.aux_field.int();
                            self.cur_val_level = 1i32;
                        }
                    } else {
                        {
                            self.cur_val = self.cur_list.aux_field.hh().lh();
                            self.cur_val_level = 0i32;
                        }
                    }
                }
            }
            80 => {
                // §422
                if (self.cur_list.mode_field == 0i32) {
                    {
                        self.cur_val = 0i32;
                        self.cur_val_level = 0i32;
                    }
                } else {
                    {
                        { let __ix160 = self.nest_ptr; let __v161 = self.cur_list; self.nest[(__ix160) as usize] = __v161; }
                        p = self.nest_ptr;
                        while ((self.nest[(p) as usize].mode_field).wrapping_abs() != 1i32) {
                            p = (p).wrapping_sub(1i32);
                        }
                        {
                            self.cur_val = self.nest[(p) as usize].pg_field;
                            self.cur_val_level = 0i32;
                        }
                    }
                }
            }
            82 => {
                // §419
                {
                    if (m == 0i32) {
                        self.cur_val = self.dead_cycles;
                    } else {
                        self.cur_val = self.insert_penalties;
                    }
                    self.cur_val_level = 0i32;
                }
            }
            81 => {
                // §421
                {
                    if ((self.page_contents == 0i32) && (!self.output_active)) {
                        if (m == 0i32) {
                            self.cur_val = 1073741823i32;
                        } else {
                            self.cur_val = 0i32;
                        }
                    } else {
                        self.cur_val = self.page_so_far[(m) as usize];
                    }
                    self.cur_val_level = 1i32;
                }
            }
            84 => {
                // §423
                {
                    if (self.eqtb[((3412i32) - 1) as usize].hh().rh() == 0i32) {
                        self.cur_val = 0i32;
                    } else {
                        self.cur_val = self.mem[(self.eqtb[((3412i32) - 1) as usize].hh().rh()) as usize].hh().lh();
                    }
                    self.cur_val_level = 0i32;
                }
            }
            83 => {
                // §420
                {
                    self.scan_eight_bit_int();
                    if (self.eqtb[(((3678i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh() == 0i32) {
                        self.cur_val = 0i32;
                    } else {
                        self.cur_val = self.mem[((self.eqtb[(((3678i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh()).wrapping_add(m)) as usize].int();
                    }
                    self.cur_val_level = 1i32;
                }
            }
            68 | 69 => {
                // §413
                {
                    self.cur_val = self.cur_chr;
                    self.cur_val_level = 0i32;
                }
            }
            77 => {
                // §425
                {
                    self.find_font_dimen(false);
                    { let __ix162 = self.fmem_ptr; self.font_info[(__ix162) as usize].set_int(0i32); }
                    {
                        self.cur_val = self.font_info[(self.cur_val) as usize].int();
                        self.cur_val_level = 1i32;
                    }
                }
            }
            78 => {
                // §426
                {
                    self.scan_font_ident();
                    if (m == 0i32) {
                        {
                            self.cur_val = self.hyphen_char[(self.cur_val) as usize];
                            self.cur_val_level = 0i32;
                        }
                    } else {
                        {
                            self.cur_val = self.skew_char[(self.cur_val) as usize];
                            self.cur_val_level = 0i32;
                        }
                    }
                }
            }
            89 => {
                // §427
                {
                    self.scan_eight_bit_int();
                    match m {
                        0 => {
                            self.cur_val = self.eqtb[(((5318i32).wrapping_add(self.cur_val)) - 1) as usize].int();
                        }
                        1 => {
                            self.cur_val = self.eqtb[(((5851i32).wrapping_add(self.cur_val)) - 1) as usize].int();
                        }
                        2 => {
                            self.cur_val = self.eqtb[(((2900i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                        }
                        3 => {
                            self.cur_val = self.eqtb[(((3156i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                        }
                        _ => {}
                    }
                    self.cur_val_level = m;
                }
            }
            70 => {
                // §424
                if (self.cur_chr > 2i32) {
                    {
                        if (self.cur_chr == 3i32) {
                            self.cur_val = self.line;
                        } else {
                            self.cur_val = self.last_badness;
                        }
                        self.cur_val_level = 0i32;
                    }
                } else {
                    {
                        if (self.cur_chr == 2i32) {
                            self.cur_val = 0i32;
                        } else {
                            self.cur_val = 0i32;
                        }
                        self.cur_val_level = self.cur_chr;
                        if ((!(self.cur_list.tail_field >= self.hi_mem_min)) && (self.cur_list.mode_field != 0i32)) {
                            match self.cur_chr {
                                0 => {
                                    if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 12i32) {
                                        self.cur_val = self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].int();
                                    }
                                }
                                1 => {
                                    if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 11i32) {
                                        self.cur_val = self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].int();
                                    }
                                }
                                2 => {
                                    if (self.mem[(self.cur_list.tail_field) as usize].hh().b0() == 10i32) {
                                        {
                                            self.cur_val = self.mem[((self.cur_list.tail_field).wrapping_add(1i32)) as usize].hh().lh();
                                            if (self.mem[(self.cur_list.tail_field) as usize].hh().b1() == 99i32) {
                                                self.cur_val_level = 3i32;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            if ((self.cur_list.mode_field == 1i32) && (self.cur_list.tail_field == self.cur_list.head_field)) {
                                match self.cur_chr {
                                    0 => {
                                        self.cur_val = self.last_penalty;
                                    }
                                    1 => {
                                        self.cur_val = self.last_kern;
                                    }
                                    2 => {
                                        if (self.last_glue != 65535i32) {
                                            self.cur_val = self.last_glue;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // §428
                {
                    {
                        if (self.interaction == 3i32) {
                        }
                        self.print_nl(262i32);
                        self.print(685i32);
                    }
                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                    self.print(686i32);
                    self.print_esc(537i32);
                    {
                        self.help_ptr = 1i32;
                        self.help_line[(0i32) as usize] = 684i32;
                    }
                    self.error();
                    if (level != 5i32) {
                        {
                            self.cur_val = 0i32;
                            self.cur_val_level = 1i32;
                        }
                    } else {
                        {
                            self.cur_val = 0i32;
                            self.cur_val_level = 0i32;
                        }
                    }
                }
            }
        }
        // §413
        while (self.cur_val_level > level) {
            // §429
            {
                if (self.cur_val_level == 2i32) {
                    self.cur_val = self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int();
                } else {
                    if (self.cur_val_level == 3i32) {
                        self.mu_error();
                    }
                }
                self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
            }
        }
        // §430
        if negative {
            if (self.cur_val_level >= 2i32) {
                {
                    self.cur_val = self.new_spec(self.cur_val);
                    // §431
                    {
                        { let __ix163 = (self.cur_val).wrapping_add(1i32); let __v164 = (self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int()).wrapping_neg(); self.mem[(__ix163) as usize].set_int(__v164); }
                        { let __ix165 = (self.cur_val).wrapping_add(2i32); let __v166 = (self.mem[((self.cur_val).wrapping_add(2i32)) as usize].int()).wrapping_neg(); self.mem[(__ix165) as usize].set_int(__v166); }
                        { let __ix167 = (self.cur_val).wrapping_add(3i32); let __v168 = (self.mem[((self.cur_val).wrapping_add(3i32)) as usize].int()).wrapping_neg(); self.mem[(__ix167) as usize].set_int(__v168); }
                    }
                }
            } else {
                // §430
                self.cur_val = (self.cur_val).wrapping_neg();
            }
        } else {
            if ((self.cur_val_level >= 2i32) && (self.cur_val_level <= 3i32)) {
                { let __ix169 = self.cur_val; let __v170 = (self.mem[(self.cur_val) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix169) as usize].set_hh_rh(__v170); }
            }
        }
    }

    /// The `scan_int` routine is used also to scan the integer part of a
    /// fraction; for example, the `\.3' in `\.{3.14159}' will be found by
    /// `scan_int`. The `scan_dimen` routine assumes that `cur_tok=point_token`
    /// after the integer part of such a fraction has been scanned by `scan_int`,
    /// and that the decimal point has been backed up to be scanned again.
    // §440
    pub fn scan_int(&mut self) {
        let mut negative: bool = false; // §440
        let mut m: i32 = 0; // §440
        let mut d: small_number = 0; // §440
        let mut vacuous: bool = false; // §440
        let mut OK_so_far: bool = false; // §440
        self.radix = 0i32;
        OK_so_far = true;
        // §441
        negative = false;
        loop {
            // §406
            loop {
                self.get_x_token();
                if (self.cur_cmd != 10i32) { break; }
            }
            // §441
            if (self.cur_tok == 3117i32) {
                {
                    negative = (!negative);
                    self.cur_tok = 3115i32;
                }
            }
            if (self.cur_tok != 3115i32) { break; }
        }
        // §440
        if (self.cur_tok == 3168i32) {
            // §442
            {
                self.get_token();
                if (self.cur_tok < 4095i32) {
                    {
                        self.cur_val = self.cur_chr;
                        if (self.cur_cmd <= 2i32) {
                            if (self.cur_cmd == 2i32) {
                                self.align_state = (self.align_state).wrapping_add(1i32);
                            } else {
                                self.align_state = (self.align_state).wrapping_sub(1i32);
                            }
                        }
                    }
                } else {
                    if (self.cur_tok < 4352i32) {
                        self.cur_val = (self.cur_tok).wrapping_sub(4096i32);
                    } else {
                        self.cur_val = (self.cur_tok).wrapping_sub(4352i32);
                    }
                }
                if (self.cur_val > 255i32) {
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(698i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[(1i32) as usize] = 699i32;
                            self.help_line[(0i32) as usize] = 700i32;
                        }
                        self.cur_val = 48i32;
                        self.back_error();
                    }
                } else {
                    // §443
                    {
                        self.get_x_token();
                        if (self.cur_cmd != 10i32) {
                            self.back_input();
                        }
                    }
                }
            }
        } else {
            // §440
            if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                self.scan_something_internal(0i32, false);
            } else {
                // §444
                {
                    'l_done_f: {
                        self.radix = 10i32;
                        m = 214748364i32;
                        if (self.cur_tok == 3111i32) {
                            {
                                self.radix = 8i32;
                                m = 268435456i32;
                                self.get_x_token();
                            }
                        } else {
                            if (self.cur_tok == 3106i32) {
                                {
                                    self.radix = 16i32;
                                    m = 134217728i32;
                                    self.get_x_token();
                                }
                            }
                        }
                        vacuous = true;
                        self.cur_val = 0i32;
                        // §445
                        while true {
                            {
                                if (((self.cur_tok < (3120i32).wrapping_add(self.radix)) && (self.cur_tok >= 3120i32)) && (self.cur_tok <= 3129i32)) {
                                    d = (self.cur_tok).wrapping_sub(3120i32);
                                } else {
                                    if (self.radix == 16i32) {
                                        if ((self.cur_tok <= 2886i32) && (self.cur_tok >= 2881i32)) {
                                            d = (self.cur_tok).wrapping_sub(2871i32);
                                        } else {
                                            if ((self.cur_tok <= 3142i32) && (self.cur_tok >= 3137i32)) {
                                                d = (self.cur_tok).wrapping_sub(3127i32);
                                            } else {
                                                break 'l_done_f;
                                            }
                                        }
                                    } else {
                                        break 'l_done_f;
                                    }
                                }
                                vacuous = false;
                                if ((self.cur_val >= m) && (((self.cur_val > m) || (d > 7i32)) || (self.radix != 10i32))) {
                                    {
                                        if OK_so_far {
                                            {
                                                {
                                                    if (self.interaction == 3i32) {
                                                    }
                                                    self.print_nl(262i32);
                                                    self.print(701i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[(1i32) as usize] = 702i32;
                                                    self.help_line[(0i32) as usize] = 703i32;
                                                }
                                                self.error();
                                                self.cur_val = 2147483647i32;
                                                OK_so_far = false;
                                            }
                                        }
                                    }
                                } else {
                                    self.cur_val = ((self.cur_val).wrapping_mul(self.radix)).wrapping_add(d);
                                }
                                self.get_x_token();
                            }
                        }
                    }
                    // §444
                    if vacuous {
                        // §446
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(664i32);
                            }
                            {
                                self.help_ptr = 3i32;
                                self.help_line[(2i32) as usize] = 665i32;
                                self.help_line[(1i32) as usize] = 666i32;
                                self.help_line[(0i32) as usize] = 667i32;
                            }
                            self.back_error();
                        }
                    } else {
                        // §444
                        if (self.cur_cmd != 10i32) {
                            self.back_input();
                        }
                    }
                }
            }
        }
        // §440
        if negative {
            self.cur_val = (self.cur_val).wrapping_neg();
        }
    }

    /// Constructions like `\.{-\'77 pt}' are legal dimensions, so `scan_dimen`
    /// may begin with `scan_int`. This explains why it is convenient to use
    /// `scan_int` also for the integer part of a decimal fraction.
    /// Several branches of `scan_dimen` work with `cur_val` as an integer and
    /// with an auxiliary fraction `f`, so that the actual quantity of interest is
    /// $`cur_val`+`f`/2^{16}$. At the end of the routine, this ``unpacked''
    /// representation is put into the single word `cur_val`, which suddenly
    /// switches significance from `integer` to `scaled`.
    // §448
    pub fn scan_dimen(&mut self, mut mu: bool, mut inf: bool, mut shortcut: bool) {
        let mut negative: bool = false; // §448
        let mut f: i32 = 0; // §448
        let mut num: i32 = 0; // §450
        let mut denom: i32 = 0; // §450
        let mut k: small_number = 0; // §450
        let mut kk: small_number = 0; // §450
        let mut p: halfword = 0; // §450
        let mut q: halfword = 0; // §450
        let mut v: scaled = 0; // §450
        let mut save_cur_val: i32 = 0; // §450
        'l_L89_f: {
            'l_done_f: {
                'l_L88_f: {
                    'l_done2_f: {
                        'l_not_found_f: {
                            'l_found_f: {
                                f = 0i32;
                                self.arith_error = false;
                                self.cur_order = 0i32;
                                negative = false;
                                if (!shortcut) {
                                    {
                                        // §441
                                        negative = false;
                                        loop {
                                            // §406
                                            loop {
                                                self.get_x_token();
                                                if (self.cur_cmd != 10i32) { break; }
                                            }
                                            // §441
                                            if (self.cur_tok == 3117i32) {
                                                {
                                                    negative = (!negative);
                                                    self.cur_tok = 3115i32;
                                                }
                                            }
                                            if (self.cur_tok != 3115i32) { break; }
                                        }
                                        // §448
                                        if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                                            // §449
                                            if mu {
                                                {
                                                    self.scan_something_internal(3i32, false);
                                                    // §451
                                                    if (self.cur_val_level >= 2i32) {
                                                        {
                                                            v = self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int();
                                                            self.delete_glue_ref(self.cur_val);
                                                            self.cur_val = v;
                                                        }
                                                    }
                                                    // §449
                                                    if (self.cur_val_level == 3i32) {
                                                        break 'l_L89_f;
                                                    }
                                                    if (self.cur_val_level != 0i32) {
                                                        self.mu_error();
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.scan_something_internal(1i32, false);
                                                    if (self.cur_val_level == 1i32) {
                                                        break 'l_L89_f;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §448
                                            {
                                                self.back_input();
                                                if (self.cur_tok == 3116i32) {
                                                    self.cur_tok = 3118i32;
                                                }
                                                if (self.cur_tok != 3118i32) {
                                                    self.scan_int();
                                                } else {
                                                    {
                                                        self.radix = 10i32;
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                                if (self.cur_tok == 3116i32) {
                                                    self.cur_tok = 3118i32;
                                                }
                                                if ((self.radix == 10i32) && (self.cur_tok == 3118i32)) {
                                                    // §452
                                                    {
                                                        'l_done1_f: {
                                                            k = 0i32;
                                                            p = 0i32;
                                                            self.get_token();
                                                            while true {
                                                                {
                                                                    self.get_x_token();
                                                                    if ((self.cur_tok > 3129i32) || (self.cur_tok < 3120i32)) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                    if (k < 17i32) {
                                                                        {
                                                                            q = self.get_avail();
                                                                            self.mem[(q) as usize].set_hh_rh(p);
                                                                            { let __v171 = (self.cur_tok).wrapping_sub(3120i32); self.mem[(q) as usize].set_hh_lh(__v171); }
                                                                            p = q;
                                                                            k = (k).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        {
                                                            let __for_end_14 = 1i32;
                                                            kk = k;
                                                            while kk >= __for_end_14 {
                                                                {
                                                                    { let __v172 = self.mem[(p) as usize].hh().lh(); self.dig[((kk).wrapping_sub(1i32)) as usize] = __v172; }
                                                                    q = p;
                                                                    p = self.mem[(p) as usize].hh().rh();
                                                                    {
                                                                        { let __v173 = self.avail; self.mem[(q) as usize].set_hh_rh(__v173); }
                                                                        self.avail = q;
                                                                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                                    }
                                                                }
                                                                kk = kk.wrapping_sub(1);
                                                            }
                                                        }
                                                        f = self.round_decimals(k);
                                                        if (self.cur_cmd != 10i32) {
                                                            self.back_input();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // §448
                                if (self.cur_val < 0i32) {
                                    {
                                        negative = (!negative);
                                        self.cur_val = (self.cur_val).wrapping_neg();
                                    }
                                }
                                // §453
                                if inf {
                                    // §454
                                    if self.scan_keyword(311i32) {
                                        {
                                            self.cur_order = 1i32;
                                            while self.scan_keyword(108i32) {
                                                {
                                                    if (self.cur_order == 3i32) {
                                                        {
                                                            {
                                                                if (self.interaction == 3i32) {
                                                                }
                                                                self.print_nl(262i32);
                                                                self.print(705i32);
                                                            }
                                                            self.print(706i32);
                                                            {
                                                                self.help_ptr = 1i32;
                                                                self.help_line[(0i32) as usize] = 707i32;
                                                            }
                                                            self.error();
                                                        }
                                                    } else {
                                                        self.cur_order = (self.cur_order).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            break 'l_L88_f;
                                        }
                                    }
                                }
                                // §455
                                save_cur_val = self.cur_val;
                                // §406
                                loop {
                                    self.get_x_token();
                                    if (self.cur_cmd != 10i32) { break; }
                                }
                                // §455
                                if ((self.cur_cmd < 68i32) || (self.cur_cmd > 89i32)) {
                                    self.back_input();
                                } else {
                                    {
                                        if mu {
                                            {
                                                self.scan_something_internal(3i32, false);
                                                // §451
                                                if (self.cur_val_level >= 2i32) {
                                                    {
                                                        v = self.mem[((self.cur_val).wrapping_add(1i32)) as usize].int();
                                                        self.delete_glue_ref(self.cur_val);
                                                        self.cur_val = v;
                                                    }
                                                }
                                                // §455
                                                if (self.cur_val_level != 3i32) {
                                                    self.mu_error();
                                                }
                                            }
                                        } else {
                                            self.scan_something_internal(1i32, false);
                                        }
                                        v = self.cur_val;
                                        break 'l_found_f;
                                    }
                                }
                                if mu {
                                    break 'l_not_found_f;
                                }
                                if self.scan_keyword(708i32) {
                                    v = self.font_info[((6i32).wrapping_add(self.param_base[(self.eqtb[((3934i32) - 1) as usize].hh().rh()) as usize])) as usize].int();
                                } else {
                                    if self.scan_keyword(709i32) {
                                        v = self.font_info[((5i32).wrapping_add(self.param_base[(self.eqtb[((3934i32) - 1) as usize].hh().rh()) as usize])) as usize].int();
                                    } else {
                                        break 'l_not_found_f;
                                    }
                                }
                                // §443
                                {
                                    self.get_x_token();
                                    if (self.cur_cmd != 10i32) {
                                        self.back_input();
                                    }
                                }
                            }
                            // §455
                            self.cur_val = { let __a174_0 = save_cur_val; let __a174_1 = v; let __a174_2 = self.xn_over_d(v, f, 65536i32); let __a174_3 = 1073741823i32; self.mult_and_add(__a174_0, __a174_1, __a174_2, __a174_3) };
                            break 'l_L89_f;
                        }
                        // §453
                        if mu {
                            // §456
                            if self.scan_keyword(337i32) {
                                break 'l_L88_f;
                            } else {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(705i32);
                                    }
                                    self.print(710i32);
                                    {
                                        self.help_ptr = 4i32;
                                        self.help_line[(3i32) as usize] = 711i32;
                                        self.help_line[(2i32) as usize] = 712i32;
                                        self.help_line[(1i32) as usize] = 713i32;
                                        self.help_line[(0i32) as usize] = 714i32;
                                    }
                                    self.error();
                                    break 'l_L88_f;
                                }
                            }
                        }
                        // §453
                        if self.scan_keyword(704i32) {
                            // §457
                            {
                                self.prepare_mag();
                                if (self.eqtb[((5280i32) - 1) as usize].int() != 1000i32) {
                                    {
                                        self.cur_val = self.xn_over_d(self.cur_val, 1000i32, self.eqtb[((5280i32) - 1) as usize].int());
                                        f = (((1000i32).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / self.eqtb[((5280i32) - 1) as usize].int());
                                        self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                                        f = (f % 65536i32);
                                    }
                                }
                            }
                        }
                        // §453
                        if self.scan_keyword(397i32) {
                            break 'l_L88_f;
                        }
                        // §458
                        if self.scan_keyword(715i32) {
                            {
                                num = 7227i32;
                                denom = 100i32;
                            }
                        } else {
                            if self.scan_keyword(716i32) {
                                {
                                    num = 12i32;
                                    denom = 1i32;
                                }
                            } else {
                                if self.scan_keyword(717i32) {
                                    {
                                        num = 7227i32;
                                        denom = 254i32;
                                    }
                                } else {
                                    if self.scan_keyword(718i32) {
                                        {
                                            num = 7227i32;
                                            denom = 2540i32;
                                        }
                                    } else {
                                        if self.scan_keyword(719i32) {
                                            {
                                                num = 7227i32;
                                                denom = 7200i32;
                                            }
                                        } else {
                                            if self.scan_keyword(720i32) {
                                                {
                                                    num = 1238i32;
                                                    denom = 1157i32;
                                                }
                                            } else {
                                                if self.scan_keyword(721i32) {
                                                    {
                                                        num = 14856i32;
                                                        denom = 1157i32;
                                                    }
                                                } else {
                                                    if self.scan_keyword(722i32) {
                                                        break 'l_done_f;
                                                    } else {
                                                        // §459
                                                        {
                                                            {
                                                                if (self.interaction == 3i32) {
                                                                }
                                                                self.print_nl(262i32);
                                                                self.print(705i32);
                                                            }
                                                            self.print(723i32);
                                                            {
                                                                self.help_ptr = 6i32;
                                                                self.help_line[(5i32) as usize] = 724i32;
                                                                self.help_line[(4i32) as usize] = 725i32;
                                                                self.help_line[(3i32) as usize] = 726i32;
                                                                self.help_line[(2i32) as usize] = 712i32;
                                                                self.help_line[(1i32) as usize] = 713i32;
                                                                self.help_line[(0i32) as usize] = 714i32;
                                                            }
                                                            self.error();
                                                            break 'l_done2_f;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §458
                        self.cur_val = self.xn_over_d(self.cur_val, num, denom);
                        f = (((num).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / denom);
                        self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                        f = (f % 65536i32);
                    }
                }
                // §453
                if (self.cur_val >= 16384i32) {
                    self.arith_error = true;
                } else {
                    self.cur_val = ((self.cur_val).wrapping_mul(65536i32)).wrapping_add(f);
                }
            }
            // §443
            {
                self.get_x_token();
                if (self.cur_cmd != 10i32) {
                    self.back_input();
                }
            }
        }
        // §448
        if (self.arith_error || ((self.cur_val).wrapping_abs() >= 1073741824i32)) {
            // §460
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(727i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 728i32;
                    self.help_line[(0i32) as usize] = 729i32;
                }
                self.error();
                self.cur_val = 1073741823i32;
                self.arith_error = false;
            }
        }
        // §448
        if negative {
            self.cur_val = (self.cur_val).wrapping_neg();
        }
    }

    /// The final member of \TeX's value-scanning trio is `scan_glue`, which
    /// makes `cur_val` point to a glue specification. The reference count of that
    /// glue spec will take account of the fact that `cur_val` is pointing to~it.
    /// The `level` parameter should be either `glue_val` or `mu_val`.
    /// Since `scan_dimen` was so much more complex than `scan_int`, we might expect
    /// `scan_glue` to be even worse. But fortunately, it is very simple, since
    /// most of the work has already been done.
    // §461
    pub fn scan_glue(&mut self, mut level: small_number) {
        let mut negative: bool = false; // §461
        let mut q: halfword = 0; // §461
        let mut mu: bool = false; // §461
        'l_exit_f: {
            mu = (level == 3i32);
            // §441
            negative = false;
            loop {
                // §406
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != 10i32) { break; }
                }
                // §441
                if (self.cur_tok == 3117i32) {
                    {
                        negative = (!negative);
                        self.cur_tok = 3115i32;
                    }
                }
                if (self.cur_tok != 3115i32) { break; }
            }
            // §461
            if ((self.cur_cmd >= 68i32) && (self.cur_cmd <= 89i32)) {
                {
                    self.scan_something_internal(level, negative);
                    if (self.cur_val_level >= 2i32) {
                        {
                            if (self.cur_val_level != level) {
                                self.mu_error();
                            }
                            break 'l_exit_f;
                        }
                    }
                    if (self.cur_val_level == 0i32) {
                        self.scan_dimen(mu, false, true);
                    } else {
                        if (level == 3i32) {
                            self.mu_error();
                        }
                    }
                }
            } else {
                {
                    self.back_input();
                    self.scan_dimen(mu, false, false);
                    if negative {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                }
            }
            // §462
            q = self.new_spec(0i32);
            { let __v175 = self.cur_val; self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v175); }
            if self.scan_keyword(730i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v176 = self.cur_val; self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v176); }
                    { let __v177 = self.cur_order; self.mem[(q) as usize].set_hh_b0(__v177); }
                }
            }
            if self.scan_keyword(731i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v178 = self.cur_val; self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v178); }
                    { let __v179 = self.cur_order; self.mem[(q) as usize].set_hh_b1(__v179); }
                }
            }
            self.cur_val = q;
        }
        // §461
    }

    /// Here's a similar procedure that returns a pointer to a rule node. This
    /// routine is called just after \TeX\ has seen \.{\\hrule} or \.{\\vrule};
    /// therefore `cur_cmd` will be either `hrule` or `vrule`. The idea is to store
    /// the default rule dimensions in the node, then to override them if
    /// `\.{height}' or `\.{width}' or `\.{depth}' specifications are
    /// found (in any order).
    // §463
    pub fn scan_rule_spec(&mut self) -> halfword {
        let mut scan_rule_spec: halfword = 0;
        let mut q: halfword = 0; // §463
        q = self.new_rule();
        if (self.cur_cmd == 35i32) {
            self.mem[((q).wrapping_add(1i32)) as usize].set_int(26214i32);
        } else {
            {
                self.mem[((q).wrapping_add(3i32)) as usize].set_int(26214i32);
                self.mem[((q).wrapping_add(2i32)) as usize].set_int(0i32);
            }
        }
        'l_reswitch_b: loop {
            if self.scan_keyword(732i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v180 = self.cur_val; self.mem[((q).wrapping_add(1i32)) as usize].set_int(__v180); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(733i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v181 = self.cur_val; self.mem[((q).wrapping_add(3i32)) as usize].set_int(__v181); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(734i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v182 = self.cur_val; self.mem[((q).wrapping_add(2i32)) as usize].set_int(__v182); }
                    continue 'l_reswitch_b;
                }
            }
            scan_rule_spec = q;
            break 'l_reswitch_b;
        }
        scan_rule_spec
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
    // §464
    pub fn str_toks(&mut self, mut b: pool_pointer) -> halfword {
        let mut str_toks: halfword = 0;
        let mut p: halfword = 0; // §464
        let mut q: halfword = 0; // §464
        let mut t: halfword = 0; // §464
        let mut k: pool_pointer = 0; // §464
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        p = 29997i32;
        self.mem[(p) as usize].set_hh_rh(0i32);
        k = b;
        while (k < self.pool_ptr) {
            {
                t = self.str_pool[(k) as usize];
                if (t == 32i32) {
                    t = 2592i32;
                } else {
                    t = (3072i32).wrapping_add(t);
                }
                {
                    {
                        q = self.avail;
                        if (q == 0i32) {
                            q = self.get_avail();
                        } else {
                            {
                                self.avail = self.mem[(q) as usize].hh().rh();
                                self.mem[(q) as usize].set_hh_rh(0i32);
                                self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                            }
                        }
                    }
                    self.mem[(p) as usize].set_hh_rh(q);
                    self.mem[(q) as usize].set_hh_lh(t);
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
    // §465
    pub fn the_toks(&mut self) -> halfword {
        let mut the_toks: halfword = 0;
        let mut old_setting: i32 = 0; // §465
        let mut p: halfword = 0; // §465
        let mut q: halfword = 0; // §465
        let mut r: halfword = 0; // §465
        let mut b: pool_pointer = 0; // §465
        self.get_x_token();
        self.scan_something_internal(5i32, false);
        if (self.cur_val_level >= 4i32) {
            // §466
            {
                p = 29997i32;
                self.mem[(p) as usize].set_hh_rh(0i32);
                if (self.cur_val_level == 4i32) {
                    {
                        q = self.get_avail();
                        self.mem[(p) as usize].set_hh_rh(q);
                        { let __v183 = (4095i32).wrapping_add(self.cur_val); self.mem[(q) as usize].set_hh_lh(__v183); }
                        p = q;
                    }
                } else {
                    if (self.cur_val != 0i32) {
                        {
                            r = self.mem[(self.cur_val) as usize].hh().rh();
                            while (r != 0i32) {
                                {
                                    {
                                        {
                                            q = self.avail;
                                            if (q == 0i32) {
                                                q = self.get_avail();
                                            } else {
                                                {
                                                    self.avail = self.mem[(q) as usize].hh().rh();
                                                    self.mem[(q) as usize].set_hh_rh(0i32);
                                                    self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                        self.mem[(p) as usize].set_hh_rh(q);
                                        { let __v184 = self.mem[(r) as usize].hh().lh(); self.mem[(q) as usize].set_hh_lh(__v184); }
                                        p = q;
                                    }
                                    r = self.mem[(r) as usize].hh().rh();
                                }
                            }
                        }
                    }
                }
                the_toks = p;
            }
        } else {
            // §465
            {
                old_setting = self.selector;
                self.selector = 21i32;
                b = self.pool_ptr;
                match self.cur_val_level {
                    0 => {
                        self.print_int(self.cur_val);
                    }
                    1 => {
                        {
                            self.print_scaled(self.cur_val);
                            self.print(397i32);
                        }
                    }
                    2 => {
                        {
                            self.print_spec(self.cur_val, 397i32);
                            self.delete_glue_ref(self.cur_val);
                        }
                    }
                    3 => {
                        {
                            self.print_spec(self.cur_val, 337i32);
                            self.delete_glue_ref(self.cur_val);
                        }
                    }
                    _ => {}
                }
                self.selector = old_setting;
                the_toks = self.str_toks(b);
            }
        }
        the_toks
    }

    /// Here's part of the `expand` subroutine that we are now ready to complete:
    // §467
    pub fn ins_the_toks(&mut self) {
        { let __v185 = self.the_toks(); self.mem[(29988i32) as usize].set_hh_rh(__v185); }
        self.begin_token_list(self.mem[(29997i32) as usize].hh().rh(), 4i32);
    }

    /// The procedure `conv_toks` uses `str_toks` to insert the token list
    /// for `convert` functions into the scanner; `\.{\\outer}' control sequences
    /// are allowed to follow `\.{\\string}' and `\.{\\meaning}'.
    // §470
    pub fn conv_toks(&mut self) {
        let mut old_setting: i32 = 0; // §470
        let mut c: i32 = 0; // §470
        let mut save_scanner_status: small_number = 0; // §470
        let mut b: pool_pointer = 0; // §470
        c = self.cur_chr;
        // §471
        match c {
            0 | 1 => {
                self.scan_int();
            }
            2 | 3 => {
                {
                    save_scanner_status = self.scanner_status;
                    self.scanner_status = 0i32;
                    self.get_token();
                    self.scanner_status = save_scanner_status;
                }
            }
            4 => {
                self.scan_font_ident();
            }
            5 => {
                if (self.job_name == 0i32) {
                    self.open_log_file();
                }
            }
            _ => {}
        }
        // §470
        old_setting = self.selector;
        self.selector = 21i32;
        b = self.pool_ptr;
        // §472
        match c {
            0 => {
                self.print_int(self.cur_val);
            }
            1 => {
                self.print_roman_int(self.cur_val);
            }
            2 => {
                if (self.cur_cs != 0i32) {
                    self.sprint_cs(self.cur_cs);
                } else {
                    self.print_char(self.cur_chr);
                }
            }
            3 => {
                self.print_meaning();
            }
            4 => {
                {
                    self.print(self.font_name[(self.cur_val) as usize]);
                    if (self.font_size[(self.cur_val) as usize] != self.font_dsize[(self.cur_val) as usize]) {
                        {
                            self.print(741i32);
                            self.print_scaled(self.font_size[(self.cur_val) as usize]);
                            self.print(397i32);
                        }
                    }
                }
            }
            5 => {
                self.print(self.job_name);
            }
            _ => {}
        }
        // §470
        self.selector = old_setting;
        { let __v186 = self.str_toks(b); self.mem[(29988i32) as usize].set_hh_rh(__v186); }
        self.begin_token_list(self.mem[(29997i32) as usize].hh().rh(), 4i32);
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
    // §473
    pub fn scan_toks(&mut self, mut macro_def: bool, mut xpand: bool) -> halfword {
        let mut scan_toks: halfword = 0;
        let mut t: halfword = 0; // §473
        let mut s: halfword = 0; // §473
        let mut p: halfword = 0; // §473
        let mut q: halfword = 0; // §473
        let mut unbalance: halfword = 0; // §473
        let mut hash_brace: halfword = 0; // §473
        'l_found_f: {
            if macro_def {
                self.scanner_status = 2i32;
            } else {
                self.scanner_status = 5i32;
            }
            self.warning_index = self.cur_cs;
            self.def_ref = self.get_avail();
            { let __ix187 = self.def_ref; self.mem[(__ix187) as usize].set_hh_lh(0i32); }
            p = self.def_ref;
            hash_brace = 0i32;
            t = 3120i32;
            if macro_def {
                // §474
                {
                    'l_done_f: {
                        'l_done1_f: {
                            while true {
                                {
                                    'l_continue_b: loop {
                                        self.get_token();
                                        if (self.cur_tok < 768i32) {
                                            break 'l_done1_f;
                                        }
                                        if (self.cur_cmd == 6i32) {
                                            // §476
                                            {
                                                s = (3328i32).wrapping_add(self.cur_chr);
                                                self.get_token();
                                                if (self.cur_tok < 512i32) {
                                                    {
                                                        hash_brace = self.cur_tok;
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[(p) as usize].set_hh_rh(q);
                                                            { let __v188 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v188); }
                                                            p = q;
                                                        }
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[(p) as usize].set_hh_rh(q);
                                                            self.mem[(q) as usize].set_hh_lh(3584i32);
                                                            p = q;
                                                        }
                                                        break 'l_done_f;
                                                    }
                                                }
                                                if (t == 3129i32) {
                                                    {
                                                        {
                                                            if (self.interaction == 3i32) {
                                                            }
                                                            self.print_nl(262i32);
                                                            self.print(744i32);
                                                        }
                                                        {
                                                            self.help_ptr = 2i32;
                                                            self.help_line[(1i32) as usize] = 745i32;
                                                            self.help_line[(0i32) as usize] = 746i32;
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
                                                                    if (self.interaction == 3i32) {
                                                                    }
                                                                    self.print_nl(262i32);
                                                                    self.print(747i32);
                                                                }
                                                                {
                                                                    self.help_ptr = 2i32;
                                                                    self.help_line[(1i32) as usize] = 748i32;
                                                                    self.help_line[(0i32) as usize] = 749i32;
                                                                }
                                                                self.back_error();
                                                            }
                                                        }
                                                        self.cur_tok = s;
                                                    }
                                                }
                                            }
                                        }
                                        // §474
                                        {
                                            q = self.get_avail();
                                            self.mem[(p) as usize].set_hh_rh(q);
                                            { let __v189 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v189); }
                                            p = q;
                                        }
                                        break 'l_continue_b;
                                    }
                                }
                            }
                        }
                        {
                            q = self.get_avail();
                            self.mem[(p) as usize].set_hh_rh(q);
                            self.mem[(q) as usize].set_hh_lh(3584i32);
                            p = q;
                        }
                        if (self.cur_cmd == 2i32) {
                            // §475
                            {
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(657i32);
                                }
                                self.align_state = (self.align_state).wrapping_add(1i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[(1i32) as usize] = 742i32;
                                    self.help_line[(0i32) as usize] = 743i32;
                                }
                                self.error();
                                break 'l_found_f;
                            }
                        }
                    }
                    // §474
                }
            } else {
                // §473
                self.scan_left_brace();
            }
            // §477
            unbalance = 1i32;
            while true {
                {
                    if xpand {
                        // §478
                        {
                            'l_done2_f: {
                                while true {
                                    {
                                        self.get_next();
                                        if (self.cur_cmd <= 100i32) {
                                            break 'l_done2_f;
                                        }
                                        if (self.cur_cmd != 109i32) {
                                            self.expand();
                                        } else {
                                            {
                                                q = self.the_toks();
                                                if (self.mem[(29997i32) as usize].hh().rh() != 0i32) {
                                                    {
                                                        { let __v190 = self.mem[(29997i32) as usize].hh().rh(); self.mem[(p) as usize].set_hh_rh(__v190); }
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
                        // §477
                        self.get_token();
                    }
                    if (self.cur_tok < 768i32) {
                        if (self.cur_cmd < 2i32) {
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
                        if (self.cur_cmd == 6i32) {
                            if macro_def {
                                // §479
                                {
                                    s = self.cur_tok;
                                    if xpand {
                                        self.get_x_token();
                                    } else {
                                        self.get_token();
                                    }
                                    if (self.cur_cmd != 6i32) {
                                        if ((self.cur_tok <= 3120i32) || (self.cur_tok > t)) {
                                            {
                                                {
                                                    if (self.interaction == 3i32) {
                                                    }
                                                    self.print_nl(262i32);
                                                    self.print(750i32);
                                                }
                                                self.sprint_cs(self.warning_index);
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[(2i32) as usize] = 751i32;
                                                    self.help_line[(1i32) as usize] = 752i32;
                                                    self.help_line[(0i32) as usize] = 753i32;
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
                    // §477
                    {
                        q = self.get_avail();
                        self.mem[(p) as usize].set_hh_rh(q);
                        { let __v191 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v191); }
                        p = q;
                    }
                }
            }
        }
        // §473
        self.scanner_status = 0i32;
        if (hash_brace != 0i32) {
            {
                q = self.get_avail();
                self.mem[(p) as usize].set_hh_rh(q);
                self.mem[(q) as usize].set_hh_lh(hash_brace);
                p = q;
            }
        }
        scan_toks = p;
        scan_toks
    }

    /// The `read_toks` procedure constructs a token list like that for any
    /// macro definition, and makes `cur_val` point to it. Parameter `r` points
    /// to the control sequence that will receive this token list.
    // §482
    pub fn read_toks(&mut self, mut n: i32, mut r: halfword) {
        let mut p: halfword = 0; // §482
        let mut q: halfword = 0; // §482
        let mut s: i32 = 0; // §482
        let mut m: small_number = 0; // §482
        self.scanner_status = 2i32;
        self.warning_index = r;
        self.def_ref = self.get_avail();
        { let __ix192 = self.def_ref; self.mem[(__ix192) as usize].set_hh_lh(0i32); }
        p = self.def_ref;
        {
            q = self.get_avail();
            self.mem[(p) as usize].set_hh_rh(q);
            self.mem[(q) as usize].set_hh_lh(3584i32);
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
                // §483
                self.begin_file_reading();
                self.cur_input.name_field = (m).wrapping_add(1i32);
                if (self.read_open[(m) as usize] == 2i32) {
                    // §484
                    if (self.interaction > 1i32) {
                        if (n < 0i32) {
                            {
                                self.print(338i32);
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
                        self.fatal_error(754i32);
                    }
                } else {
                    // §483
                    if (self.read_open[(m) as usize] == 1i32) {
                        // §485
                        if { let mut __f = ::core::mem::take(&mut self.read_file[(m) as usize]); let __r = self.input_ln(&mut __f, false); self.read_file[(m) as usize] = __f; __r } {
                            self.read_open[(m) as usize] = 0i32;
                        } else {
                            {
                                { let mut __f = ::core::mem::take(&mut self.read_file[(m) as usize]); let __r = self.a_close(&mut __f); self.read_file[(m) as usize] = __f; __r };
                                self.read_open[(m) as usize] = 2i32;
                            }
                        }
                    } else {
                        // §486
                        {
                            if (!{ let mut __f = ::core::mem::take(&mut self.read_file[(m) as usize]); let __r = self.input_ln(&mut __f, true); self.read_file[(m) as usize] = __f; __r }) {
                                {
                                    { let mut __f = ::core::mem::take(&mut self.read_file[(m) as usize]); let __r = self.a_close(&mut __f); self.read_file[(m) as usize] = __f; __r };
                                    self.read_open[(m) as usize] = 2i32;
                                    if (self.align_state != 1000000i32) {
                                        {
                                            self.runaway();
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                self.print_nl(262i32);
                                                self.print(755i32);
                                            }
                                            self.print_esc(534i32);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[(0i32) as usize] = 756i32;
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
                // §483
                self.cur_input.limit_field = self.last;
                if ((self.eqtb[((5311i32) - 1) as usize].int() < 0i32) || (self.eqtb[((5311i32) - 1) as usize].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix193 = self.cur_input.limit_field; let __v194 = self.eqtb[((5311i32) - 1) as usize].int(); self.buffer[(__ix193) as usize] = __v194; }
                }
                self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                self.cur_input.loc_field = self.cur_input.start_field;
                self.cur_input.state_field = 33i32;
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
                            self.mem[(p) as usize].set_hh_rh(q);
                            { let __v195 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v195); }
                            p = q;
                        }
                    }
                }
            }
            self.end_file_reading();
            if (self.align_state == 1000000i32) { break; }
        }
        // §482
        self.cur_val = self.def_ref;
        self.scanner_status = 0i32;
        self.align_state = s;
    }

    /// Here is a procedure that ignores text until coming to an \.{\\or},
    /// \.{\\else}, or \.{\\fi} at the current level of $\.{\\if}\ldots\.{\\fi}$
    /// nesting. After it has acted, `cur_chr` will indicate the token that
    /// was found, but `cur_tok` will not be set (because this makes the
    /// procedure run faster).
    // §494
    pub fn pass_text(&mut self) {
        let mut l: i32 = 0; // §494
        let mut save_scanner_status: small_number = 0; // §494
        'l_done_f: {
            save_scanner_status = self.scanner_status;
            self.scanner_status = 1i32;
            l = 0i32;
            self.skip_line = self.line;
            while true {
                {
                    self.get_next();
                    if (self.cur_cmd == 106i32) {
                        {
                            if (l == 0i32) {
                                break 'l_done_f;
                            }
                            if (self.cur_chr == 2i32) {
                                l = (l).wrapping_sub(1i32);
                            }
                        }
                    } else {
                        if (self.cur_cmd == 105i32) {
                            l = (l).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.scanner_status = save_scanner_status;
    }

    /// Here's a procedure that changes the `if_limit` code corresponding to
    /// a given value of `cond_ptr`.
    // §497
    pub fn change_if_limit(&mut self, mut l: small_number, mut p: halfword) {
        let mut q: halfword = 0; // §497
        'l_exit_f: {
            if (p == self.cond_ptr) {
                self.if_limit = l;
            } else {
                {
                    q = self.cond_ptr;
                    while true {
                        {
                            if (q == 0i32) {
                                self.confusion(757i32);
                            }
                            if (self.mem[(q) as usize].hh().rh() == p) {
                                {
                                    self.mem[(q) as usize].set_hh_b0(l);
                                    break 'l_exit_f;
                                }
                            }
                            q = self.mem[(q) as usize].hh().rh();
                        }
                    }
                }
            }
        }
    }

    /// A condition is started when the `expand` procedure encounters
    /// an `if_test` command; in that case `expand` reduces to `conditional`,
    /// which is a recursive procedure.
    // §498
    pub fn conditional(&mut self) {
        let mut b: bool = false; // §498
        let mut r: i32 = 0; // §498
        let mut m: i32 = 0; // §498
        let mut n: i32 = 0; // §498
        let mut p: halfword = 0; // §498
        let mut q: halfword = 0; // §498
        let mut save_scanner_status: small_number = 0; // §498
        let mut save_cond_ptr: halfword = 0; // §498
        let mut this_if: small_number = 0; // §498
        'l_exit_f: {
            'l_common_ending_f: {
                // §495
                {
                    p = self.get_node(2i32);
                    { let __v196 = self.cond_ptr; self.mem[(p) as usize].set_hh_rh(__v196); }
                    { let __v197 = self.if_limit; self.mem[(p) as usize].set_hh_b0(__v197); }
                    { let __v198 = self.cur_if; self.mem[(p) as usize].set_hh_b1(__v198); }
                    { let __v199 = self.if_line; self.mem[((p).wrapping_add(1i32)) as usize].set_int(__v199); }
                    self.cond_ptr = p;
                    self.cur_if = self.cur_chr;
                    self.if_limit = 1i32;
                    self.if_line = self.line;
                }
                // §498
                save_cond_ptr = self.cond_ptr;
                this_if = self.cur_chr;
                // §501
                match this_if {
                    0 | 1 => {
                        // §506
                        {
                            {
                                self.get_x_token();
                                if (self.cur_cmd == 0i32) {
                                    if (self.cur_chr == 257i32) {
                                        {
                                            self.cur_cmd = 13i32;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(4096i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > 13i32) || (self.cur_chr > 255i32)) {
                                {
                                    m = 0i32;
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
                                if (self.cur_cmd == 0i32) {
                                    if (self.cur_chr == 257i32) {
                                        {
                                            self.cur_cmd = 13i32;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(4096i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > 13i32) || (self.cur_chr > 255i32)) {
                                {
                                    self.cur_cmd = 0i32;
                                    self.cur_chr = 256i32;
                                }
                            }
                            if (this_if == 0i32) {
                                b = (n == self.cur_chr);
                            } else {
                                b = (m == self.cur_cmd);
                            }
                        }
                    }
                    2 | 3 => {
                        // §503
                        {
                            if (this_if == 2i32) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            n = self.cur_val;
                            // §406
                            loop {
                                self.get_x_token();
                                if (self.cur_cmd != 10i32) { break; }
                            }
                            // §503
                            if ((self.cur_tok >= 3132i32) && (self.cur_tok <= 3134i32)) {
                                r = (self.cur_tok).wrapping_sub(3072i32);
                            } else {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(781i32);
                                    }
                                    self.print_cmd_chr(105i32, this_if);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 782i32;
                                    }
                                    self.back_error();
                                    r = 61i32;
                                }
                            }
                            if (this_if == 2i32) {
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
                    4 => {
                        // §504
                        {
                            self.scan_int();
                            b = (((self.cur_val) % 2) != 0);
                        }
                    }
                    5 => {
                        // §501
                        b = ((self.cur_list.mode_field).wrapping_abs() == 1i32);
                    }
                    6 => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == 102i32);
                    }
                    7 => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == 203i32);
                    }
                    8 => {
                        b = (self.cur_list.mode_field < 0i32);
                    }
                    9 | 10 | 11 => {
                        // §505
                        {
                            self.scan_eight_bit_int();
                            p = self.eqtb[(((3678i32).wrapping_add(self.cur_val)) - 1) as usize].hh().rh();
                            if (this_if == 9i32) {
                                b = (p == 0i32);
                            } else {
                                if (p == 0i32) {
                                    b = false;
                                } else {
                                    if (this_if == 10i32) {
                                        b = (self.mem[(p) as usize].hh().b0() == 0i32);
                                    } else {
                                        b = (self.mem[(p) as usize].hh().b0() == 1i32);
                                    }
                                }
                            }
                        }
                    }
                    12 => {
                        // §507
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = 0i32;
                            self.get_next();
                            n = self.cur_cs;
                            p = self.cur_cmd;
                            q = self.cur_chr;
                            self.get_next();
                            if (self.cur_cmd != p) {
                                b = false;
                            } else {
                                if (self.cur_cmd < 111i32) {
                                    b = (self.cur_chr == q);
                                } else {
                                    // §508
                                    {
                                        p = self.mem[(self.cur_chr) as usize].hh().rh();
                                        q = self.mem[(self.eqtb[((n) - 1) as usize].hh().rh()) as usize].hh().rh();
                                        if (p == q) {
                                            b = true;
                                        } else {
                                            {
                                                while ((p != 0i32) && (q != 0i32)) {
                                                    if (self.mem[(p) as usize].hh().lh() != self.mem[(q) as usize].hh().lh()) {
                                                        p = 0i32;
                                                    } else {
                                                        {
                                                            p = self.mem[(p) as usize].hh().rh();
                                                            q = self.mem[(q) as usize].hh().rh();
                                                        }
                                                    }
                                                }
                                                b = ((p == 0i32) && (q == 0i32));
                                            }
                                        }
                                    }
                                }
                            }
                            // §507
                            self.scanner_status = save_scanner_status;
                        }
                    }
                    13 => {
                        // §501
                        {
                            self.scan_four_bit_int();
                            b = (self.read_open[(self.cur_val) as usize] == 2i32);
                        }
                    }
                    14 => {
                        b = true;
                    }
                    15 => {
                        b = false;
                    }
                    16 => {
                        // §509
                        {
                            self.scan_int();
                            n = self.cur_val;
                            if (self.eqtb[((5299i32) - 1) as usize].int() > 1i32) {
                                {
                                    self.begin_diagnostic();
                                    self.print(783i32);
                                    self.print_int(n);
                                    self.print_char(125i32);
                                    self.end_diagnostic(false);
                                }
                            }
                            while (n != 0i32) {
                                {
                                    self.pass_text();
                                    if (self.cond_ptr == save_cond_ptr) {
                                        if (self.cur_chr == 4i32) {
                                            n = (n).wrapping_sub(1i32);
                                        } else {
                                            break 'l_common_ending_f;
                                        }
                                    } else {
                                        if (self.cur_chr == 2i32) {
                                            // §496
                                            {
                                                p = self.cond_ptr;
                                                self.if_line = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                                self.cur_if = self.mem[(p) as usize].hh().b1();
                                                self.if_limit = self.mem[(p) as usize].hh().b0();
                                                self.cond_ptr = self.mem[(p) as usize].hh().rh();
                                                self.free_node(p, 2i32);
                                            }
                                        }
                                    }
                                }
                            }
                            // §509
                            self.change_if_limit(4i32, save_cond_ptr);
                            break 'l_exit_f;
                        }
                    }
                    _ => {}
                }
                // §498
                if (self.eqtb[((5299i32) - 1) as usize].int() > 1i32) {
                    // §502
                    {
                        self.begin_diagnostic();
                        if b {
                            self.print(779i32);
                        } else {
                            self.print(780i32);
                        }
                        self.end_diagnostic(false);
                    }
                }
                // §498
                if b {
                    {
                        self.change_if_limit(3i32, save_cond_ptr);
                        break 'l_exit_f;
                    }
                }
                // §500
                while true {
                    {
                        self.pass_text();
                        if (self.cond_ptr == save_cond_ptr) {
                            {
                                if (self.cur_chr != 4i32) {
                                    break 'l_common_ending_f;
                                }
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(777i32);
                                }
                                self.print_esc(775i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[(0i32) as usize] = 778i32;
                                }
                                self.error();
                            }
                        } else {
                            if (self.cur_chr == 2i32) {
                                // §496
                                {
                                    p = self.cond_ptr;
                                    self.if_line = self.mem[((p).wrapping_add(1i32)) as usize].int();
                                    self.cur_if = self.mem[(p) as usize].hh().b1();
                                    self.if_limit = self.mem[(p) as usize].hh().b0();
                                    self.cond_ptr = self.mem[(p) as usize].hh().rh();
                                    self.free_node(p, 2i32);
                                }
                            }
                        }
                    }
                }
            }
            // §498
            if (self.cur_chr == 2i32) {
                // §496
                {
                    p = self.cond_ptr;
                    self.if_line = self.mem[((p).wrapping_add(1i32)) as usize].int();
                    self.cur_if = self.mem[(p) as usize].hh().b1();
                    self.if_limit = self.mem[(p) as usize].hh().b0();
                    self.cond_ptr = self.mem[(p) as usize].hh().rh();
                    self.free_node(p, 2i32);
                }
            } else {
                // §498
                self.if_limit = 2i32;
            }
        }
    }

    /// Here now is the first of the system-dependent routines for file name scanning.
    // §515
    pub fn begin_name(&mut self) {
        self.area_delimiter = 0i32;
        self.ext_delimiter = 0i32;
    }

    /// And here's the second. The string pool might change as the file name is
    /// being scanned, since a new \.{\\csname} might be entered; therefore we keep
    /// `area_delimiter` and `ext_delimiter` relative to the beginning of the current
    /// string, instead of assigning an absolute address like `pool_ptr` to them.
    // §516
    pub fn more_name(&mut self, mut c: ASCII_code) -> bool {
        let mut more_name: bool = false;
        if (c == 32i32) {
            more_name = false;
        } else {
            {
                {
                    if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                        self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                    }
                }
                {
                    self.str_pool[(self.pool_ptr) as usize] = c;
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                if ((c == 62i32) || (c == 58i32)) {
                    {
                        self.area_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]);
                        self.ext_delimiter = 0i32;
                    }
                } else {
                    if ((c == 46i32) && (self.ext_delimiter == 0i32)) {
                        self.ext_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]);
                    }
                }
                more_name = true;
            }
        }
        more_name
    }

    /// The third.
    // §517
    pub fn end_name(&mut self) {
        if ((self.str_ptr).wrapping_add(3i32) > max_strings) {
            self.overflow(258i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        if (self.area_delimiter == 0i32) {
            self.cur_area = 338i32;
        } else {
            {
                self.cur_area = self.str_ptr;
                { let __ix200 = (self.str_ptr).wrapping_add(1i32); let __v201 = (self.str_start[(self.str_ptr) as usize]).wrapping_add(self.area_delimiter); self.str_start[(__ix200) as usize] = __v201; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
            }
        }
        if (self.ext_delimiter == 0i32) {
            {
                self.cur_ext = 338i32;
                self.cur_name = self.make_string();
            }
        } else {
            {
                self.cur_name = self.str_ptr;
                { let __ix202 = (self.str_ptr).wrapping_add(1i32); let __v203 = (((self.str_start[(self.str_ptr) as usize]).wrapping_add(self.ext_delimiter)).wrapping_sub(self.area_delimiter)).wrapping_sub(1i32); self.str_start[(__ix202) as usize] = __v203; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                self.cur_ext = self.make_string();
            }
        }
    }

    /// Another system-dependent routine is needed to convert three internal
    /// \TeX\ strings
    /// into the `name_of_file` value that is used to open files. The present code
    /// allows both lowercase and uppercase letters in the file name.
    // §519
    pub fn pack_file_name(&mut self, mut n: str_number, mut a: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §519
        let mut c: ASCII_code = 0; // §519
        let mut j: pool_pointer = 0; // §519
        k = 0i32;
        {
            let __for_end_2 = (self.str_start[((a).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            j = self.str_start[(a) as usize];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[(j) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v204 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v204; }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[((n).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            j = self.str_start[(n) as usize];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[(j) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v205 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v205; }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[((e).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            j = self.str_start[(e) as usize];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[(j) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v206 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v206; }
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
                self.name_of_file[((k) - 1) as usize] = b' ';
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
    // §523
    pub fn pack_buffered_name(&mut self, mut n: small_number, mut a: i32, mut b: i32) {
        let mut k: i32 = 0; // §523
        let mut c: ASCII_code = 0; // §523
        let mut j: i32 = 0; // §523
        if ((((n).wrapping_add(b)).wrapping_sub(a)).wrapping_add(5i32) > file_name_size) {
            b = (((a).wrapping_add(file_name_size)).wrapping_sub(n)).wrapping_sub(5i32);
        }
        k = 0i32;
        {
            let __for_end_2 = n;
            j = 1i32;
            while j <= __for_end_2 {
                {
                    c = self.xord[(self.TEX_format_default[((j) - 1) as usize]) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v207 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v207; }
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
                    c = self.buffer[(j) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v208 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v208; }
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 20i32;
            j = 17i32;
            while j <= __for_end_2 {
                {
                    c = self.xord[(self.TEX_format_default[((j) - 1) as usize]) as usize];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        { let __v209 = self.xchr[(c) as usize]; self.name_of_file[((k) - 1) as usize] = __v209; }
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
                self.name_of_file[((k) - 1) as usize] = b' ';
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
    // §525
    pub fn make_name_string(&mut self) -> str_number {
        let mut make_name_string: str_number = 0;
        let mut k: i32 = 0; // §525
        if ((((self.pool_ptr).wrapping_add(self.name_length) > pool_size) || (self.str_ptr == max_strings)) || ((self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]) > 0i32)) {
            make_name_string = 63i32;
        } else {
            {
                {
                    let __for_end_4 = self.name_length;
                    k = 1i32;
                    while k <= __for_end_4 {
                        {
                            { let __ix210 = self.pool_ptr; let __v211 = self.xord[(self.name_of_file[((k) - 1) as usize]) as usize]; self.str_pool[(__ix210) as usize] = __v211; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                        k = k.wrapping_add(1);
                    }
                }
                make_name_string = self.make_string();
            }
        }
        make_name_string
    }

    /// Now let's consider the ``driver''
    /// routines by which \TeX\ deals with file names
    /// in a system-independent manner.  First comes a procedure that looks for a
    /// file name in the input by calling `get_x_token` for the information.
    // §526
    pub fn scan_file_name(&mut self) {
        'l_done_f: {
            self.name_in_progress = true;
            self.begin_name();
            // §406
            loop {
                self.get_x_token();
                if (self.cur_cmd != 10i32) { break; }
            }
            // §526
            while true {
                {
                    if ((self.cur_cmd > 12i32) || (self.cur_chr > 255i32)) {
                        {
                            self.back_input();
                            break 'l_done_f;
                        }
                    }
                    if (!self.more_name(self.cur_chr)) {
                        break 'l_done_f;
                    }
                    self.get_x_token();
                }
            }
        }
        self.end_name();
        self.name_in_progress = false;
    }

    /// Here is a routine that manufactures the output file names, assuming that
    /// `job_name<>0`. It ignores and changes the current settings of `cur_area`
    /// and `cur_ext`.
    // §529
    pub fn pack_job_name(&mut self, mut s: str_number) {
        self.cur_area = 338i32;
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
    // §530
    pub fn prompt_file_name(&mut self, mut s: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §530
        if (self.interaction == 2i32) {
        }
        if (s == 787i32) {
            {
                if (self.interaction == 3i32) {
                }
                self.print_nl(262i32);
                self.print(788i32);
            }
        } else {
            {
                if (self.interaction == 3i32) {
                }
                self.print_nl(262i32);
                self.print(789i32);
            }
        }
        self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
        self.print(790i32);
        if (e == 791i32) {
            self.show_context();
        }
        self.print_nl(792i32);
        self.print(s);
        if (self.interaction < 2i32) {
            self.fatal_error(793i32);
        }
        crate::system::break_in(&mut self.term_in, true);
        {
            self.print(568i32);
            self.term_input();
        }
        // §531
        {
            'l_done_f: {
                self.begin_name();
                k = self.first;
                while ((self.buffer[(k) as usize] == 32i32) && (k < self.last)) {
                    k = (k).wrapping_add(1i32);
                }
                while true {
                    {
                        if (k == self.last) {
                            break 'l_done_f;
                        }
                        if (!self.more_name(self.buffer[(k) as usize])) {
                            break 'l_done_f;
                        }
                        k = (k).wrapping_add(1i32);
                    }
                }
            }
            self.end_name();
        }
        // §530
        if (self.cur_ext == 338i32) {
            self.cur_ext = e;
        }
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// The `open_log_file` routine is used to open the transcript file and to help
    /// it catch up to what has previously been printed on the terminal.
    // §534
    pub fn open_log_file(&mut self) {
        let mut old_setting: i32 = 0; // §534
        let mut k: i32 = 0; // §534
        let mut l: i32 = 0; // §534
        let mut months: [u8; 36] = [0u8; 36]; // §534
        old_setting = self.selector;
        if (self.job_name == 0i32) {
            self.job_name = 796i32;
        }
        self.pack_job_name(797i32);
        while (!{ let mut __f = ::core::mem::take(&mut self.log_file); let __r = self.a_open_out(&mut __f); self.log_file = __f; __r }) {
            // §535
            {
                self.selector = 17i32;
                self.prompt_file_name(799i32, 797i32);
            }
        }
        // §534
        self.log_name = { let mut __f = ::core::mem::take(&mut self.log_file); let __r = self.a_make_name_string(&mut __f); self.log_file = __f; __r };
        self.selector = 18i32;
        self.log_opened = true;
        // §536
        {
            {
                crate::system::wr_str(&mut self.log_file, "This is TeX, Version 3.141592653");
            }
            self.slow_print(self.format_ident);
            self.print(800i32);
            self.print_int(self.sys_day);
            self.print_char(32i32);
            crate::system::copy_str(&mut months, "JANFEBMARAPRMAYJUNJULAUGSEPOCTNOVDEC");
            {
                let __for_end_3 = (3i32).wrapping_mul(self.sys_month);
                k = ((3i32).wrapping_mul(self.sys_month)).wrapping_sub(2i32);
                while k <= __for_end_3 {
                    {
                        let __w0 = months[((k) - 1) as usize];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_char(32i32);
            self.print_int(self.sys_year);
            self.print_char(32i32);
            self.print_two((self.sys_time / 60i32));
            self.print_char(58i32);
            self.print_two((self.sys_time % 60i32));
        }
        // §534
        { let __ix212 = self.input_ptr; let __v213 = self.cur_input; self.input_stack[(__ix212) as usize] = __v213; }
        self.print_nl(798i32);
        l = self.input_stack[(0i32) as usize].limit_field;
        if (self.buffer[(l) as usize] == self.eqtb[((5311i32) - 1) as usize].int()) {
            l = (l).wrapping_sub(1i32);
        }
        {
            let __for_end_2 = l;
            k = 1i32;
            while k <= __for_end_2 {
                self.print(self.buffer[(k) as usize]);
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
    // §537
    pub fn start_input(&mut self) {
        'l_done_f: {
            self.scan_file_name();
            if (self.cur_ext == 338i32) {
                self.cur_ext = 791i32;
            }
            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
            while true {
                {
                    self.begin_file_reading();
                    if { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.a_open_in(&mut __f); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r } {
                        break 'l_done_f;
                    }
                    if (self.cur_area == 338i32) {
                        {
                            self.pack_file_name(self.cur_name, 784i32, self.cur_ext);
                            if { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.a_open_in(&mut __f); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r } {
                                break 'l_done_f;
                            }
                        }
                    }
                    self.end_file_reading();
                    self.prompt_file_name(787i32, 791i32);
                }
            }
        }
        self.cur_input.name_field = { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.a_make_name_string(&mut __f); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r };
        if (self.job_name == 0i32) {
            {
                self.job_name = self.cur_name;
                self.open_log_file();
            }
        }
        if ((self.term_offset).wrapping_add((self.str_start[((self.cur_input.name_field).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.cur_input.name_field) as usize])) > (max_print_line).wrapping_sub(2i32)) {
            self.print_ln();
        } else {
            if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                self.print_char(32i32);
            }
        }
        self.print_char(40i32);
        self.open_parens = (self.open_parens).wrapping_add(1i32);
        self.slow_print(self.cur_input.name_field);
        crate::system::break_out(&mut self.term_out);
        self.cur_input.state_field = 33i32;
        if (self.cur_input.name_field == (self.str_ptr).wrapping_sub(1i32)) {
            {
                {
                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                }
                self.cur_input.name_field = self.cur_name;
            }
        }
        // §538
        {
            self.line = 1i32;
            if { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.input_ln(&mut __f, false); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r } {
            }
            self.firm_up_the_line();
            if ((self.eqtb[((5311i32) - 1) as usize].int() < 0i32) || (self.eqtb[((5311i32) - 1) as usize].int() > 255i32)) {
                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
            } else {
                { let __ix214 = self.cur_input.limit_field; let __v215 = self.eqtb[((5311i32) - 1) as usize].int(); self.buffer[(__ix214) as usize] = __v215; }
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
    // §560
    pub fn read_font_info(&mut self, mut u: halfword, mut nom: str_number, mut aire: str_number, mut s: scaled) -> internal_font_number {
        let mut read_font_info: internal_font_number = 0;
        let mut k: font_index = 0; // §560
        let mut file_opened: bool = false; // §560
        let mut lf: halfword = 0; // §560
        let mut lh: halfword = 0; // §560
        let mut bc: halfword = 0; // §560
        let mut ec: halfword = 0; // §560
        let mut nw: halfword = 0; // §560
        let mut nh: halfword = 0; // §560
        let mut nd: halfword = 0; // §560
        let mut ni: halfword = 0; // §560
        let mut nl: halfword = 0; // §560
        let mut nk: halfword = 0; // §560
        let mut ne: halfword = 0; // §560
        let mut np: halfword = 0; // §560
        let mut f: internal_font_number = 0; // §560
        let mut g: internal_font_number = 0; // §560
        let mut a: eight_bits = 0; // §560
        let mut b: eight_bits = 0; // §560
        let mut c: eight_bits = 0; // §560
        let mut d: eight_bits = 0; // §560
        let mut qw: four_quarters = four_quarters::default(); // §560
        let mut sw: scaled = 0; // §560
        let mut bch_label: i32 = 0; // §560
        let mut bchar: i32 = 0; // §560
        let mut z: scaled = 0; // §560
        let mut alpha: i32 = 0; // §560
        let mut beta: i32 = 0; // §560
        'l_done_f: {
            'l_L11_f: {
                g = 0i32;
                // §563
                file_opened = false;
                if (aire == 338i32) {
                    self.pack_file_name(nom, 785i32, 811i32);
                } else {
                    self.pack_file_name(nom, aire, 811i32);
                }
                if (!{ let mut __f = ::core::mem::take(&mut self.tfm_file); let __r = self.b_open_in(&mut __f); self.tfm_file = __f; __r }) {
                    break 'l_L11_f;
                }
                file_opened = true;
                // §565
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
                // §566
                lf = ((lf).wrapping_sub(6i32)).wrapping_sub(lh);
                if (np < 7i32) {
                    lf = ((lf).wrapping_add(7i32)).wrapping_sub(np);
                }
                if ((self.font_ptr == font_max) || ((self.fmem_ptr).wrapping_add(lf) > font_mem_size)) {
                    // §567
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(802i32);
                        }
                        self.sprint_cs(u);
                        self.print_char(61i32);
                        self.print_file_name(nom, aire, 338i32);
                        if (s >= 0i32) {
                            {
                                self.print(741i32);
                                self.print_scaled(s);
                                self.print(397i32);
                            }
                        } else {
                            if (s != (1000i32).wrapping_neg()) {
                                {
                                    self.print(803i32);
                                    self.print_int((s).wrapping_neg());
                                }
                            }
                        }
                        self.print(812i32);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[(3i32) as usize] = 813i32;
                            self.help_line[(2i32) as usize] = 814i32;
                            self.help_line[(1i32) as usize] = 815i32;
                            self.help_line[(0i32) as usize] = 816i32;
                        }
                        self.error();
                        break 'l_done_f;
                    }
                }
                // §566
                f = (self.font_ptr).wrapping_add(1i32);
                { let __v216 = (self.fmem_ptr).wrapping_sub(bc); self.char_base[(f) as usize] = __v216; }
                { let __v217 = ((self.char_base[(f) as usize]).wrapping_add(ec)).wrapping_add(1i32); self.width_base[(f) as usize] = __v217; }
                { let __v218 = (self.width_base[(f) as usize]).wrapping_add(nw); self.height_base[(f) as usize] = __v218; }
                { let __v219 = (self.height_base[(f) as usize]).wrapping_add(nh); self.depth_base[(f) as usize] = __v219; }
                { let __v220 = (self.depth_base[(f) as usize]).wrapping_add(nd); self.italic_base[(f) as usize] = __v220; }
                { let __v221 = (self.italic_base[(f) as usize]).wrapping_add(ni); self.lig_kern_base[(f) as usize] = __v221; }
                { let __v222 = ((self.lig_kern_base[(f) as usize]).wrapping_add(nl)).wrapping_sub((256i32).wrapping_mul(128i32)); self.kern_base[(f) as usize] = __v222; }
                { let __v223 = ((self.kern_base[(f) as usize]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_add(nk); self.exten_base[(f) as usize] = __v223; }
                { let __v224 = (self.exten_base[(f) as usize]).wrapping_add(ne); self.param_base[(f) as usize] = __v224; }
                // §568
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
                        self.font_check[(f) as usize] = qw;
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
                    if (z < 65536i32) {
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
                    self.font_dsize[(f) as usize] = z;
                    if (s != (1000i32).wrapping_neg()) {
                        if (s >= 0i32) {
                            z = s;
                        } else {
                            z = self.xn_over_d(z, (s).wrapping_neg(), 1000i32);
                        }
                    }
                    self.font_size[(f) as usize] = z;
                }
                // §569
                {
                    let __for_end_4 = (self.width_base[(f) as usize]).wrapping_sub(1i32);
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
                                self.font_info[(k) as usize].set_qqqq(qw);
                            }
                            if ((((a >= nw) || ((b / 16i32) >= nh)) || ((b % 16i32) >= nd)) || ((c / 4i32) >= ni)) {
                                break 'l_L11_f;
                            }
                            match (c % 4i32) {
                                1 => {
                                    if (d >= nl) {
                                        break 'l_L11_f;
                                    }
                                }
                                3 => {
                                    if (d >= ne) {
                                        break 'l_L11_f;
                                    }
                                }
                                2 => {
                                    // §570
                                    {
                                        'l_not_found_f: {
                                            {
                                                if ((d < bc) || (d > ec)) {
                                                    break 'l_L11_f;
                                                }
                                            }
                                            while (d < ((k).wrapping_add(bc)).wrapping_sub(self.fmem_ptr)) {
                                                {
                                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(d)) as usize].qqqq();
                                                    if (((qw.b2()).wrapping_sub(0i32) % 4i32) != 2i32) {
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
                                    // §569
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §571
                {
                    // §572
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
                    // §571
                    {
                        let __for_end_5 = (self.lig_kern_base[(f) as usize]).wrapping_sub(1i32);
                        k = self.width_base[(f) as usize];
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
                                    self.font_info[(k) as usize].set_int(sw);
                                } else {
                                    if (a == 255i32) {
                                        self.font_info[(k) as usize].set_int((sw).wrapping_sub(alpha));
                                    } else {
                                        break 'l_L11_f;
                                    }
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    if (self.font_info[(self.width_base[(f) as usize]) as usize].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[(self.height_base[(f) as usize]) as usize].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[(self.depth_base[(f) as usize]) as usize].int() != 0i32) {
                        break 'l_L11_f;
                    }
                    if (self.font_info[(self.italic_base[(f) as usize]) as usize].int() != 0i32) {
                        break 'l_L11_f;
                    }
                }
                // §573
                bch_label = 32767i32;
                bchar = 256i32;
                if (nl > 0i32) {
                    {
                        {
                            let __for_end_6 = ((self.kern_base[(f) as usize]).wrapping_add((256i32).wrapping_mul(128i32))).wrapping_sub(1i32);
                            k = self.lig_kern_base[(f) as usize];
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
                                        self.font_info[(k) as usize].set_qqqq(qw);
                                    }
                                    if (a > 128i32) {
                                        {
                                            if (((256i32).wrapping_mul(c)).wrapping_add(d) >= nl) {
                                                break 'l_L11_f;
                                            }
                                            if (a == 255i32) {
                                                if (k == self.lig_kern_base[(f) as usize]) {
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
                                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(b)) as usize].qqqq();
                                                    if (!(qw.b0() > 0i32)) {
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
                                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(d)) as usize].qqqq();
                                                    if (!(qw.b0() > 0i32)) {
                                                        break 'l_L11_f;
                                                    }
                                                }
                                            } else {
                                                if (((256i32).wrapping_mul((c).wrapping_sub(128i32))).wrapping_add(d) >= nk) {
                                                    break 'l_L11_f;
                                                }
                                            }
                                            if (a < 128i32) {
                                                if ((((k).wrapping_sub(self.lig_kern_base[(f) as usize])).wrapping_add(a)).wrapping_add(1i32) >= nl) {
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
                    let __for_end_4 = (self.exten_base[(f) as usize]).wrapping_sub(1i32);
                    k = (self.kern_base[(f) as usize]).wrapping_add((256i32).wrapping_mul(128i32));
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
                                self.font_info[(k) as usize].set_int(sw);
                            } else {
                                if (a == 255i32) {
                                    self.font_info[(k) as usize].set_int((sw).wrapping_sub(alpha));
                                } else {
                                    break 'l_L11_f;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §574
                {
                    let __for_end_4 = (self.param_base[(f) as usize]).wrapping_sub(1i32);
                    k = self.exten_base[(f) as usize];
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
                                self.font_info[(k) as usize].set_qqqq(qw);
                            }
                            if (a != 0i32) {
                                {
                                    {
                                        if ((a < bc) || (a > ec)) {
                                            break 'l_L11_f;
                                        }
                                    }
                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(a)) as usize].qqqq();
                                    if (!(qw.b0() > 0i32)) {
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
                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(b)) as usize].qqqq();
                                    if (!(qw.b0() > 0i32)) {
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
                                    qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(c)) as usize].qqqq();
                                    if (!(qw.b0() > 0i32)) {
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
                                qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(d)) as usize].qqqq();
                                if (!(qw.b0() > 0i32)) {
                                    break 'l_L11_f;
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                // §575
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
                                    { let __ix225 = self.param_base[(f) as usize]; let __v226 = ((sw).wrapping_mul(16i32)).wrapping_add((self.tfm_file.buf / 16i32)); self.font_info[(__ix225) as usize].set_int(__v226); }
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
                                        { let __ix227 = ((self.param_base[(f) as usize]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[(__ix227) as usize].set_int(sw); }
                                    } else {
                                        if (a == 255i32) {
                                            { let __ix228 = ((self.param_base[(f) as usize]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[(__ix228) as usize].set_int((sw).wrapping_sub(alpha)); }
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
                            { let __ix229 = ((self.param_base[(f) as usize]).wrapping_add(k)).wrapping_sub(1i32); self.font_info[(__ix229) as usize].set_int(0i32); }
                            k = k.wrapping_add(1);
                        }
                    }
                }
                // §576
                if (np >= 7i32) {
                    self.font_params[(f) as usize] = np;
                } else {
                    self.font_params[(f) as usize] = 7i32;
                }
                { let __v230 = self.eqtb[((5309i32) - 1) as usize].int(); self.hyphen_char[(f) as usize] = __v230; }
                { let __v231 = self.eqtb[((5310i32) - 1) as usize].int(); self.skew_char[(f) as usize] = __v231; }
                if (bch_label < nl) {
                    { let __v232 = (bch_label).wrapping_add(self.lig_kern_base[(f) as usize]); self.bchar_label[(f) as usize] = __v232; }
                } else {
                    self.bchar_label[(f) as usize] = 0i32;
                }
                self.font_bchar[(f) as usize] = (bchar).wrapping_add(0i32);
                self.font_false_bchar[(f) as usize] = (bchar).wrapping_add(0i32);
                if (bchar <= ec) {
                    if (bchar >= bc) {
                        {
                            qw = self.font_info[((self.char_base[(f) as usize]).wrapping_add(bchar)) as usize].qqqq();
                            if (qw.b0() > 0i32) {
                                self.font_false_bchar[(f) as usize] = 256i32;
                            }
                        }
                    }
                }
                self.font_name[(f) as usize] = nom;
                self.font_area[(f) as usize] = aire;
                self.font_bc[(f) as usize] = bc;
                self.font_ec[(f) as usize] = ec;
                self.font_glue[(f) as usize] = 0i32;
                { let __v233 = (self.char_base[(f) as usize]).wrapping_sub(0i32); self.char_base[(f) as usize] = __v233; }
                { let __v234 = (self.width_base[(f) as usize]).wrapping_sub(0i32); self.width_base[(f) as usize] = __v234; }
                { let __v235 = (self.lig_kern_base[(f) as usize]).wrapping_sub(0i32); self.lig_kern_base[(f) as usize] = __v235; }
                { let __v236 = (self.kern_base[(f) as usize]).wrapping_sub(0i32); self.kern_base[(f) as usize] = __v236; }
                { let __v237 = (self.exten_base[(f) as usize]).wrapping_sub(0i32); self.exten_base[(f) as usize] = __v237; }
                { let __v238 = (self.param_base[(f) as usize]).wrapping_sub(1i32); self.param_base[(f) as usize] = __v238; }
                self.fmem_ptr = (self.fmem_ptr).wrapping_add(lf);
                self.font_ptr = f;
                g = f;
                break 'l_done_f;
            }
            // §560
            {
                // §561
                if (self.interaction == 3i32) {
                }
                self.print_nl(262i32);
                self.print(802i32);
            }
            self.sprint_cs(u);
            self.print_char(61i32);
            self.print_file_name(nom, aire, 338i32);
            if (s >= 0i32) {
                {
                    self.print(741i32);
                    self.print_scaled(s);
                    self.print(397i32);
                }
            } else {
                if (s != (1000i32).wrapping_neg()) {
                    {
                        self.print(803i32);
                        self.print_int((s).wrapping_neg());
                    }
                }
            }
            if file_opened {
                self.print(804i32);
            } else {
                self.print(805i32);
            }
            {
                self.help_ptr = 5i32;
                self.help_line[(4i32) as usize] = 806i32;
                self.help_line[(3i32) as usize] = 807i32;
                self.help_line[(2i32) as usize] = 808i32;
                self.help_line[(1i32) as usize] = 809i32;
                self.help_line[(0i32) as usize] = 810i32;
            }
            self.error();
        }
        // §560
        if file_opened {
            { let mut __f = ::core::mem::take(&mut self.tfm_file); let __r = self.b_close(&mut __f); self.tfm_file = __f; __r };
        }
        read_font_info = g;
        read_font_info
    }

    /// When \TeX\ wants to typeset a character that doesn't exist, the
    /// character node is not created; thus the output routine can assume
    /// that characters exist when it sees them. The following procedure
    /// prints a warning message unless the user has suppressed it.
    // §581
    pub fn char_warning(&mut self, mut f: internal_font_number, mut c: eight_bits) {
        if (self.eqtb[((5298i32) - 1) as usize].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_nl(825i32);
                self.print(c);
                self.print(826i32);
                self.slow_print(self.font_name[(f) as usize]);
                self.print_char(33i32);
                self.end_diagnostic(false);
            }
        }
    }

    /// Here is a function that returns a pointer to a character node for a
    /// given character in a given font. If that character doesn't exist,
    /// `null` is returned instead.
    // §582
    pub fn new_character(&mut self, mut f: internal_font_number, mut c: eight_bits) -> halfword {
        let mut new_character: halfword = 0;
        let mut p: halfword = 0; // §582
        'l_exit_f: {
            if (self.font_bc[(f) as usize] <= c) {
                if (self.font_ec[(f) as usize] >= c) {
                    if (self.font_info[(((self.char_base[(f) as usize]).wrapping_add(c)).wrapping_add(0i32)) as usize].qqqq().b0() > 0i32) {
                        {
                            p = self.get_avail();
                            self.mem[(p) as usize].set_hh_b0(f);
                            self.mem[(p) as usize].set_hh_b1((c).wrapping_add(0i32));
                            new_character = p;
                            break 'l_exit_f;
                        }
                    }
                }
            }
            self.char_warning(f, c);
            new_character = 0i32;
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
    // §597
    pub fn write_dvi(&mut self, mut a: dvi_index, mut b: dvi_index) {
        let mut k: dvi_index = 0; // §597
        {
            let __for_end_2 = b;
            k = a;
            while k <= __for_end_2 {
                {
                    let __w = self.dvi_buf[(k) as usize];
                    crate::system::write_byte(&mut self.dvi_file, __w);
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// To put a byte in the buffer without paying the cost of invoking a procedure
    /// each time, we use the macro `dvi_out`.
    // §598
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

}
