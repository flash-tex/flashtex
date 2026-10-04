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
    /// @<Declare the procedure called `print_cmd_chr`
    // §1457
    pub fn not_native_font_error(&mut self, mut cmd: i32, mut c: i32, mut f: i32) {
        {
            if (self.interaction == error_stop_mode) {}
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(66840i32);
        }
        self.print_cmd_chr(cmd, c);
        self.print(66841i32);
        self.print(self.font_name[crate::ix::U((f) as usize)]);
        self.print(66845i32);
        self.error();
    }

    /// Here is a procedure that displays the contents of `eqtb[n]`
    /// symbolically.
    // §278
    pub fn show_eqtb(&mut self, mut n: halfword) {
        if (n < active_base) {
            self.print_char(63i32);
        } else {
            if ((n < glue_base) || ((n > eqtb_size) && (n <= eqtb_top))) {
                // §249
                {
                    self.sprint_cs(n);
                    self.print_char(61i32);
                    self.print_cmd_chr(
                        self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().b0(),
                        self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(),
                    );
                    if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().b0() >= call) {
                        {
                            self.print_char(58i32);
                            self.show_token_list(
                                self.mem[crate::ix::U(
                                    (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh())
                                        as usize,
                                )]
                                .hh()
                                .rh(),
                                (268435455i32).wrapping_neg(),
                                32i32,
                            );
                        }
                    }
                }
            } else {
                // §278
                if (n < local_base) {
                    // §255
                    if (n < skip_base) {
                        {
                            self.print_skip_param((n).wrapping_sub(1205764i32));
                            self.print_char(61i32);
                            if (n < 1205780i32) {
                                self.print_spec(
                                    self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(),
                                    65689i32,
                                );
                            } else {
                                self.print_spec(
                                    self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(),
                                    65625i32,
                                );
                            }
                        }
                    } else {
                        if (n < mu_skip_base) {
                            {
                                self.print_esc(65687i32);
                                self.print_int((n).wrapping_sub(1205783i32));
                                self.print_char(61i32);
                                self.print_spec(
                                    self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(),
                                    65689i32,
                                );
                            }
                        } else {
                            {
                                self.print_esc(65688i32);
                                self.print_int((n).wrapping_sub(1206039i32));
                                self.print_char(61i32);
                                self.print_spec(
                                    self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh(),
                                    65625i32,
                                );
                            }
                        }
                    }
                } else {
                    // §278
                    if (n < int_base) {
                        // §259
                        if ((n == par_shape_loc) || ((n >= etex_pen_base) && (n < etex_pens))) {
                            {
                                self.print_cmd_chr(set_shape, n);
                                self.print_char(61i32);
                                if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()
                                    == (268435455i32).wrapping_neg())
                                {
                                    self.print_char(48i32);
                                } else {
                                    if (n > par_shape_loc) {
                                        {
                                            self.print_int(
                                                self.mem[crate::ix::U(
                                                    ((self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                        .hh()
                                                        .rh())
                                                    .wrapping_add(1i32))
                                                        as usize,
                                                )]
                                                .int(),
                                            );
                                            self.print_char(32i32);
                                            self.print_int(
                                                self.mem[crate::ix::U(
                                                    ((self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                        .hh()
                                                        .rh())
                                                    .wrapping_add(2i32))
                                                        as usize,
                                                )]
                                                .int(),
                                            );
                                            if (self.mem[crate::ix::U(
                                                ((self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                    .hh()
                                                    .rh())
                                                .wrapping_add(1i32))
                                                    as usize,
                                            )]
                                            .int()
                                                > 1i32)
                                            {
                                                self.print_esc(65700i32);
                                            }
                                        }
                                    } else {
                                        self.print_int(
                                            self.mem[crate::ix::U(
                                                (self.eqtb
                                                    [crate::ix::U(((par_shape_loc) - 1) as usize)]
                                                .hh()
                                                .rh())
                                                    as usize,
                                            )]
                                            .hh()
                                            .lh(),
                                        );
                                    }
                                }
                            }
                        } else {
                            if (n < toks_base) {
                                {
                                    self.print_cmd_chr(assign_toks, n);
                                    self.print_char(61i32);
                                    if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()
                                        != (268435455i32).wrapping_neg())
                                    {
                                        self.show_token_list(
                                            self.mem[crate::ix::U(
                                                (self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                    .hh()
                                                    .rh())
                                                    as usize,
                                            )]
                                            .hh()
                                            .rh(),
                                            (268435455i32).wrapping_neg(),
                                            32i32,
                                        );
                                    }
                                }
                            } else {
                                if (n < box_base) {
                                    {
                                        self.print_esc(65699i32);
                                        self.print_int((n).wrapping_sub(1206307i32));
                                        self.print_char(61i32);
                                        if (self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()
                                            != (268435455i32).wrapping_neg())
                                        {
                                            self.show_token_list(
                                                self.mem[crate::ix::U(
                                                    (self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                        .hh()
                                                        .rh())
                                                        as usize,
                                                )]
                                                .hh()
                                                .rh(),
                                                (268435455i32).wrapping_neg(),
                                                32i32,
                                            );
                                        }
                                    }
                                } else {
                                    if (n < cur_font_loc) {
                                        {
                                            self.print_esc(65701i32);
                                            self.print_int((n).wrapping_sub(1206567i32));
                                            self.print_char(61i32);
                                            if (self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                .hh()
                                                .rh()
                                                == (268435455i32).wrapping_neg())
                                            {
                                                self.print(65702i32);
                                            } else {
                                                {
                                                    self.depth_threshold = 0i32;
                                                    self.breadth_max = 1i32;
                                                    self.show_node_list(
                                                        self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                            .hh()
                                                            .rh(),
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        if (n < cat_code_base) {
                                            // §260
                                            {
                                                if (n == cur_font_loc) {
                                                    self.print(65703i32);
                                                } else {
                                                    if (n < 1207080i32) {
                                                        {
                                                            self.print_esc(65704i32);
                                                            self.print_int(
                                                                (n).wrapping_sub(1206824i32),
                                                            );
                                                        }
                                                    } else {
                                                        if (n < 1207336i32) {
                                                            {
                                                                self.print_esc(65705i32);
                                                                self.print_int(
                                                                    (n).wrapping_sub(1207080i32),
                                                                );
                                                            }
                                                        } else {
                                                            {
                                                                self.print_esc(65706i32);
                                                                self.print_int(
                                                                    (n).wrapping_sub(1207336i32),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                self.print_char(61i32);
                                                self.print_esc(
                                                    self.hash[crate::ix::U(
                                                        (((font_id_base).wrapping_add(
                                                            self.eqtb
                                                                [crate::ix::U(((n) - 1) as usize)]
                                                            .hh()
                                                            .rh(),
                                                        )) - 1179650)
                                                            as usize,
                                                    )]
                                                    .rh(),
                                                );
                                            }
                                        } else {
                                            // §261
                                            if (n < math_code_base) {
                                                {
                                                    if (n < lc_code_base) {
                                                        {
                                                            self.print_esc(65707i32);
                                                            self.print_int(
                                                                (n).wrapping_sub(1207592i32),
                                                            );
                                                        }
                                                    } else {
                                                        if (n < uc_code_base) {
                                                            {
                                                                self.print_esc(65708i32);
                                                                self.print_int(
                                                                    (n).wrapping_sub(2321704i32),
                                                                );
                                                            }
                                                        } else {
                                                            if (n < sf_code_base) {
                                                                {
                                                                    self.print_esc(65709i32);
                                                                    self.print_int(
                                                                        (n).wrapping_sub(
                                                                            3435816i32,
                                                                        ),
                                                                    );
                                                                }
                                                            } else {
                                                                {
                                                                    self.print_esc(65710i32);
                                                                    self.print_int(
                                                                        (n).wrapping_sub(
                                                                            4549928i32,
                                                                        ),
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.print_char(61i32);
                                                    self.print_int(
                                                        self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                            .hh()
                                                            .rh(),
                                                    );
                                                }
                                            } else {
                                                {
                                                    self.print_esc(65711i32);
                                                    self.print_int((n).wrapping_sub(5664040i32));
                                                    self.print_char(61i32);
                                                    self.print_int(
                                                        self.eqtb[crate::ix::U(((n) - 1) as usize)]
                                                            .hh()
                                                            .rh(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // §278
                        if (n < dimen_base) {
                            // §268
                            {
                                if (n < count_base) {
                                    self.print_param((n).wrapping_sub(7892264i32));
                                } else {
                                    if (n < del_code_base) {
                                        {
                                            self.print_esc(65777i32);
                                            self.print_int((n).wrapping_sub(7892352i32));
                                        }
                                    } else {
                                        {
                                            self.print_esc(65778i32);
                                            self.print_int((n).wrapping_sub(7892608i32));
                                        }
                                    }
                                }
                                self.print_char(61i32);
                                self.print_int(self.eqtb[crate::ix::U(((n) - 1) as usize)].int());
                            }
                        } else {
                            // §278
                            if (n <= eqtb_size) {
                                // §277
                                {
                                    if (n < scaled_base) {
                                        self.print_length_param((n).wrapping_sub(9006720i32));
                                    } else {
                                        {
                                            self.print_esc(65803i32);
                                            self.print_int((n).wrapping_sub(9006743i32));
                                        }
                                    }
                                    self.print_char(61i32);
                                    self.print_scaled(
                                        self.eqtb[crate::ix::U(((n) - 1) as usize)].int(),
                                    );
                                    self.print(65689i32);
                                }
                            } else {
                                // §278
                                self.print_char(63i32);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Here is the subroutine that searches the hash table for an identifier
    /// that matches a given string of length `l>0` appearing in `buffer[j..
    /// (j+l-1)]`. If the identifier is found, the corresponding hash table address
    /// is returned. Otherwise, if the global variable `no_new_control_sequence`
    /// is `true`, the dummy address `undefined_control_sequence` is returned.
    /// Otherwise the identifier is inserted into the hash table and its location
    /// is returned.
    // §286
    pub fn id_lookup(&mut self, mut j: i32, mut l: i32) -> halfword {
        let mut id_lookup: halfword = 0;
        let mut h: i32 = 0; // §286
        let mut d: i32 = 0; // §286
        let mut p: halfword = 0; // §286
        let mut k: halfword = 0; // §286
        let mut ll: i32 = 0; // §286
        'l_found_f: {
            // §288
            h = 0i32;
            {
                let __for_end_3 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                k = j;
                while k <= __for_end_3 {
                    {
                        h = ((h).wrapping_add(h))
                            .wrapping_add(self.buffer[crate::ix::U((k) as usize)]);
                        while (h >= hash_prime) {
                            h = (h).wrapping_sub(8501i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            // §286
            p = (h).wrapping_add(1179650i32);
            ll = l;
            {
                let __for_end_3 = (l).wrapping_sub(1i32);
                d = 0i32;
                while d <= __for_end_3 {
                    if (self.buffer[crate::ix::U(((j).wrapping_add(d)) as usize)] >= 65536i32) {
                        ll = (ll).wrapping_add(1i32);
                    }
                    d = d.wrapping_add(1);
                }
            }
            while true {
                {
                    if (self.hash[crate::ix::U(((p) - 1179650) as usize)].rh() > 0i32) {
                        if (self.length(self.hash[crate::ix::U(((p) - 1179650) as usize)].rh())
                            == ll)
                        {
                            if self.str_eq_buf(
                                self.hash[crate::ix::U(((p) - 1179650) as usize)].rh(),
                                j,
                            ) {
                                break 'l_found_f;
                            }
                        }
                    }
                    if (self.hash[crate::ix::U(((p) - 1179650) as usize)].lh() == 0i32) {
                        {
                            if self.no_new_control_sequence {
                                p = undefined_control_sequence;
                            } else {
                                // §287
                                {
                                    if (self.hash[crate::ix::U(((p) - 1179650) as usize)].rh()
                                        > 0i32)
                                    {
                                        {
                                            if (self.hash_high < hash_extra) {
                                                {
                                                    self.hash_high =
                                                        (self.hash_high).wrapping_add(1i32);
                                                    {
                                                        let __v110 = (self.hash_high)
                                                            .wrapping_add(9006998i32);
                                                        self.hash[crate::ix::U(
                                                            ((p) - 1179650) as usize,
                                                        )]
                                                        .set_lh(__v110);
                                                    }
                                                    p = (self.hash_high).wrapping_add(9006998i32);
                                                }
                                            } else {
                                                {
                                                    loop {
                                                        if (self.hash_used == hash_base) {
                                                            self.overflow(65807i32, 615000i32);
                                                        }
                                                        self.hash_used =
                                                            (self.hash_used).wrapping_sub(1i32);
                                                        if (self.hash[crate::ix::U(
                                                            ((self.hash_used) - 1179650) as usize,
                                                        )]
                                                        .rh()
                                                            == 0i32)
                                                        {
                                                            break;
                                                        }
                                                    }
                                                    {
                                                        let __v111 = self.hash_used;
                                                        self.hash[crate::ix::U(
                                                            ((p) - 1179650) as usize,
                                                        )]
                                                        .set_lh(__v111);
                                                    }
                                                    p = self.hash_used;
                                                }
                                            }
                                        }
                                    }
                                    {
                                        if ((self.pool_ptr).wrapping_add(ll) > pool_size) {
                                            self.overflow(
                                                65539i32,
                                                (pool_size).wrapping_sub(self.init_pool_ptr),
                                            );
                                        }
                                    }
                                    d = (self.pool_ptr).wrapping_sub(
                                        self.str_start[crate::ix::U(
                                            ((self.str_ptr).wrapping_sub(65536i32)) as usize,
                                        )],
                                    );
                                    while (self.pool_ptr
                                        > self.str_start[crate::ix::U(
                                            ((self.str_ptr).wrapping_sub(65536i32)) as usize,
                                        )])
                                    {
                                        {
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                            {
                                                let __ix112 = (self.pool_ptr).wrapping_add(l);
                                                let __v113 = self.str_pool
                                                    [crate::ix::U((self.pool_ptr) as usize)];
                                                self.str_pool[crate::ix::U((__ix112) as usize)] =
                                                    __v113;
                                            }
                                        }
                                    }
                                    {
                                        let __for_end_9 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                                        k = j;
                                        while k <= __for_end_9 {
                                            {
                                                if (self.buffer[crate::ix::U((k) as usize)]
                                                    < 65536i32)
                                                {
                                                    {
                                                        if (self.buffer[crate::ix::U((k) as usize)]
                                                            > 65535i32)
                                                        {
                                                            {
                                                                {
                                                                    let __ix114 = self.pool_ptr;
                                                                    let __v115 = ((self.buffer
                                                                        [crate::ix::U(
                                                                            (k) as usize,
                                                                        )])
                                                                    .wrapping_sub(65536i32)
                                                                        / 1024i32)
                                                                        .wrapping_add(55296i32);
                                                                    self.str_pool[crate::ix::U(
                                                                        (__ix114) as usize,
                                                                    )] = __v115;
                                                                }
                                                                self.pool_ptr = (self.pool_ptr)
                                                                    .wrapping_add(1i32);
                                                                {
                                                                    let __ix116 = self.pool_ptr;
                                                                    let __v117 =
                                                                        (self.buffer[crate::ix::U(
                                                                            (k) as usize,
                                                                        )] % 1024i32)
                                                                            .wrapping_add(56320i32);
                                                                    self.str_pool[crate::ix::U(
                                                                        (__ix116) as usize,
                                                                    )] = __v117;
                                                                }
                                                                self.pool_ptr = (self.pool_ptr)
                                                                    .wrapping_add(1i32);
                                                            }
                                                        } else {
                                                            {
                                                                {
                                                                    let __ix118 = self.pool_ptr;
                                                                    let __v119 = self.buffer
                                                                        [crate::ix::U(
                                                                            (k) as usize,
                                                                        )];
                                                                    self.str_pool[crate::ix::U(
                                                                        (__ix118) as usize,
                                                                    )] = __v119;
                                                                }
                                                                self.pool_ptr = (self.pool_ptr)
                                                                    .wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        {
                                                            if ((55296i32).wrapping_add(
                                                                ((self.buffer
                                                                    [crate::ix::U((k) as usize)])
                                                                .wrapping_sub(65536i32)
                                                                    / 1024i32),
                                                            ) > 65535i32)
                                                            {
                                                                {
                                                                    {
                                                                        let __ix120 = self.pool_ptr;
                                                                        let __v121 = (((55296i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                / 1024i32),
                                                                        ))
                                                                        .wrapping_sub(65536i32)
                                                                            / 1024i32)
                                                                            .wrapping_add(55296i32);
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix120) as usize,
                                                                            )] = __v121;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                    {
                                                                        let __ix122 = self.pool_ptr;
                                                                        let __v123 = ((55296i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                / 1024i32),
                                                                        ) % 1024i32)
                                                                            .wrapping_add(56320i32);
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix122) as usize,
                                                                            )] = __v123;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                }
                                                            } else {
                                                                {
                                                                    {
                                                                        let __ix124 = self.pool_ptr;
                                                                        let __v125 = (55296i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                / 1024i32),
                                                                        );
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix124) as usize,
                                                                            )] = __v125;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                        {
                                                            if ((56320i32).wrapping_add(
                                                                ((self.buffer
                                                                    [crate::ix::U((k) as usize)])
                                                                .wrapping_sub(65536i32)
                                                                    % 1024i32),
                                                            ) > 65535i32)
                                                            {
                                                                {
                                                                    {
                                                                        let __ix126 = self.pool_ptr;
                                                                        let __v127 = (((56320i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                % 1024i32),
                                                                        ))
                                                                        .wrapping_sub(65536i32)
                                                                            / 1024i32)
                                                                            .wrapping_add(55296i32);
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix126) as usize,
                                                                            )] = __v127;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                    {
                                                                        let __ix128 = self.pool_ptr;
                                                                        let __v129 = ((56320i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                % 1024i32),
                                                                        ) % 1024i32)
                                                                            .wrapping_add(56320i32);
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix128) as usize,
                                                                            )] = __v129;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                }
                                                            } else {
                                                                {
                                                                    {
                                                                        let __ix130 = self.pool_ptr;
                                                                        let __v131 = (56320i32)
                                                                            .wrapping_add(
                                                                            ((self.buffer
                                                                                [crate::ix::U(
                                                                                    (k) as usize,
                                                                                )])
                                                                            .wrapping_sub(65536i32)
                                                                                % 1024i32),
                                                                        );
                                                                        self.str_pool
                                                                            [crate::ix::U(
                                                                                (__ix130) as usize,
                                                                            )] = __v131;
                                                                    }
                                                                    self.pool_ptr = (self.pool_ptr)
                                                                        .wrapping_add(1i32);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            k = k.wrapping_add(1);
                                        }
                                    }
                                    {
                                        let __v132 = self.make_string();
                                        self.hash[crate::ix::U(((p) - 1179650) as usize)]
                                            .set_rh(__v132);
                                    }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(d);
                                    self.cs_count = (self.cs_count).wrapping_add(1i32);
                                }
                            }
                            // §286
                            break 'l_found_f;
                        }
                    }
                    p = self.hash[crate::ix::U(((p) - 1179650) as usize)].lh();
                }
            }
        }
        id_lookup = p;
        id_lookup
    }

    /// Here is the subroutine that searches the primitive table for an identifier
    // §289
    pub fn prim_lookup(&mut self, mut s: str_number) -> halfword {
        let mut prim_lookup: halfword = 0;
        let mut h: i32 = 0; // §289
        let mut p: halfword = 0; // §289
        let mut k: halfword = 0; // §289
        let mut j: i32 = 0; // §289
        let mut l: i32 = 0; // §289
        'l_found_f: {
            if (s <= biggest_char) {
                {
                    if (s < 0i32) {
                        {
                            p = undefined_primitive;
                            break 'l_found_f;
                        }
                    } else {
                        p = (s % prim_prime).wrapping_add(1i32);
                    }
                }
            } else {
                {
                    j = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
                    if (s == self.str_ptr) {
                        l = (self.pool_ptr).wrapping_sub(
                            self.str_start
                                [crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)],
                        );
                    } else {
                        l = self.length(s);
                    }
                    // §291
                    h = self.str_pool[crate::ix::U((j) as usize)];
                    {
                        let __for_end_5 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                        k = (j).wrapping_add(1i32);
                        while k <= __for_end_5 {
                            {
                                h = ((h).wrapping_add(h))
                                    .wrapping_add(self.str_pool[crate::ix::U((k) as usize)]);
                                while (h >= prim_prime) {
                                    h = (h).wrapping_sub(1777i32);
                                }
                            }
                            k = k.wrapping_add(1);
                        }
                    }
                    // §289
                    p = (h).wrapping_add(1i32);
                }
            }
            while true {
                {
                    if (self.prim[crate::ix::U((p) as usize)].rh() > 65536i32) {
                        {
                            if (self.length(
                                (self.prim[crate::ix::U((p) as usize)].rh()).wrapping_sub(1i32),
                            ) == l)
                            {
                                if self.str_eq_str(
                                    (self.prim[crate::ix::U((p) as usize)].rh()).wrapping_sub(1i32),
                                    s,
                                ) {
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
                                // §290
                                {
                                    if (self.prim[crate::ix::U((p) as usize)].rh() > 0i32) {
                                        {
                                            loop {
                                                if (self.prim_used == prim_base) {
                                                    self.overflow(65808i32, prim_size);
                                                }
                                                self.prim_used =
                                                    (self.prim_used).wrapping_sub(1i32);
                                                if (self.prim
                                                    [crate::ix::U((self.prim_used) as usize)]
                                                .rh()
                                                    == 0i32)
                                                {
                                                    break;
                                                }
                                            }
                                            {
                                                let __v133 = self.prim_used;
                                                self.prim[crate::ix::U((p) as usize)]
                                                    .set_lh(__v133);
                                            }
                                            p = self.prim_used;
                                        }
                                    }
                                    self.prim[crate::ix::U((p) as usize)]
                                        .set_rh((s).wrapping_add(1i32));
                                }
                            }
                            // §289
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
    // §294
    pub fn primitive(&mut self, mut s: str_number, mut c: quarterword, mut o: halfword) {
        let mut k: pool_pointer = 0; // §294
        let mut j: i32 = 0; // §294
        let mut l: small_number = 0; // §294
        let mut prim_val: i32 = 0; // §294
        if (s < 256i32) {
            {
                self.cur_val = (s).wrapping_add(1114113i32);
                prim_val = self.prim_lookup(s);
            }
        } else {
            {
                k = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
                l = (self.str_start
                    [crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])
                .wrapping_sub(k);
                if ((self.first).wrapping_add(l) > (buf_size).wrapping_add(1i32)) {
                    self.overflow(65538i32, buf_size);
                }
                {
                    let __for_end_4 = (l).wrapping_sub(1i32);
                    j = 0i32;
                    while j <= __for_end_4 {
                        {
                            let __ix134 = (self.first).wrapping_add(j);
                            let __v135 =
                                self.str_pool[crate::ix::U(((k).wrapping_add(j)) as usize)];
                            self.buffer[crate::ix::U((__ix134) as usize)] = __v135;
                        }
                        j = j.wrapping_add(1);
                    }
                }
                self.cur_val = self.id_lookup(self.first, l);
                {
                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    self.pool_ptr = self.str_start
                        [crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                }
                {
                    let __ix136 = self.cur_val;
                    self.hash[crate::ix::U(((__ix136) - 1179650) as usize)].set_rh(s);
                }
                prim_val = self.prim_lookup(s);
            }
        }
        {
            let __ix137 = self.cur_val;
            self.eqtb[crate::ix::U(((__ix137) - 1) as usize)].set_hh_b1(level_one);
        }
        {
            let __ix138 = self.cur_val;
            self.eqtb[crate::ix::U(((__ix138) - 1) as usize)].set_hh_b0(c);
        }
        {
            let __ix139 = self.cur_val;
            self.eqtb[crate::ix::U(((__ix139) - 1) as usize)].set_hh_rh(o);
        }
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)]
            .set_hh_b1(level_one);
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)]
            .set_hh_b0(c);
        self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(prim_val)) - 1) as usize)]
            .set_hh_rh(o);
    }

    /// @<Declare \eTeX\ procedures for tr...
    // §314
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
    // §1471
    pub fn print_group(&mut self, mut e: bool) {
        'l_exit_f: {
            match self.cur_group {
                bottom_level => {
                    self.print(66860i32);
                    break 'l_exit_f;
                }
                simple_group | semi_simple_group => {
                    if (self.cur_group == semi_simple_group) {
                        self.print(66861i32);
                    }
                    self.print(66862i32);
                }
                hbox_group | adjusted_hbox_group => {
                    if (self.cur_group == adjusted_hbox_group) {
                        self.print(66863i32);
                    }
                    self.print(66488i32);
                }
                vbox_group => {
                    self.print(66396i32);
                }
                vtop_group => {
                    self.print(66487i32);
                }
                align_group | no_align_group => {
                    if (self.cur_group == no_align_group) {
                        self.print(66864i32);
                    }
                    self.print(66865i32);
                }
                output_group => {
                    self.print(65690i32);
                }
                disc_group => {
                    self.print(66866i32);
                }
                insert_group => {
                    self.print(65618i32);
                }
                vcenter_group => {
                    self.print(65855i32);
                }
                math_group | math_choice_group | math_shift_group | math_left_group => {
                    self.print(65633i32);
                    if (self.cur_group == math_choice_group) {
                        self.print(66867i32);
                    } else {
                        if (self.cur_group == math_shift_group) {
                            self.print(66868i32);
                        } else {
                            if (self.cur_group == math_left_group) {
                                self.print(66869i32);
                            }
                        }
                    }
                }
                _ => {}
            }
            self.print(66870i32);
            self.print_int(self.cur_level);
            self.print_char(41i32);
            if (self.save_stack[crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)].int()
                != 0i32)
            {
                {
                    if e {
                        self.print(65655i32);
                    } else {
                        self.print(66303i32);
                    }
                    self.print_int(
                        self.save_stack
                            [crate::ix::U(((self.save_ptr).wrapping_sub(1i32)) as usize)]
                        .int(),
                    );
                }
            }
        }
    }

    /// The `group_trace` procedure is called when a new level of grouping
    /// begins (`e=false`) or ends (`e=true`) with `saved(-1)` containing the
    /// line number.
    /// @<Declare \eTeX\ procedures for tr...
    // §1472
    pub fn group_trace(&mut self, mut e: bool) {
        self.begin_diagnostic();
        self.print_char(123i32);
        if e {
            self.print(66871i32);
        } else {
            self.print(66872i32);
        }
        self.print_group(e);
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// Here we read a line from the current pseudo file into `buffer`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1567
    pub fn pseudo_input(&mut self) -> bool {
        let mut pseudo_input: bool = false;
        let mut p: halfword = 0; // §1567
        let mut sz: i32 = 0; // §1567
        let mut w: four_quarters = four_quarters::default(); // §1567
        let mut r: halfword = 0; // §1567
        self.last = self.first;
        p = self.mem[crate::ix::U((self.pseudo_files) as usize)]
            .hh()
            .lh();
        if (p == (268435455i32).wrapping_neg()) {
            pseudo_input = false;
        } else {
            {
                {
                    let __ix140 = self.pseudo_files;
                    let __v141 = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.mem[crate::ix::U((__ix140) as usize)].set_hh_lh(__v141);
                }
                sz = self.mem[crate::ix::U((p) as usize)].hh().lh();
                if (((4i32).wrapping_mul(sz)).wrapping_sub(3i32)
                    >= (buf_size).wrapping_sub(self.last))
                {
                    // §35
                    if (self.format_ident == 0i32) {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "Buffer size exceeded!");
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            crate::system::final_end(self);
                        }
                    } else {
                        {
                            self.cur_input.loc_field = self.first;
                            self.cur_input.limit_field = (self.last).wrapping_sub(1i32);
                            self.overflow(65538i32, buf_size);
                        }
                    }
                }
                // §1567
                self.last = self.first;
                {
                    let __for_end_4 = ((p).wrapping_add(sz)).wrapping_sub(1i32);
                    r = (p).wrapping_add(1i32);
                    while r <= __for_end_4 {
                        {
                            w = self.mem[crate::ix::U((r) as usize)].qqqq();
                            self.buffer[crate::ix::U((self.last) as usize)] = w.b0();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(1i32)) as usize)] =
                                w.b1();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(2i32)) as usize)] =
                                w.b2();
                            self.buffer[crate::ix::U(((self.last).wrapping_add(3i32)) as usize)] =
                                w.b3();
                            self.last = (self.last).wrapping_add(4i32);
                        }
                        r = r.wrapping_add(1);
                    }
                }
                if (self.last >= self.max_buf_stack) {
                    self.max_buf_stack = (self.last).wrapping_add(1i32);
                }
                while ((self.last > self.first)
                    && (self.buffer[crate::ix::U(((self.last).wrapping_sub(1i32)) as usize)]
                        == 32i32))
                {
                    self.last = (self.last).wrapping_sub(1i32);
                }
                self.free_node(p, sz);
                pseudo_input = true;
            }
        }
        pseudo_input
    }

    /// When we are done with a pseudo file we `close' it.
    /// @<Declare \eTeX\ procedures for tr...
    // §1568
    pub fn pseudo_close(&mut self) {
        let mut p: halfword = 0; // §1568
        let mut q: halfword = 0; // §1568
        p = self.mem[crate::ix::U((self.pseudo_files) as usize)]
            .hh()
            .rh();
        q = self.mem[crate::ix::U((self.pseudo_files) as usize)]
            .hh()
            .lh();
        {
            {
                let __ix142 = self.pseudo_files;
                let __v143 = self.avail;
                self.mem[crate::ix::U((__ix142) as usize)].set_hh_rh(__v143);
            }
            self.avail = self.pseudo_files;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        self.pseudo_files = p;
        while (q != (268435455i32).wrapping_neg()) {
            {
                p = q;
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, self.mem[crate::ix::U((p) as usize)].hh().lh());
            }
        }
    }

    /// When a group ends that was apparently entered in a different input
    /// file, the `group_warning` procedure is invoked in order to update the
    /// `grp_stack`.  If moreover \.{\\tracingnesting} is positive we want to
    /// give a warning message.  The situation is, however, somewhat complicated
    /// by two facts:  (1)~There may be `grp_stack` elements without a
    /// corresponding \.{\\input} file or \.{\\scantokens} pseudo file (e.g.,
    /// error insertions from the terminal); and (2)~the relevant information is
    /// recorded in the `name_field` of the `input_stack` only loosely
    /// synchronized with the `in_open` variable indexing `grp_stack`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1585
    pub fn group_warning(&mut self) {
        let mut i: i32 = 0; // §1585
        let mut w: bool = false; // §1585
        self.base_ptr = self.input_ptr;
        {
            let __ix144 = self.base_ptr;
            let __v145 = self.cur_input;
            self.input_stack[crate::ix::U((__ix144) as usize)] = __v145;
        }
        i = self.in_open;
        w = false;
        while ((self.grp_stack[crate::ix::U((i) as usize)] == self.cur_boundary) && (i > 0i32)) {
            {
                // §1586
                if (self.eqtb[crate::ix::U(((7892329i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)]
                            .state_field
                            == token_list)
                            || (self.input_stack[crate::ix::U((self.base_ptr) as usize)]
                                .index_field
                                > i))
                        {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field
                            > 17i32)
                        {
                            w = true;
                        }
                    }
                }
                // §1585
                {
                    let __v146 = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                        .hh()
                        .rh();
                    self.grp_stack[crate::ix::U((i) as usize)] = __v146;
                }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(66929i32);
                self.print_group(true);
                self.print(66930i32);
                self.print_ln();
                if (self.eqtb[crate::ix::U(((7892329i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == spotless) {
                    self.history = warning_issued;
                }
            }
        }
    }

    /// When a conditional ends that was apparently started in a different
    /// input file, the `if_warning` procedure is invoked in order to update the
    /// `if_stack`.  If moreover \.{\\tracingnesting} is positive we want to
    /// give a warning message (with the same complications as above).
    /// @<Declare \eTeX\ procedures for tr...
    // §1587
    pub fn if_warning(&mut self) {
        let mut i: i32 = 0; // §1587
        let mut w: bool = false; // §1587
        self.base_ptr = self.input_ptr;
        {
            let __ix147 = self.base_ptr;
            let __v148 = self.cur_input;
            self.input_stack[crate::ix::U((__ix147) as usize)] = __v148;
        }
        i = self.in_open;
        w = false;
        while (self.if_stack[crate::ix::U((i) as usize)] == self.cond_ptr) {
            {
                // §1586
                if (self.eqtb[crate::ix::U(((7892329i32) - 1) as usize)].int() > 0i32) {
                    {
                        while ((self.input_stack[crate::ix::U((self.base_ptr) as usize)]
                            .state_field
                            == token_list)
                            || (self.input_stack[crate::ix::U((self.base_ptr) as usize)]
                                .index_field
                                > i))
                        {
                            self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                        }
                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field
                            > 17i32)
                        {
                            w = true;
                        }
                    }
                }
                // §1587
                {
                    let __v149 = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
                    self.if_stack[crate::ix::U((i) as usize)] = __v149;
                }
                i = (i).wrapping_sub(1i32);
            }
        }
        if w {
            {
                self.print_nl(66929i32);
                self.print_cmd_chr(if_test, self.cur_if);
                if (self.if_line != 0i32) {
                    {
                        self.print(66891i32);
                        self.print_int(self.if_line);
                    }
                }
                self.print(66930i32);
                self.print_ln();
                if (self.eqtb[crate::ix::U(((7892329i32) - 1) as usize)].int() > 1i32) {
                    self.show_context();
                }
                if (self.history == spotless) {
                    self.history = warning_issued;
                }
            }
        }
    }

    /// Conversely, the `file_warning` procedure is invoked when a file ends
    /// and some groups entered or conditionals started while reading from that
    /// file are still incomplete.
    /// @<Declare \eTeX\ procedures for tr...
    // §1588
    pub fn file_warning(&mut self) {
        let mut p: halfword = 0; // §1588
        let mut l: quarterword = 0; // §1588
        let mut c: quarterword = 0; // §1588
        let mut i: i32 = 0; // §1588
        p = self.save_ptr;
        l = self.cur_level;
        c = self.cur_group;
        self.save_ptr = self.cur_boundary;
        while (self.grp_stack[crate::ix::U((self.in_open) as usize)] != self.save_ptr) {
            {
                self.cur_level = (self.cur_level).wrapping_sub(1i32);
                self.print_nl(66931i32);
                self.print_group(true);
                self.print(66932i32);
                self.cur_group = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                    .hh()
                    .b1();
                self.save_ptr = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                    .hh()
                    .rh();
            }
        }
        self.save_ptr = p;
        self.cur_level = l;
        self.cur_group = c;
        p = self.cond_ptr;
        l = self.if_limit;
        c = self.cur_if;
        i = self.if_line;
        while (self.if_stack[crate::ix::U((self.in_open) as usize)] != self.cond_ptr) {
            {
                self.print_nl(66931i32);
                self.print_cmd_chr(if_test, self.cur_if);
                if (self.if_limit == fi_code) {
                    self.print_esc(66158i32);
                }
                if (self.if_line != 0i32) {
                    {
                        self.print(66891i32);
                        self.print_int(self.if_line);
                    }
                }
                self.print(66932i32);
                self.if_line =
                    self.mem[crate::ix::U(((self.cond_ptr).wrapping_add(1i32)) as usize)].int();
                self.cur_if = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().b1();
                self.if_limit = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().b0();
                self.cond_ptr = self.mem[crate::ix::U((self.cond_ptr) as usize)].hh().rh();
            }
        }
        self.cond_ptr = p;
        self.if_limit = l;
        self.cur_if = c;
        self.if_line = i;
        self.print_ln();
        if (self.eqtb[crate::ix::U(((7892329i32) - 1) as usize)].int() > 1i32) {
            self.show_context();
        }
        if (self.history == spotless) {
            self.history = warning_issued;
        }
    }

    /// The `delete_sa_ref` procedure is called when a pointer to an array
    /// element representing a register is being removed; this means that the
    /// reference count should be decreased by one.  If the reduced reference
    /// count is `null` and the register has been (globally) assigned its
    /// default value the array element should disappear, possibly together with
    /// some index nodes.  This procedure will never be used for mark class
    /// nodes.
    // §1632
    pub fn delete_sa_ref(&mut self, mut q: halfword) {
        let mut p: halfword = 0; // §1632
        let mut i: small_number = 0; // §1632
        let mut s: small_number = 0; // §1632
        'l_exit_f: {
            {
                let __v150 = (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                    .hh()
                    .lh())
                .wrapping_sub(1i32);
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(__v150);
            }
            if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                .hh()
                .lh()
                != (268435455i32).wrapping_neg())
            {
                break 'l_exit_f;
            }
            if (self.mem[crate::ix::U((q) as usize)].hh().b0() < dimen_val_limit) {
                if (self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    s = word_node_size;
                } else {
                    break 'l_exit_f;
                }
            } else {
                {
                    if (self.mem[crate::ix::U((q) as usize)].hh().b0() < mu_val_limit) {
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                            .hh()
                            .rh()
                            == zero_glue)
                        {
                            self.delete_glue_ref(zero_glue);
                        } else {
                            break 'l_exit_f;
                        }
                    } else {
                        if (self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                            .hh()
                            .rh()
                            != (268435455i32).wrapping_neg())
                        {
                            break 'l_exit_f;
                        }
                    }
                    s = pointer_node_size;
                }
            }
            loop {
                i = (self.mem[crate::ix::U((q) as usize)].hh().b0() % 64i32);
                p = q;
                q = self.mem[crate::ix::U((p) as usize)].hh().rh();
                self.free_node(p, s);
                if (q == (268435455i32).wrapping_neg()) {
                    {
                        self.sa_root[crate::ix::U((i) as usize)] = (268435455i32).wrapping_neg();
                        break 'l_exit_f;
                    }
                }
                {
                    if (((i) % 2) != 0) {
                        self.mem[crate::ix::U(
                            (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                        )]
                        .set_hh_rh((268435455i32).wrapping_neg());
                    } else {
                        self.mem[crate::ix::U(
                            (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                        )]
                        .set_hh_lh((268435455i32).wrapping_neg());
                    }
                    {
                        let __v151 =
                            (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_sub(1i32);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v151);
                    }
                }
                s = index_node_size;
                if (self.mem[crate::ix::U((q) as usize)].hh().b1() > 0i32) {
                    break;
                }
            }
        }
    }

    /// Here is a procedure that displays the contents of an array element
    /// symbolically.  It is used under similar circumstances as is
    /// `restore_trace` (together with `show_eqtb`) for the quantities kept in
    /// the `eqtb` array.
    /// @<Declare \eTeX\ procedures for tr...
    // §1634
    pub fn show_sa(&mut self, mut p: halfword, mut s: str_number) {
        let mut t: small_number = 0; // §1634
        self.begin_diagnostic();
        self.print_char(123i32);
        self.print(s);
        self.print_char(32i32);
        if (p == (268435455i32).wrapping_neg()) {
            self.print_char(63i32);
        } else {
            {
                t = (self.mem[crate::ix::U((p) as usize)].hh().b0() / 64i32);
                if (t < box_val) {
                    self.print_cmd_chr(register, p);
                } else {
                    if (t == box_val) {
                        {
                            self.print_esc(65701i32);
                            self.print_sa_num(p);
                        }
                    } else {
                        if (t == tok_val) {
                            self.print_cmd_chr(toks_register, p);
                        } else {
                            self.print_char(63i32);
                        }
                    }
                }
                self.print_char(61i32);
                if (t == int_val) {
                    self.print_int(self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int());
                } else {
                    if (t == dimen_val) {
                        {
                            self.print_scaled(
                                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int(),
                            );
                            self.print(65689i32);
                        }
                    } else {
                        {
                            p = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                .hh()
                                .rh();
                            if (t == glue_val) {
                                self.print_spec(p, 65689i32);
                            } else {
                                if (t == mu_val) {
                                    self.print_spec(p, 65625i32);
                                } else {
                                    if (t == box_val) {
                                        if (p == (268435455i32).wrapping_neg()) {
                                            self.print(65702i32);
                                        } else {
                                            {
                                                self.depth_threshold = 0i32;
                                                self.breadth_max = 1i32;
                                                self.show_node_list(p);
                                            }
                                        }
                                    } else {
                                        if (t == tok_val) {
                                            {
                                                if (p != (268435455i32).wrapping_neg()) {
                                                    self.show_token_list(
                                                        self.mem[crate::ix::U((p) as usize)]
                                                            .hh()
                                                            .rh(),
                                                        (268435455i32).wrapping_neg(),
                                                        32i32,
                                                    );
                                                }
                                            }
                                        } else {
                                            self.print_char(63i32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The individual saved items are kept in pointer or word nodes similar
    /// to those used for the array elements: a word node with value zero is,
    /// however, saved as pointer node with the otherwise impossible `sa_index`
    /// value `tok_val_limit`.
    // §1648
    pub fn sa_save(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §1648
        let mut i: quarterword = 0; // §1648
        if (self.cur_level != self.sa_level) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(65857i32, save_size);
                        }
                    }
                }
                {
                    let __ix152 = self.save_ptr;
                    self.save_stack[crate::ix::U((__ix152) as usize)].set_hh_b0(restore_sa);
                }
                {
                    let __ix153 = self.save_ptr;
                    let __v154 = self.sa_level;
                    self.save_stack[crate::ix::U((__ix153) as usize)].set_hh_b1(__v154);
                }
                {
                    let __ix155 = self.save_ptr;
                    let __v156 = self.sa_chain;
                    self.save_stack[crate::ix::U((__ix155) as usize)].set_hh_rh(__v156);
                }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                self.sa_chain = (268435455i32).wrapping_neg();
                self.sa_level = self.cur_level;
            }
        }
        i = self.mem[crate::ix::U((p) as usize)].hh().b0();
        if (i < dimen_val_limit) {
            {
                if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32) {
                    {
                        q = self.get_node(pointer_node_size);
                        i = tok_val_limit;
                    }
                } else {
                    {
                        q = self.get_node(word_node_size);
                        {
                            let __v157 =
                                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int();
                            self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)]
                                .set_int(__v157);
                        }
                    }
                }
                self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)]
                    .set_hh_rh((268435455i32).wrapping_neg());
            }
        } else {
            {
                q = self.get_node(pointer_node_size);
                {
                    let __v158 = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                        .hh()
                        .rh();
                    self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_rh(__v158);
                }
            }
        }
        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_hh_lh(p);
        self.mem[crate::ix::U((q) as usize)].set_hh_b0(i);
        {
            let __v159 = self.mem[crate::ix::U((p) as usize)].hh().b1();
            self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v159);
        }
        {
            let __v160 = self.sa_chain;
            self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v160);
        }
        self.sa_chain = q;
        {
            let __v161 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .lh())
            .wrapping_add(1i32);
            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v161);
        }
    }

    /// @<Declare \eTeX\ procedures for tr...
    // §1649
    pub fn sa_destroy(&mut self, mut p: halfword) {
        if (self.mem[crate::ix::U((p) as usize)].hh().b0() < mu_val_limit) {
            self.delete_glue_ref(
                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                    .hh()
                    .rh(),
            );
        } else {
            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .rh()
                != (268435455i32).wrapping_neg())
            {
                if (self.mem[crate::ix::U((p) as usize)].hh().b0() < box_val_limit) {
                    self.flush_node_list(
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                            .hh()
                            .rh(),
                    );
                } else {
                    self.delete_token_ref(
                        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                            .hh()
                            .rh(),
                    );
                }
            }
        }
    }

    /// The procedure `sa_def` assigns a new value to sparse array elements,
    /// and saves the former value if appropriate.  This procedure is used only
    /// for skip, muskip, box, and token list registers.  The counterpart of
    /// `sa_def` for count and dimen registers is called `sa_w_def`.
    // §1650
    pub fn sa_def(&mut self, mut p: halfword, mut e: halfword) {
        {
            let __v162 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .lh())
            .wrapping_add(1i32);
            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v162);
        }
        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
            .hh()
            .rh()
            == e)
        {
            {
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65859i32);
                }
                self.sa_destroy(p);
            }
        } else {
            {
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65860i32);
                }
                if (self.mem[crate::ix::U((p) as usize)].hh().b1() == self.cur_level) {
                    self.sa_destroy(p);
                } else {
                    self.sa_save(p);
                }
                {
                    let __v163 = self.cur_level;
                    self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v163);
                }
                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65861i32);
                }
            }
        }
        self.delete_sa_ref(p);
    }

    /// The procedure `sa_def` assigns a new value to sparse array elements,
    /// and saves the former value if appropriate.  This procedure is used only
    /// for skip, muskip, box, and token list registers.  The counterpart of
    /// `sa_def` for count and dimen registers is called `sa_w_def`.
    // §1650
    pub fn sa_w_def(&mut self, mut p: halfword, mut w: i32) {
        {
            let __v164 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .lh())
            .wrapping_add(1i32);
            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v164);
        }
        if (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == w) {
            {
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65859i32);
                }
            }
        } else {
            {
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65860i32);
                }
                if (self.mem[crate::ix::U((p) as usize)].hh().b1() != self.cur_level) {
                    self.sa_save(p);
                }
                {
                    let __v165 = self.cur_level;
                    self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v165);
                }
                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
                if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                    self.show_sa(p, 65861i32);
                }
            }
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_def` and `sa_w_def` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1651
    pub fn gsa_def(&mut self, mut p: halfword, mut e: halfword) {
        {
            let __v166 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .lh())
            .wrapping_add(1i32);
            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v166);
        }
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 65862i32);
        }
        self.sa_destroy(p);
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(level_one);
        self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_rh(e);
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 65861i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_def` and `sa_w_def` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    /// @<Declare \eTeX\ procedures for tr...
    // §1651
    pub fn gsa_w_def(&mut self, mut p: halfword, mut w: i32) {
        {
            let __v167 = (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                .hh()
                .lh())
            .wrapping_add(1i32);
            self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_hh_lh(__v167);
        }
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 65862i32);
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_b1(level_one);
        self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(w);
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.show_sa(p, 65861i32);
        }
        self.delete_sa_ref(p);
    }

    /// The `sa_restore` procedure restores the sparse array entries pointed
    /// at by `sa_chain`
    /// @<Declare \eTeX\ procedures for tr...
    // §1652
    pub fn sa_restore(&mut self) {
        let mut p: halfword = 0; // §1652
        loop {
            p = self.mem[crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)]
                .hh()
                .lh();
            if (self.mem[crate::ix::U((p) as usize)].hh().b1() == level_one) {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() >= dimen_val_limit) {
                        self.sa_destroy(self.sa_chain);
                    }
                    if (self.eqtb[crate::ix::U(((7892301i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 65864i32);
                    }
                }
            } else {
                {
                    if (self.mem[crate::ix::U((p) as usize)].hh().b0() < dimen_val_limit) {
                        if (self.mem[crate::ix::U((self.sa_chain) as usize)].hh().b0()
                            < dimen_val_limit)
                        {
                            {
                                let __v168 = self.mem
                                    [crate::ix::U(((self.sa_chain).wrapping_add(2i32)) as usize)]
                                .int();
                                self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)]
                                    .set_int(__v168);
                            }
                        } else {
                            self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].set_int(0i32);
                        }
                    } else {
                        {
                            self.sa_destroy(p);
                            {
                                let __v169 = self.mem
                                    [crate::ix::U(((self.sa_chain).wrapping_add(1i32)) as usize)]
                                .hh()
                                .rh();
                                self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                    .set_hh_rh(__v169);
                            }
                        }
                    }
                    {
                        let __v170 = self.mem[crate::ix::U((self.sa_chain) as usize)].hh().b1();
                        self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v170);
                    }
                    if (self.eqtb[crate::ix::U(((7892301i32) - 1) as usize)].int() > 0i32) {
                        self.show_sa(p, 65865i32);
                    }
                }
            }
            self.delete_sa_ref(p);
            p = self.sa_chain;
            self.sa_chain = self.mem[crate::ix::U((p) as usize)].hh().rh();
            if (self.mem[crate::ix::U((p) as usize)].hh().b0() < dimen_val_limit) {
                self.free_node(p, word_node_size);
            } else {
                self.free_node(p, pointer_node_size);
            }
            if (self.sa_chain == (268435455i32).wrapping_neg()) {
                break;
            }
        }
    }

    /// Procedure `new_save_level` is called when a group begins. The
    /// argument is a group identification code like ``hbox_group`'. After
    /// calling this routine, it is safe to put five more entries on `save_stack`.
    /// In some cases integer-valued items are placed onto the
    /// `save_stack` just below a `level_boundary` word, because this is a
    /// convenient place to keep information that is supposed to ``pop up'' just
    /// when the group has finished.
    /// For example, when `\.{\\hbox to 100pt}\grp' is being treated, the 100pt
    /// dimension is stored on `save_stack` just before `new_save_level` is
    /// called.
    /// We use the notation `saved(k)` to stand for an integer item that
    /// appears in location `save_ptr+k` of the save stack.
    // §304
    pub fn new_save_level(&mut self, mut c: group_code) {
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(65857i32, save_size);
                }
            }
        }
        if (self.eTeX_mode == 1i32) {
            {
                {
                    let __ix171 = (self.save_ptr).wrapping_add(0i32);
                    let __v172 = self.line;
                    self.save_stack[crate::ix::U((__ix171) as usize)].set_int(__v172);
                }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
        {
            let __ix173 = self.save_ptr;
            self.save_stack[crate::ix::U((__ix173) as usize)].set_hh_b0(level_boundary);
        }
        {
            let __ix174 = self.save_ptr;
            let __v175 = self.cur_group;
            self.save_stack[crate::ix::U((__ix174) as usize)].set_hh_b1(__v175);
        }
        {
            let __ix176 = self.save_ptr;
            let __v177 = self.cur_boundary;
            self.save_stack[crate::ix::U((__ix176) as usize)].set_hh_rh(__v177);
        }
        if (self.cur_level == max_quarterword) {
            self.overflow(65858i32, 65535i32);
        }
        self.cur_boundary = self.save_ptr;
        self.cur_group = c;
        if (self.eqtb[crate::ix::U(((7892326i32) - 1) as usize)].int() > 0i32) {
            self.group_trace(false);
        }
        self.cur_level = (self.cur_level).wrapping_add(1i32);
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
    }

    /// Just before an entry of `eqtb` is changed, the following procedure should
    /// be called to update the other data structures properly. It is important
    /// to keep in mind that reference counts in `mem` include references from
    /// within `save_stack`, so these counts must be handled carefully.
    // §305
    pub fn eq_destroy(&mut self, mut w: memory_word) {
        let mut q: halfword = 0; // §305
        match w.hh().b0() {
            call | long_call | outer_call | long_outer_call => {
                self.delete_token_ref(w.hh().rh());
            }
            glue_ref => {
                self.delete_glue_ref(w.hh().rh());
            }
            shape_ref => {
                q = w.hh().rh();
                if (q != (268435455i32).wrapping_neg()) {
                    self.free_node(
                        q,
                        ((self.mem[crate::ix::U((q) as usize)].hh().lh())
                            .wrapping_add(self.mem[crate::ix::U((q) as usize)].hh().lh()))
                        .wrapping_add(1i32),
                    );
                }
            }
            box_ref => {
                self.flush_node_list(w.hh().rh());
            }
            toks_register | register => {
                // §1645
                if ((w.hh().rh() < mem_bot) || (w.hh().rh() > lo_mem_stat_max)) {
                    self.delete_sa_ref(w.hh().rh());
                }
            }
            _ => {
                // §305
            }
        }
    }

    /// To save a value of `eqtb[p]` that was established at level `l`, we
    /// can use the following subroutine.
    // §306
    pub fn eq_save(&mut self, mut p: halfword, mut l: quarterword) {
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                    self.overflow(65857i32, save_size);
                }
            }
        }
        if (l == level_zero) {
            {
                let __ix178 = self.save_ptr;
                self.save_stack[crate::ix::U((__ix178) as usize)].set_hh_b0(restore_zero);
            }
        } else {
            {
                {
                    let __ix179 = self.save_ptr;
                    let __v180 = self.eqtb[crate::ix::U(((p) - 1) as usize)];
                    self.save_stack[crate::ix::U((__ix179) as usize)] = __v180;
                }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                {
                    let __ix181 = self.save_ptr;
                    self.save_stack[crate::ix::U((__ix181) as usize)].set_hh_b0(restore_old_value);
                }
            }
        }
        {
            let __ix182 = self.save_ptr;
            self.save_stack[crate::ix::U((__ix182) as usize)].set_hh_b1(l);
        }
        {
            let __ix183 = self.save_ptr;
            self.save_stack[crate::ix::U((__ix183) as usize)].set_hh_rh(p);
        }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
    }

    /// The procedure `eq_define` defines an `eqtb` entry having specified
    /// `eq_type` and `equiv` fields, and saves the former value if appropriate.
    /// This procedure is used only for entries in the first four regions of `eqtb`,
    /// i.e., only for entries that have `eq_type` and `equiv` fields.
    /// After calling this routine, it is safe to put four more entries on
    /// `save_stack`, provided that there was room for four more entries before
    /// the call, since `eq_save` makes the necessary test.
    // §307
    pub fn eq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        'l_exit_f: {
            if (((self.eTeX_mode == 1i32)
                && (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b0() == t))
                && (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().rh() == e))
            {
                {
                    if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 65859i32);
                    }
                    self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
                    break 'l_exit_f;
                }
            }
            if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 65860i32);
            }
            if (self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1() == self.cur_level) {
                self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
            } else {
                if (self.cur_level > level_one) {
                    self.eq_save(p, self.eqtb[crate::ix::U(((p) - 1) as usize)].hh().b1());
                }
            }
            {
                let __v184 = self.cur_level;
                self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(__v184);
            }
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
            if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 65861i32);
            }
        }
    }

    /// The counterpart of `eq_define` for the remaining (fullword) positions in
    /// `eqtb` is called `eq_word_define`. Since `xeq_level[p]>=level_one` for all
    /// `p`, a ``restore_zero`' will never be used in this case.
    // §308
    pub fn eq_word_define(&mut self, mut p: halfword, mut w: i32) {
        'l_exit_f: {
            if ((self.eTeX_mode == 1i32)
                && (self.eqtb[crate::ix::U(((p) - 1) as usize)].int() == w))
            {
                {
                    if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                        self.restore_trace(p, 65859i32);
                    }
                    break 'l_exit_f;
                }
            }
            if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 65860i32);
            }
            if (self.xeq_level[crate::ix::U(((p) - 7892264) as usize)] != self.cur_level) {
                {
                    self.eq_save(p, self.xeq_level[crate::ix::U(((p) - 7892264) as usize)]);
                    {
                        let __v185 = self.cur_level;
                        self.xeq_level[crate::ix::U(((p) - 7892264) as usize)] = __v185;
                    }
                }
            }
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
                self.restore_trace(p, 65861i32);
            }
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §309
    pub fn geq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 65862i32);
        }
        {
            self.eq_destroy(self.eqtb[crate::ix::U(((p) - 1) as usize)]);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b1(level_one);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_b0(t);
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_hh_rh(e);
        }
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 65861i32);
        }
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §309
    pub fn geq_word_define(&mut self, mut p: halfword, mut w: i32) {
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 65862i32);
        }
        {
            self.eqtb[crate::ix::U(((p) - 1) as usize)].set_int(w);
            self.xeq_level[crate::ix::U(((p) - 7892264) as usize)] = level_one;
        }
        if (self.eqtb[crate::ix::U(((7892325i32) - 1) as usize)].int() > 0i32) {
            self.restore_trace(p, 65861i32);
        }
    }

    /// Subroutine `save_for_after` puts a token on the stack for save-keeping.
    // §310
    pub fn save_for_after(&mut self, mut t: halfword) {
        if (self.cur_level > level_one) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(7i32)) {
                            self.overflow(65857i32, save_size);
                        }
                    }
                }
                {
                    let __ix186 = self.save_ptr;
                    self.save_stack[crate::ix::U((__ix186) as usize)].set_hh_b0(insert_token);
                }
                {
                    let __ix187 = self.save_ptr;
                    self.save_stack[crate::ix::U((__ix187) as usize)].set_hh_b1(level_zero);
                }
                {
                    let __ix188 = self.save_ptr;
                    self.save_stack[crate::ix::U((__ix188) as usize)].set_hh_rh(t);
                }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
    }

    /// The `unsave` routine goes the other way, taking items off of `save_stack`.
    /// This routine takes care of restoration when a level ends; everything
    /// belonging to the topmost group is cleared off of the save stack.
    // §311
    pub fn unsave(&mut self) {
        let mut p: halfword = 0; // §311
        let mut l: quarterword = 0; // §311
        let mut t: halfword = 0; // §311
        let mut a: bool = false; // §311
        a = false;
        if (self.cur_level > level_one) {
            {
                'l_done_f: {
                    self.cur_level = (self.cur_level).wrapping_sub(1i32);
                    // §312
                    while true {
                        {
                            self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                                .hh()
                                .b0()
                                == level_boundary)
                            {
                                break 'l_done_f;
                            }
                            p = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                                .hh()
                                .rh();
                            if (self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                                .hh()
                                .b0()
                                == insert_token)
                            {
                                // §356
                                {
                                    t = self.cur_tok;
                                    self.cur_tok = p;
                                    if a {
                                        {
                                            p = self.get_avail();
                                            {
                                                let __v189 = self.cur_tok;
                                                self.mem[crate::ix::U((p) as usize)]
                                                    .set_hh_lh(__v189);
                                            }
                                            {
                                                let __v190 = self.cur_input.loc_field;
                                                self.mem[crate::ix::U((p) as usize)]
                                                    .set_hh_rh(__v190);
                                            }
                                            self.cur_input.loc_field = p;
                                            self.cur_input.start_field = p;
                                            if (self.cur_tok < right_brace_limit) {
                                                if (self.cur_tok < left_brace_limit) {
                                                    self.align_state =
                                                        (self.align_state).wrapping_sub(1i32);
                                                } else {
                                                    self.align_state =
                                                        (self.align_state).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            self.back_input();
                                            a = (self.eTeX_mode == 1i32);
                                        }
                                    }
                                    self.cur_tok = t;
                                }
                            } else {
                                // §312
                                if (self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                                    .hh()
                                    .b0()
                                    == restore_sa)
                                {
                                    {
                                        self.sa_restore();
                                        self.sa_chain = p;
                                        self.sa_level = self.save_stack
                                            [crate::ix::U((self.save_ptr) as usize)]
                                        .hh()
                                        .b1();
                                    }
                                } else {
                                    {
                                        if (self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                                            .hh()
                                            .b0()
                                            == restore_old_value)
                                        {
                                            {
                                                l = self.save_stack
                                                    [crate::ix::U((self.save_ptr) as usize)]
                                                .hh()
                                                .b1();
                                                self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                                            }
                                        } else {
                                            {
                                                let __ix191 = self.save_ptr;
                                                let __v192 = self.eqtb[crate::ix::U(
                                                    ((undefined_control_sequence) - 1) as usize,
                                                )];
                                                self.save_stack[crate::ix::U((__ix191) as usize)] =
                                                    __v192;
                                            }
                                        }
                                        // §313
                                        if ((p < int_base) || (p > eqtb_size)) {
                                            if (self.eqtb[crate::ix::U(((p) - 1) as usize)]
                                                .hh()
                                                .b1()
                                                == level_one)
                                            {
                                                {
                                                    self.eq_destroy(
                                                        self.save_stack[crate::ix::U(
                                                            (self.save_ptr) as usize,
                                                        )],
                                                    );
                                                    if (self.eqtb
                                                        [crate::ix::U(((7892301i32) - 1) as usize)]
                                                    .int()
                                                        > 0i32)
                                                    {
                                                        self.restore_trace(p, 65864i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.eq_destroy(
                                                        self.eqtb[crate::ix::U(((p) - 1) as usize)],
                                                    );
                                                    {
                                                        let __v193 = self.save_stack[crate::ix::U(
                                                            (self.save_ptr) as usize,
                                                        )];
                                                        self.eqtb
                                                            [crate::ix::U(((p) - 1) as usize)] =
                                                            __v193;
                                                    }
                                                    if (self.eqtb
                                                        [crate::ix::U(((7892301i32) - 1) as usize)]
                                                    .int()
                                                        > 0i32)
                                                    {
                                                        self.restore_trace(p, 65865i32);
                                                    }
                                                }
                                            }
                                        } else {
                                            if (self.xeq_level
                                                [crate::ix::U(((p) - 7892264) as usize)]
                                                != level_one)
                                            {
                                                {
                                                    {
                                                        let __v194 = self.save_stack[crate::ix::U(
                                                            (self.save_ptr) as usize,
                                                        )];
                                                        self.eqtb
                                                            [crate::ix::U(((p) - 1) as usize)] =
                                                            __v194;
                                                    }
                                                    self.xeq_level
                                                        [crate::ix::U(((p) - 7892264) as usize)] =
                                                        l;
                                                    if (self.eqtb
                                                        [crate::ix::U(((7892301i32) - 1) as usize)]
                                                    .int()
                                                        > 0i32)
                                                    {
                                                        self.restore_trace(p, 65865i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    if (self.eqtb
                                                        [crate::ix::U(((7892301i32) - 1) as usize)]
                                                    .int()
                                                        > 0i32)
                                                    {
                                                        self.restore_trace(p, 65864i32);
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
                // §312
                if (self.eqtb[crate::ix::U(((7892326i32) - 1) as usize)].int() > 0i32) {
                    self.group_trace(true);
                }
                if (self.grp_stack[crate::ix::U((self.in_open) as usize)] == self.cur_boundary) {
                    self.group_warning();
                }
                self.cur_group = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                    .hh()
                    .b1();
                self.cur_boundary = self.save_stack[crate::ix::U((self.save_ptr) as usize)]
                    .hh()
                    .rh();
                if (self.eTeX_mode == 1i32) {
                    self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                }
            }
        } else {
            // §311
            self.confusion(65863i32);
        }
    }

    /// The `prepare_mag` subroutine is called whenever \TeX\ wants to use `mag`
    /// for magnification.
    // §318
    pub fn prepare_mag(&mut self) {
        if ((self.mag_set > 0i32)
            && (self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int() != self.mag_set))
        {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65867i32);
                }
                self.print_int(self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                self.print(65868i32);
                self.print_nl(65869i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 65870i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65871i32;
                }
                self.int_error(self.mag_set);
                self.geq_word_define(7892281i32, self.mag_set);
            }
        }
        if ((self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int() <= 0i32)
            || (self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int() > 32768i32))
        {
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
                self.int_error(self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                self.geq_word_define(7892281i32, 1000i32);
            }
        }
        self.mag_set = self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int();
    }

    /// Here's the way we sometimes want to display a token list, given a pointer
    /// to its reference count; the pointer may be null.
    // §325
    pub fn token_show(&mut self, mut p: halfword) {
        if (p != (268435455i32).wrapping_neg()) {
            self.show_token_list(
                self.mem[crate::ix::U((p) as usize)].hh().rh(),
                (268435455i32).wrapping_neg(),
                10000000i32,
            );
        }
    }

    /// The `print_meaning` subroutine displays `cur_cmd` and `cur_chr` in
    /// symbolic form, including the expansion of a macro or mark.
    // §326
    pub fn print_meaning(&mut self) {
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.cur_cmd >= call) {
            {
                self.print_char(58i32);
                self.print_ln();
                self.token_show(self.cur_chr);
            }
        } else {
            if ((self.cur_cmd == top_bot_mark) && (self.cur_chr < marks_code)) {
                {
                    self.print_char(58i32);
                    self.print_ln();
                    self.token_show(self.cur_mark[crate::ix::U((self.cur_chr) as usize)]);
                }
            }
        }
    }

    /// Here is a procedure that displays the current command.
    // §329
    pub fn show_cur_cmd_chr(&mut self) {
        let mut n: i32 = 0; // §329
        let mut l: i32 = 0; // §329
        let mut p: halfword = 0; // §329
        self.begin_diagnostic();
        self.print_nl(123i32);
        if (self.cur_list.mode_field != self.shown_mode) {
            {
                self.print_mode(self.cur_list.mode_field);
                self.print(65593i32);
                self.shown_mode = self.cur_list.mode_field;
            }
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.eqtb[crate::ix::U(((7892327i32) - 1) as usize)].int() > 0i32) {
            if (self.cur_cmd >= if_test) {
                if (self.cur_cmd <= fi_or_else) {
                    {
                        self.print(65593i32);
                        if (self.cur_cmd == fi_or_else) {
                            {
                                self.print_cmd_chr(if_test, self.cur_if);
                                self.print_char(32i32);
                                n = 0i32;
                                l = self.if_line;
                            }
                        } else {
                            {
                                n = 1i32;
                                l = self.line;
                            }
                        }
                        p = self.cond_ptr;
                        while (p != (268435455i32).wrapping_neg()) {
                            {
                                n = (n).wrapping_add(1i32);
                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                            }
                        }
                        self.print(65887i32);
                        self.print_int(n);
                        self.print_char(41i32);
                        if (l != 0i32) {
                            {
                                self.print(66891i32);
                                self.print_int(l);
                            }
                        }
                    }
                }
            }
        }
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The status at each level is indicated by printing two lines, where the first
    /// line indicates what was read so far and the second line shows what remains
    /// to be read. The context is cropped, if necessary, so that the first line
    /// contains at most `half_error_line` characters, and the second contains
    /// at most `error_line`. Non-current input levels whose `token_type` is
    /// ``backed_up`' are shown only if they have not been fully read.
    // §341
    pub fn show_context(&mut self) {
        let mut old_setting: i32 = 0; // §341
        let mut nn: i32 = 0; // §341
        let mut bottom_line: bool = false; // §341
        let mut i: i32 = 0; // §345
        let mut j: i32 = 0; // §345
        let mut l: i32 = 0; // §345
        let mut m: i32 = 0; // §345
        let mut n: i32 = 0; // §345
        let mut p: i32 = 0; // §345
        let mut q: i32 = 0; // §345
        'l_done_f: {
            self.base_ptr = self.input_ptr;
            {
                let __ix195 = self.base_ptr;
                let __v196 = self.cur_input;
                self.input_stack[crate::ix::U((__ix195) as usize)] = __v196;
            }
            nn = (1i32).wrapping_neg();
            bottom_line = false;
            while true {
                {
                    self.cur_input = self.input_stack[crate::ix::U((self.base_ptr) as usize)];
                    if (self.cur_input.state_field != token_list) {
                        if ((self.cur_input.name_field > 19i32) || (self.base_ptr == 0i32)) {
                            bottom_line = true;
                        }
                    }
                    if (((self.base_ptr == self.input_ptr) || bottom_line)
                        || (nn < self.eqtb[crate::ix::U(((7892318i32) - 1) as usize)].int()))
                    {
                        // §342
                        {
                            if ((((self.base_ptr == self.input_ptr)
                                || (self.cur_input.state_field != token_list))
                                || (self.cur_input.index_field != backed_up))
                                || (self.cur_input.loc_field != (268435455i32).wrapping_neg()))
                            {
                                {
                                    self.tally = 0i32;
                                    old_setting = self.selector;
                                    if (self.cur_input.state_field != token_list) {
                                        {
                                            // §343
                                            if (self.cur_input.name_field <= 17i32) {
                                                if (self.cur_input.name_field == 0i32) {
                                                    if (self.base_ptr == 0i32) {
                                                        self.print_nl(65893i32);
                                                    } else {
                                                        self.print_nl(65894i32);
                                                    }
                                                } else {
                                                    {
                                                        self.print_nl(65895i32);
                                                        if (self.cur_input.name_field == 17i32) {
                                                            self.print_char(42i32);
                                                        } else {
                                                            self.print_int(
                                                                (self.cur_input.name_field)
                                                                    .wrapping_sub(1i32),
                                                            );
                                                        }
                                                        self.print_char(62i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.print_nl(65896i32);
                                                    if (self.cur_input.index_field == self.in_open)
                                                    {
                                                        self.print_int(self.line);
                                                    } else {
                                                        self.print_int(
                                                            self.line_stack[crate::ix::U(
                                                                (((self.cur_input.index_field)
                                                                    .wrapping_add(1i32))
                                                                    - 1)
                                                                    as usize,
                                                            )],
                                                        );
                                                    }
                                                }
                                            }
                                            self.print_char(32i32);
                                            // §348
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = pseudo;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.buffer[crate::ix::U(
                                                (self.cur_input.limit_field) as usize,
                                            )] == self.eqtb
                                                [crate::ix::U(((7892312i32) - 1) as usize)]
                                            .int())
                                            {
                                                j = self.cur_input.limit_field;
                                            } else {
                                                j = (self.cur_input.limit_field).wrapping_add(1i32);
                                            }
                                            if (j > 0i32) {
                                                {
                                                    let __for_end_12 = (j).wrapping_sub(1i32);
                                                    i = self.cur_input.start_field;
                                                    while i <= __for_end_12 {
                                                        {
                                                            if (i == self.cur_input.loc_field) {
                                                                {
                                                                    self.first_count = self.tally;
                                                                    self.trick_count = (((self
                                                                        .tally)
                                                                        .wrapping_add(1i32))
                                                                    .wrapping_add(self.error_line))
                                                                    .wrapping_sub(
                                                                        self.half_error_line,
                                                                    );
                                                                    if (self.trick_count
                                                                        < self.error_line)
                                                                    {
                                                                        self.trick_count =
                                                                            self.error_line;
                                                                    }
                                                                }
                                                            }
                                                            self.print_char(
                                                                self.buffer
                                                                    [crate::ix::U((i) as usize)],
                                                            );
                                                        }
                                                        i = i.wrapping_add(1);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // §342
                                        {
                                            // §344
                                            match self.cur_input.index_field {
                                                parameter => {
                                                    self.print_nl(65897i32);
                                                }
                                                u_template | v_template => {
                                                    self.print_nl(65898i32);
                                                }
                                                backed_up | backed_up_char => {
                                                    if (self.cur_input.loc_field
                                                        == (268435455i32).wrapping_neg())
                                                    {
                                                        self.print_nl(65899i32);
                                                    } else {
                                                        self.print_nl(65900i32);
                                                    }
                                                }
                                                inserted => {
                                                    self.print_nl(65901i32);
                                                }
                                                macro_ => {
                                                    self.print_ln();
                                                    self.print_cs(self.cur_input.name_field);
                                                }
                                                output_text => {
                                                    self.print_nl(65902i32);
                                                }
                                                every_par_text => {
                                                    self.print_nl(65903i32);
                                                }
                                                every_math_text => {
                                                    self.print_nl(65904i32);
                                                }
                                                every_display_text => {
                                                    self.print_nl(65905i32);
                                                }
                                                every_hbox_text => {
                                                    self.print_nl(65906i32);
                                                }
                                                every_vbox_text => {
                                                    self.print_nl(65907i32);
                                                }
                                                every_job_text => {
                                                    self.print_nl(65908i32);
                                                }
                                                every_cr_text => {
                                                    self.print_nl(65909i32);
                                                }
                                                mark_text => {
                                                    self.print_nl(65910i32);
                                                }
                                                every_eof_text => {
                                                    self.print_nl(65911i32);
                                                }
                                                inter_char_text => {
                                                    self.print_nl(65912i32);
                                                }
                                                write_text => {
                                                    self.print_nl(65913i32);
                                                }
                                                _ => {
                                                    self.print_nl(63i32);
                                                }
                                            }
                                            // §349
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = pseudo;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.cur_input.index_field < macro_) {
                                                self.show_token_list(
                                                    self.cur_input.start_field,
                                                    self.cur_input.loc_field,
                                                    100000i32,
                                                );
                                            } else {
                                                self.show_token_list(
                                                    self.mem[crate::ix::U(
                                                        (self.cur_input.start_field) as usize,
                                                    )]
                                                    .hh()
                                                    .rh(),
                                                    self.cur_input.loc_field,
                                                    100000i32,
                                                );
                                            }
                                        }
                                    }
                                    // §342
                                    self.selector = old_setting;
                                    // §347
                                    if (self.trick_count == 1000000i32) {
                                        {
                                            self.first_count = self.tally;
                                            self.trick_count = (((self.tally).wrapping_add(1i32))
                                                .wrapping_add(self.error_line))
                                            .wrapping_sub(self.half_error_line);
                                            if (self.trick_count < self.error_line) {
                                                self.trick_count = self.error_line;
                                            }
                                        }
                                    }
                                    if (self.tally < self.trick_count) {
                                        m = (self.tally).wrapping_sub(self.first_count);
                                    } else {
                                        m = (self.trick_count).wrapping_sub(self.first_count);
                                    }
                                    if ((l).wrapping_add(self.first_count) <= self.half_error_line)
                                    {
                                        {
                                            p = 0i32;
                                            n = (l).wrapping_add(self.first_count);
                                        }
                                    } else {
                                        {
                                            self.print(65557i32);
                                            p = (((l).wrapping_add(self.first_count))
                                                .wrapping_sub(self.half_error_line))
                                            .wrapping_add(3i32);
                                            n = self.half_error_line;
                                        }
                                    }
                                    {
                                        let __for_end_9 = (self.first_count).wrapping_sub(1i32);
                                        q = p;
                                        while q <= __for_end_9 {
                                            self.print_char(
                                                self.trick_buf
                                                    [crate::ix::U((q % self.error_line) as usize)],
                                            );
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    self.print_ln();
                                    {
                                        let __for_end_9 = n;
                                        q = 1i32;
                                        while q <= __for_end_9 {
                                            self.print_raw_char(32i32, true);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) <= self.error_line) {
                                        p = (self.first_count).wrapping_add(m);
                                    } else {
                                        p = (self.first_count).wrapping_add(
                                            ((self.error_line).wrapping_sub(n)).wrapping_sub(3i32),
                                        );
                                    }
                                    {
                                        let __for_end_9 = (p).wrapping_sub(1i32);
                                        q = self.first_count;
                                        while q <= __for_end_9 {
                                            self.print_char(
                                                self.trick_buf
                                                    [crate::ix::U((q % self.error_line) as usize)],
                                            );
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) > self.error_line) {
                                        self.print(65557i32);
                                    }
                                    // §342
                                    nn = (nn).wrapping_add(1i32);
                                }
                            }
                        }
                    } else {
                        // §341
                        if (nn == self.eqtb[crate::ix::U(((7892318i32) - 1) as usize)].int()) {
                            {
                                self.print_nl(65557i32);
                                nn = (nn).wrapping_add(1i32);
                            }
                        }
                    }
                    if bottom_line {
                        break 'l_done_f;
                    }
                    self.base_ptr = (self.base_ptr).wrapping_sub(1i32);
                }
            }
        }
        self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
    }

    /// Here is a procedure that starts a new level of token-list input, given
    /// a token list `p` and its type `t`. If `t=macro`, the calling routine should
    /// set `name` and `loc`.
    // §353
    pub fn begin_token_list(&mut self, mut p: halfword, mut t: quarterword) {
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(65914i32, stack_size);
                    }
                }
            }
            {
                let __ix197 = self.input_ptr;
                let __v198 = self.cur_input;
                self.input_stack[crate::ix::U((__ix197) as usize)] = __v198;
            }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = token_list;
        self.cur_input.start_field = p;
        self.cur_input.index_field = t;
        if (t >= macro_) {
            {
                {
                    let __v199 =
                        (self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_add(1i32);
                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v199);
                }
                if (t == macro_) {
                    self.cur_input.limit_field = self.param_ptr;
                } else {
                    {
                        self.cur_input.loc_field = self.mem[crate::ix::U((p) as usize)].hh().rh();
                        if (self.eqtb[crate::ix::U(((7892294i32) - 1) as usize)].int() > 1i32) {
                            {
                                self.begin_diagnostic();
                                self.print_nl(65626i32);
                                match t {
                                    mark_text => {
                                        self.print_esc(65641i32);
                                    }
                                    write_text => {
                                        self.print_esc(65915i32);
                                    }
                                    _ => {
                                        self.print_cmd_chr(
                                            assign_toks,
                                            (t).wrapping_add(1206289i32),
                                        );
                                    }
                                }
                                self.print(65875i32);
                                self.token_show(p);
                                self.end_diagnostic(false);
                            }
                        }
                    }
                }
            }
        } else {
            self.cur_input.loc_field = p;
        }
    }

    /// When a token list has been fully scanned, the following computations
    /// should be done as we leave that level of input. The `token_type` tends
    /// to be equal to either `backed_up` or `inserted` about 2/3 of the time.
    // §354
    pub fn end_token_list(&mut self) {
        if (self.cur_input.index_field >= backed_up) {
            {
                if (self.cur_input.index_field <= inserted) {
                    self.flush_list(self.cur_input.start_field);
                } else {
                    {
                        self.delete_token_ref(self.cur_input.start_field);
                        if (self.cur_input.index_field == macro_) {
                            while (self.param_ptr > self.cur_input.limit_field) {
                                {
                                    self.param_ptr = (self.param_ptr).wrapping_sub(1i32);
                                    self.flush_list(
                                        self.param_stack[crate::ix::U((self.param_ptr) as usize)],
                                    );
                                }
                            }
                        } else {
                            if ((self.cur_input.index_field == output_text)
                                && (!self.output_can_end))
                            {
                                self.fatal_error(65916i32);
                            }
                        }
                    }
                }
            }
        } else {
            if (self.cur_input.index_field == u_template) {
                if (self.align_state > 500000i32) {
                    self.align_state = 0i32;
                } else {
                    self.fatal_error(65917i32);
                }
            }
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
        }
        {
            if (self.interrupt != 0i32) {
                self.pause_for_instructions();
            }
        }
    }

    /// Sometimes \TeX\ has read too far and wants to ``unscan'' what it has
    /// seen. The `back_input` procedure takes care of this by putting the token
    /// just scanned back into the input stream, ready to be read again. This
    /// procedure can be used only if `cur_tok` represents the token to be
    /// replaced. Some applications of \TeX\ use this procedure a lot,
    /// so it has been slightly optimized for speed.
    // §355
    pub fn back_input(&mut self) {
        let mut p: halfword = 0; // §355
        while (((self.cur_input.loc_field == (268435455i32).wrapping_neg())
            && (self.cur_input.index_field != v_template))
            && (self.cur_input.index_field != output_text))
        {
            self.end_token_list();
        }
        p = self.get_avail();
        {
            let __v200 = self.cur_tok;
            self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v200);
        }
        if (self.cur_tok < right_brace_limit) {
            if (self.cur_tok < left_brace_limit) {
                self.align_state = (self.align_state).wrapping_sub(1i32);
            } else {
                self.align_state = (self.align_state).wrapping_add(1i32);
            }
        }
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(65914i32, stack_size);
                    }
                }
            }
            {
                let __ix201 = self.input_ptr;
                let __v202 = self.cur_input;
                self.input_stack[crate::ix::U((__ix201) as usize)] = __v202;
            }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = token_list;
        self.cur_input.start_field = p;
        self.cur_input.index_field = backed_up;
        self.cur_input.loc_field = p;
    }

    /// The `back_error` routine is used when we want to replace an offending token
    /// just before issuing an error message. This routine, like `back_input`,
    /// requires that `cur_tok` has been set. We disable interrupts during the
    /// call of `back_input` so that the help message won't be lost.
    // §357
    pub fn back_error(&mut self) {
        self.OK_to_interrupt = false;
        self.back_input();
        self.OK_to_interrupt = true;
        self.error();
    }

    /// The `back_error` routine is used when we want to replace an offending token
    /// just before issuing an error message. This routine, like `back_input`,
    /// requires that `cur_tok` has been set. We disable interrupts during the
    /// call of `back_input` so that the help message won't be lost.
    // §357
    pub fn ins_error(&mut self) {
        self.OK_to_interrupt = false;
        self.back_input();
        self.cur_input.index_field = inserted;
        self.OK_to_interrupt = true;
        self.error();
    }

    /// The `begin_file_reading` procedure starts a new level of input for lines
    /// of characters to be read from a file, or as an insertion from the
    /// terminal. It does not take care of opening the file, nor does it set `loc`
    /// or `limit` or `line`.
    // §358
    pub fn begin_file_reading(&mut self) {
        if (self.in_open == max_in_open) {
            self.overflow(65918i32, max_in_open);
        }
        if (self.first == buf_size) {
            self.overflow(65538i32, buf_size);
        }
        self.in_open = (self.in_open).wrapping_add(1i32);
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(65914i32, stack_size);
                    }
                }
            }
            {
                let __ix203 = self.input_ptr;
                let __v204 = self.cur_input;
                self.input_stack[crate::ix::U((__ix203) as usize)] = __v204;
            }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.index_field = self.in_open;
        self.full_source_filename_stack[crate::ix::U((self.cur_input.index_field) as usize)] = 0i32;
        {
            let __ix205 = self.cur_input.index_field;
            let __v206 = false;
            self.eof_seen[crate::ix::U(((__ix205) - 1) as usize)] = __v206;
        }
        {
            let __ix207 = self.cur_input.index_field;
            let __v208 = self.cur_boundary;
            self.grp_stack[crate::ix::U((__ix207) as usize)] = __v208;
        }
        {
            let __ix209 = self.cur_input.index_field;
            let __v210 = self.cond_ptr;
            self.if_stack[crate::ix::U((__ix209) as usize)] = __v210;
        }
        {
            let __ix211 = self.cur_input.index_field;
            let __v212 = self.line;
            self.line_stack[crate::ix::U(((__ix211) - 1) as usize)] = __v212;
        }
        self.cur_input.start_field = self.first;
        self.cur_input.state_field = mid_line;
        self.cur_input.name_field = 0i32;
        // §1706
        self.cur_input.synctex_tag_field = 0i32;
    }

    /// Conversely, the variables must be downdated when such a level of input
    /// is finished:
    // §359
    pub fn end_file_reading(&mut self) {
        self.first = self.cur_input.start_field;
        self.line = self.line_stack[crate::ix::U(((self.cur_input.index_field) - 1) as usize)];
        if ((self.cur_input.name_field == 18i32) || (self.cur_input.name_field == 19i32)) {
            self.pseudo_close();
        } else {
            if (self.cur_input.name_field > 17i32) {
                {
                    let mut __f0 = ::core::mem::take(
                        &mut self.input_file[crate::ix::U((self.cur_input.index_field) as usize)],
                    );
                    let __r = self.u_close(&mut __f0);
                    self.input_file[crate::ix::U((self.cur_input.index_field) as usize)] = __f0;
                    __r
                };
            }
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[crate::ix::U((self.input_ptr) as usize)];
        }
        self.in_open = (self.in_open).wrapping_sub(1i32);
    }

    /// In order to keep the stack from overflowing during a long sequence of
    /// inserted `\.{\\show}' commands, the following routine removes completed
    /// error-inserted lines from memory.
    // §360
    pub fn clear_for_error_prompt(&mut self) {
        while ((((self.cur_input.state_field != token_list)
            && (self.cur_input.name_field == 0i32))
            && (self.input_ptr > 0i32))
            && (self.cur_input.loc_field > self.cur_input.limit_field))
        {
            self.end_file_reading();
        }
        self.print_ln();
        crate::system::break_in(&mut self.term_in, true);
    }

    /// Before getting into `get_next`, let's consider the subroutine that
    /// is called when an `\.{\\outer}' control sequence has been scanned or
    /// when the end of a file has been reached. These two cases are distinguished
    /// by `cur_cs`, which is zero at the end of a file.
    // §366
    pub fn check_outer_validity(&mut self) {
        let mut p: halfword = 0; // §366
        let mut q: halfword = 0; // §366
        if (self.scanner_status != normal) {
            {
                self.deletions_allowed = false;
                // §367
                if (self.cur_cs != 0i32) {
                    {
                        if (((self.cur_input.state_field == token_list)
                            || (self.cur_input.name_field < 1i32))
                            || (self.cur_input.name_field > 17i32))
                        {
                            {
                                p = self.get_avail();
                                {
                                    let __v213 = (cs_token_flag).wrapping_add(self.cur_cs);
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v213);
                                }
                                self.begin_token_list(p, backed_up);
                            }
                        }
                        self.cur_cmd = spacer;
                        self.cur_chr = 32i32;
                    }
                }
                // §366
                if (self.scanner_status > skipping) {
                    // §368
                    {
                        self.runaway();
                        if (self.cur_cs == 0i32) {
                            {
                                if (self.interaction == error_stop_mode) {}
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(65926i32);
                            }
                        } else {
                            {
                                self.cur_cs = 0i32;
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65927i32);
                                }
                            }
                        }
                        self.print(65928i32);
                        // §369
                        p = self.get_avail();
                        match self.scanner_status {
                            defining => {
                                self.print(65889i32);
                                self.mem[crate::ix::U((p) as usize)].set_hh_lh(4194429i32);
                            }
                            matching => {
                                self.print(65934i32);
                                {
                                    let __v214 = self.par_token;
                                    self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v214);
                                }
                                self.long_state = outer_call;
                            }
                            aligning => {
                                self.print(65891i32);
                                self.mem[crate::ix::U((p) as usize)].set_hh_lh(4194429i32);
                                q = p;
                                p = self.get_avail();
                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                self.mem[crate::ix::U((p) as usize)].set_hh_lh(34749082i32);
                                self.align_state = (1000000i32).wrapping_neg();
                            }
                            absorbing => {
                                self.print(65892i32);
                                self.mem[crate::ix::U((p) as usize)].set_hh_lh(4194429i32);
                            }
                            _ => {}
                        }
                        self.begin_token_list(p, inserted);
                        // §368
                        self.print(65929i32);
                        self.sprint_cs(self.warning_index);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[crate::ix::U((3i32) as usize)] = 65930i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 65931i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 65932i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 65933i32;
                        }
                        self.error();
                    }
                } else {
                    // §366
                    {
                        {
                            if (self.interaction == error_stop_mode) {}
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(65920i32);
                        }
                        self.print_cmd_chr(if_test, self.cur_if);
                        self.print(65921i32);
                        self.print_int(self.skip_line);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[crate::ix::U((2i32) as usize)] = 65922i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 65923i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 65924i32;
                        }
                        if (self.cur_cs != 0i32) {
                            self.cur_cs = 0i32;
                        } else {
                            self.help_line[crate::ix::U((2i32) as usize)] = 65925i32;
                        }
                        self.cur_tok = 34749085i32;
                        self.ins_error();
                    }
                }
                self.deletions_allowed = true;
            }
        }
    }

    /// Now we're ready to take the plunge into `get_next` itself. Parts of
    /// this routine are executed more often than any other instructions of \TeX.
    // §371
    pub fn get_next(&mut self) {
        let mut k: i32 = 0; // §371
        let mut t: halfword = 0; // §371
        let mut cat: i32 = 0; // §371
        let mut c: UnicodeScalar = 0; // §371
        let mut lower: UTF16_code = 0; // §371
        let mut d: small_number = 0; // §371
        let mut sup_count: small_number = 0; // §371
                                             // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.cur_cs = 0i32;
                if (self.cur_input.state_field != token_list) {
                    // §373
                    {
                        'l_L25_b: loop {
                            if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                                {
                                    self.cur_chr = self.buffer
                                        [crate::ix::U((self.cur_input.loc_field) as usize)];
                                    self.cur_input.loc_field =
                                        (self.cur_input.loc_field).wrapping_add(1i32);
                                    if (((((self.cur_chr >= 55296i32)
                                        && (self.cur_chr < 56320i32))
                                        && (self.cur_input.loc_field
                                            <= self.cur_input.limit_field))
                                        && (self.buffer
                                            [crate::ix::U((self.cur_input.loc_field) as usize)]
                                            >= 56320i32))
                                        && (self.buffer
                                            [crate::ix::U((self.cur_input.loc_field) as usize)]
                                            < 57344i32))
                                    {
                                        {
                                            lower = (self.buffer[crate::ix::U(
                                                (self.cur_input.loc_field) as usize,
                                            )])
                                            .wrapping_sub(56320i32);
                                            self.cur_input.loc_field =
                                                (self.cur_input.loc_field).wrapping_add(1i32);
                                            self.cur_chr = ((65536i32).wrapping_add(
                                                ((self.cur_chr).wrapping_sub(55296i32))
                                                    .wrapping_mul(1024i32),
                                            ))
                                            .wrapping_add(lower);
                                        }
                                    }
                                    'l_reswitch_b: loop {
                                        self.cur_cmd = self.eqtb[crate::ix::U(
                                            (((cat_code_base).wrapping_add(self.cur_chr)) - 1)
                                                as usize,
                                        )]
                                        .hh()
                                        .rh();
                                        // §374
                                        match (self.cur_input.state_field)
                                            .wrapping_add(self.cur_cmd)
                                        {
                                            10 | 26 | 42 | 27 | 43 => {
                                                continue 'l_L25_b;
                                            }
                                            1 | 17 | 33 => {
                                                // §384
                                                {
                                                    'l_found_f: {
                                                        if (self.cur_input.loc_field
                                                            > self.cur_input.limit_field)
                                                        {
                                                            self.cur_cs = null_cs;
                                                        } else {
                                                            {
                                                                'l_L26_b: loop {
                                                                    k = self.cur_input.loc_field;
                                                                    self.cur_chr = self.buffer
                                                                        [crate::ix::U(
                                                                            (k) as usize,
                                                                        )];
                                                                    cat = self.eqtb[crate::ix::U(
                                                                        (((cat_code_base)
                                                                            .wrapping_add(
                                                                                self.cur_chr,
                                                                            ))
                                                                            - 1)
                                                                            as usize,
                                                                    )]
                                                                    .hh()
                                                                    .rh();
                                                                    k = (k).wrapping_add(1i32);
                                                                    if (cat == letter) {
                                                                        self.cur_input
                                                                            .state_field =
                                                                            skip_blanks;
                                                                    } else {
                                                                        if (cat == spacer) {
                                                                            self.cur_input
                                                                                .state_field =
                                                                                skip_blanks;
                                                                        } else {
                                                                            self.cur_input
                                                                                .state_field =
                                                                                mid_line;
                                                                        }
                                                                    }
                                                                    if ((cat == letter)
                                                                        && (k
                                                                            <= self
                                                                                .cur_input
                                                                                .limit_field))
                                                                    {
                                                                        // §386
                                                                        {
                                                                            loop {
                                                                                self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                                cat = self.eqtb[crate::ix::U((((cat_code_base).wrapping_add(self.cur_chr)) - 1) as usize)].hh().rh();
                                                                                k = (k)
                                                                                    .wrapping_add(
                                                                                        1i32,
                                                                                    );
                                                                                if ((cat != letter) || (k > self.cur_input.limit_field)) { break; }
                                                                            }
                                                                            // §385
                                                                            {
                                                                                if (((cat == sup_mark) && (self.buffer[crate::ix::U((k) as usize)] == self.cur_chr)) && (k < self.cur_input.limit_field)) {
                                                                                    {
                                                                                        sup_count = 2i32;
                                                                                        while (((sup_count < 6i32) && (((k).wrapping_add((2i32).wrapping_mul(sup_count))).wrapping_sub(2i32) <= self.cur_input.limit_field)) && (self.buffer[crate::ix::U((((k).wrapping_add(sup_count)).wrapping_sub(1i32)) as usize)] == self.cur_chr)) {
                                                                                            sup_count = (sup_count).wrapping_add(1i32);
                                                                                        }
                                                                                        {
                                                                                            let __for_end_22 = sup_count;
                                                                                            d = 1i32;
                                                                                            while d <= __for_end_22 {
                                                                                                if (!(((self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 48i32) && (self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 57i32)) || ((self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 97i32) && (self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 102i32)))) {
                                                                                                    {
                                                                                                        c = self.buffer[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
                                                                                                        if (c < 128i32) {
                                                                                                            {
                                                                                                                if (c < 64i32) {
                                                                                                                    self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_add(64i32);
                                                                                                                } else {
                                                                                                                    self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_sub(64i32);
                                                                                                                }
                                                                                                                d = 2i32;
                                                                                                                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                                while (k <= self.cur_input.limit_field) {
                                                                                                                    {
                                                                                                                        { let __v215 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v215; }
                                                                                                                        k = (k).wrapping_add(1i32);
                                                                                                                    }
                                                                                                                }
                                                                                                                continue 'l_L26_b;
                                                                                                            }
                                                                                                        } else {
                                                                                                            sup_count = 0i32;
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                d = d.wrapping_add(1);
                                                                                            }
                                                                                        }
                                                                                        if (sup_count > 0i32) {
                                                                                            {
                                                                                                self.cur_chr = 0i32;
                                                                                                {
                                                                                                    let __for_end_24 = sup_count;
                                                                                                    d = 1i32;
                                                                                                    while d <= __for_end_24 {
                                                                                                        {
                                                                                                            c = self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)];
                                                                                                            if (c <= 57i32) {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(48i32);
                                                                                                            } else {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(87i32);
                                                                                                            }
                                                                                                        }
                                                                                                        d = d.wrapping_add(1);
                                                                                                    }
                                                                                                }
                                                                                                if (self.cur_chr > biggest_usv) {
                                                                                                    self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                                                } else {
                                                                                                    {
                                                                                                        { let __v216 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v216; }
                                                                                                        d = ((2i32).wrapping_mul(sup_count)).wrapping_sub(1i32);
                                                                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                        while (k <= self.cur_input.limit_field) {
                                                                                                            {
                                                                                                                { let __v217 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v217; }
                                                                                                                k = (k).wrapping_add(1i32);
                                                                                                            }
                                                                                                        }
                                                                                                        continue 'l_L26_b;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            // §386
                                                                            if (cat != letter) {
                                                                                k = (k)
                                                                                    .wrapping_sub(
                                                                                        1i32,
                                                                                    );
                                                                            }
                                                                            if (k
                                                                                > (self
                                                                                    .cur_input
                                                                                    .loc_field)
                                                                                    .wrapping_add(
                                                                                        1i32,
                                                                                    ))
                                                                            {
                                                                                {
                                                                                    self.cur_cs = self.id_lookup(self.cur_input.loc_field, (k).wrapping_sub(self.cur_input.loc_field));
                                                                                    self.cur_input.loc_field = k;
                                                                                    break 'l_found_f;
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        // §385
                                                                        {
                                                                            if (((cat == sup_mark) && (self.buffer[crate::ix::U((k) as usize)] == self.cur_chr)) && (k < self.cur_input.limit_field)) {
                                                                                {
                                                                                    sup_count = 2i32;
                                                                                    while (((sup_count < 6i32) && (((k).wrapping_add((2i32).wrapping_mul(sup_count))).wrapping_sub(2i32) <= self.cur_input.limit_field)) && (self.buffer[crate::ix::U((((k).wrapping_add(sup_count)).wrapping_sub(1i32)) as usize)] == self.cur_chr)) {
                                                                                        sup_count = (sup_count).wrapping_add(1i32);
                                                                                    }
                                                                                    {
                                                                                        let __for_end_21 = sup_count;
                                                                                        d = 1i32;
                                                                                        while d <= __for_end_21 {
                                                                                            if (!(((self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 48i32) && (self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 57i32)) || ((self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 97i32) && (self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 102i32)))) {
                                                                                                {
                                                                                                    c = self.buffer[crate::ix::U(((k).wrapping_add(1i32)) as usize)];
                                                                                                    if (c < 128i32) {
                                                                                                        {
                                                                                                            if (c < 64i32) {
                                                                                                                self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_add(64i32);
                                                                                                            } else {
                                                                                                                self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = (c).wrapping_sub(64i32);
                                                                                                            }
                                                                                                            d = 2i32;
                                                                                                            self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                            while (k <= self.cur_input.limit_field) {
                                                                                                                {
                                                                                                                    { let __v218 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v218; }
                                                                                                                    k = (k).wrapping_add(1i32);
                                                                                                                }
                                                                                                            }
                                                                                                            continue 'l_L26_b;
                                                                                                        }
                                                                                                    } else {
                                                                                                        sup_count = 0i32;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                            d = d.wrapping_add(1);
                                                                                        }
                                                                                    }
                                                                                    if (sup_count > 0i32) {
                                                                                        {
                                                                                            self.cur_chr = 0i32;
                                                                                            {
                                                                                                let __for_end_23 = sup_count;
                                                                                                d = 1i32;
                                                                                                while d <= __for_end_23 {
                                                                                                    {
                                                                                                        c = self.buffer[crate::ix::U(((((k).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)];
                                                                                                        if (c <= 57i32) {
                                                                                                            self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(48i32);
                                                                                                        } else {
                                                                                                            self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(87i32);
                                                                                                        }
                                                                                                    }
                                                                                                    d = d.wrapping_add(1);
                                                                                                }
                                                                                            }
                                                                                            if (self.cur_chr > biggest_usv) {
                                                                                                self.cur_chr = self.buffer[crate::ix::U((k) as usize)];
                                                                                            } else {
                                                                                                {
                                                                                                    { let __v219 = self.cur_chr; self.buffer[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] = __v219; }
                                                                                                    d = ((2i32).wrapping_mul(sup_count)).wrapping_sub(1i32);
                                                                                                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                    while (k <= self.cur_input.limit_field) {
                                                                                                        {
                                                                                                            { let __v220 = self.buffer[crate::ix::U(((k).wrapping_add(d)) as usize)]; self.buffer[crate::ix::U((k) as usize)] = __v220; }
                                                                                                            k = (k).wrapping_add(1i32);
                                                                                                        }
                                                                                                    }
                                                                                                    continue 'l_L26_b;
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    // §384
                                                                    if (self.buffer[crate::ix::U(
                                                                        (self.cur_input.loc_field)
                                                                            as usize,
                                                                    )] > 65535i32)
                                                                    {
                                                                        {
                                                                            self.cur_cs = self
                                                                                .id_lookup(
                                                                                    self.cur_input
                                                                                        .loc_field,
                                                                                    1i32,
                                                                                );
                                                                            self.cur_input
                                                                                .loc_field = (self
                                                                                .cur_input
                                                                                .loc_field)
                                                                                .wrapping_add(1i32);
                                                                            break 'l_found_f;
                                                                        }
                                                                    }
                                                                    self.cur_cs = (single_base)
                                                                        .wrapping_add(
                                                                            self.buffer
                                                                                [crate::ix::U(
                                                                                    (self
                                                                                        .cur_input
                                                                                        .loc_field)
                                                                                        as usize,
                                                                                )],
                                                                        );
                                                                    self.cur_input.loc_field =
                                                                        (self.cur_input.loc_field)
                                                                            .wrapping_add(1i32);
                                                                    break 'l_L26_b;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_cmd = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .rh();
                                                    if (self.cur_cmd >= outer_call) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            14 | 30 | 46 => {
                                                // §383
                                                {
                                                    self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                                    self.cur_cmd = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .rh();
                                                    self.cur_input.state_field = mid_line;
                                                    if (self.cur_cmd >= outer_call) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            8 | 24 | 40 => {
                                                // §382
                                                {
                                                    'l_L27_f: {
                                                        if (self.cur_chr
                                                            == self.buffer[crate::ix::U(
                                                                (self.cur_input.loc_field) as usize,
                                                            )])
                                                        {
                                                            if (self.cur_input.loc_field
                                                                < self.cur_input.limit_field)
                                                            {
                                                                {
                                                                    sup_count = 2i32;
                                                                    while (((sup_count < 6i32) && (((self.cur_input.loc_field).wrapping_add((2i32).wrapping_mul(sup_count))).wrapping_sub(2i32) <= self.cur_input.limit_field)) && (self.cur_chr == self.buffer[crate::ix::U((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(1i32)) as usize)])) {
                                                                        sup_count = (sup_count).wrapping_add(1i32);
                                                                    }
                                                                    {
                                                                        let __for_end_17 =
                                                                            sup_count;
                                                                        d = 1i32;
                                                                        while d <= __for_end_17 {
                                                                            if (!(((self.buffer[crate::ix::U(((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 48i32) && (self.buffer[crate::ix::U(((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 57i32)) || ((self.buffer[crate::ix::U(((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] >= 97i32) && (self.buffer[crate::ix::U(((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)] <= 102i32)))) {
                                                                                {
                                                                                    c = self.buffer[crate::ix::U(((self.cur_input.loc_field).wrapping_add(1i32)) as usize)];
                                                                                    if (c < 128i32) {
                                                                                        {
                                                                                            self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(2i32);
                                                                                            if (c < 64i32) {
                                                                                                self.cur_chr = (c).wrapping_add(64i32);
                                                                                            } else {
                                                                                                self.cur_chr = (c).wrapping_sub(64i32);
                                                                                            }
                                                                                            continue 'l_reswitch_b;
                                                                                        }
                                                                                    }
                                                                                    break 'l_L27_f;
                                                                                }
                                                                            }
                                                                            d = d.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    self.cur_chr = 0i32;
                                                                    {
                                                                        let __for_end_17 =
                                                                            sup_count;
                                                                        d = 1i32;
                                                                        while d <= __for_end_17 {
                                                                            {
                                                                                c = self.buffer[crate::ix::U(((((self.cur_input.loc_field).wrapping_add(sup_count)).wrapping_sub(2i32)).wrapping_add(d)) as usize)];
                                                                                if (c <= 57i32) {
                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(48i32);
                                                                                } else {
                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(c)).wrapping_sub(87i32);
                                                                                }
                                                                            }
                                                                            d = d.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    if (self.cur_chr > biggest_usv)
                                                                    {
                                                                        {
                                                                            self.cur_chr = self
                                                                                .buffer
                                                                                [crate::ix::U(
                                                                                    (self
                                                                                        .cur_input
                                                                                        .loc_field)
                                                                                        as usize,
                                                                                )];
                                                                            break 'l_L27_f;
                                                                        }
                                                                    }
                                                                    self.cur_input.loc_field =
                                                                        ((self
                                                                            .cur_input
                                                                            .loc_field)
                                                                            .wrapping_add(
                                                                                (2i32)
                                                                                    .wrapping_mul(
                                                                                        sup_count,
                                                                                    ),
                                                                            ))
                                                                        .wrapping_sub(1i32);
                                                                    continue 'l_reswitch_b;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_input.state_field = mid_line;
                                                }
                                            }
                                            16 | 32 | 48 => {
                                                // §376
                                                {
                                                    {
                                                        if (self.interaction == error_stop_mode) {}
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(65544i32);
                                                        }
                                                        self.print(65935i32);
                                                    }
                                                    {
                                                        self.help_ptr = 2i32;
                                                        self.help_line
                                                            [crate::ix::U((1i32) as usize)] =
                                                            65936i32;
                                                        self.help_line
                                                            [crate::ix::U((0i32) as usize)] =
                                                            65937i32;
                                                    }
                                                    self.deletions_allowed = false;
                                                    self.error();
                                                    self.deletions_allowed = true;
                                                    {
                                                        __goto_1 = 0;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                            11 => {
                                                // §379
                                                {
                                                    self.cur_input.state_field = skip_blanks;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            6 => {
                                                // §378
                                                {
                                                    self.cur_input.loc_field =
                                                        (self.cur_input.limit_field)
                                                            .wrapping_add(1i32);
                                                    self.cur_cmd = spacer;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            22 | 15 | 31 | 47 => {
                                                // §380
                                                {
                                                    self.cur_input.loc_field =
                                                        (self.cur_input.limit_field)
                                                            .wrapping_add(1i32);
                                                    continue 'l_L25_b;
                                                }
                                            }
                                            38 => {
                                                // §381
                                                {
                                                    self.cur_input.loc_field =
                                                        (self.cur_input.limit_field)
                                                            .wrapping_add(1i32);
                                                    self.cur_cs = self.par_loc;
                                                    self.cur_cmd = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .b0();
                                                    self.cur_chr = self.eqtb[crate::ix::U(
                                                        ((self.cur_cs) - 1) as usize,
                                                    )]
                                                    .hh()
                                                    .rh();
                                                    if (self.cur_cmd >= outer_call) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            2 => {
                                                // §377
                                                self.align_state =
                                                    (self.align_state).wrapping_add(1i32);
                                            }
                                            18 | 34 => {
                                                self.cur_input.state_field = mid_line;
                                                self.align_state =
                                                    (self.align_state).wrapping_add(1i32);
                                            }
                                            3 => {
                                                self.align_state =
                                                    (self.align_state).wrapping_sub(1i32);
                                            }
                                            19 | 35 => {
                                                self.cur_input.state_field = mid_line;
                                                self.align_state =
                                                    (self.align_state).wrapping_sub(1i32);
                                            }
                                            20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41
                                            | 44 | 45 => {
                                                self.cur_input.state_field = mid_line;
                                            }
                                            _ => {
                                                // §374
                                            }
                                        }
                                        break 'l_reswitch_b;
                                    }
                                }
                            } else {
                                // §373
                                {
                                    self.cur_input.state_field = new_line;
                                    // §390
                                    if (self.cur_input.name_field > 17i32) {
                                        // §392
                                        {
                                            self.line = (self.line).wrapping_add(1i32);
                                            self.first = self.cur_input.start_field;
                                            if (!self.force_eof) {
                                                if (self.cur_input.name_field <= 19i32) {
                                                    {
                                                        if self.pseudo_input() {
                                                            self.firm_up_the_line();
                                                        } else {
                                                            if ((self.eqtb[crate::ix::U(
                                                                ((every_eof_loc) - 1) as usize,
                                                            )]
                                                            .hh()
                                                            .rh()
                                                                != (268435455i32).wrapping_neg())
                                                                && (!self.eof_seen[crate::ix::U(
                                                                    ((self.cur_input.index_field)
                                                                        - 1)
                                                                        as usize,
                                                                )]))
                                                            {
                                                                {
                                                                    self.cur_input.limit_field =
                                                                        (self.first)
                                                                            .wrapping_sub(1i32);
                                                                    {
                                                                        let __ix221 = self
                                                                            .cur_input
                                                                            .index_field;
                                                                        let __v222 = true;
                                                                        self.eof_seen
                                                                            [crate::ix::U(
                                                                                ((__ix221) - 1)
                                                                                    as usize,
                                                                            )] = __v222;
                                                                    }
                                                                    self.begin_token_list(
                                                                        self.eqtb[crate::ix::U(
                                                                            ((every_eof_loc) - 1)
                                                                                as usize,
                                                                        )]
                                                                        .hh()
                                                                        .rh(),
                                                                        every_eof_text,
                                                                    );
                                                                    {
                                                                        __goto_1 = 0;
                                                                        continue 'l_dispatch_1;
                                                                    }
                                                                }
                                                            } else {
                                                                self.force_eof = true;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        if {
                                                            let mut __f0 = ::core::mem::take(
                                                                &mut self.input_file[crate::ix::U(
                                                                    (self.cur_input.index_field)
                                                                        as usize,
                                                                )],
                                                            );
                                                            let __r =
                                                                self.input_ln(&mut __f0, true);
                                                            self.input_file[crate::ix::U(
                                                                (self.cur_input.index_field)
                                                                    as usize,
                                                            )] = __f0;
                                                            __r
                                                        } {
                                                            self.firm_up_the_line();
                                                        } else {
                                                            if ((self.eqtb[crate::ix::U(
                                                                ((every_eof_loc) - 1) as usize,
                                                            )]
                                                            .hh()
                                                            .rh()
                                                                != (268435455i32).wrapping_neg())
                                                                && (!self.eof_seen[crate::ix::U(
                                                                    ((self.cur_input.index_field)
                                                                        - 1)
                                                                        as usize,
                                                                )]))
                                                            {
                                                                {
                                                                    self.cur_input.limit_field =
                                                                        (self.first)
                                                                            .wrapping_sub(1i32);
                                                                    {
                                                                        let __ix223 = self
                                                                            .cur_input
                                                                            .index_field;
                                                                        let __v224 = true;
                                                                        self.eof_seen
                                                                            [crate::ix::U(
                                                                                ((__ix223) - 1)
                                                                                    as usize,
                                                                            )] = __v224;
                                                                    }
                                                                    self.begin_token_list(
                                                                        self.eqtb[crate::ix::U(
                                                                            ((every_eof_loc) - 1)
                                                                                as usize,
                                                                        )]
                                                                        .hh()
                                                                        .rh(),
                                                                        every_eof_text,
                                                                    );
                                                                    {
                                                                        __goto_1 = 0;
                                                                        continue 'l_dispatch_1;
                                                                    }
                                                                }
                                                            } else {
                                                                self.force_eof = true;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if self.force_eof {
                                                {
                                                    if (self.eqtb
                                                        [crate::ix::U(((7892329i32) - 1) as usize)]
                                                    .int()
                                                        > 0i32)
                                                    {
                                                        if ((self.grp_stack[crate::ix::U(
                                                            (self.in_open) as usize,
                                                        )] != self.cur_boundary)
                                                            || (self.if_stack[crate::ix::U(
                                                                (self.in_open) as usize,
                                                            )] != self.cond_ptr))
                                                        {
                                                            self.file_warning();
                                                        }
                                                    }
                                                    if (self.cur_input.name_field >= 19i32) {
                                                        {
                                                            self.print_char(41i32);
                                                            self.open_parens = (self.open_parens)
                                                                .wrapping_sub(1i32);
                                                            crate::system::break_out(
                                                                &mut self.term_out,
                                                            );
                                                        }
                                                    }
                                                    self.force_eof = false;
                                                    self.end_file_reading();
                                                    self.check_outer_validity();
                                                    {
                                                        __goto_1 = 0;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                            if ((self.eqtb
                                                [crate::ix::U(((7892312i32) - 1) as usize)]
                                            .int()
                                                < 0i32)
                                                || (self.eqtb
                                                    [crate::ix::U(((7892312i32) - 1) as usize)]
                                                .int()
                                                    > 255i32))
                                            {
                                                self.cur_input.limit_field =
                                                    (self.cur_input.limit_field).wrapping_sub(1i32);
                                            } else {
                                                {
                                                    let __ix225 = self.cur_input.limit_field;
                                                    let __v226 = self.eqtb
                                                        [crate::ix::U(((7892312i32) - 1) as usize)]
                                                    .int();
                                                    self.buffer[crate::ix::U((__ix225) as usize)] =
                                                        __v226;
                                                }
                                            }
                                            self.first =
                                                (self.cur_input.limit_field).wrapping_add(1i32);
                                            self.cur_input.loc_field = self.cur_input.start_field;
                                        }
                                    } else {
                                        // §390
                                        {
                                            if (!(self.cur_input.name_field == 0i32)) {
                                                {
                                                    self.cur_cmd = 0i32;
                                                    self.cur_chr = 0i32;
                                                    {
                                                        __goto_1 = 1;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                            if (self.input_ptr > 0i32) {
                                                {
                                                    self.end_file_reading();
                                                    {
                                                        __goto_1 = 0;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                            if (self.selector < log_only) {
                                                self.open_log_file();
                                            }
                                            if (self.interaction > nonstop_mode) {
                                                {
                                                    if ((self.eqtb[crate::ix::U(
                                                        ((7892312i32) - 1) as usize,
                                                    )]
                                                    .int()
                                                        < 0i32)
                                                        || (self.eqtb[crate::ix::U(
                                                            ((7892312i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 255i32))
                                                    {
                                                        self.cur_input.limit_field =
                                                            (self.cur_input.limit_field)
                                                                .wrapping_add(1i32);
                                                    }
                                                    if (self.cur_input.limit_field
                                                        == self.cur_input.start_field)
                                                    {
                                                        self.print_nl(65939i32);
                                                    }
                                                    self.print_ln();
                                                    self.first = self.cur_input.start_field;
                                                    {
                                                        self.print(42i32);
                                                        self.term_input();
                                                    }
                                                    self.cur_input.limit_field = self.last;
                                                    if ((self.eqtb[crate::ix::U(
                                                        ((7892312i32) - 1) as usize,
                                                    )]
                                                    .int()
                                                        < 0i32)
                                                        || (self.eqtb[crate::ix::U(
                                                            ((7892312i32) - 1) as usize,
                                                        )]
                                                        .int()
                                                            > 255i32))
                                                    {
                                                        self.cur_input.limit_field =
                                                            (self.cur_input.limit_field)
                                                                .wrapping_sub(1i32);
                                                    } else {
                                                        {
                                                            let __ix227 =
                                                                self.cur_input.limit_field;
                                                            let __v228 = self.eqtb[crate::ix::U(
                                                                ((7892312i32) - 1) as usize,
                                                            )]
                                                            .int();
                                                            self.buffer[crate::ix::U(
                                                                (__ix227) as usize,
                                                            )] = __v228;
                                                        }
                                                    }
                                                    self.first = (self.cur_input.limit_field)
                                                        .wrapping_add(1i32);
                                                    self.cur_input.loc_field =
                                                        self.cur_input.start_field;
                                                }
                                            } else {
                                                self.fatal_error(65940i32);
                                            }
                                        }
                                    }
                                    // §373
                                    {
                                        if (self.interrupt != 0i32) {
                                            self.pause_for_instructions();
                                        }
                                    }
                                    continue 'l_L25_b;
                                }
                            }
                            break 'l_L25_b;
                        }
                    }
                } else {
                    // §387
                    if (self.cur_input.loc_field != (268435455i32).wrapping_neg()) {
                        {
                            t = self.mem[crate::ix::U((self.cur_input.loc_field) as usize)]
                                .hh()
                                .lh();
                            self.cur_input.loc_field = self.mem
                                [crate::ix::U((self.cur_input.loc_field) as usize)]
                            .hh()
                            .rh();
                            if (t >= cs_token_flag) {
                                {
                                    self.cur_cs = (t).wrapping_sub(33554431i32);
                                    self.cur_cmd = self.eqtb
                                        [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                    .hh()
                                    .b0();
                                    self.cur_chr = self.eqtb
                                        [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                    .hh()
                                    .rh();
                                    if (self.cur_cmd >= outer_call) {
                                        if (self.cur_cmd == dont_expand) {
                                            // §388
                                            {
                                                self.cur_cs = (self.mem[crate::ix::U(
                                                    (self.cur_input.loc_field) as usize,
                                                )]
                                                .hh()
                                                .lh())
                                                .wrapping_sub(33554431i32);
                                                self.cur_input.loc_field =
                                                    (268435455i32).wrapping_neg();
                                                self.cur_cmd = self.eqtb
                                                    [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                                .hh()
                                                .b0();
                                                self.cur_chr = self.eqtb
                                                    [crate::ix::U(((self.cur_cs) - 1) as usize)]
                                                .hh()
                                                .rh();
                                                if (self.cur_cmd > max_command) {
                                                    {
                                                        self.cur_cmd = relax;
                                                        self.cur_chr = no_expand_flag;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §387
                                            {
                                                if ((self.cur_cs == end_write)
                                                    && (self.cur_list.mode_field == 0i32))
                                                {
                                                    self.fatal_error(65938i32);
                                                }
                                                self.check_outer_validity();
                                            }
                                        }
                                    }
                                }
                            } else {
                                {
                                    self.cur_cmd = (t / max_char_val);
                                    self.cur_chr = (t % max_char_val);
                                    match self.cur_cmd {
                                        left_brace => {
                                            self.align_state =
                                                (self.align_state).wrapping_add(1i32);
                                        }
                                        right_brace => {
                                            self.align_state =
                                                (self.align_state).wrapping_sub(1i32);
                                        }
                                        out_param => {
                                            // §389
                                            {
                                                self.begin_token_list(
                                                    self.param_stack[crate::ix::U(
                                                        (((self.cur_input.limit_field)
                                                            .wrapping_add(self.cur_chr))
                                                        .wrapping_sub(1i32))
                                                            as usize,
                                                    )],
                                                    parameter,
                                                );
                                                {
                                                    __goto_1 = 0;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        }
                                        _ => {
                                            // §387
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        {
                            self.end_token_list();
                            {
                                __goto_1 = 0;
                                continue 'l_dispatch_1;
                            }
                        }
                    }
                }
                // §372
                if (self.cur_cmd <= car_ret) {
                    if (self.cur_cmd >= tab_mark) {
                        if (self.align_state == 0i32) {
                            // §837
                            {
                                if ((self.scanner_status == aligning)
                                    || (self.cur_align == (268435455i32).wrapping_neg()))
                                {
                                    self.fatal_error(65917i32);
                                }
                                self.cur_cmd = self.mem
                                    [crate::ix::U(((self.cur_align).wrapping_add(5i32)) as usize)]
                                .hh()
                                .lh();
                                {
                                    let __ix229 = (self.cur_align).wrapping_add(5i32);
                                    let __v230 = self.cur_chr;
                                    self.mem[crate::ix::U((__ix229) as usize)].set_hh_lh(__v230);
                                }
                                if (self.cur_cmd == omit) {
                                    self.begin_token_list(omit_template, v_template);
                                } else {
                                    self.begin_token_list(
                                        self.mem[crate::ix::U(
                                            ((self.cur_align).wrapping_add(2i32)) as usize,
                                        )]
                                        .int(),
                                        v_template,
                                    );
                                }
                                self.align_state = 1000000i32;
                                {
                                    __goto_1 = 0;
                                    continue 'l_dispatch_1;
                                }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // exit
                 // §371
            }
            break 'l_dispatch_1;
        }
    }

    /// If the user has set the `pausing` parameter to some positive value,
    /// and if nonstop mode has not been selected, each line of input is displayed
    /// on the terminal and the transcript file, followed by `\.{=>}'.
    /// \TeX\ waits for a response. If the response is simply `carriage_return`, the
    /// line is accepted as it stands, otherwise the line typed is
    /// used instead of the line in the file.
    // §393
    pub fn firm_up_the_line(&mut self) {
        let mut k: i32 = 0; // §393
        self.cur_input.limit_field = self.last;
        if (self.eqtb[crate::ix::U(((7892292i32) - 1) as usize)].int() > 0i32) {
            if (self.interaction > nonstop_mode) {
                {
                    self.print_ln();
                    if (self.cur_input.start_field < self.cur_input.limit_field) {
                        {
                            let __for_end_6 = (self.cur_input.limit_field).wrapping_sub(1i32);
                            k = self.cur_input.start_field;
                            while k <= __for_end_6 {
                                self.print(self.buffer[crate::ix::U((k) as usize)]);
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                    self.first = self.cur_input.limit_field;
                    {
                        self.print(65941i32);
                        self.term_input();
                    }
                    if (self.last > self.first) {
                        {
                            {
                                let __for_end_7 = (self.last).wrapping_sub(1i32);
                                k = self.first;
                                while k <= __for_end_7 {
                                    {
                                        let __ix231 = ((k)
                                            .wrapping_add(self.cur_input.start_field))
                                        .wrapping_sub(self.first);
                                        let __v232 = self.buffer[crate::ix::U((k) as usize)];
                                        self.buffer[crate::ix::U((__ix231) as usize)] = __v232;
                                    }
                                    k = k.wrapping_add(1);
                                }
                            }
                            self.cur_input.limit_field = ((self.cur_input.start_field)
                                .wrapping_add(self.last))
                            .wrapping_sub(self.first);
                        }
                    }
                }
            }
        }
    }

    /// No new control sequences will be defined except during a call of
    /// `get_token`, or when \.{\\csname} compresses a token list, because
    /// `no_new_control_sequence` is always `true` at other times.
    // §395
    pub fn get_token(&mut self) {
        self.no_new_control_sequence = false;
        self.get_next();
        self.no_new_control_sequence = true;
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(max_char_val)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
        }
    }

    /// After parameter scanning is complete, the parameters are moved to the
    /// `param_stack`. Then the macro body is fed to the scanner; in other words,
    /// `macro_call` places the defined text of the control sequence at the
    /// top of\/ \TeX's input stack, so that `get_next` will proceed to read it
    /// next.
    /// The global variable `cur_cs` contains the `eqtb` address of the control sequence
    /// being expanded, when `macro_call` begins. If this control sequence has not been
    /// declared \.{\\long}, i.e., if its command code in the `eq_type` field is
    /// not `long_call` or `long_outer_call`, its parameters are not allowed to contain
    /// the control sequence \.{\\par}. If an illegal \.{\\par} appears, the macro
    /// call is aborted, and the \.{\\par} will be rescanned.
    /// @<Declare the procedure called `macro_call`
    // §423
    pub fn macro_call(&mut self) {
        let mut r: halfword = 0; // §423
        let mut p: halfword = 0; // §423
        let mut q: halfword = 0; // §423
        let mut s: halfword = 0; // §423
        let mut t: halfword = 0; // §423
        let mut u: halfword = 0; // §423
        let mut v: halfword = 0; // §423
        let mut rbrace_ptr: halfword = 0; // §423
        let mut n: small_number = 0; // §423
        let mut unbalance: halfword = 0; // §423
        let mut m: halfword = 0; // §423
        let mut ref_count: halfword = 0; // §423
        let mut save_scanner_status: small_number = 0; // §423
        let mut save_warning_index: halfword = 0; // §423
        let mut match_chr: UTF16_code = 0; // §423
        'l_exit_f: {
            save_scanner_status = self.scanner_status;
            save_warning_index = self.warning_index;
            self.warning_index = self.cur_cs;
            ref_count = self.cur_chr;
            r = self.mem[crate::ix::U((ref_count) as usize)].hh().rh();
            n = 0i32;
            if (self.eqtb[crate::ix::U(((7892294i32) - 1) as usize)].int() > 0i32) {
                // §435
                {
                    self.begin_diagnostic();
                    if (self.eqtb[crate::ix::U(((7892322i32) - 1) as usize)].int() > 0i32) {
                        if (self.input_ptr
                            < self.eqtb[crate::ix::U(((7892322i32) - 1) as usize)].int())
                        {
                            {
                                v = self.input_ptr;
                                self.print_ln();
                                self.print_char(126i32);
                                while (v > 0i32) {
                                    {
                                        self.print_char(46i32);
                                        v = (v).wrapping_sub(1i32);
                                    }
                                }
                                self.print_cs(self.warning_index);
                                self.token_show(ref_count);
                            }
                        } else {
                            {
                                self.print_char(126i32);
                                self.print_char(126i32);
                                self.print_cs(self.warning_index);
                            }
                        }
                    } else {
                        {
                            self.print_ln();
                            self.print_cs(self.warning_index);
                            self.token_show(ref_count);
                        }
                    }
                    self.end_diagnostic(false);
                }
            }
            // §423
            if (self.mem[crate::ix::U((r) as usize)].hh().lh() == protected_token) {
                r = self.mem[crate::ix::U((r) as usize)].hh().rh();
            }
            if (self.mem[crate::ix::U((r) as usize)].hh().lh() != end_match_token) {
                // §425
                {
                    self.scanner_status = matching;
                    unbalance = 0i32;
                    self.long_state = self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)]
                        .hh()
                        .b0();
                    if (self.long_state >= outer_call) {
                        self.long_state = (self.long_state).wrapping_sub(2i32);
                    }
                    loop {
                        // goto labels: continue, found
                        let mut __goto_1: i32 = 0;
                        'l_dispatch_1: loop {
                            if __goto_1 <= 0 {
                                self.mem[crate::ix::U((temp_head) as usize)]
                                    .set_hh_rh((268435455i32).wrapping_neg());
                                if ((self.mem[crate::ix::U((r) as usize)].hh().lh()
                                    >= end_match_token)
                                    || (self.mem[crate::ix::U((r) as usize)].hh().lh()
                                        < match_token))
                                {
                                    s = (268435455i32).wrapping_neg();
                                } else {
                                    {
                                        match_chr =
                                            (self.mem[crate::ix::U((r) as usize)].hh().lh())
                                                .wrapping_sub(27262976i32);
                                        s = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                        r = s;
                                        p = temp_head;
                                        m = 0i32;
                                    }
                                }
                            }
                            if __goto_1 <= 1 {
                                // continue
                                // §426
                                self.get_token();
                                if (self.cur_tok == self.mem[crate::ix::U((r) as usize)].hh().lh())
                                {
                                    // §428
                                    {
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                        if ((self.mem[crate::ix::U((r) as usize)].hh().lh()
                                            >= match_token)
                                            && (self.mem[crate::ix::U((r) as usize)].hh().lh()
                                                <= end_match_token))
                                        {
                                            {
                                                if (self.cur_tok < left_brace_limit) {
                                                    self.align_state =
                                                        (self.align_state).wrapping_sub(1i32);
                                                }
                                                {
                                                    __goto_1 = 2;
                                                    continue 'l_dispatch_1;
                                                }
                                            }
                                        } else {
                                            {
                                                __goto_1 = 1;
                                                continue 'l_dispatch_1;
                                            }
                                        }
                                    }
                                }
                                // §431
                                if (s != r) {
                                    if (s == (268435455i32).wrapping_neg()) {
                                        // §432
                                        {
                                            {
                                                if (self.interaction == error_stop_mode) {}
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(65974i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(65975i32);
                                            {
                                                self.help_ptr = 4i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] =
                                                    65976i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] =
                                                    65977i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] =
                                                    65978i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] =
                                                    65979i32;
                                            }
                                            self.error();
                                            break 'l_exit_f;
                                        }
                                    } else {
                                        // §431
                                        {
                                            t = s;
                                            loop {
                                                'l_done_f: {
                                                    {
                                                        q = self.get_avail();
                                                        self.mem[crate::ix::U((p) as usize)]
                                                            .set_hh_rh(q);
                                                        {
                                                            let __v233 = self.mem
                                                                [crate::ix::U((t) as usize)]
                                                            .hh()
                                                            .lh();
                                                            self.mem[crate::ix::U((q) as usize)]
                                                                .set_hh_lh(__v233);
                                                        }
                                                        p = q;
                                                    }
                                                    m = (m).wrapping_add(1i32);
                                                    u = self.mem[crate::ix::U((t) as usize)]
                                                        .hh()
                                                        .rh();
                                                    v = s;
                                                    while true {
                                                        {
                                                            if (u == r) {
                                                                if (self.cur_tok
                                                                    != self.mem[crate::ix::U(
                                                                        (v) as usize,
                                                                    )]
                                                                    .hh()
                                                                    .lh())
                                                                {
                                                                    break 'l_done_f;
                                                                } else {
                                                                    {
                                                                        r = self.mem[crate::ix::U(
                                                                            (v) as usize,
                                                                        )]
                                                                        .hh()
                                                                        .rh();
                                                                        {
                                                                            __goto_1 = 1;
                                                                            continue 'l_dispatch_1;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if (self.mem
                                                                [crate::ix::U((u) as usize)]
                                                            .hh()
                                                            .lh()
                                                                != self.mem
                                                                    [crate::ix::U((v) as usize)]
                                                                .hh()
                                                                .lh())
                                                            {
                                                                break 'l_done_f;
                                                            }
                                                            u = self.mem
                                                                [crate::ix::U((u) as usize)]
                                                            .hh()
                                                            .rh();
                                                            v = self.mem
                                                                [crate::ix::U((v) as usize)]
                                                            .hh()
                                                            .rh();
                                                        }
                                                    }
                                                }
                                                t = self.mem[crate::ix::U((t) as usize)].hh().rh();
                                                if (t == r) {
                                                    break;
                                                }
                                            }
                                            r = s;
                                        }
                                    }
                                }
                                // §426
                                if (self.cur_tok == self.par_token) {
                                    if (self.long_state != long_call) {
                                        // §430
                                        {
                                            if (self.long_state == call) {
                                                {
                                                    self.runaway();
                                                    {
                                                        if (self.interaction == error_stop_mode) {}
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(65544i32);
                                                        }
                                                        self.print(65969i32);
                                                    }
                                                    self.sprint_cs(self.warning_index);
                                                    self.print(65970i32);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line
                                                            [crate::ix::U((2i32) as usize)] =
                                                            65971i32;
                                                        self.help_line
                                                            [crate::ix::U((1i32) as usize)] =
                                                            65972i32;
                                                        self.help_line
                                                            [crate::ix::U((0i32) as usize)] =
                                                            65973i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                            {
                                                let __v234 = self.mem
                                                    [crate::ix::U((temp_head) as usize)]
                                                .hh()
                                                .rh();
                                                self.pstack[crate::ix::U((n) as usize)] = __v234;
                                            }
                                            self.align_state =
                                                (self.align_state).wrapping_sub(unbalance);
                                            {
                                                let __for_end_11 = n;
                                                m = 0i32;
                                                while m <= __for_end_11 {
                                                    self.flush_list(
                                                        self.pstack[crate::ix::U((m) as usize)],
                                                    );
                                                    m = m.wrapping_add(1);
                                                }
                                            }
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                // §426
                                if (self.cur_tok < right_brace_limit) {
                                    if (self.cur_tok < left_brace_limit) {
                                        // §433
                                        {
                                            'l_done1_f: {
                                                unbalance = 1i32;
                                                while true {
                                                    {
                                                        {
                                                            {
                                                                q = self.avail;
                                                                if (q
                                                                    == (268435455i32)
                                                                        .wrapping_neg())
                                                                {
                                                                    q = self.get_avail();
                                                                } else {
                                                                    {
                                                                        self.avail = self.mem
                                                                            [crate::ix::U(
                                                                                (q) as usize,
                                                                            )]
                                                                        .hh()
                                                                        .rh();
                                                                        self.mem[crate::ix::U(
                                                                            (q) as usize,
                                                                        )]
                                                                        .set_hh_rh(
                                                                            (268435455i32)
                                                                                .wrapping_neg(),
                                                                        );
                                                                        self.dyn_used = (self
                                                                            .dyn_used)
                                                                            .wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                            self.mem[crate::ix::U((p) as usize)]
                                                                .set_hh_rh(q);
                                                            {
                                                                let __v235 = self.cur_tok;
                                                                self.mem
                                                                    [crate::ix::U((q) as usize)]
                                                                .set_hh_lh(__v235);
                                                            }
                                                            p = q;
                                                        }
                                                        self.get_token();
                                                        if (self.cur_tok == self.par_token) {
                                                            if (self.long_state != long_call) {
                                                                // §430
                                                                {
                                                                    if (self.long_state == call) {
                                                                        {
                                                                            self.runaway();
                                                                            {
                                                                                if (self.interaction == error_stop_mode) {
                                                                                }
                                                                                if self.file_line_error_style_p {
                                                                                    self.print_file_line();
                                                                                } else {
                                                                                    self.print_nl(65544i32);
                                                                                }
                                                                                self.print(
                                                                                    65969i32,
                                                                                );
                                                                            }
                                                                            self.sprint_cs(
                                                                                self.warning_index,
                                                                            );
                                                                            self.print(65970i32);
                                                                            {
                                                                                self.help_ptr =
                                                                                    3i32;
                                                                                self.help_line[crate::ix::U((2i32) as usize)] = 65971i32;
                                                                                self.help_line[crate::ix::U((1i32) as usize)] = 65972i32;
                                                                                self.help_line[crate::ix::U((0i32) as usize)] = 65973i32;
                                                                            }
                                                                            self.back_error();
                                                                        }
                                                                    }
                                                                    {
                                                                        let __v236 = self.mem
                                                                            [crate::ix::U(
                                                                                (temp_head)
                                                                                    as usize,
                                                                            )]
                                                                        .hh()
                                                                        .rh();
                                                                        self.pstack[crate::ix::U(
                                                                            (n) as usize,
                                                                        )] = __v236;
                                                                    }
                                                                    self.align_state = (self
                                                                        .align_state)
                                                                        .wrapping_sub(unbalance);
                                                                    {
                                                                        let __for_end_17 = n;
                                                                        m = 0i32;
                                                                        while m <= __for_end_17 {
                                                                            self.flush_list(self.pstack[crate::ix::U((m) as usize)]);
                                                                            m = m.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    break 'l_exit_f;
                                                                }
                                                            }
                                                        }
                                                        // §433
                                                        if (self.cur_tok < right_brace_limit) {
                                                            if (self.cur_tok < left_brace_limit) {
                                                                unbalance =
                                                                    (unbalance).wrapping_add(1i32);
                                                            } else {
                                                                {
                                                                    unbalance = (unbalance)
                                                                        .wrapping_sub(1i32);
                                                                    if (unbalance == 0i32) {
                                                                        break 'l_done1_f;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            rbrace_ptr = p;
                                            {
                                                q = self.get_avail();
                                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                {
                                                    let __v237 = self.cur_tok;
                                                    self.mem[crate::ix::U((q) as usize)]
                                                        .set_hh_lh(__v237);
                                                }
                                                p = q;
                                            }
                                        }
                                    } else {
                                        // §429
                                        {
                                            self.back_input();
                                            {
                                                if (self.interaction == error_stop_mode) {}
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(65961i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(65962i32);
                                            {
                                                self.help_ptr = 6i32;
                                                self.help_line[crate::ix::U((5i32) as usize)] =
                                                    65963i32;
                                                self.help_line[crate::ix::U((4i32) as usize)] =
                                                    65964i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] =
                                                    65965i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] =
                                                    65966i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] =
                                                    65967i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] =
                                                    65968i32;
                                            }
                                            self.align_state =
                                                (self.align_state).wrapping_add(1i32);
                                            self.long_state = call;
                                            self.cur_tok = self.par_token;
                                            self.ins_error();
                                            {
                                                __goto_1 = 1;
                                                continue 'l_dispatch_1;
                                            }
                                        }
                                    }
                                } else {
                                    // §427
                                    {
                                        if (self.cur_tok == space_token) {
                                            if (self.mem[crate::ix::U((r) as usize)].hh().lh()
                                                <= end_match_token)
                                            {
                                                if (self.mem[crate::ix::U((r) as usize)].hh().lh()
                                                    >= match_token)
                                                {
                                                    {
                                                        __goto_1 = 1;
                                                        continue 'l_dispatch_1;
                                                    }
                                                }
                                            }
                                        }
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            {
                                                let __v238 = self.cur_tok;
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_lh(__v238);
                                            }
                                            p = q;
                                        }
                                    }
                                }
                                // §426
                                m = (m).wrapping_add(1i32);
                                if (self.mem[crate::ix::U((r) as usize)].hh().lh()
                                    > end_match_token)
                                {
                                    {
                                        __goto_1 = 1;
                                        continue 'l_dispatch_1;
                                    }
                                }
                                if (self.mem[crate::ix::U((r) as usize)].hh().lh() < match_token) {
                                    {
                                        __goto_1 = 1;
                                        continue 'l_dispatch_1;
                                    }
                                }
                            }
                            if __goto_1 <= 2 {
                                // found
                                if (s != (268435455i32).wrapping_neg()) {
                                    // §434
                                    {
                                        if ((m == 1i32)
                                            && (self.mem[crate::ix::U((p) as usize)].hh().lh()
                                                < right_brace_limit))
                                        {
                                            {
                                                self.mem[crate::ix::U((rbrace_ptr) as usize)]
                                                    .set_hh_rh((268435455i32).wrapping_neg());
                                                {
                                                    {
                                                        let __v239 = self.avail;
                                                        self.mem[crate::ix::U((p) as usize)]
                                                            .set_hh_rh(__v239);
                                                    }
                                                    self.avail = p;
                                                    self.dyn_used =
                                                        (self.dyn_used).wrapping_sub(1i32);
                                                }
                                                p = self.mem[crate::ix::U((temp_head) as usize)]
                                                    .hh()
                                                    .rh();
                                                {
                                                    let __v240 = self.mem
                                                        [crate::ix::U((p) as usize)]
                                                    .hh()
                                                    .rh();
                                                    self.pstack[crate::ix::U((n) as usize)] =
                                                        __v240;
                                                }
                                                {
                                                    {
                                                        let __v241 = self.avail;
                                                        self.mem[crate::ix::U((p) as usize)]
                                                            .set_hh_rh(__v241);
                                                    }
                                                    self.avail = p;
                                                    self.dyn_used =
                                                        (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                        } else {
                                            {
                                                let __v242 = self.mem
                                                    [crate::ix::U((temp_head) as usize)]
                                                .hh()
                                                .rh();
                                                self.pstack[crate::ix::U((n) as usize)] = __v242;
                                            }
                                        }
                                        n = (n).wrapping_add(1i32);
                                        if (self.eqtb[crate::ix::U(((7892294i32) - 1) as usize)]
                                            .int()
                                            > 0i32)
                                        {
                                            if ((self.eqtb
                                                [crate::ix::U(((7892322i32) - 1) as usize)]
                                            .int()
                                                == 0i32)
                                                || (self.input_ptr
                                                    < self.eqtb[crate::ix::U(
                                                        ((7892322i32) - 1) as usize,
                                                    )]
                                                    .int()))
                                            {
                                                {
                                                    self.begin_diagnostic();
                                                    self.print_nl(match_chr);
                                                    self.print_int(n);
                                                    self.print(65980i32);
                                                    self.show_token_list(
                                                        self.pstack[crate::ix::U(
                                                            ((n).wrapping_sub(1i32)) as usize,
                                                        )],
                                                        (268435455i32).wrapping_neg(),
                                                        1000i32,
                                                    );
                                                    self.end_diagnostic(false);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            break 'l_dispatch_1;
                        }
                        if (self.mem[crate::ix::U((r) as usize)].hh().lh() == end_match_token) {
                            break;
                        }
                    }
                }
            }
            // §424
            while (((self.cur_input.loc_field == (268435455i32).wrapping_neg())
                && (self.cur_input.index_field != v_template))
                && (self.cur_input.index_field != output_text))
            {
                self.end_token_list();
            }
            self.begin_token_list(ref_count, macro_);
            self.cur_input.name_field = self.warning_index;
            self.cur_input.loc_field = self.mem[crate::ix::U((r) as usize)].hh().rh();
            if (n > 0i32) {
                {
                    if ((self.param_ptr).wrapping_add(n) > self.max_param_stack) {
                        {
                            self.max_param_stack = (self.param_ptr).wrapping_add(n);
                            if (self.max_param_stack > param_size) {
                                self.overflow(65960i32, param_size);
                            }
                        }
                    }
                    {
                        let __for_end_5 = (n).wrapping_sub(1i32);
                        m = 0i32;
                        while m <= __for_end_5 {
                            {
                                let __ix243 = (self.param_ptr).wrapping_add(m);
                                let __v244 = self.pstack[crate::ix::U((m) as usize)];
                                self.param_stack[crate::ix::U((__ix243) as usize)] = __v244;
                            }
                            m = m.wrapping_add(1);
                        }
                    }
                    self.param_ptr = (self.param_ptr).wrapping_add(n);
                }
            }
        }
        // §423
        self.scanner_status = save_scanner_status;
        self.warning_index = save_warning_index;
    }

    /// Sometimes the expansion looks too far ahead, so we want to insert
    /// a harmless \.{\\relax} into the user's input.
    /// @<Declare the procedure called `insert_relax`
    // §413
    pub fn insert_relax(&mut self) {
        self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
        self.back_input();
        self.cur_tok = 34749088i32;
        self.back_input();
        self.cur_input.index_field = inserted;
    }

    /// There are eight almost identical doubly linked trees, one for the
    /// sparse array of the up to 32512 additional registers of each kind,
    /// one for inter-character token lists at specified class transitions, and
    /// one for the sparse array of the up to 32767 additional mark classes.
    /// The root of each such tree, if it exists, is an index node containing 64
    /// pointers to subtrees for $64^4$ consecutive array elements.  Similar index
    /// nodes are the starting points for all nonempty subtrees for $64^3$, $64^2$,
    /// and 64 consecutive array elements.  These four levels of index nodes are
    /// followed by a fifth level with nodes for the individual array elements.
    /// Each index node is 33 words long.  The pointers to the 64 possible
    /// subtrees or nodes are kept in the `info` and `link` fields of the last 32
    /// words.  (It would be both elegant and efficient to declare them as
    /// array, unfortunately \PASCAL\ doesn't allow this.)
    /// The fields in the first word of each index node and in the nodes for the
    /// ...
    // §1626
    pub fn new_index(&mut self, mut i: quarterword, mut q: halfword) {
        let mut k: small_number = 0; // §1626
        self.cur_ptr = self.get_node(index_node_size);
        {
            let __ix245 = self.cur_ptr;
            self.mem[crate::ix::U((__ix245) as usize)].set_hh_b0(i);
        }
        {
            let __ix246 = self.cur_ptr;
            self.mem[crate::ix::U((__ix246) as usize)].set_hh_b1(0i32);
        }
        {
            let __ix247 = self.cur_ptr;
            self.mem[crate::ix::U((__ix247) as usize)].set_hh_rh(q);
        }
        {
            let __for_end_2 = 32i32;
            k = 1i32;
            while k <= __for_end_2 {
                {
                    let __ix248 = (self.cur_ptr).wrapping_add(k);
                    let __v249 = self.sa_null;
                    self.mem[crate::ix::U((__ix248) as usize)] = __v249;
                }
                k = k.wrapping_add(1);
            }
        }
    }

    /// Given a type `t` and a twenty-four-bit number `n`, the `find_sa_element`
    /// procedure returns (in `cur_ptr`) a pointer to the node for the
    /// corresponding array element, or `null` when no such element exists.  The
    /// third parameter `w` is set `true` if the element must exist, e.g.,
    /// because it is about to be modified.  The procedure has two main
    /// branches:  one follows the existing tree structure, the other (only used
    /// when `w` is `true`) creates the missing nodes.
    /// We use macros to extract the six-bit pieces from a twenty-four-bit register
    /// number or mark class and to fetch or store one of the 64 pointers from
    /// an index node. (Note that the `hex_dig` macros are mis-named since the conversion
    /// from 4-bit to 6-bit fields for \XeTeX!)
    // §1630
    pub fn find_sa_element(&mut self, mut t: small_number, mut n: halfword, mut w: bool) {
        let mut q: halfword = 0; // §1630
        let mut i: small_number = 0; // §1630
        'l_exit_f: {
            'l_L49_f: {
                'l_L48_f: {
                    'l_L47_f: {
                        'l_not_found1_f: {
                            'l_not_found_f: {
                                self.cur_ptr = self.sa_root[crate::ix::U((t) as usize)];
                                {
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        if w {
                                            break 'l_not_found_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = (n / 262144i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .lh();
                                }
                                {
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        if w {
                                            break 'l_not_found1_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = ((n / 4096i32) % 64i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .lh();
                                }
                                {
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        if w {
                                            break 'l_L47_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = ((n / 64i32) % 64i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .lh();
                                }
                                {
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        if w {
                                            break 'l_L48_f;
                                        } else {
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                q = self.cur_ptr;
                                i = (n % 64i32);
                                if (((i) % 2) != 0) {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .rh();
                                } else {
                                    self.cur_ptr = self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .hh()
                                    .lh();
                                }
                                if ((self.cur_ptr == (268435455i32).wrapping_neg()) && w) {
                                    break 'l_L49_f;
                                }
                                break 'l_exit_f;
                            }
                            self.new_index(t, (268435455i32).wrapping_neg());
                            {
                                let __v250 = self.cur_ptr;
                                self.sa_root[crate::ix::U((t) as usize)] = __v250;
                            }
                            q = self.cur_ptr;
                            i = (n / 262144i32);
                        }
                        self.new_index(i, q);
                        {
                            if (((i) % 2) != 0) {
                                {
                                    let __v251 = self.cur_ptr;
                                    self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .set_hh_rh(__v251);
                                }
                            } else {
                                {
                                    let __v252 = self.cur_ptr;
                                    self.mem[crate::ix::U(
                                        (((q).wrapping_add((i / 2i32))).wrapping_add(1i32))
                                            as usize,
                                    )]
                                    .set_hh_lh(__v252);
                                }
                            }
                            {
                                let __v253 = (self.mem[crate::ix::U((q) as usize)].hh().b1())
                                    .wrapping_add(1i32);
                                self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v253);
                            }
                        }
                        q = self.cur_ptr;
                        i = ((n / 4096i32) % 64i32);
                    }
                    self.new_index(i, q);
                    {
                        if (((i) % 2) != 0) {
                            {
                                let __v254 = self.cur_ptr;
                                self.mem[crate::ix::U(
                                    (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                                )]
                                .set_hh_rh(__v254);
                            }
                        } else {
                            {
                                let __v255 = self.cur_ptr;
                                self.mem[crate::ix::U(
                                    (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                                )]
                                .set_hh_lh(__v255);
                            }
                        }
                        {
                            let __v256 =
                                (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32);
                            self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v256);
                        }
                    }
                    q = self.cur_ptr;
                    i = ((n / 64i32) % 64i32);
                }
                self.new_index(i, q);
                {
                    if (((i) % 2) != 0) {
                        {
                            let __v257 = self.cur_ptr;
                            self.mem[crate::ix::U(
                                (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                            )]
                            .set_hh_rh(__v257);
                        }
                    } else {
                        {
                            let __v258 = self.cur_ptr;
                            self.mem[crate::ix::U(
                                (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                            )]
                            .set_hh_lh(__v258);
                        }
                    }
                    {
                        let __v259 =
                            (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v259);
                    }
                }
                q = self.cur_ptr;
                i = (n % 64i32);
            }
            if (t == mark_val) {
                // §1631
                {
                    self.cur_ptr = self.get_node(mark_class_node_size);
                    {
                        let __ix260 = (self.cur_ptr).wrapping_add(1i32);
                        let __v261 = self.sa_null;
                        self.mem[crate::ix::U((__ix260) as usize)] = __v261;
                    }
                    {
                        let __ix262 = (self.cur_ptr).wrapping_add(2i32);
                        let __v263 = self.sa_null;
                        self.mem[crate::ix::U((__ix262) as usize)] = __v263;
                    }
                    {
                        let __ix264 = (self.cur_ptr).wrapping_add(3i32);
                        let __v265 = self.sa_null;
                        self.mem[crate::ix::U((__ix264) as usize)] = __v265;
                    }
                }
            } else {
                {
                    if (t <= dimen_val) {
                        {
                            self.cur_ptr = self.get_node(word_node_size);
                            {
                                let __ix266 = (self.cur_ptr).wrapping_add(2i32);
                                self.mem[crate::ix::U((__ix266) as usize)].set_int(0i32);
                            }
                            {
                                let __ix267 = (self.cur_ptr).wrapping_add(1i32);
                                self.mem[crate::ix::U((__ix267) as usize)].set_hh_rh(n);
                            }
                        }
                    } else {
                        {
                            self.cur_ptr = self.get_node(pointer_node_size);
                            if (t <= mu_val) {
                                {
                                    {
                                        let __ix268 = (self.cur_ptr).wrapping_add(1i32);
                                        self.mem[crate::ix::U((__ix268) as usize)]
                                            .set_hh_rh(zero_glue);
                                    }
                                    {
                                        let __v269 = (self.mem[crate::ix::U((zero_glue) as usize)]
                                            .hh()
                                            .rh())
                                        .wrapping_add(1i32);
                                        self.mem[crate::ix::U((zero_glue) as usize)]
                                            .set_hh_rh(__v269);
                                    }
                                }
                            } else {
                                {
                                    let __ix270 = (self.cur_ptr).wrapping_add(1i32);
                                    self.mem[crate::ix::U((__ix270) as usize)]
                                        .set_hh_rh((268435455i32).wrapping_neg());
                                }
                            }
                        }
                    }
                    {
                        let __ix271 = (self.cur_ptr).wrapping_add(1i32);
                        self.mem[crate::ix::U((__ix271) as usize)]
                            .set_hh_lh((268435455i32).wrapping_neg());
                    }
                }
            }
            {
                let __ix272 = self.cur_ptr;
                self.mem[crate::ix::U((__ix272) as usize)]
                    .set_hh_b0(((64i32).wrapping_mul(t)).wrapping_add(i));
            }
            {
                let __ix273 = self.cur_ptr;
                self.mem[crate::ix::U((__ix273) as usize)].set_hh_b1(level_one);
            }
            // §1630
            {
                let __ix274 = self.cur_ptr;
                self.mem[crate::ix::U((__ix274) as usize)].set_hh_rh(q);
            }
            {
                if (((i) % 2) != 0) {
                    {
                        let __v275 = self.cur_ptr;
                        self.mem[crate::ix::U(
                            (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                        )]
                        .set_hh_rh(__v275);
                    }
                } else {
                    {
                        let __v276 = self.cur_ptr;
                        self.mem[crate::ix::U(
                            (((q).wrapping_add((i / 2i32))).wrapping_add(1i32)) as usize,
                        )]
                        .set_hh_lh(__v276);
                    }
                }
                {
                    let __v277 =
                        (self.mem[crate::ix::U((q) as usize)].hh().b1()).wrapping_add(1i32);
                    self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v277);
                }
            }
        }
    }

    /// \[25] Expanding the next token.
    /// Only a dozen or so command codes `>max_command` can possibly be returned by
    /// `get_next`; in increasing order, they are `undefined_cs`, `expand_after`,
    /// `no_expand`, `input`, `if_test`, `fi_or_else`, `cs_name`, `convert`, `the`,
    /// `top_bot_mark`, `call`, `long_call`, `outer_call`, `long_outer_call`, and
    /// `end_template`.{\emergencystretch=40pt\par}
    /// The `expand` subroutine is used when `cur_cmd>max_command`. It removes a
    /// ``call'' or a conditional or one of the other special operations just
    /// listed.  It follows that `expand` might invoke itself recursively. In all
    /// cases, `expand` destroys the current token, but it sets things up so that
    /// the next `get_next` will deliver the appropriate next token. The value of
    /// `cur_tok` need not be known when `expand` is called.
    /// Since several of the basic scanning routines communicate via global variables,
    /// their values are saved as local variables of `expand` so that
    /// ...
    // §396
    pub fn expand(&mut self) {
        let mut t: halfword = 0; // §396
        let mut b: bool = false; // §396
        let mut p: halfword = 0; // §396
        let mut q: halfword = 0; // §396
        let mut r: halfword = 0; // §396
        let mut j: i32 = 0; // §396
        let mut cv_backup: i32 = 0; // §396
        let mut cvl_backup: small_number = 0; // §396
        let mut radix_backup: small_number = 0; // §396
        let mut co_backup: small_number = 0; // §396
        let mut backup_backup: halfword = 0; // §396
        let mut save_scanner_status: small_number = 0; // §396
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(65942i32, self.expand_depth);
        }
        cv_backup = self.cur_val;
        cvl_backup = self.cur_val_level;
        radix_backup = self.radix;
        co_backup = self.cur_order;
        backup_backup = self.mem[crate::ix::U((backup_head) as usize)].hh().rh();
        'l_reswitch_b: loop {
            if (self.cur_cmd < call) {
                // §399
                {
                    if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() > 1i32) {
                        self.show_cur_cmd_chr();
                    }
                    match self.cur_cmd {
                        top_bot_mark => {
                            // §420
                            {
                                t = (self.cur_chr % marks_code);
                                if (self.cur_chr >= marks_code) {
                                    self.scan_register_num();
                                } else {
                                    self.cur_val = 0i32;
                                }
                                if (self.cur_val == 0i32) {
                                    self.cur_ptr = self.cur_mark[crate::ix::U((t) as usize)];
                                } else {
                                    // §1635
                                    {
                                        self.find_sa_element(mark_val, self.cur_val, false);
                                        if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                            if (((t) % 2) != 0) {
                                                self.cur_ptr = self.mem[crate::ix::U(
                                                    (((self.cur_ptr).wrapping_add((t / 2i32)))
                                                        .wrapping_add(1i32))
                                                        as usize,
                                                )]
                                                .hh()
                                                .rh();
                                            } else {
                                                self.cur_ptr = self.mem[crate::ix::U(
                                                    (((self.cur_ptr).wrapping_add((t / 2i32)))
                                                        .wrapping_add(1i32))
                                                        as usize,
                                                )]
                                                .hh()
                                                .lh();
                                            }
                                        }
                                    }
                                }
                                // §420
                                if (self.cur_ptr != (268435455i32).wrapping_neg()) {
                                    self.begin_token_list(self.cur_ptr, mark_text);
                                }
                            }
                        }
                        expand_after => {
                            // §399
                            if (self.cur_chr == 0i32) {
                                // §400
                                {
                                    self.get_token();
                                    t = self.cur_tok;
                                    self.get_token();
                                    if (self.cur_cmd > max_command) {
                                        self.expand();
                                    } else {
                                        self.back_input();
                                    }
                                    self.cur_tok = t;
                                    self.back_input();
                                }
                            } else {
                                // §1576
                                {
                                    self.get_token();
                                    if ((self.cur_cmd == if_test) && (self.cur_chr != if_case_code))
                                    {
                                        {
                                            self.cur_chr = (self.cur_chr).wrapping_add(32i32);
                                            continue 'l_reswitch_b;
                                        }
                                    }
                                    {
                                        if (self.interaction == error_stop_mode) {}
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66025i32);
                                    }
                                    self.print_esc(66155i32);
                                    self.print(66928i32);
                                    self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                                    self.print_char(39i32);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 65937i32;
                                    }
                                    self.back_error();
                                }
                            }
                        }
                        no_expand => {
                            // §399
                            if (self.cur_chr == 0i32) {
                                // §401
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = normal;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    t = self.cur_tok;
                                    self.back_input();
                                    if ((t >= cs_token_flag) && (t != end_write_token)) {
                                        {
                                            p = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)]
                                                .set_hh_lh(34749090i32);
                                            {
                                                let __v278 = self.cur_input.loc_field;
                                                self.mem[crate::ix::U((p) as usize)]
                                                    .set_hh_rh(__v278);
                                            }
                                            self.cur_input.start_field = p;
                                            self.cur_input.loc_field = p;
                                        }
                                    }
                                }
                            } else {
                                // §402
                                {
                                    save_scanner_status = self.scanner_status;
                                    self.scanner_status = normal;
                                    self.get_token();
                                    self.scanner_status = save_scanner_status;
                                    if (self.cur_cs < hash_base) {
                                        self.cur_cs = self
                                            .prim_lookup((self.cur_cs).wrapping_sub(1114113i32));
                                    } else {
                                        self.cur_cs = self.prim_lookup(
                                            self.hash
                                                [crate::ix::U(((self.cur_cs) - 1179650) as usize)]
                                            .rh(),
                                        );
                                    }
                                    if (self.cur_cs != undefined_primitive) {
                                        {
                                            t = self.eqtb[crate::ix::U(
                                                (((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1)
                                                    as usize,
                                            )]
                                            .hh()
                                            .b0();
                                            if (t > max_command) {
                                                {
                                                    self.cur_cmd = t;
                                                    self.cur_chr = self.eqtb[crate::ix::U(
                                                        (((prim_eqtb_base)
                                                            .wrapping_add(self.cur_cs))
                                                            - 1)
                                                            as usize,
                                                    )]
                                                    .hh()
                                                    .rh();
                                                    self.cur_tok = ((self.cur_cmd)
                                                        .wrapping_mul(max_char_val))
                                                    .wrapping_add(self.cur_chr);
                                                    self.cur_cs = 0i32;
                                                    continue 'l_reswitch_b;
                                                }
                                            } else {
                                                {
                                                    self.back_input();
                                                    p = self.get_avail();
                                                    self.mem[crate::ix::U((p) as usize)]
                                                        .set_hh_lh(34749092i32);
                                                    {
                                                        let __v279 = self.cur_input.loc_field;
                                                        self.mem[crate::ix::U((p) as usize)]
                                                            .set_hh_rh(__v279);
                                                    }
                                                    self.cur_input.loc_field = p;
                                                    self.cur_input.start_field = p;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        cs_name => {
                            // §406
                            {
                                r = self.get_avail();
                                p = r;
                                b = self.is_in_csname;
                                self.is_in_csname = true;
                                loop {
                                    self.get_x_token();
                                    if (self.cur_cs == 0i32) {
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            {
                                                let __v280 = self.cur_tok;
                                                self.mem[crate::ix::U((q) as usize)]
                                                    .set_hh_lh(__v280);
                                            }
                                            p = q;
                                        }
                                    }
                                    if (self.cur_cs != 0i32) {
                                        break;
                                    }
                                }
                                if (self.cur_cmd != end_cs_name) {
                                    // §407
                                    {
                                        {
                                            if (self.interaction == error_stop_mode) {}
                                            if self.file_line_error_style_p {
                                                self.print_file_line();
                                            } else {
                                                self.print_nl(65544i32);
                                            }
                                            self.print(65949i32);
                                        }
                                        self.print_esc(65810i32);
                                        self.print(65950i32);
                                        {
                                            self.help_ptr = 2i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] =
                                                65951i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] =
                                                65952i32;
                                        }
                                        self.back_error();
                                    }
                                }
                                // §406
                                self.is_in_csname = b;
                                // §408
                                j = self.first;
                                p = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                while (p != (268435455i32).wrapping_neg()) {
                                    {
                                        if (j >= self.max_buf_stack) {
                                            {
                                                self.max_buf_stack = (j).wrapping_add(1i32);
                                                if (self.max_buf_stack == buf_size) {
                                                    self.overflow(65538i32, buf_size);
                                                }
                                            }
                                        }
                                        {
                                            let __v281 =
                                                (self.mem[crate::ix::U((p) as usize)].hh().lh()
                                                    % max_char_val);
                                            self.buffer[crate::ix::U((j) as usize)] = __v281;
                                        }
                                        j = (j).wrapping_add(1i32);
                                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    }
                                }
                                if ((j > (self.first).wrapping_add(1i32))
                                    || (self.buffer[crate::ix::U((self.first) as usize)]
                                        > 65535i32))
                                {
                                    {
                                        self.no_new_control_sequence = false;
                                        self.cur_cs = self
                                            .id_lookup(self.first, (j).wrapping_sub(self.first));
                                        self.no_new_control_sequence = true;
                                    }
                                } else {
                                    if (j == self.first) {
                                        self.cur_cs = null_cs;
                                    } else {
                                        self.cur_cs = (single_base).wrapping_add(
                                            self.buffer[crate::ix::U((self.first) as usize)],
                                        );
                                    }
                                }
                                // §406
                                self.flush_list(r);
                                if (self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)]
                                    .hh()
                                    .b0()
                                    == undefined_cs)
                                {
                                    {
                                        self.eq_define(self.cur_cs, relax, too_big_usv);
                                    }
                                }
                                self.cur_tok = (self.cur_cs).wrapping_add(33554431i32);
                                self.back_input();
                            }
                        }
                        convert => {
                            // §399
                            self.conv_toks();
                        }
                        the => {
                            self.ins_the_toks();
                        }
                        if_test => {
                            self.conditional();
                        }
                        fi_or_else => {
                            // §545
                            {
                                if (self.eqtb[crate::ix::U(((7892327i32) - 1) as usize)].int()
                                    > 0i32)
                                {
                                    if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int()
                                        <= 1i32)
                                    {
                                        self.show_cur_cmd_chr();
                                    }
                                }
                                if (self.cur_chr > self.if_limit) {
                                    if (self.if_limit == if_code) {
                                        self.insert_relax();
                                    } else {
                                        {
                                            {
                                                if (self.interaction == error_stop_mode) {}
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(66159i32);
                                            }
                                            self.print_cmd_chr(fi_or_else, self.cur_chr);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] =
                                                    66160i32;
                                            }
                                            self.error();
                                        }
                                    }
                                } else {
                                    {
                                        while (self.cur_chr != fi_code) {
                                            self.pass_text();
                                        }
                                        // §531
                                        {
                                            if (self.if_stack
                                                [crate::ix::U((self.in_open) as usize)]
                                                == self.cond_ptr)
                                            {
                                                self.if_warning();
                                            }
                                            p = self.cond_ptr;
                                            self.if_line = self.mem
                                                [crate::ix::U(((p).wrapping_add(1i32)) as usize)]
                                            .int();
                                            self.cur_if =
                                                self.mem[crate::ix::U((p) as usize)].hh().b1();
                                            self.if_limit =
                                                self.mem[crate::ix::U((p) as usize)].hh().b0();
                                            self.cond_ptr =
                                                self.mem[crate::ix::U((p) as usize)].hh().rh();
                                            self.free_node(p, if_node_size);
                                        }
                                    }
                                }
                            }
                        }
                        input => {
                            // §412
                            if (self.cur_chr == 1i32) {
                                self.force_eof = true;
                            } else {
                                // §1560
                                if (self.cur_chr == 2i32) {
                                    self.pseudo_start();
                                } else {
                                    // §412
                                    if self.name_in_progress {
                                        self.insert_relax();
                                    } else {
                                        self.start_input();
                                    }
                                }
                            }
                        }
                        _ => {
                            // §404
                            {
                                {
                                    if (self.interaction == error_stop_mode) {}
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65943i32);
                                }
                                {
                                    self.help_ptr = 5i32;
                                    self.help_line[crate::ix::U((4i32) as usize)] = 65944i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 65945i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 65946i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 65947i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 65948i32;
                                }
                                self.error();
                            }
                        }
                    }
                }
            } else {
                // §396
                if (self.cur_cmd < end_template) {
                    self.macro_call();
                } else {
                    // §409
                    {
                        self.cur_tok = 34749087i32;
                        self.back_input();
                    }
                }
            }
            // §396
            self.cur_val = cv_backup;
            self.cur_val_level = cvl_backup;
            self.radix = radix_backup;
            self.cur_order = co_backup;
            self.mem[crate::ix::U((backup_head) as usize)].set_hh_rh(backup_backup);
            self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
            break 'l_reswitch_b;
        }
    }

    /// Here is a recursive procedure that is \TeX's usual way to get the
    /// next token of input. It has been slightly optimized to take account of
    /// common cases.
    // §414
    pub fn get_x_token(&mut self) {
        // goto labels: restart, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.get_next();
                if (self.cur_cmd <= max_command) {
                    {
                        __goto_1 = 1;
                        continue 'l_dispatch_1;
                    }
                }
                if (self.cur_cmd >= call) {
                    if (self.cur_cmd < end_template) {
                        self.macro_call();
                    } else {
                        {
                            self.cur_cs = frozen_endv;
                            self.cur_cmd = endv;
                            {
                                __goto_1 = 1;
                                continue 'l_dispatch_1;
                            }
                        }
                    }
                } else {
                    self.expand();
                }
                {
                    __goto_1 = 0;
                    continue 'l_dispatch_1;
                }
            }
            if __goto_1 <= 1 {
                // done
                if (self.cur_cs == 0i32) {
                    self.cur_tok =
                        ((self.cur_cmd).wrapping_mul(max_char_val)).wrapping_add(self.cur_chr);
                } else {
                    self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                }
            }
            break 'l_dispatch_1;
        }
    }

    /// The `get_x_token` procedure is essentially equivalent to two consecutive
    /// procedure calls: `get_next; x_token`.
    // §415
    pub fn x_token(&mut self) {
        while (self.cur_cmd > max_command) {
            {
                self.expand();
                self.get_next();
            }
        }
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(max_char_val)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
        }
    }

    /// The `scan_left_brace` routine is called when a left brace is supposed to be
    /// the next non-blank token. (The term ``left brace'' means, more precisely,
    /// a character whose catcode is `left_brace`.) \TeX\ allows \.{\\relax} to
    /// appear before the `left_brace`.
    // §437
    pub fn scan_left_brace(&mut self) {
        // §438
        loop {
            self.get_x_token();
            if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) {
                break;
            }
        }
        // §437
        if (self.cur_cmd != left_brace) {
            {
                {
                    if (self.interaction == error_stop_mode) {}
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65981i32);
                }
                {
                    self.help_ptr = 4i32;
                    self.help_line[crate::ix::U((3i32) as usize)] = 65982i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 65983i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 65984i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65985i32;
                }
                self.back_error();
                self.cur_tok = 2097275i32;
                self.cur_cmd = left_brace;
                self.cur_chr = 123i32;
                self.align_state = (self.align_state).wrapping_add(1i32);
            }
        }
    }

    /// The `scan_optional_equals` routine looks for an optional `\.=' sign preceded
    /// by optional spaces; `\.{\\relax}' is not ignored here.
    // §439
    pub fn scan_optional_equals(&mut self) {
        // §440
        loop {
            self.get_x_token();
            if (self.cur_cmd != spacer) {
                break;
            }
        }
        // §439
        if (self.cur_tok != 25165885i32) {
            self.back_input();
        }
    }
}
